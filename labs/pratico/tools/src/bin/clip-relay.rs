//! clip-relay: host-side half of the write-only clipboard bridge for `just jail-claude` with
//! PRATICO_JAIL_CLIPBOARD=1. Reads what the jail writes into the FIFO and hands it to the host clipboard.

use std::ffi::CString;
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{exit, Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;

const CAP: u64 = 65536; // 64 KiB, independent of clip's own cap

extern "C" {
    fn signal(signum: i32, handler: usize) -> usize;
    fn kill(pid: i32, sig: i32) -> i32;
    fn unlink(path: *const std::ffi::c_char) -> i32;
    fn _exit(code: i32) -> !;
}

static FIFO: OnceLock<CString> = OnceLock::new();
static SINK_PID: AtomicI32 = AtomicI32::new(0);

/// EXIT/INT/TERM trap: stop the sink's process group, remove the FIFO, leave with 0. Only
/// async-signal-safe calls, since it runs in a signal handler.
extern "C" fn cleanup(_sig: i32) {
    let pid = SINK_PID.load(Ordering::SeqCst);
    // SAFETY: kill, unlink and _exit are async-signal-safe; the path outlives the process.
    unsafe {
        if pid > 0 {
            kill(-pid, 15);
        }
        if let Some(p) = FIFO.get() {
            unlink(p.as_ptr());
        }
        _exit(0)
    }
}

fn on_path(tool: &str) -> bool {
    std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(tool).is_file()))
}

fn env_set(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|v| !v.is_empty())
}

fn sink() -> Option<(&'static str, Command)> {
    if let Some(s) = std::env::var_os("PRATICO_CLIP_SINK").filter(|v| !v.is_empty()) {
        let mut c = Command::new("sh");
        c.arg("-c").arg(s);
        Some(("test sink (PRATICO_CLIP_SINK)", c))
    } else if on_path("wl-copy") && env_set("WAYLAND_DISPLAY") {
        Some(("wl-copy", Command::new("wl-copy")))
    } else if on_path("xclip") && env_set("DISPLAY") {
        let mut c = Command::new("xclip");
        c.args(["-selection", "clipboard"]);
        Some(("xclip", c))
    } else {
        None
    }
}

fn main() {
    let Some(fifo) = std::env::args().nth(1).filter(|a| !a.is_empty()) else {
        eprintln!("clip-relay: usage: clip-relay <fifo-path>");
        exit(1);
    };
    let Some((desc, _)) = sink() else {
        eprintln!("clip-relay: no clipboard tool/display found on the host — not starting");
        exit(1);
    };
    println!("clipboard relay: write-only, via {desc}, {fifo}");
    let _ = std::io::stdout().flush();

    let _ = FIFO.set(CString::new(fifo.clone()).expect("fifo path has a NUL"));
    // SAFETY: installs a handler that only calls async-signal-safe functions.
    unsafe {
        for sig in [2, 15] {
            signal(sig, cleanup as extern "C" fn(i32) as usize);
        }
    }

    loop {
        // Opening a FIFO blocks until a writer shows up; the read ends at EOF or at the cap.
        let mut data = Vec::new();
        match std::fs::File::open(&fifo) {
            Ok(f) => {
                let _ = f.take(CAP).read_to_end(&mut data);
            }
            Err(_) => cleanup(0), // the FIFO is gone: jail-run removed it, so stop
        }
        let Some((_, mut cmd)) = sink() else { continue };
        // Own process group, so the cleanup can take the sink down as a unit.
        let Ok(mut child) = cmd.process_group(0).stdin(Stdio::piped()).spawn() else {
            continue;
        };
        SINK_PID.store(child.id() as i32, Ordering::SeqCst);
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(&data);
        }
        let _ = child.wait();
        SINK_PID.store(0, Ordering::SeqCst);
    }
}
