//! clip: write-only clipboard push, from inside the ai-jail sandbox or outside it.
//!   printf 'hello' | clip        copy stdin
//!   clip --text "hello"          copy an argument
//!
//! No paste/read counterpart exists here or in clip-relay, by design: the jail may write to the
//! host clipboard, never read it. JAIL_CLIPBOARD_PASTE=1 on jail-run is the (bigger) grant for that.

use std::io::{Read, Write};
use std::os::unix::fs::FileTypeExt;
use std::path::PathBuf;
use std::process::{exit, Command, Stdio};

const CAP: usize = 65536; // 64 KiB
const NO_RELAY: &str =
    "clip: no relay running — start the session with JAIL_CLIPBOARD=1 jail-run claude";

fn repo_root() -> PathBuf {
    let top = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_string());
    top.map_or_else(
        || std::env::current_dir().unwrap_or_default(),
        PathBuf::from,
    )
}

fn on_path(tool: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(tool).is_file()))
}

fn env_set(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|v| !v.is_empty())
}

/// Feed `payload` to a child's stdin. A reader that quits early (timeout) is not our error.
fn feed(cmd: &mut Command, payload: &[u8]) -> bool {
    let Ok(mut child) = cmd.stdin(Stdio::piped()).spawn() else {
        return false;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(payload);
    }
    child.wait().is_ok_and(|s| s.success())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let payload: Vec<u8> = if args.first().map(String::as_str) == Some("--text") {
        let Some(text) = args.get(1) else {
            eprintln!("clip: --text needs an argument");
            exit(1);
        };
        text.clone().into_bytes()
    } else {
        let mut buf = Vec::new();
        let _ = std::io::stdin().read_to_end(&mut buf);
        buf
    };

    let size = payload.len();
    if size > CAP {
        eprintln!("clip: payload is {size} bytes, over the {CAP}-byte cap — not a file transfer, refusing");
        exit(1);
    }

    let fifo = repo_root().join(".tmp/clip.fifo");
    if std::fs::metadata(&fifo).is_ok_and(|m| m.file_type().is_fifo()) {
        // `timeout` bounds the wait for a reader: a dead relay would block the open forever.
        let mut cmd = Command::new("timeout");
        cmd.args(["2", "sh", "-c", "cat > \"$1\"", "_"]).arg(&fifo);
        if feed(&mut cmd, &payload) {
            println!("clip: copied ({size} bytes) via jail relay");
            return;
        }
        eprintln!("{NO_RELAY}");
        exit(1);
    }

    // No FIFO: not in a jailed session (or the relay already exited): try the host directly.
    let (tool, mut cmd) = if on_path("wl-copy") && env_set("WAYLAND_DISPLAY") {
        ("wl-copy", Command::new("wl-copy"))
    } else if on_path("xclip") && env_set("DISPLAY") {
        let mut c = Command::new("xclip");
        c.args(["-selection", "clipboard"]);
        ("xclip", c)
    } else if on_path("pbcopy") {
        ("pbcopy", Command::new("pbcopy"))
    } else {
        eprintln!("{NO_RELAY}");
        exit(1);
    };
    if !feed(&mut cmd, &payload) {
        exit(1);
    }
    println!("clip: copied ({size} bytes) via {tool}");
}
