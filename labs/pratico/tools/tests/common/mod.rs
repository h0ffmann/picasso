//! Shared helpers: a scratch directory, fake executables, and a way to run a built binary.
#![allow(dead_code)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

static N: AtomicU32 = AtomicU32::new(0);

pub struct Scratch(pub PathBuf);

impl Scratch {
    pub fn new() -> Self {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let d = std::env::temp_dir().join(format!("pratico-tools-{}-{n}", std::process::id()));
        std::fs::create_dir_all(d.join("bin")).unwrap();
        // physical path: jail-run resolves its directories with canonicalize
        Scratch(d.canonicalize().unwrap())
    }
    pub fn bin(&self) -> PathBuf {
        self.0.join("bin")
    }
    /// A fake executable `name` whose body is a /bin/sh script.
    pub fn fake(&self, name: &str, body: &str) {
        let p = self.bin().join(name);
        std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    /// PATH with the fakes first, then the directories of the given built binaries, then the real one.
    pub fn path(&self, built: &[&str]) -> String {
        let mut parts = vec![self.bin()];
        parts.extend(
            built
                .iter()
                .map(|b| Path::new(b).parent().unwrap().to_path_buf()),
        );
        parts.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        std::env::join_paths(parts)
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub fn run(bin: &str, dir: &Path, path: &str, env: &[(&str, &str)], args: &[&str]) -> Output {
    let mut c = Command::new(bin);
    c.current_dir(dir).env("PATH", path).args(args);
    for k in [
        "GH_TOKEN",
        "GITHUB_TOKEN",
        "JAIL_CLIPBOARD",
        "JAIL_CLIPBOARD_PASTE",
        "JAIL_DRY_RUN",
        "JAIL_OCR_OUT",
        "RUNNER_TEMP",
        "PRATICO_CLIP_SINK",
        "PRATICO_ROOT",
        "OCR_LLM_URL",
        "OCR_LLM_MODEL",
        "OCR_LLM_TOKEN",
        "OCR_USE_ANTHROPIC",
        "WAYLAND_DISPLAY",
        "DISPLAY",
    ] {
        c.env_remove(k);
    }
    for (k, v) in env {
        c.env(k, v);
    }
    c.output().unwrap()
}

pub fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

pub fn err(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}
