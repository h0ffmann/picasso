#![allow(dead_code)]
//! Thin `git -C <dir>` helpers shared by the two submodule tools (`#[path]`-included by both).
//! The scripts ran under `set -e`: a failing git command ends the run with its own status.

use std::path::Path;
use std::process::{exit, Command, Stdio};

fn cmd(dir: &Path, args: &[&str]) -> Command {
    let mut c = Command::new("git");
    c.arg("-C").arg(dir).args(args);
    c
}

/// Run, output inherited; a failure exits with git's status.
pub fn run(dir: &Path, args: &[&str]) {
    match cmd(dir, args).status() {
        Ok(s) if s.success() => {}
        Ok(s) => exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("git: {e}");
            exit(127);
        }
    }
}

/// Run quietly, as `git ... >/dev/null 2>&1` in a condition: did it succeed?
pub fn ok(dir: &Path, args: &[&str]) -> bool {
    cmd(dir, args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// stdout, trimmed. `quiet` hides git's stderr (the `2>/dev/null` calls); a failure exits unless quiet,
/// where it returns None.
pub fn out(dir: &Path, args: &[&str], quiet: bool) -> Option<String> {
    let mut c = cmd(dir, args);
    if quiet {
        c.stderr(Stdio::null());
    }
    match c.output() {
        Ok(o) if o.status.success() => Some(String::from_utf8_lossy(&o.stdout).trim().to_string()),
        Ok(_) if quiet => None,
        Ok(o) => exit(o.status.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("git: {e}");
            exit(127);
        }
    }
}

pub fn get(dir: &Path, args: &[&str]) -> String {
    out(dir, args, false).unwrap_or_default()
}

/// The top level of the repository that holds `target`, if it is in one.
pub fn toplevel(target: &str) -> Option<String> {
    out(Path::new(target), &["rev-parse", "--show-toplevel"], true)
}
