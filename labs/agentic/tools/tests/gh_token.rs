//! gh-token, 1:1 with the old --self-test, against a fake `gh` on PATH. No real token is used.
mod common;
use common::*;

const BIN: &str = env!("CARGO_BIN_EXE_gh-token");
const FAKE_GH: &str = r#"[ "$1" = auth ] && [ "$2" = token ] && { echo host-oauth-token; exit 0; }
exit 1"#;

fn go(s: &Scratch, env: &[(&str, &str)], args: &[&str]) -> std::process::Output {
    run(BIN, &s.0, &s.path(&[]), env, args)
}

#[test]
fn fine_grained_token_wins_and_is_printed() {
    let s = Scratch::new();
    s.fake("gh", FAKE_GH);
    let env = [("GH_TOKEN", "fine-grained"), ("GITHUB_TOKEN", "other")];
    assert_eq!(out(&go(&s, &env, &["--source"])).trim(), "env:GH_TOKEN");
    assert_eq!(out(&go(&s, &env, &[])), "fine-grained");
}

#[test]
fn empty_gh_token_falls_through() {
    let s = Scratch::new();
    s.fake("gh", FAKE_GH);
    let env = [("GH_TOKEN", ""), ("GITHUB_TOKEN", "ci-token")];
    assert_eq!(out(&go(&s, &env, &["--source"])).trim(), "env:GITHUB_TOKEN");
}

#[test]
fn host_login_is_the_last_resort() {
    let s = Scratch::new();
    s.fake("gh", FAKE_GH);
    let env = [("GH_TOKEN", ""), ("GITHUB_TOKEN", "")];
    assert_eq!(out(&go(&s, &env, &["--source"])).trim(), "gh:host-login");
    assert_eq!(out(&go(&s, &env, &[])).trim(), "host-oauth-token");
}

#[test]
fn logged_out_gh_is_no_source() {
    let s = Scratch::new();
    s.fake("gh", "exit 1");
    let env = [("GH_TOKEN", ""), ("GITHUB_TOKEN", "")];
    let o = go(&s, &env, &["--source"]);
    assert_eq!(out(&o).trim(), "none");
    assert!(!o.status.success());
    let o = go(&s, &env, &[]);
    assert!(
        !o.status.success(),
        "printing the token must fail so the caller can warn"
    );
    assert!(out(&o).is_empty());
}

#[test]
fn source_never_leaks_the_token() {
    let s = Scratch::new();
    s.fake("gh", FAKE_GH);
    let o = go(&s, &[("GH_TOKEN", "secret")], &["--source"]);
    assert!(!out(&o).contains("secret") && !err(&o).contains("secret"));
    let o = go(&s, &[("GH_TOKEN", "secret")], &["--help"]);
    assert!(!out(&o).contains("secret") && !err(&o).contains("secret"));
}

#[test]
fn unknown_argument_is_exit_2() {
    let s = Scratch::new();
    let o = go(&s, &[], &["--bogus"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(err(&o).contains("unknown argument '--bogus'"));
}
