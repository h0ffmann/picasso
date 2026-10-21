//! Add the nixos-cuda binary cache to Nix, so torchWithCuda is fetched, not compiled (hours otherwise).
//!
//!   setup-cuda-cache --dry-run              print the change, touch nothing
//!   sudo setup-cuda-cache                   apply it (replacing a stale block), then restart the daemon
//!   sudo setup-cuda-cache --remove          take the block out again
//!   setup-cuda-cache --verify [expr-file]   does CUDA torch resolve as a fetch, not a build?
//!   setup-cuda-cache --doctor               which Nix config still names the dead Cachix cache
//!   sudo setup-cuda-cache --fix-stale       strip the dead cache from nix.custom.conf only
use std::env;
use std::fs;
use std::process::{exit, Command};

// The cache moved off Cachix in Nov 2025; the old cuda-maintainers.cachix.org answers 401, which
// Determinate Nix treats as fatal for every `nix develop`, so a stale block must be replaced, not kept.
const CACHE: &str = "https://cache.nixos-cuda.org";
const KEY: &str = "cache.nixos-cuda.org:74DUi4Ye579gUqzH4ziL9IyiJBlDpMRn9MBN8oNan9M=";
const STALE_CACHE: &str = "https://cuda-maintainers.cachix.org";
const STALE_HOST: &str = "cuda-maintainers.cachix.org";

const USAGE: &str = "\
setup-cuda-cache — the nixos-cuda binary cache in Nix's nix.custom.conf
  setup-cuda-cache --dry-run              print the change, touch nothing
  sudo setup-cuda-cache                   apply it (replacing a stale block), then restart the daemon
  sudo setup-cuda-cache --remove          take the block out again
  setup-cuda-cache --verify [expr-file]   does CUDA torch resolve as a fetch, not a build?
  setup-cuda-cache --doctor               which Nix config still names the dead Cachix cache, and the fix
  sudo setup-cuda-cache --fix-stale       strip the dead cache from nix.custom.conf, keep everything else
";

struct Cfg {
    conf: String,
    user_conf: String,
    system_conf: String,
    nix_config: String,
    user: String,
    exe: String,
}

fn out(cmd: &str, args: &[&str]) -> Option<String> {
    let o = Command::new(cmd).args(args).output().ok()?;
    o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

fn cfg() -> Cfg {
    let var = |k: &str| env::var(k).ok().filter(|v| !v.is_empty());
    let user = var("SUDO_USER")
        .or_else(|| var("USER"))
        .or_else(|| out("id", &["-un"]))
        .unwrap_or_default();
    // `cachix use` without root writes here; under sudo, HOME is root's, so resolve the caller's.
    let home = out("getent", &["passwd", &user])
        .and_then(|l| l.split(':').nth(5).map(str::to_string))
        .filter(|h| !h.is_empty())
        .or_else(|| var("HOME"))
        .unwrap_or_default();
    Cfg {
        // Determinate Nix marks /etc/nix/nix.conf "do not modify" and !includes this one.
        conf: var("NIX_CUSTOM_CONF").unwrap_or_else(|| "/etc/nix/nix.custom.conf".into()),
        user_conf: var("NIX_USER_CONF").unwrap_or_else(|| format!("{home}/.config/nix/nix.conf")),
        system_conf: var("NIX_SYSTEM_CONF").unwrap_or_else(|| "/etc/nix/nix.conf".into()),
        nix_config: env::var("NIX_CONFIG").unwrap_or_default(),
        user,
        exe: env::current_exe()
            .and_then(fs::canonicalize)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "setup-cuda-cache".into()),
    }
}

fn block(user: &str) -> String {
    format!(
        "\n# nix-config/labs/cuda: CUDA binary cache (setup-cuda-cache). Substituters are a trusted-user setting,\n\
         # so the user must be listed here too or Nix silently ignores the cache.\n\
         trusted-users = root {user}\n\
         extra-substituters = {CACHE}\n\
         extra-trusted-public-keys = {KEY}\n"
    )
}

fn file_has(path: &str, needle: &str) -> bool {
    fs::read_to_string(path).is_ok_and(|t| t.contains(needle))
}

fn already_configured(c: &Cfg) -> bool {
    file_has(&c.conf, CACHE)
}

fn stale_configured(c: &Cfg) -> bool {
    file_has(&c.conf, STALE_CACHE)
}

/// Applies `f` to every line, keeping the file's final-newline state (what `sed -i` does).
fn map_lines(text: &str, f: impl FnOnce(Vec<&str>) -> Vec<String>) -> String {
    let mut s = f(text.lines().collect()).join("\n");
    if !s.is_empty() && text.ends_with('\n') {
        s.push('\n');
    }
    s
}

/// `s#[[:space:]]*https://cuda-maintainers\.cachix\.org/?([[:space:]]|$)#\1#g`
fn strip_url(line: &str) -> String {
    let mut res = String::new();
    let mut rest = line;
    while let Some(i) = rest.find(STALE_CACHE) {
        let mut end = i + STALE_CACHE.len();
        if rest[end..].starts_with('/') && rest[end + 1..].chars().next().is_none_or(char::is_whitespace) {
            end += 1;
        }
        if rest[end..].chars().next().is_none_or(char::is_whitespace) {
            res.push_str(rest[..i].trim_end());
            rest = &rest[end..];
        } else {
            res.push_str(&rest[..end]);
            rest = &rest[end..];
        }
    }
    res.push_str(rest);
    res
}

/// `s#[[:space:]]*cuda-maintainers\.cachix\.org-1:[^[:space:]]*##g`
fn strip_key(line: &str) -> String {
    let pat = "cuda-maintainers.cachix.org-1:";
    let mut res = String::new();
    let mut rest = line;
    while let Some(i) = rest.find(pat) {
        res.push_str(rest[..i].trim_end());
        let after = &rest[i + pat.len()..];
        rest = after.find(char::is_whitespace).map_or("", |j| &after[j..]);
    }
    res.push_str(rest);
    res
}

fn is_emptied_setting(line: &str) -> bool {
    let t = line.trim_start();
    let t = t.strip_prefix("extra-").unwrap_or(t);
    ["substituters", "trusted-substituters", "trusted-public-keys"]
        .iter()
        .any(|k| t.strip_prefix(k).is_some_and(|r| r.trim_start().strip_prefix('=').is_some_and(|v| v.trim().is_empty())))
}

/// Takes the dead cache's URL and key out of any setting line, keeping the other entries on it, and
/// drops a substituters/keys line left empty (an empty `substituters =` would also drop cache.nixos.org).
fn strip_stale(text: &str) -> String {
    map_lines(text, |ls| {
        ls.into_iter().map(|l| strip_key(&strip_url(l))).filter(|l| !is_emptied_setting(l)).collect()
    })
}

/// Drops the block this program wrote (either cache), in place: `/^# (marola|nix-config/labs/cuda): CUDA binary cache/,/^extra-trusted-public-keys = /d`.
fn remove_block(text: &str) -> String {
    map_lines(text, |ls| {
        let (mut keep, mut inside) = (Vec::new(), false);
        for l in ls {
            if inside {
                inside = !l.starts_with("extra-trusted-public-keys = ");
            } else if l.starts_with("# marola: CUDA binary cache") || l.starts_with("# nix-config/labs/cuda: CUDA binary cache") {
                inside = true;
            } else {
                keep.push(l.to_string());
            }
        }
        keep
    })
}

fn edit(path: &str, f: impl Fn(&str) -> String) {
    let r = fs::read_to_string(path).and_then(|t| fs::write(path, f(&t)));
    if let Err(e) = r {
        eprintln!("setup-cuda-cache: {path}: {e}");
        exit(1);
    }
}

fn stale_cmd(f: &str) -> String {
    format!("sed -i -E 's#[[:space:]]*(https://)?cuda-maintainers\\.cachix\\.org[^[:space:]]*##g' {f}")
}

/// Every file Nix reads settings from that can carry the dead cache; `.1` is true if any does.
fn doctor(c: &Cfg) -> (String, bool) {
    let (mut o, mut found) = (String::new(), false);
    for f in [&c.system_conf, &c.conf, &c.user_conf] {
        let Ok(text) = fs::read_to_string(f) else { continue };
        if !text.contains(STALE_HOST) {
            continue;
        }
        found = true;
        o += &format!("{f} still names the dead cache:\n");
        for (n, l) in text.lines().enumerate().filter(|(_, l)| l.contains(STALE_HOST)) {
            o += &format!("  {}:{l}\n", n + 1);
        }
        o += &if *f == c.conf {
            format!("  fix: sudo {} --fix-stale\n", c.exe)
        } else if *f == c.system_conf {
            format!("  fix (Determinate Nix marks this file 'do not modify'; edit by hand): sudo {}\n", stale_cmd(f))
        } else {
            format!("  fix: {}\n", stale_cmd(f))
        };
    }
    if c.nix_config.contains(STALE_HOST) {
        found = true;
        o += "NIX_CONFIG in this shell names the dead cache; remove it where it is exported\n";
    }
    if !found {
        o += &format!("no Nix config here names {STALE_HOST}\n");
    }
    (o, found)
}

/// The check that matters: does CUDA torch resolve as a fetch rather than a build?
fn verify(expr_file: Option<&str>) -> i32 {
    let mut cmd = Command::new("nix");
    cmd.args(["build", "--dry-run", "--impure"]);
    match expr_file {
        Some(f) if !f.is_empty() => {
            let Ok(expr) = fs::read_to_string(f) else {
                println!("verify: no such file {f}");
                return 2;
            };
            cmd.args(["--expr", expr.trim_end_matches('\n')]);
        }
        _ => {
            cmd.env("NIXPKGS_ALLOW_UNFREE", "1").arg("nixpkgs#python3Packages.torchWithCuda");
        }
    }
    let text = cmd.output().map_or(String::new(), |o| {
        format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))
    });
    let hits: Vec<&str> = text.lines().filter(|l| l.contains("will be built") || l.contains("will be fetched")).collect();
    if hits.is_empty() {
        println!("nothing to do — already realised");
    }
    hits.iter().for_each(|l| println!("{l}"));
    0
}

fn need_root(c: &Cfg) {
    if out("id", &["-u"]).as_deref() == Some("0") {
        return;
    }
    eprintln!("needs root to write {} — re-run with sudo, or --dry-run to see the change", c.conf);
    exit(1);
}

fn backup(c: &Cfg, strict: bool) {
    let stamp = out("date", &["+%Y%m%d%H%M%S"]).unwrap_or_default();
    let ok = Command::new("cp")
        .args(["-a", &c.conf, &format!("{}.bak.{stamp}", c.conf)])
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if strict && !ok {
        eprintln!("setup-cuda-cache: could not back up {}", c.conf);
        exit(1);
    }
}

fn restart_daemon() {
    if !Command::new("systemctl").args(["restart", "nix-daemon"]).status().is_ok_and(|s| s.success()) {
        exit(1);
    }
    println!("nix-daemon restarted");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let arg = args.first().map(String::as_str).unwrap_or("");
    let c = cfg();
    match arg {
        "--help" | "-h" => print!("{USAGE}"),
        "--verify" => exit(verify(args.get(1).map(String::as_str))),
        "--doctor" => {
            let (o, found) = doctor(&c);
            print!("{o}");
            exit(i32::from(found));
        }
        "--fix-stale" => {
            need_root(&c);
            if !stale_configured(&c) {
                println!("{} does not name the dead cache; run --doctor to see where it is", c.conf);
                return;
            }
            backup(&c, true);
            edit(&c.conf, |t| strip_stale(&remove_block(t)));
            println!("removed {STALE_CACHE} from {} (previous version backed up alongside it)", c.conf);
            restart_daemon();
            print!("{}", doctor(&c).0);
        }
        "--remove" => {
            need_root(&c);
            backup(&c, false);
            edit(&c.conf, remove_block);
            println!("removed the CUDA cache block from {} (previous version backed up alongside it)", c.conf);
            restart_daemon();
        }
        _ => apply(&c, arg == "--dry-run"),
    }
}

fn apply(c: &Cfg, dry_run: bool) {
    if already_configured(c) && !stale_configured(c) {
        println!("already configured in {} — nothing to do", c.conf);
        println!("verify with: setup-cuda-cache --verify");
        return;
    }
    if dry_run {
        if stale_configured(c) {
            println!("would first remove the stale {STALE_CACHE} block from {} (it answers 401 and breaks nix develop)", c.conf);
        }
        println!("would append to {}:", c.conf);
        print!("{}", block(&c.user));
        println!("\nthen: systemctl restart nix-daemon");
        return;
    }
    need_root(c);
    backup(c, false);
    if stale_configured(c) {
        edit(&c.conf, |t| strip_stale(&remove_block(t)));
        println!("removed the stale {STALE_CACHE} from {}", c.conf);
    }
    if !already_configured(c) {
        use std::io::Write;
        let r = fs::OpenOptions::new().append(true).create(true).open(&c.conf).and_then(|mut f| f.write_all(block(&c.user).as_bytes()));
        if let Err(e) = r {
            eprintln!("setup-cuda-cache: {}: {e}", c.conf);
            exit(1);
        }
    }
    println!("appended to {} (previous version backed up alongside it)", c.conf);
    restart_daemon();
    println!("\nnow verify from the repo root — you want 'fetched', not 'built':\n  setup-cuda-cache --verify");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp(name: &str) -> PathBuf {
        let d = env::temp_dir().join(format!("setup-cuda-cache-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn count(text: &str, pat: &str) -> usize {
        text.lines().filter(|l| l.contains(pat)).count()
    }

    fn starts(text: &str, pat: &str) -> usize {
        text.lines().filter(|l| l.starts_with(pat)).count()
    }

    #[test]
    fn block_sets_one_substituter_trusted_users_and_a_live_key() {
        let b = block("me");
        assert_eq!(starts(&b, "extra-substituters"), 1);
        assert_eq!(starts(&b, "trusted-users"), 1, "without trusted-users the cache is ignored");
        assert_eq!(b.matches("nixos-cuda.org:").count(), 1);
        assert_eq!(count(&b, "cachix"), 0, "the dead Cachix cache is not named");
    }

    #[test]
    fn detects_applied_and_stale_blocks() {
        let d = tmp("detect");
        let mk = |text: &str| Cfg {
            conf: { fs::write(d.join("c"), text).unwrap(); d.join("c").display().to_string() },
            user_conf: String::new(), system_conf: String::new(), nix_config: String::new(), user: "me".into(), exe: String::new(),
        };
        assert!(already_configured(&mk(&block("me"))));
        assert!(stale_configured(&mk(&format!("extra-substituters = {STALE_CACHE}\n"))));
        let absent = Cfg { conf: "/definitely/absent".into(), ..mk("") };
        assert!(!already_configured(&absent), "a missing conf file is not mistaken for a configured one");
        assert!(!stale_configured(&absent));
    }

    #[test]
    fn remove_block_strips_either_vintage_and_keeps_the_rest() {
        let old = format!("keep-me = 1\n\n# marola: CUDA binary cache (old)\n# so the user must be listed here too\ntrusted-users = root me\nextra-substituters = {STALE_CACHE}\nextra-trusted-public-keys = cuda-maintainers.cachix.org-1:abc=\nkeep-me-too = 2\n");
        let s = remove_block(&old);
        assert_eq!(["cachix", "trusted-users", "CUDA binary cache"].iter().map(|p| count(&s, p)).sum::<usize>(), 0);
        assert_eq!(count(&s, "keep-me"), 2);
        let new = format!("# nix-config/labs/cuda: CUDA binary cache (x)\ntrusted-users = root me\nextra-substituters = {CACHE}\nextra-trusted-public-keys = k\n");
        assert_eq!(remove_block(&new), "");
    }

    #[test]
    fn strip_stale_keeps_other_entries_and_drops_emptied_lines() {
        let t = format!("substituters = https://cache.nixos.org/ {STALE_CACHE} https://nix-community.cachix.org\ntrusted-public-keys = a:1= cuda-maintainers.cachix.org-1:0dq= b:2=\nextra-substituters = {STALE_CACHE}\nkeep = 1\n");
        let s = strip_stale(&t);
        assert_eq!(count(&s, "cuda-maintainers"), 0);
        assert_eq!(starts(&s, "substituters = https://cache.nixos.org/ https://nix-community.cachix.org"), 1);
        assert_eq!(s.lines().filter(|l| *l == "substituters = https://cache.nixos.org/ https://nix-community.cachix.org").count(), 1);
        assert_eq!(s.lines().filter(|l| *l == "trusted-public-keys = a:1= b:2=").count(), 1);
        assert_eq!(starts(&s, "extra-substituters"), 0);
        assert_eq!(s.lines().filter(|l| *l == "keep = 1").count(), 1);
        assert_eq!(strip_url(&format!("x {STALE_CACHE}/")), "x");
        assert_eq!(strip_url(&format!("x {STALE_CACHE}/other y")), format!("x {STALE_CACHE}/other y"));
    }

    #[test]
    fn doctor_fails_on_the_dead_cache_and_passes_on_clean_config() {
        let d = tmp("doctor");
        let user = d.join("user.conf");
        let c = Cfg {
            conf: d.join("none").display().to_string(), system_conf: d.join("none").display().to_string(),
            user_conf: user.display().to_string(), nix_config: String::new(), user: "me".into(), exe: "/x/setup-cuda-cache".into(),
        };
        fs::write(&user, format!("substituters = {STALE_CACHE}\n")).unwrap();
        let (o, found) = doctor(&c);
        assert!(found, "the user's nix.conf names the dead cache");
        assert!(o.contains("1:substituters"), "{o}");
        fs::write(&user, "").unwrap();
        assert!(!doctor(&c).1);
        let c = Cfg { nix_config: format!("substituters = {STALE_CACHE}"), ..c };
        assert!(doctor(&c).1, "NIX_CONFIG counts too");
    }
}
