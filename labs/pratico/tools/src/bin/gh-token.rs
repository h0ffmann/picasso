//! gh-token: resolve the GitHub token a jailed agent should get, on the HOST side of the jail.
//!
//! `just jail-claude` passes `--env GH_TOKEN`, the name-only form: ai-jail forwards the variable only if
//! the launching shell already has it. Two ways that comes up empty, and both end with
//! `gh auth login` inside the sandbox:
//!   - .env has no GH_TOKEN= line (the shellHook loads .env, but it cannot invent a key), or
//!   - `just jco` was run outside `nix develop`, so .env was never loaded at all.
//!
//! Either way the login lands in the jail's ephemeral HOME and dies with the sandbox. So: prefer
//! a fine-grained key from .env, and fall back to the host's own gh login.
//!
//!   gh-token            print the token, nothing else
//!   gh-token --source   print where it came from, never the token itself

use std::process::{exit, Command, Stdio};

const HELP: &str = "gh-token — resolve the GitHub token a jailed agent should get, on the HOST side of the jail.
  gh-token            print the token, nothing else
  gh-token --source   print where it came from (env:GH_TOKEN | env:GITHUB_TOKEN | gh:host-login | none)
Order: GH_TOKEN, then GITHUB_TOKEN, then the host's `gh auth token`. Never a login inside the jail.
";

fn env_set(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|v| !v.is_empty())
}

/// Order matters: a fine-grained .env key is scoped to this repo, `gh auth token` is the
/// full-scope OAuth token behind the interactive login. Prefer the narrower one when it exists.
fn token_source() -> &'static str {
    if env_set("GH_TOKEN") {
        return "env:GH_TOKEN";
    }
    if env_set("GITHUB_TOKEN") {
        return "env:GITHUB_TOKEN";
    }
    let logged_in = Command::new("gh")
        .args(["auth", "token"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if logged_in {
        "gh:host-login"
    } else {
        "none"
    }
}

fn token_value() -> i32 {
    match token_source() {
        "env:GH_TOKEN" => print_raw("GH_TOKEN"),
        "env:GITHUB_TOKEN" => print_raw("GITHUB_TOKEN"),
        "gh:host-login" => Command::new("gh")
            .args(["auth", "token"])
            .status()
            .map_or(1, |s| s.code().unwrap_or(1)),
        _ => 1,
    }
}

/// `printf '%s'`: the value as is, no trailing newline.
fn print_raw(name: &str) -> i32 {
    use std::io::Write;
    let v = std::env::var(name).unwrap_or_default();
    let mut out = std::io::stdout();
    let _ = out.write_all(v.as_bytes());
    let _ = out.flush();
    0
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--source") => {
            let s = token_source();
            println!("{s}");
            exit(i32::from(s == "none"));
        }
        Some("--help" | "-h") => print!("{HELP}"),
        None => exit(token_value()),
        Some(other) => {
            eprintln!("gh-token: unknown argument '{other}'");
            exit(2);
        }
    }
}
