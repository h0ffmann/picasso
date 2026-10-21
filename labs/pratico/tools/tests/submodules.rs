//! ww-lab-submodule and ww3-submodule against throwaway local repositories (no network).
mod common;
use common::*;
use std::path::Path;
use std::process::{Command, Output};

const WWLAB: &str = env!("CARGO_BIN_EXE_ww-lab-submodule");
const WW3: &str = env!("CARGO_BIN_EXE_ww3-submodule");

fn git_cmd(dir: &Path) -> Command {
    let mut c = Command::new("git");
    c.current_dir(dir)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "protocol.file.allow")
        .env("GIT_CONFIG_VALUE_0", "always")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com");
    c
}

fn git(dir: &Path, args: &[&str]) -> String {
    let o = git_cmd(dir).args(args).output().unwrap();
    assert!(o.status.success(), "git {args:?}: {}", err(&o));
    out(&o).trim().to_string()
}

/// A repo on `branch` with one commit per (file, content), and a bare clone of it.
fn make(
    s: &Scratch,
    name: &str,
    branch: &str,
    files: &[(&str, &str)],
) -> (std::path::PathBuf, std::path::PathBuf) {
    let work = s.0.join(format!("{name}-work"));
    std::fs::create_dir_all(&work).unwrap();
    git(&work, &["init", "-q", "-b", branch]);
    for (f, c) in files {
        let p = work.join(f);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, c).unwrap();
    }
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "first"]);
    let bare = s.0.join(format!("{name}.git"));
    git(
        &s.0,
        &[
            "clone",
            "-q",
            "--bare",
            work.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    (work, bare)
}

fn consumer(s: &Scratch) -> std::path::PathBuf {
    let c = s.0.join("ww-lab");
    std::fs::create_dir_all(&c).unwrap();
    git(&c, &["init", "-q", "-b", "main"]);
    std::fs::write(c.join("README"), "x").unwrap();
    git(&c, &["add", "-A"]);
    git(&c, &["commit", "-q", "-m", "init"]);
    c
}

fn tool(bin: &str, dir: &Path, args: &[&str]) -> Output {
    let mut c = Command::new(bin);
    c.current_dir(dir).args(args);
    for (k, v) in git_cmd(dir).get_envs() {
        if let Some(v) = v {
            c.env(k, v);
        }
    }
    c.output().unwrap()
}

#[test]
fn ww3_adds_the_fork_syncs_from_upstream_and_pins() {
    let s = Scratch::new();
    let (_, fork) = make(&s, "fork", "develop", &[("a", "1")]);
    // upstream = the fork plus one commit
    let up_work = s.0.join("up-work");
    git(
        &s.0,
        &[
            "clone",
            "-q",
            fork.to_str().unwrap(),
            up_work.to_str().unwrap(),
        ],
    );
    std::fs::write(up_work.join("b"), "2").unwrap();
    git(&up_work, &["add", "-A"]);
    git(&up_work, &["commit", "-q", "-m", "upstream work"]);
    let upstream = s.0.join("upstream.git");
    git(
        &s.0,
        &[
            "clone",
            "-q",
            "--bare",
            up_work.to_str().unwrap(),
            upstream.to_str().unwrap(),
        ],
    );

    let c = consumer(&s);
    let (f, u) = (fork.to_str().unwrap(), upstream.to_str().unwrap());
    let o = tool(WW3, &c, &[".", "--fork", f, "--upstream", u, "--commit"]);
    assert!(o.status.success(), "{}", err(&o));
    assert!(git(&c, &["log", "-1", "--format=%s"]).starts_with("WW3 submodule: WW3 @ "));
    assert_eq!(
        git(&c, &["config", "-f", ".gitmodules", "submodule.WW3.branch"]),
        "develop"
    );
    assert_eq!(git(&c.join("WW3"), &["remote", "get-url", "upstream"]), u);
    assert!(!c.join("WW3/b").exists(), "not synced yet");

    let o = tool(
        WW3,
        &c,
        &[".", "--fork", f, "--upstream", u, "--sync", "--commit"],
    );
    assert!(o.status.success(), "{}", err(&o));
    assert!(out(&o).contains("(1 commits from upstream)"), "{}", out(&o));
    assert!(c.join("WW3/b").exists(), "fast-forwarded to upstream");
    assert!(git(&c, &["log", "-1", "--format=%s"]).ends_with("(fork/develop)"));

    let o = tool(
        WW3,
        &c,
        &[".", "--fork", f, "--upstream", u, "--sync", "--commit"],
    );
    // Nothing was pushed, so a rerun resets the local branch to the fork's tip and fast-forwards
    // to the same upstream commit: the pin does not move.
    assert!(out(&o).contains("nothing to commit"), "{}", out(&o));
}

#[test]
fn ww3_rejects_flags_that_make_no_sense() {
    let s = Scratch::new();
    let c = consumer(&s);
    for args in [
        &[".", "--merge"][..],
        &[".", "--push"],
        &[".", "--pr", "1", "--bump"],
        &[".", "--bogus"],
        &[".", "--fork"],
    ] {
        assert_eq!(tool(WW3, &c, args).status.code(), Some(2), "{args:?}");
    }
    let o = tool(WW3, &s.0, &["/definitely/not/a/repo"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(err(&o).contains("is not a git repository"));
}

#[test]
fn ww_lab_submodule_sparse_checks_out_only_the_lab_and_bumps() {
    let s = Scratch::new();
    let (work, bare) = make(
        &s,
        "nix-config",
        "main",
        &[
            ("labs/pratico/flake.nix", "{}"),
            ("flake.nix", "{}"),
            ("labs/other/x", "x"),
        ],
    );
    let url = format!("file://{}", bare.display());
    let c = consumer(&s);

    let o = tool(WWLAB, &c, &[".", "--url", &url, "--commit"]);
    assert!(o.status.success(), "{}", err(&o));
    assert!(c.join("nix-config/labs/pratico/flake.nix").exists());
    assert!(
        !c.join("nix-config/labs/other").exists(),
        "only labs/pratico on disk"
    );
    assert!(
        !c.join("nix-config/flake.nix").exists(),
        "non-cone: no top-level files either"
    );
    assert!(git(&c, &["log", "-1", "--format=%s"]).ends_with("(labs/pratico only)"));
    assert_eq!(
        git(
            &c,
            &[
                "config",
                "-f",
                ".gitmodules",
                "submodule.nix-config.shallow"
            ]
        ),
        "true"
    );

    let pin = git(&c, &["rev-parse", "HEAD:nix-config"]);
    std::fs::write(work.join("labs/pratico/new"), "n").unwrap();
    git(&work, &["add", "-A"]);
    git(&work, &["commit", "-q", "-m", "second"]);
    git(&work, &["push", "-q", bare.to_str().unwrap(), "main"]);
    let o = tool(WWLAB, &c, &[".", "--url", &url, "--bump", "--commit"]);
    assert!(o.status.success(), "{}", err(&o));
    assert!(out(&o).contains("(origin/main)"));
    assert_ne!(
        git(&c, &["rev-parse", "HEAD:nix-config"]),
        pin,
        "the pin moved"
    );
    assert!(c.join("nix-config/labs/pratico/new").exists());
}
