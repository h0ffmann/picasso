//! jail-run, 1:1 with the old --self-test: a fake ai-jail prints its argv, one per line, and what
//! the child inherited goes to stderr (presence only, never a value).
mod common;
use common::*;

const BIN: &str = env!("CARGO_BIN_EXE_jail-run");
const RELAY: &str = env!("CARGO_BIN_EXE_clip-relay");

const FAKE_JAIL: &str = r#"printf '%s\n' "$@"
printf 'child-env: GH_TOKEN=%s GITHUB_TOKEN=%s\n' "${GH_TOKEN:+present}" "${GITHUB_TOKEN:+present}" >&2"#;
const FAKE_GH_TOKEN: &str = r#"[ "${1:-}" = --source ] && { echo env:GH_TOKEN; exit 0; }
echo tok"#;

fn setup() -> Scratch {
    let s = Scratch::new();
    s.fake("ai-jail", FAKE_JAIL);
    s.fake("gh-token", FAKE_GH_TOKEN);
    s
}

fn flat(o: &std::process::Output) -> String {
    out(o).lines().collect::<Vec<_>>().join(" ") + " "
}

fn go(s: &Scratch, env: &[(&str, &str)], args: &[&str]) -> std::process::Output {
    run(BIN, &s.0, &s.path(&[RELAY]), env, args)
}

fn home() -> String {
    std::env::var("HOME").unwrap()
}

#[test]
fn no_mode_is_a_usage_error() {
    let s = setup();
    assert_eq!(go(&s, &[], &[]).status.code(), Some(2));
}

#[test]
fn dry_run_forwards_the_command_after_the_dashes() {
    let s = setup();
    assert_eq!(
        flat(&go(&s, &[], &["--dry-run", "--", "true"])),
        "--no-save-config --dry-run -- true "
    );
}

#[test]
fn claude_flags_and_token() {
    let s = setup();
    let h = home();
    let o = go(&s, &[("GH_TOKEN", "t")], &["claude", "--model", "x"]);
    let f = flat(&o);
    assert!(f.contains("--env GH_TOKEN"));
    assert!(f.contains(&format!(
        "--rw-map {h}/.claude --rw-map {h}/.claude.json --map {h}/.ssh"
    )));
    assert!(f.contains("--exec --env GH_TOKEN claude --model x"));
    assert!(
        !f.contains("--display"),
        "no display passthrough unless asked"
    );
    let e = err(&o);
    assert!(
        e.contains("GH_TOKEN from env:GH_TOKEN"),
        "says where it came from"
    );
    assert!(
        e.contains("child-env: GH_TOKEN=present"),
        "the token is in ai-jail's environment"
    );
    assert!(
        !e.lines().any(|l| l == "tok") && !f.contains(" tok "),
        "the token value is never printed"
    );
}

#[test]
fn paste_adds_the_display_flags() {
    let s = setup();
    let o = go(
        &s,
        &[("GH_TOKEN", "t"), ("JAIL_CLIPBOARD_PASTE", "1")],
        &["claude"],
    );
    assert!(flat(&o).contains("--display --env DISPLAY"));
}

#[test]
fn opencode_has_its_own_dirs_not_claudes() {
    let s = setup();
    let h = home();
    let f = flat(&go(&s, &[("GH_TOKEN", "t")], &["opencode"]));
    assert!(f.contains(&format!("--rw-map {h}/.config/opencode")));
    assert!(!f.contains(&format!("{h}/.claude")));
    assert!(f.contains("--exec --env GH_TOKEN opencode"));
}

#[test]
fn clipboard_relay_runs_beside_the_jail_and_the_fifo_is_removed() {
    let s = setup();
    let o = go(
        &s,
        &[
            ("GH_TOKEN", "t"),
            ("JAIL_CLIPBOARD", "1"),
            ("CLIP_SINK", "cat >/dev/null"),
        ],
        &["claude"],
    );
    assert!(flat(&o).contains("--exec --env GH_TOKEN claude"));
    assert!(
        !s.0.join(".tmp/clip.fifo").exists(),
        "the fifo is removed when the jail exits"
    );
}

#[test]
fn ocr_is_a_reviewer_not_an_author() {
    let s = setup();
    let h = home();
    let outd = s.0.join("out");
    std::fs::create_dir_all(&outd).unwrap();
    let od = outd.to_str().unwrap();
    let env = [
        ("GH_TOKEN", "x"),
        ("GITHUB_TOKEN", "x"),
        ("JAIL_OCR_OUT", od),
    ];
    let o = go(&s, &env, &["ocr", "review", "--format", "json"]);
    let f = flat(&o);
    assert!(
        !f.contains("GH_TOKEN") && !f.contains("GITHUB_TOKEN"),
        "no token on the sandbox line"
    );
    assert!(
        err(&o).contains("child-env: GH_TOKEN= GITHUB_TOKEN=\n"),
        "both unset in the child, not merely unflagged"
    );
    assert!(!f.contains(&format!("{h}/.claude")));
    assert!(f.contains(&format!("--map {} --rw-map {od}", s.0.display())));
    assert!(f.contains(&format!("--env HOME={od}")));
    assert!(f.contains("--clean --private-home --no-agent-state"));
    assert!(f.contains("--network"));
    assert!(f.contains("--exec ocr review --format json"));

    let env = [
        ("JAIL_OCR_OUT", od),
        ("OCR_LLM_URL", "u"),
        ("OCR_LLM_MODEL", "m"),
    ];
    let f = flat(&go(&s, &env, &["ocr"]));
    assert!(f.contains("--env OCR_LLM_URL --env OCR_LLM_MODEL"));
    assert!(
        !f.contains("OCR_LLM_TOKEN"),
        "variables that are not set are not invented"
    );
}

#[test]
fn ocr_out_falls_back_to_runner_temp() {
    let s = setup();
    let rt = s.0.join("rt");
    let f = flat(&go(&s, &[("RUNNER_TEMP", rt.to_str().unwrap())], &["ocr"]));
    assert!(f.contains(&format!("--rw-map {}", rt.display())));
    assert!(rt.is_dir());
}

#[test]
fn no_token_warns_but_still_starts() {
    let s = setup();
    s.fake("gh-token", "exit 1");
    let o = go(&s, &[], &["opencode"]);
    assert!(err(&o).contains("no GitHub token"));
    assert!(flat(&o).contains("opencode"));
}
