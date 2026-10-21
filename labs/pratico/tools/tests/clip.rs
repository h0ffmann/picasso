//! clip and clip-relay: the cap, the argument forms, and the FIFO round trip with a test sink.
mod common;
use common::*;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const CLIP: &str = env!("CARGO_BIN_EXE_clip");
const RELAY: &str = env!("CARGO_BIN_EXE_clip-relay");

#[test]
fn text_needs_an_argument() {
    let s = Scratch::new();
    let o = run(CLIP, &s.0, &s.path(&[]), &[], &["--text"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("--text needs an argument"));
}

#[test]
fn over_the_cap_is_refused() {
    let s = Scratch::new();
    let big = "x".repeat(65537);
    let o = run(CLIP, &s.0, &s.path(&[]), &[], &["--text", &big]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("payload is 65537 bytes"));
}

#[test]
fn no_relay_and_no_tool_says_how_to_start_one() {
    let s = Scratch::new();
    // an empty PATH dir: no wl-copy, xclip or pbcopy, and no git (so the root is the cwd)
    let o = run(
        CLIP,
        &s.0,
        s.bin().to_str().unwrap(),
        &[],
        &["--text", "hi"],
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("PRATICO_JAIL_CLIPBOARD=1 just jail-claude"));
}

#[test]
fn host_tool_fallback_gets_the_payload() {
    let s = Scratch::new();
    let sink = s.0.join("got");
    s.fake("wl-copy", &format!("cat > {}", sink.display()));
    let o = run(
        CLIP,
        &s.0,
        &s.path(&[]),
        &[("WAYLAND_DISPLAY", "w0")],
        &["--text", "hello"],
    );
    assert!(o.status.success(), "{}", err(&o));
    assert!(out(&o).contains("copied (5 bytes) via wl-copy"));
    assert_eq!(std::fs::read_to_string(&sink).unwrap(), "hello");
}

#[test]
fn stdin_goes_through_the_relay_fifo_and_the_relay_cleans_up() {
    let s = Scratch::new();
    let fifo = s.0.join(".tmp/clip.fifo");
    std::fs::create_dir_all(s.0.join(".tmp")).unwrap();
    assert!(Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .unwrap()
        .success());
    let got = s.0.join("got");
    let sink = format!("cat > {}", got.display());
    let mut relay = Command::new(RELAY)
        .arg(&fifo)
        .env("PRATICO_CLIP_SINK", &sink)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut clip = Command::new(CLIP)
        .current_dir(std::env::temp_dir()) // elsewhere: PRATICO_ROOT names the lab directory
        .env("PRATICO_ROOT", &s.0)
        .env("PATH", s.path(&[]))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    clip.stdin.take().unwrap().write_all(b"via relay").unwrap();
    let o = clip.wait_with_output().unwrap();
    assert!(
        out(&o).contains("copied (9 bytes) via jail relay"),
        "{}",
        out(&o)
    );

    let t = Instant::now();
    while std::fs::read_to_string(&got).unwrap_or_default() != "via relay" {
        assert!(
            t.elapsed() < Duration::from_secs(5),
            "the sink never got the text"
        );
        std::thread::sleep(Duration::from_millis(20));
    }

    // SIGTERM is the trap: the relay removes its FIFO and leaves with 0
    assert!(Command::new("kill")
        .arg(relay.id().to_string())
        .status()
        .unwrap()
        .success());
    assert!(relay.wait().unwrap().success());
    assert!(!fifo.exists(), "the relay must remove the FIFO");
}

#[test]
fn relay_without_a_sink_refuses_to_start() {
    let s = Scratch::new();
    let o = run(RELAY, &s.0, s.bin().to_str().unwrap(), &[], &["f"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("no clipboard tool/display found"));
    let o = run(RELAY, &s.0, s.bin().to_str().unwrap(), &[], &[]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("usage: clip-relay <fifo-path>"));
}
