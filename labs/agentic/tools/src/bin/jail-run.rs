//! jail-run: run a coding agent inside ai-jail from the current directory (which holds the
//! project's .ai-jail policy), with the host's GitHub token forwarded.

use std::ffi::CString;
use std::path::PathBuf;
use std::process::{exit, Child, Command, ExitStatus, Stdio};
use std::time::Duration;

const USAGE: &str = "jail-run — a coding agent inside ai-jail, from the current directory
  jail-run claude [args]        Claude Code; ~/.claude and ~/.claude.json mapped rw, ~/.ssh ro
  jail-run opencode [args]      OpenCode; its own ~/.config, ~/.local/share, ~/.cache dirs mapped rw
  jail-run ocr [args]           Open Code Review; a reviewer, not an author — NO GitHub token, no
                                agent state, the project read-only, one writable dir for output
  jail-run --dry-run -- <cmd>   print the sandbox invocation ai-jail would use, run nothing
  JAIL_DRY_RUN=1                same, for a mode above: print the sandbox line, start nothing
  JAIL_CLIPBOARD=1              write-only clipboard bridge — `clip` inside the jail reaches the host
  JAIL_CLIPBOARD_PASTE=1        real X11/Wayland display passthrough — the jail can READ your clipboard
  JAIL_OCR_OUT=<dir>            ocr's one writable directory (default: $RUNNER_TEMP, else a mktemp -d)
                                It is also the jail's HOME, so ~/.opencodereview lands there.
The GitHub token is resolved on the host by gh-token and forwarded as GH_TOKEN for claude and
opencode: ~/.config/gh is never mapped in, so a login inside the jail would die with it. The ocr
mode never asks for it — OCR_LLM_URL / OCR_LLM_MODEL / OCR_LLM_TOKEN / OCR_USE_ANTHROPIC are the
only variables it gets, and only when they are set on the host.
";

const FIFO: &str = ".tmp/clip.fifo";

extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
    fn kill(pid: i32, sig: i32) -> i32;
    fn mkfifo(path: *const std::ffi::c_char, mode: u32) -> i32;
    fn getuid() -> u32;
}

extern "C" fn on_signal(_sig: i32) {}

fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

fn home_path(rel: &str) -> String {
    format!("{}/{rel}", std::env::var("HOME").unwrap_or_default())
}

/// The EXIT trap: stop the relay, remove the FIFO.
fn cleanup(relay: &mut Option<Child>) {
    if let Some(mut r) = relay.take() {
        // SAFETY: plain kill(2) on a child we spawned and have not reaped.
        unsafe { kill(r.id() as i32, 15) };
        let _ = r.wait();
    }
    let _ = std::fs::remove_file(FIFO);
}

fn finish(relay: &mut Option<Child>, code: i32) -> ! {
    cleanup(relay);
    exit(code)
}

fn code_of(s: ExitStatus) -> i32 {
    use std::os::unix::process::ExitStatusExt;
    s.code().unwrap_or_else(|| 128 + s.signal().unwrap_or(0))
}

fn start_relay() -> Option<Child> {
    let _ = std::fs::create_dir_all(".tmp");
    let _ = std::fs::remove_file(FIFO);
    let path = CString::new(FIFO).expect("no NUL");
    // SAFETY: path is a valid C string.
    unsafe { mkfifo(path.as_ptr(), 0o666) };
    let mut relay = Command::new("clip-relay").arg(FIFO).spawn().ok()?;
    std::thread::sleep(Duration::from_millis(200));
    // A relay that already quit (no clipboard tool on this host) is not an error for the jail.
    if relay.try_wait().ok().flatten().is_some() {
        return None;
    }
    Some(relay)
}

fn ocr_out_dir() -> PathBuf {
    let dir = env_nonempty("JAIL_OCR_OUT")
        .or_else(|| env_nonempty("RUNNER_TEMP"))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let o = Command::new("mktemp").arg("-d").output().ok();
            let s = o.map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_string());
            PathBuf::from(s.unwrap_or_default())
        });
    let _ = std::fs::create_dir_all(&dir);
    // ai-jail mounts the path it is given; a symlink is not one.
    dir.canonicalize().unwrap_or(dir)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = args.first().map(String::as_str).unwrap_or("");
    match mode {
        "--help" | "-h" => {
            print!("{USAGE}");
            return;
        }
        "--dry-run" => {
            let mut rest = &args[1..];
            if rest.first().map(String::as_str) == Some("--") {
                rest = &rest[1..];
            }
            let st = Command::new("ai-jail")
                .args(["--no-save-config", "--dry-run", "--"])
                .args(rest)
                .status();
            exit(run_code(st));
        }
        "claude" | "opencode" | "ocr" => {}
        _ => {
            eprint!("{USAGE}");
            exit(2);
        }
    }
    let rest = &args[1..];

    let dry = std::env::var("JAIL_DRY_RUN").as_deref() == Ok("1");
    let mut relay: Option<Child> = None;
    // Bash traps INT/TERM and finishes the foreground job first; a no-op handler (reset by
    // exec, unlike SIG_IGN) gives the same: ai-jail gets the signal too, we clean up after it.
    // SAFETY: the handler does nothing.
    unsafe {
        for sig in [2, 15] {
            signal(sig, on_signal as extern "C" fn(i32) as usize);
        }
    }

    // The FIFO lives under the project's .tmp/: the project directory is what ai-jail maps into
    // the sandbox, so a path under $XDG_RUNTIME_DIR would be invisible from inside.
    if mode == "claude" && std::env::var("JAIL_CLIPBOARD").as_deref() == Ok("1") {
        relay = start_relay();
    }

    let mut ocr_out = PathBuf::new();
    let mut ocr_env: Vec<String> = Vec::new();
    if mode == "ocr" {
        ocr_out = ocr_out_dir();
        for v in [
            "OCR_LLM_URL",
            "OCR_LLM_MODEL",
            "OCR_LLM_TOKEN",
            "OCR_USE_ANTHROPIC",
        ] {
            if env_nonempty(v).is_some() {
                ocr_env.extend(["--env".into(), v.into()]);
            }
        }
        let pwd = std::env::current_dir().unwrap_or_default();
        eprintln!(
            "jail-run: ocr — {} read-only, {} writable (and HOME, so ~/.opencodereview is there), no GitHub token",
            pwd.display(),
            ocr_out.display()
        );
    }

    let mut paste: Vec<String> = Vec::new();
    if std::env::var("JAIL_CLIPBOARD_PASTE").as_deref() == Ok("1") {
        eprintln!("jail-run: JAIL_CLIPBOARD_PASTE=1 — real X11/Wayland display passthrough is on, the jail can read your clipboard (and, on X11, more)");
        // SAFETY: getuid cannot fail.
        let uid = unsafe { getuid() };
        paste = [
            "--display",
            "--env",
            "DISPLAY",
            "--env",
            "WAYLAND_DISPLAY",
            "--env",
        ]
        .map(String::from)
        .into_iter()
        .chain([format!("XDG_RUNTIME_DIR=/run/user/{uid}")])
        .collect();
    }

    let mut gh_token: Option<String> = None;
    if mode != "ocr" {
        let src = Command::new("gh-token")
            .arg("--source")
            .stderr(Stdio::inherit())
            .output();
        match src {
            Ok(o) if o.status.success() => {
                let src = String::from_utf8_lossy(&o.stdout).trim_end().to_string();
                let tok = Command::new("gh-token").stderr(Stdio::inherit()).output();
                match tok {
                    Ok(t) if t.status.success() => {
                        // $(...) strips trailing newlines
                        gh_token = Some(
                            String::from_utf8_lossy(&t.stdout)
                                .trim_end_matches('\n')
                                .to_string(),
                        );
                    }
                    Ok(t) => finish(&mut relay, code_of(t.status)),
                    Err(_) => finish(&mut relay, 127),
                }
                eprintln!(
                    "jail-run: GH_TOKEN from {src} — gh works inside the jail, no login needed"
                );
            }
            _ => {
                eprintln!("jail-run: no GitHub token (no GH_TOKEN in the environment, and gh is logged out on the host).");
                eprintln!("          gh will not work inside the jail. Fix once, on the host: gh auth login   (or export GH_TOKEN)");
            }
        }
    }

    let mut cmd = match mode {
        "claude" => {
            let mut c = Command::new("ai-jail");
            c.args(["--no-save-config", "--rw-map"])
                .arg(home_path(".claude"))
                .arg("--rw-map")
                .arg(home_path(".claude.json"))
                .arg("--map")
                .arg(home_path(".ssh"))
                .args(["--network", "--terminal-passthrough"]);
            if dry {
                c.arg("--dry-run");
            }
            c.args(["--exec", "--env", "GH_TOKEN"])
                .args(&paste)
                .arg("claude");
            c
        }
        "opencode" => {
            let mut c = Command::new("ai-jail");
            c.arg("--no-save-config");
            for d in [
                ".config/opencode",
                ".local/share/opencode",
                ".cache/opencode",
            ] {
                c.arg("--rw-map").arg(home_path(d));
            }
            c.args(["--network", "--terminal-passthrough"]);
            if dry {
                c.arg("--dry-run");
            }
            c.args(["--exec", "--env", "GH_TOKEN", "opencode"]);
            c
        }
        _ => {
            // A reviewer gets read access and one place to write, and nothing else. --clean is
            // load-bearing: without it the *reviewed* project's own .ai-jail is merged in, and
            // marola's asks for ~/.claude rw, so the repo under review would be choosing the
            // reviewer's mounts. That also drops its `mask` list, so the patterns are restated.
            // --map "$PWD" after ai-jail's own rw bind of the project is what makes it read-only.
            // --network is all or nothing in ai-jail 126a67e (--allow-tcp-port is refused and
            // --lockdown throws away --rw-map), so the jail reaches Ollama and, honestly,
            // everything else the host can reach.
            let mut c = Command::new("ai-jail");
            c.env_remove("GH_TOKEN").env_remove("GITHUB_TOKEN");
            c.args([
                "--no-save-config",
                "--clean",
                "--private-home",
                "--no-agent-state",
            ]);
            for m in [".env", ".env.*", "*.pem", "*.key"] {
                c.args(["--mask", m]);
            }
            c.arg("--map")
                .arg(std::env::current_dir().unwrap_or_default())
                .arg("--rw-map")
                .arg(&ocr_out)
                .arg("--env")
                .arg(format!("HOME={}", ocr_out.display()))
                .args(&ocr_env)
                .arg("--network");
            if dry {
                c.arg("--dry-run");
            }
            c.args(["--exec", "ocr"]);
            c
        }
    };
    cmd.args(rest);
    if let Some(t) = &gh_token {
        cmd.env("GH_TOKEN", t);
    }
    let code = run_code(cmd.status());
    finish(&mut relay, code)
}

fn run_code(st: std::io::Result<ExitStatus>) -> i32 {
    match st {
        Ok(s) => code_of(s),
        Err(e) => {
            eprintln!("jail-run: ai-jail: {e}");
            127
        }
    }
}
