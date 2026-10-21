//! social: every docs/social/instagram_*/README.md carries a status, and a post that is POSTED
//! on Instagram goes to Threads with the same slides, caption and alt texts (docs/adr/0001).
//!
//! `post` and `release` read THREADS_USER_ID and THREADS_ACCESS_TOKEN from the environment.
//! Threads cannot take an upload, only a public URL, so the slides are fetched from
//! raw.githubusercontent.com at <sha>.

mod cves;
mod post;
mod render;
mod threads;

use post::{PostError, Result};
use std::process::ExitCode;

const USAGE: &str = "usage:
    social check                         # every post has a valid front matter
    social changed <before-sha> <sha>    # posts that became POSTED between two commits
    social latest                        # the newest POSTED post
    social post <dir> --sha <sha> [--dry-run]
    social release <tag> [--dry-run]     # a text post for a vMAJOR.MINOR.PATCH release
    social refresh                       # print a refreshed long-lived Threads token
    social cves <dir with npm.zip and PyPI.zip>   # monthly CVE counts as JSON on stdout
    social render                        # redraw docs/social/instagram_001/slide-*.png (headless Chromium; CHROMIUM=<path>)
    social --self-test";

fn run(args: &[String]) -> Result<Option<i32>> {
    let a: Vec<&str> = args.iter().map(String::as_str).collect();
    let root = post::root();
    Ok(Some(match a.as_slice() {
        ["--self-test"] => {
            post::self_test(&root)?;
            println!("social: self-test passed");
            0
        }
        ["check"] => post::check(&root)?,
        ["changed", before, after] => {
            println!("{}", post::changed(&root, before, after)?.join("\n"));
            0
        }
        ["latest"] => {
            println!("{}", post::latest(&root)?.unwrap_or_default());
            0
        }
        ["post", d, "--sha", sha, rest @ ..] if rest.is_empty() || rest == ["--dry-run"] => {
            post::post_cmd(&root, &root.join(d), sha, !rest.is_empty())?
        }
        ["release", tag, rest @ ..] if rest.is_empty() || rest == ["--dry-run"] => {
            post::release_cmd(tag, !rest.is_empty())?
        }
        ["refresh"] => threads::refresh()?,
        ["cves", dir] => cves::run(dir)?,
        ["render"] => render::run(&root)?,
        _ => return Ok(None),
    }))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(Some(code)) => ExitCode::from(code as u8),
        Ok(None) => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
        Err(PostError(e)) => {
            eprintln!("social: {e}");
            ExitCode::from(1)
        }
    }
}
