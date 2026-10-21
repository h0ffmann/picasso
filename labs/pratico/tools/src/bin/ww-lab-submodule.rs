//! ww-lab-submodule: wire nix-config into a consumer repo (ww-lab) as a git submodule with only
//! labs/pratico checked out. Git cannot submodule a subdirectory, so the submodule points at the
//! whole repo and sparse-checkout keeps everything but labs/pratico off disk.
//!
//! Idempotent: run it once to add the submodule, again after a fresh clone to materialise it, and
//! with --bump to move the pin to the latest origin/<branch>. Every git call is explicit about
//! its directory, so run it from anywhere.

#[path = "../git.rs"]
mod git;

use git::{get, ok, out, run, toplevel};
use std::path::Path;
use std::process::exit;

const USAGE: &str =
    "ww-lab-submodule — wire nix-config into a consumer repo (ww-lab) as a git submodule with
only labs/pratico checked out. Git cannot submodule a subdirectory, so the submodule points
at the whole repo and sparse-checkout keeps everything but labs/pratico off disk.

Idempotent: run it once to add the submodule, again after a fresh clone to materialise it,
and with --bump to move the pin to the latest origin/<branch>. Every git call is explicit
about its directory, so run it from anywhere.

  ww-lab-submodule ~/code/ww-lab                 # add or (re)initialise
  ww-lab-submodule ~/code/ww-lab --bump          # pin to latest origin/main
  ww-lab-submodule ~/code/ww-lab --branch feat/x # track a branch (before merge)
  ww-lab-submodule ~/code/ww-lab --commit        # also commit .gitmodules + pin

Sparse-checkout is local state, not recorded in .gitmodules, which is why fresh clones
need this tool (or `git -C nix-config sparse-checkout set labs/pratico`) once.
";

fn usage_error(msg: &str) -> ! {
    eprintln!("ww-lab-submodule: {msg}");
    exit(2)
}

fn main() {
    let mut url = "git@github.com:h0ffmann/nix-config.git".to_string();
    let mut path = "nix-config".to_string();
    let sparse = "labs/pratico";
    let mut branch = "main".to_string();
    let (mut bump, mut commit) = (false, false);
    let mut target: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .unwrap_or_else(|| usage_error(&format!("{name} needs a value")))
        };
        match a.as_str() {
            "--bump" => bump = true,
            "--commit" => commit = true,
            "--branch" => branch = value("--branch"),
            "--url" => url = value("--url"),
            "--path" => path = value("--path"),
            "--help" | "-h" => {
                print!("{USAGE}");
                return;
            }
            o if o.starts_with('-') => usage_error(&format!("unknown option '{o}'")),
            _ => {
                if target.is_some() {
                    usage_error("one target directory only");
                }
                target = Some(a);
            }
        }
    }
    let target = target.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    });

    let Some(root) = toplevel(&target) else {
        eprintln!("ww-lab-submodule: {target} is not a git repository");
        exit(1);
    };
    let root = Path::new(&root);
    let sub_path = root.join(&path);
    let sub = sub_path.as_path();
    let key = |k: &str| format!("submodule.{path}.{k}");

    // 1. The submodule entry. Present in .gitmodules: make sure it is initialised; absent: add.
    if ok(root, &["config", "-f", ".gitmodules", "--get", &key("url")]) {
        println!("ww-lab-submodule: {path} already in .gitmodules — initialising if needed");
        run(
            root,
            &["submodule", "update", "--init", "--depth", "1", "--", &path],
        );
    } else {
        println!("ww-lab-submodule: adding {url} at {path} (branch {branch})");
        run(
            root,
            &[
                "submodule",
                "add",
                "--depth",
                "1",
                "-b",
                &branch,
                "--",
                &url,
                &path,
            ],
        );
    }
    // Recorded in .gitmodules, so every clone fetches shallow and knows which branch to follow.
    run(
        root,
        &["config", "-f", ".gitmodules", &key("shallow"), "true"],
    );
    run(
        root,
        &["config", "-f", ".gitmodules", &key("branch"), &branch],
    );

    // 2. Only labs/pratico on disk. Non-cone mode on purpose: cone mode always materialises the
    // repo's top-level files (flake.nix, configuration.nix, ...), which is exactly the noise a
    // consumer should not see. The leading slash anchors the pattern at the repo root.
    run(
        sub,
        &[
            "sparse-checkout",
            "set",
            "--no-cone",
            &format!("/{sparse}/"),
        ],
    );

    // 3. Optionally move the pin to the tip of the tracked branch.
    if bump {
        let before = get(sub, &["rev-parse", "HEAD"]);
        run(sub, &["fetch", "--depth", "1", "origin", &branch]);
        run(sub, &["checkout", "--quiet", "--detach", "FETCH_HEAD"]);
        let after = get(sub, &["rev-parse", "HEAD"]);
        if before == after {
            println!(
                "ww-lab-submodule: already at origin/{branch} ({})",
                &after[..7.min(after.len())]
            );
        } else {
            println!(
                "ww-lab-submodule: {} -> {} (origin/{branch})",
                &before[..7.min(before.len())],
                &after[..7.min(after.len())]
            );
        }
    }

    // 4. Stage what the consumer repo must commit: .gitmodules and the submodule pointer.
    run(root, &["add", ".gitmodules", &path]);
    if commit {
        if ok(root, &["diff", "--cached", "--quiet"]) {
            println!("ww-lab-submodule: nothing to commit");
        } else {
            let short = get(sub, &["rev-parse", "--short", "HEAD"]);
            run(
                root,
                &[
                    "commit",
                    "-q",
                    "-m",
                    &format!("nix-config submodule: {path} @ {short} ({sparse} only)"),
                ],
            );
            println!("ww-lab-submodule: committed");
        }
    } else {
        println!("ww-lab-submodule: staged .gitmodules and {path} — commit when ready (or rerun with --commit)");
    }

    if !sub.join(sparse).is_dir() {
        eprintln!(
            "ww-lab-submodule: {}/{sparse} is missing after checkout",
            sub.display()
        );
        exit(1);
    }
    let listed = out(sub, &["sparse-checkout", "list"], false)
        .unwrap_or_default()
        .replace('\n', " ");
    println!(
        "ww-lab-submodule: {} -> {}, checked out: {listed} ",
        sub.display(),
        get(sub, &["rev-parse", "--short", "HEAD"])
    );
    println!("  nix develop {path}/{sparse}#ww3");
    println!("  just -f {path}/{sparse}/justfile toolchain");
}
