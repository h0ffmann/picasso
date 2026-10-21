//! ww3-submodule: keep a fork of NOAA-EMC/WW3 as a submodule of ww-lab, and keep that fork moving
//! with upstream. The fork is the pin ww-lab builds against; upstream is where the real
//! development (and its pull requests) happens.
//!
//! Idempotent: rerun after a fresh clone (initialises the submodule), rerun to refresh.

#[path = "../git.rs"]
mod git;

use git::{get, ok, run, toplevel};
use std::path::Path;
use std::process::exit;

const USAGE: &str =
    "ww3-submodule — keep a fork of NOAA-EMC/WW3 as a submodule of ww-lab, and keep that fork
moving with upstream. The fork is the pin ww-lab builds against; upstream is where the
real development (and its pull requests) happens.

  ww3-submodule ~/code/ww-lab                    # add the fork as ./WW3 (or init it)
  ww3-submodule ~/code/ww-lab --sync --push      # fork <- upstream/develop, push fork
  ww3-submodule ~/code/ww-lab --bump --commit    # ww-lab pin -> latest fork branch
  ww3-submodule ~/code/ww-lab --pr 1234          # check out upstream PR #1234 in WW3/
  ww3-submodule ~/code/ww-lab --sync --push --bump --commit   # the usual refresh

Options
  --fork <url>      your fork (default git@github.com:h0ffmann/WW3.git) — the submodule's origin
  --upstream <url>  the original (default https://github.com/NOAA-EMC/WW3.git) — remote \"upstream\"
  --path <dir>      submodule directory in ww-lab (default WW3)
  --branch <name>   branch to track on both (default develop, WW3's integration branch)
  --sync            fast-forward the fork's <branch> to upstream/<branch> (no merge commits;
                    if the fork has its own commits, use --merge to merge upstream in)
  --merge           with --sync: merge upstream instead of fast-forwarding
  --push            with --sync: push the updated branch to the fork
  --pr <n>          fetch upstream pull request <n> and check it out (detached) — pinning
                    ww-lab to a PR that is not merged yet; --commit records it
  --bump            move the pin to origin/<branch> of the fork
  --commit          commit .gitmodules and the new pin in ww-lab

Idempotent: rerun after a fresh clone (initialises the submodule), rerun to refresh.
";

fn usage_error(msg: &str) -> ! {
    eprintln!("ww3-submodule: {msg}");
    exit(2)
}

fn main() {
    let mut fork = "git@github.com:h0ffmann/WW3.git".to_string();
    let mut upstream = "https://github.com/NOAA-EMC/WW3.git".to_string();
    let mut path = "WW3".to_string();
    let mut branch = "develop".to_string();
    let (mut sync, mut merge, mut push, mut bump, mut commit) = (false, false, false, false, false);
    let mut pr = String::new();
    let mut target: Option<String> = None;

    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = |name: &str| {
            args.next()
                .unwrap_or_else(|| usage_error(&format!("{name} needs a value")))
        };
        match a.as_str() {
            "--fork" => fork = value("--fork"),
            "--upstream" => upstream = value("--upstream"),
            "--path" => path = value("--path"),
            "--branch" => branch = value("--branch"),
            "--pr" => pr = value("--pr"),
            "--sync" => sync = true,
            "--merge" => merge = true,
            "--push" => push = true,
            "--bump" => bump = true,
            "--commit" => commit = true,
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
    if merge && !sync {
        usage_error("--merge only makes sense with --sync");
    }
    if push && !sync {
        usage_error("--push only makes sense with --sync");
    }
    if !pr.is_empty() && bump {
        usage_error("--pr and --bump both move the pin — pick one");
    }

    let Some(root) = toplevel(&target) else {
        eprintln!("ww3-submodule: {target} is not a git repository");
        exit(1);
    };
    let root = Path::new(&root);
    let sub_path = root.join(&path);
    let sub = sub_path.as_path();
    let short = |rev: &str| get(sub, &["rev-parse", "--short", rev]);
    // The pin ww-lab has committed for the submodule (what a bump is measured against), if any.
    let pinned = || {
        git::out(
            root,
            &["rev-parse", "--short", &format!("HEAD:{path}")],
            true,
        )
        .unwrap_or_else(|| short("HEAD"))
    };

    // 1. The submodule: the fork is origin. Full history on purpose: --sync needs it to
    //    fast-forward, and WW3's regtests and docs are what make the clone big, not the history.
    if ok(
        root,
        &[
            "config",
            "-f",
            ".gitmodules",
            "--get",
            &format!("submodule.{path}.url"),
        ],
    ) {
        println!("ww3-submodule: {path} already in .gitmodules — initialising if needed");
        run(root, &["submodule", "update", "--init", "--", &path]);
    } else {
        println!("ww3-submodule: adding fork {fork} at {path} (branch {branch})");
        run(
            root,
            &["submodule", "add", "-b", &branch, "--", &fork, &path],
        );
    }
    run(
        root,
        &[
            "config",
            "-f",
            ".gitmodules",
            &format!("submodule.{path}.branch"),
            &branch,
        ],
    );

    // 2. The original as a second remote inside the submodule, so the fork can follow it.
    if ok(sub, &["remote", "get-url", "upstream"]) {
        run(sub, &["remote", "set-url", "upstream", &upstream]);
    } else {
        run(sub, &["remote", "add", "upstream", &upstream]);
    }

    // 3. Fork <- upstream. Work on a real local branch so the result can be pushed to the fork.
    if sync {
        let up = format!("upstream/{branch}");
        run(sub, &["fetch", "--quiet", "origin", &branch]);
        run(sub, &["fetch", "--quiet", "upstream", &branch]);
        run(
            sub,
            &[
                "checkout",
                "--quiet",
                "-B",
                &branch,
                &format!("origin/{branch}"),
            ],
        );
        let before = short("HEAD");
        if merge {
            run(sub, &["merge", "--quiet", "--no-edit", &up]);
        } else if !ok_inherit(sub, &["merge", "--quiet", "--ff-only", &up]) {
            eprintln!("ww3-submodule: fork/{branch} has diverged from upstream/{branch} — rerun with --merge,");
            eprintln!(
                "               or rebase the fork's own commits by hand inside {}",
                sub.display()
            );
            exit(1);
        }
        let after = short("HEAD");
        if before == after {
            println!("ww3-submodule: fork/{branch} already up to date with upstream ({after})");
        } else {
            let n = get(sub, &["rev-list", "--count", &format!("{before}..{after}")]);
            println!(
                "ww3-submodule: fork/{branch} {before} -> {after} ({n} commits from upstream)"
            );
        }
        if push {
            run(sub, &["push", "--quiet", "origin", &branch]);
            println!("ww3-submodule: pushed {branch} to fork");
        }
    }

    // 4. Where ww-lab's pin goes: an upstream PR head, or the fork's branch tip.
    if !pr.is_empty() {
        run(
            sub,
            &[
                "fetch",
                "--quiet",
                "upstream",
                &format!("refs/pull/{pr}/head:refs/remotes/upstream/pr/{pr}"),
            ],
        );
        run(
            sub,
            &[
                "checkout",
                "--quiet",
                "--detach",
                &format!("upstream/pr/{pr}"),
            ],
        );
        println!(
            "ww3-submodule: {path} at upstream PR #{pr} ({})",
            short("HEAD")
        );
    } else if bump {
        let before = pinned();
        run(sub, &["fetch", "--quiet", "origin", &branch]);
        run(
            sub,
            &[
                "checkout",
                "--quiet",
                "--detach",
                &format!("origin/{branch}"),
            ],
        );
        let after = short("HEAD");
        if before == after {
            println!("ww3-submodule: pin already at fork/{branch} ({after})");
        } else {
            println!("ww3-submodule: pin {before} -> {after} (fork/{branch})");
        }
    }

    // 5. Record it in ww-lab.
    run(root, &["add", ".gitmodules", &path]);
    if commit {
        if ok(root, &["diff", "--cached", "--quiet"]) {
            println!("ww3-submodule: nothing to commit");
        } else {
            let what = if pr.is_empty() {
                format!("fork/{branch}")
            } else {
                format!("upstream PR #{pr}")
            };
            run(
                root,
                &[
                    "commit",
                    "-q",
                    "-m",
                    &format!("WW3 submodule: {path} @ {} ({what})", short("HEAD")),
                ],
            );
            println!("ww3-submodule: committed");
        }
    } else {
        println!("ww3-submodule: staged .gitmodules and {path} — commit when ready (or rerun with --commit)");
    }

    let url = |remote: &str| get(sub, &["remote", "get-url", remote]);
    println!(
        "ww3-submodule: {} -> {}  origin={}  upstream={}",
        sub.display(),
        short("HEAD"),
        url("origin"),
        url("upstream")
    );
    println!("  nix develop ./nix-config/labs/pratico#ww3 --command cmake -S {path}/model -B build -DSWITCH=... -DCMAKE_INSTALL_PREFIX=install");
}

/// A command whose failure is handled by the caller, with git's own messages still shown.
fn ok_inherit(dir: &Path, args: &[&str]) -> bool {
    std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .is_ok_and(|s| s.success())
}
