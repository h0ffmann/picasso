//! Benchmark ouroboric-sharingan with the `claude` CLI: trigger evals and output evals.
//!
//!     sharingan bench trigger [--runs 3] [--workers 4]   does the description make Claude load the skill?
//!     sharingan bench output  [--runs 2] [--ids 1,3]     each case with the skill and without it, graded
//!     sharingan bench all                                both; writes benchmark.json and benchmark.md
//!
//! Run it from the repo root (the skill is `.claude/skills/ouroboric-sharingan`; override with
//! SHARINGAN_SKILL_DIR). Results go to --out (default ~/.cache/ouroboric-sharingan/bench/<UTC time>).
//! Copy the summary into evals/BENCHMARK.md by hand. No model or effort is pinned unless --model
//! is given, so the numbers describe the CLI's default.
//!
//! The trigger detection follows skill-creator's run_eval.py: the first tool call decides. This is
//! a re-implementation, not a copy: the real skill directory is installed in a throwaway project.
//! The self-tests (no model calls) are `cargo test`.

use regex::Regex;
use serde_json::{Map, Value, json};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::{fs, thread};

use crate::licence::{TAG_COPYRIGHT, TAG_LICENSE};

const GIT_ENV: [(&str, &str); 6] = [
    ("GIT_AUTHOR_NAME", "Eval"),
    ("GIT_AUTHOR_EMAIL", "eval@example.invalid"),
    ("GIT_COMMITTER_NAME", "Eval"),
    ("GIT_COMMITTER_EMAIL", "eval@example.invalid"),
    ("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z"),
    ("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z"),
];
const TOOLS: [&str; 6] = ["Read", "Glob", "Grep", "Write", "Edit", "Skill"];
const BASH: [&str; 13] = [
    "git",
    "python3",
    "ls",
    "cat",
    "head",
    "find",
    "mkdir",
    "cp",
    "chmod",
    "diff",
    "sha256sum",
    "wc",
    "grep",
];

// CLAUDECODE guards an interactive terminal; a nested `claude -p` is fine. The session and
// messaging variables would tie the eval run to the calling session, so they are dropped too.
const DROP_ENV: [&str; 6] = [
    "CLAUDECODE",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_REMOTE_SESSION_ID",
    "CLAUDE_CODE_PROJECTS_SESSION",
    "CLAUDE_CODE_MESSAGING_SOCKET",
    "CLAUDE_CODE_MESSAGING_TOKEN",
];

fn claude_cmd() -> Command {
    let mut c = Command::new("claude");
    for k in DROP_ENV {
        c.env_remove(k);
    }
    c
}

pub struct Skill {
    pub dir: PathBuf,
    pub name: String,
}

impl Skill {
    fn from_env() -> Skill {
        let dir = std::env::var("SHARINGAN_SKILL_DIR").map_or_else(
            |_| PathBuf::from(".claude/skills/ouroboric-sharingan"),
            PathBuf::from,
        );
        let dir = dir.canonicalize().unwrap_or(dir);
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        Skill { dir, name }
    }
    fn evals(&self) -> PathBuf {
        self.dir.join("evals")
    }
    fn json(&self, file: &str) -> Value {
        let p = self.evals().join(file);
        serde_json::from_str(
            &fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display())),
        )
        .unwrap_or_else(|e| panic!("{}: {e}", p.display()))
    }
}

struct TempDir(PathBuf);
impl TempDir {
    fn new(prefix: &str) -> TempDir {
        static N: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "{prefix}{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&p).unwrap();
        TempDir(p)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// shutil.copytree minus the ignored names.
fn copy_tree(src: &Path, dst: &Path, ignore: &[&str]) {
    fs::create_dir_all(dst).unwrap();
    for e in fs::read_dir(src).unwrap().flatten() {
        let name = e.file_name();
        if ignore.contains(&name.to_string_lossy().as_ref()) {
            continue;
        }
        let to = dst.join(&name);
        if e.path().is_dir() {
            copy_tree(&e.path(), &to, ignore);
        } else {
            fs::copy(e.path(), &to).unwrap();
        }
    }
}

fn install_skill(s: &Skill, project: &Path) {
    copy_tree(
        &s.dir,
        &project.join(".claude/skills").join(&s.name),
        &["evals", "__pycache__"],
    );
}

fn git(repo: &Path, args: &[&str]) -> std::process::Output {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .envs(GIT_ENV)
        .output()
        .expect("git not found");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

fn git_commit_all(repo: &Path, msg: &str) -> String {
    if !repo.join(".git").exists() {
        git(repo, &["init", "-q", "-b", "main"]);
    }
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "--allow-empty", "-m", msg]);
    String::from_utf8_lossy(&git(repo, &["rev-parse", "HEAD"]).stdout)
        .trim()
        .to_string()
}

// --- the throwaway destination: a small picasso -----------------------------------------------

fn dest_files() -> Vec<(&'static str, String)> {
    let s = |x: &str| x.to_string();
    vec![
        (
            "LICENSE",
            s(
                "GNU AFFERO GENERAL PUBLIC LICENSE\nVersion 3, 19 November 2007\n\n(eval fixture: header only)\n",
            ),
        ),
        (
            "LICENSES/AGPL-3.0-only.txt",
            s("GNU AFFERO GENERAL PUBLIC LICENSE\nVersion 3, 19 November 2007\n"),
        ),
        (
            "LICENSES/CC-BY-4.0.txt",
            s("Attribution 4.0 International\n"),
        ),
        (
            "REUSE.toml",
            format!(
                "version = 1\n\n[[annotations]]\npath = [\"**\"]\nprecedence = \"closest\"\n\
             {TAG_COPYRIGHT} = \"2026 Example Owner\"\n{TAG_LICENSE} = \"AGPL-3.0-only\"\n\n\
             [[annotations]]\npath = [\"**/*.md\", \"docs/**\"]\nprecedence = \"closest\"\n\
             {TAG_COPYRIGHT} = \"2026 Example Owner\"\n{TAG_LICENSE} = \"CC-BY-4.0\"\n"
            ),
        ),
        (
            "README.md",
            s("# dest\n\nCode is AGPL-3.0-only, docs are CC BY 4.0 (REUSE.toml).\n"),
        ),
        (
            "AGENTS.md",
            s(
                "# AGENTS.md\n\nCode is AGPL-3.0-only, documentation is CC BY 4.0; REUSE.toml says which \
             file is which.\n\n## Files that need a human's approval (hard rule)\n\n\
             An agent changes these only when the human asks for that change by name. Otherwise propose \
             the diff and wait.\n\n- `LICENSE`, `LICENSES/`, `REUSE.toml`\n- `.github/workflows/`\n\
             - `AGENTS.md`, `CLAUDE.md`\n\nVendored skills keep their upstream licence and are listed in \
             `.claude/skills/README.md`.\n",
            ),
        ),
        ("CLAUDE.md", s("@AGENTS.md\n")),
        (
            ".claude/skills/README.md",
            s(
                "# Vendored agent skills\n\nEach `SKILL.md` is the upstream file with one note under its \
             front matter: where it came from.\n\n| Skill | Upstream | Commit | Licence | Use it for |\n\
             |---|---|---|---|---|\n| `nix` | [someone/nix-skill](https://github.com/someone/nix-skill) \
             | `c9a423b2af83` | MIT | Nix reference |\n\nLicence texts are in `LICENSES/` \
             (`<owner>-<repo>.<SPDX id>`).\n",
            ),
        ),
        (
            ".claude/skills/LICENSES/someone-nix-skill.MIT",
            s("MIT License\n\nCopyright (c) 2026 Someone\n"),
        ),
        (
            ".claude/skills/nix/SKILL.md",
            s(
                "---\nname: nix\ndescription: Nix reference.\n---\n\n> Vendored from someone/nix-skill.\n",
            ),
        ),
        ("docs/index.md", s("# docs\n")),
        (
            ".github/workflows/ci.yml",
            s(
                "name: ci\non: [push]\njobs:\n  ok:\n    runs-on: ubuntu-latest\n    steps:\n      - run: true\n",
            ),
        ),
    ]
}

/// The fixture repos and the destination, under `root`; returns the destination and each
/// fixture repo's commit (by sorted name).
fn build_workspace(s: &Skill, root: &Path, with_skill: bool) -> (PathBuf, Map<String, Value>) {
    let acme = root.join("acme");
    copy_tree(&s.evals().join("files/acme"), &acme, &[]);
    let mut repos: Vec<PathBuf> = fs::read_dir(&acme)
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    repos.sort();
    let shas = repos
        .iter()
        .map(|r| {
            let n = r.file_name().unwrap().to_string_lossy().into_owned();
            let sha = git_commit_all(r, &format!("{n}: initial"));
            (n, Value::String(sha))
        })
        .collect();
    let dest = root.join("dest");
    for (rel, text) in dest_files() {
        let p = dest.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }
    if with_skill {
        install_skill(s, &dest);
    }
    git_commit_all(&dest, "dest: initial");
    (dest, shas)
}

fn run_checks(dest: &Path, checks: &[Value]) -> Vec<Value> {
    let status = String::from_utf8_lossy(&git(dest, &["status", "--porcelain"]).stdout)
        .trim()
        .to_string();
    checks
        .iter()
        .map(|c| {
            let kind = c["type"].as_str().unwrap_or("");
            let path = c.get("path").and_then(Value::as_str).unwrap_or("");
            let (ok, why) = match kind {
                "diff_empty" => (
                    status.is_empty(),
                    if status.is_empty() {
                        "working tree clean".to_string()
                    } else {
                        status.clone()
                    },
                ),
                "exists" => (dest.join(path).exists(), path.to_string()),
                "unchanged" => {
                    let same = Command::new("git")
                        .arg("-C")
                        .arg(dest)
                        .args(["diff", "--quiet", "HEAD", "--", path])
                        .status()
                        .map(|s| s.success())
                        .unwrap_or(false);
                    (
                        same,
                        format!("{path} {}", if same { "unchanged" } else { "changed" }),
                    )
                }
                "not_identical" => {
                    let a = dest.join(path);
                    let b = dest
                        .join(c["to"].as_str().unwrap_or(""))
                        .canonicalize()
                        .ok();
                    let ok = !a.exists() || fs::read(&a).ok() != b.and_then(|b| fs::read(b).ok());
                    let what = if !a.exists() {
                        "absent"
                    } else if ok {
                        "differs"
                    } else {
                        "is a verbatim copy"
                    };
                    (ok, format!("{path} {what}"))
                }
                _ => (false, format!("unknown check {kind}")),
            };
            json!({ "text": format!("check {kind} {path}").trim(), "passed": ok, "evidence": why })
        })
        .collect()
}

// --- running claude ---------------------------------------------------------------------------

struct Run {
    result: String,
    skill_loaded: bool,
    tools: Vec<String>,
    seconds: f64,
    cost_usd: Option<f64>,
    turns: Value,
    models: Vec<String>,
    output_tokens: Option<u64>,
    input_tokens: u64,
}

fn run_claude(
    s: &Skill,
    prompt: &str,
    cwd: &Path,
    add_dirs: &[PathBuf],
    model: Option<&str>,
    timeout: u64,
    tools: Option<&[&str]>,
) -> Run {
    let mut cmd = claude_cmd();
    cmd.args([
        "-p",
        prompt,
        "--output-format",
        "stream-json",
        "--verbose",
        "--no-session-persistence",
        "--strict-mcp-config",
        "--permission-mode",
        "acceptEdits",
        "--disallowedTools",
        "WebFetch",
        "WebSearch",
        "Agent",
    ]);
    match tools {
        Some(t) => {
            cmd.args(["--tools", &t.join(",")]);
        }
        None => {
            cmd.arg("--allowedTools").args(TOOLS);
            cmd.args(BASH.iter().map(|c| format!("Bash({c}:*)")));
        }
    }
    for d in add_dirs {
        cmd.arg("--add-dir").arg(d);
    }
    if let Some(m) = model {
        cmd.args(["--model", m]);
    }
    let t0 = Instant::now();
    let mut child = cmd
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("the claude CLI could not be started");
    let mut out = child.stdout.take().unwrap();
    let reader = thread::spawn(move || {
        let mut b = vec![];
        let _ = out.read_to_end(&mut b);
        b
    });
    while child.try_wait().ok().flatten().is_none() {
        if t0.elapsed() > Duration::from_secs(timeout) {
            let _ = child.kill();
            let _ = child.wait();
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    let raw = String::from_utf8_lossy(&reader.join().unwrap_or_default()).into_owned();
    let seconds = (t0.elapsed().as_secs_f64() * 10.0).round() / 10.0;
    let (mut data, mut tools_used) = (Value::Null, vec![]);
    for line in raw.lines() {
        let Ok(ev) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match ev["type"].as_str() {
            Some("result") => data = ev,
            Some("assistant") => {
                for c in ev["message"]["content"].as_array().into_iter().flatten() {
                    if c["type"] == "tool_use" {
                        let name = c["name"].as_str().unwrap_or("");
                        tools_used.push(match (name, c["input"]["skill"].as_str()) {
                            ("Skill", Some(k)) => format!("{name}:{k}"),
                            ("Skill", None) => format!("{name}:None"),
                            _ => name.to_string(),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    let usage = &data["usage"];
    let n = |k: &str| usage[k].as_u64();
    let result = data["result"]
        .as_str()
        .filter(|r| !r.is_empty())
        .map_or_else(
            || {
                let tail: Vec<char> = raw.chars().rev().take(2000).collect();
                format!("(no result) {}", tail.into_iter().rev().collect::<String>())
            },
            String::from,
        );
    Run {
        result,
        skill_loaded: tools_used.contains(&format!("Skill:{}", s.name)),
        tools: tools_used,
        seconds,
        cost_usd: data["total_cost_usd"].as_f64(),
        turns: data["num_turns"].clone(),
        models: {
            let mut m: Vec<String> = data["modelUsage"]
                .as_object()
                .map(|o| o.keys().cloned().collect())
                .unwrap_or_default();
            m.sort();
            m
        },
        output_tokens: n("output_tokens"),
        input_tokens: [
            "input_tokens",
            "cache_read_input_tokens",
            "cache_creation_input_tokens",
        ]
        .iter()
        .map(|k| n(k).unwrap_or(0))
        .sum(),
    }
}

fn trigger_once(s: &Skill, query: &str, model: Option<&str>, timeout: u64) -> bool {
    let tmp = TempDir::new("sharingan-trigger-");
    install_skill(s, &tmp.0);
    let mut cmd = claude_cmd();
    cmd.args([
        "-p",
        query,
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
        "--no-session-persistence",
        "--strict-mcp-config",
    ]);
    if let Some(m) = model {
        cmd.args(["--model", m]);
    }
    let Ok(mut child) = cmd
        .current_dir(&tmp.0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let mut out = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    thread::spawn(move || {
        let mut b = [0u8; 65536];
        while let Ok(n) = out.read(&mut b) {
            if n == 0 || tx.send(b[..n].to_vec()).is_err() {
                break;
            }
        }
    });
    let hit = (|| {
        let (mut buf, mut pending, mut args) = (Vec::<u8>::new(), None::<String>, String::new());
        let t0 = Instant::now();
        while t0.elapsed() < Duration::from_secs(timeout) {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(chunk) => buf.extend(chunk),
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if child.try_wait().ok().flatten().is_some() {
                        return false;
                    }
                    continue;
                }
                Err(_) => return false,
            }
            while let Some(i) = buf.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = buf.drain(..=i).collect();
                let Ok(ev) = serde_json::from_slice::<Value>(&line) else {
                    continue;
                };
                match ev["type"].as_str() {
                    Some("stream_event") => {
                        let se = &ev["event"];
                        match se["type"].as_str() {
                            Some("content_block_start")
                                if se["content_block"]["type"] == "tool_use" =>
                            {
                                let name = se["content_block"]["name"].as_str().unwrap_or("");
                                if name != "Skill" && name != "Read" {
                                    return false;
                                }
                                pending = Some(name.to_string());
                                args.clear();
                            }
                            Some("content_block_delta") if pending.is_some() => {
                                args += se["delta"]["partial_json"].as_str().unwrap_or("");
                                if args.contains(&s.name) {
                                    return true;
                                }
                            }
                            Some("content_block_stop") if pending.is_some() => {
                                return args.contains(&s.name);
                            }
                            Some("message_stop") if pending.is_none() => return false,
                            _ => {}
                        }
                    }
                    Some("result") => return false,
                    _ => {}
                }
            }
        }
        false
    })();
    let _ = child.kill();
    let _ = child.wait();
    hit
}

fn take(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn grade(
    s: &Skill,
    case: &Value,
    answer: &str,
    diff: &str,
    model: Option<&str>,
    timeout: u64,
) -> Vec<Value> {
    let expectations: Vec<&str> = case["expectations"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .collect();
    let diff = take(diff, 30000);
    let prompt = format!(
        "You grade one run of an AI coding agent against a list of expectations.\n\
Judge only from the evidence below: the agent's final answer and the git diff of the repository it\n\
worked in. An expectation passes only if the evidence shows it clearly; a vague or partial answer\n\
fails. Do not use tools. Reply with JSON only, no prose, in this shape:\n\
{{\"expectations\": [{{\"text\": \"<expectation>\", \"passed\": true, \"evidence\": \"<short quote or reason>\"}}]}}\n\
\n\
TASK GIVEN TO THE AGENT:\n{}\n\
\n\
EXPECTATIONS:\n{}\n\
\n\
AGENT'S FINAL ANSWER:\n{}\n\
\n\
GIT STATUS AND DIFF OF THE REPOSITORY AFTER THE RUN (empty means nothing changed):\n{}\n",
        case["prompt"].as_str().unwrap_or(""),
        expectations
            .iter()
            .map(|e| format!("- {e}"))
            .collect::<Vec<_>>()
            .join("\n"),
        take(answer, 40000),
        if diff.is_empty() { "(empty)" } else { &diff },
    );
    let tmp = TempDir::new("sharingan-grade-");
    let r = run_claude(s, &prompt, &tmp.0, &[], model, timeout, Some(&[]));
    let got = Regex::new(r"(?s)\{.*\}")
        .unwrap()
        .find(&r.result)
        .and_then(|m| serde_json::from_str::<Value>(m.as_str()).ok())
        .and_then(|v| v["expectations"].as_array().cloned())
        .unwrap_or_default();
    if got.len() != expectations.len() {
        return expectations
            .iter()
            .map(|e| json!({"text": e, "passed": false, "evidence": "grader output unusable"}))
            .collect();
    }
    expectations.iter().zip(&got).map(|(e, g)| json!({
        "text": e, "passed": g["passed"].as_bool().unwrap_or(false), "evidence": g["evidence"].as_str().unwrap_or("")
    })).collect()
}

fn output_once(
    s: &Skill,
    case: &Value,
    with_skill: bool,
    run_dir: &Path,
    model: Option<&str>,
    timeout: u64,
) -> Value {
    let tmp = TempDir::new("sharingan-output-");
    let (dest, shas) = build_workspace(s, &tmp.0, with_skill);
    let r = run_claude(
        s,
        case["prompt"].as_str().unwrap_or(""),
        &dest,
        &[tmp.0.join("acme")],
        model,
        timeout,
        None,
    );
    git(&dest, &["add", "-A"]);
    let exclude = format!(":!.claude/skills/{}", s.name);
    let diff = String::from_utf8_lossy(
        &git(
            &dest,
            &["diff", "--cached", "--stat", "-p", "--", ".", &exclude],
        )
        .stdout,
    )
    .into_owned();
    git(&dest, &["reset", "-q"]);
    let checks = run_checks(&dest, case["checks"].as_array().map_or(&[][..], |v| v));
    let mut items = grade(s, case, &r.result, &diff, model, timeout);
    items.extend(checks);
    fs::create_dir_all(run_dir).unwrap();
    fs::write(run_dir.join("response.md"), &r.result).unwrap();
    fs::write(run_dir.join("diff.patch"), &diff).unwrap();
    let passed = items.iter().filter(|i| i["passed"] == true).count();
    let total = items.len();
    let result = json!({
        "eval": case["name"],
        "config": if with_skill { "with_skill" } else { "without_skill" },
        "pass_rate": (passed as f64 / total.max(1) as f64 * 1000.0).round() / 1000.0,
        "passed": passed, "total": total, "items": items,
        "upstream_commits": shas,
        "skill_loaded": r.skill_loaded, "tools": r.tools, "seconds": r.seconds, "cost_usd": r.cost_usd,
        "turns": r.turns, "models": r.models, "input_tokens": r.input_tokens, "output_tokens": r.output_tokens,
    });
    fs::write(run_dir.join("grading.json"), crate::licence::dumps(&result)).unwrap();
    result
}

// --- commands ---------------------------------------------------------------------------------

fn stats(xs: &[Option<f64>]) -> String {
    let xs: Vec<f64> = xs.iter().flatten().copied().collect();
    let n = xs.len() as f64;
    match xs.len() {
        0 => "–".into(),
        1 => format!("{:.2}", xs[0]),
        _ => {
            let m = xs.iter().sum::<f64>() / n;
            let sd = (xs.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
            format!("{m:.2} ± {sd:.2}")
        }
    }
}

/// ThreadPoolExecutor.map: results in input order.
fn par_map<T: Sync, R: Send>(items: &[T], workers: usize, f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<Option<R>>> = Mutex::new(items.iter().map(|_| None).collect());
    thread::scope(|sc| {
        for _ in 0..workers.max(1) {
            sc.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::SeqCst);
                    if i >= items.len() {
                        break;
                    }
                    let r = f(&items[i]);
                    out.lock().unwrap()[i] = Some(r);
                }
            });
        }
    });
    out.into_inner()
        .unwrap()
        .into_iter()
        .map(Option::unwrap)
        .collect()
}

struct Opts {
    mode: String,
    runs: Option<usize>,
    workers: usize,
    timeout: Option<u64>,
    ids: Option<String>,
    model: Option<String>,
    out: Option<PathBuf>,
}

fn cmd_trigger(s: &Skill, a: &Opts, runs: usize, timeout: u64, out: &Path) -> Value {
    let cases = s.json("trigger_evals.json");
    let cases = cases.as_array().unwrap();
    let jobs: Vec<(usize, usize)> = (0..cases.len())
        .flat_map(|c| (0..runs).map(move |i| (c, i)))
        .collect();
    let hits = par_map(&jobs, a.workers, |(c, _)| {
        trigger_once(
            s,
            cases[*c]["query"].as_str().unwrap_or(""),
            a.model.as_deref(),
            timeout,
        )
    });
    let rows: Vec<Value> = cases
        .iter()
        .enumerate()
        .map(|(ci, c)| {
            let got: Vec<bool> = jobs
                .iter()
                .zip(&hits)
                .filter(|((j, _), _)| *j == ci)
                .map(|(_, h)| *h)
                .collect();
            let rate = got.iter().filter(|h| **h).count() as f64 / got.len() as f64;
            let mut row = c.as_object().unwrap().clone();
            row.insert("trigger_rate".into(), json!(rate));
            row.insert(
                "pass".into(),
                json!((rate >= 0.5) == (c["should_trigger"] == true)),
            );
            Value::Object(row)
        })
        .collect();
    let count = |f: &dyn Fn(bool, bool) -> bool| {
        rows.iter()
            .filter(|r| f(r["pass"] == true, r["should_trigger"] == true))
            .count()
    };
    let res = json!({
        "runs": runs, "cases": rows, "passed": count(&|p, _| p), "total": rows.len(),
        "precision_recall": {
            "true_positive": count(&|p, t| p && t),
            "false_negative": count(&|p, t| !p && t),
            "false_positive": count(&|p, t| !p && !t),
        },
    });
    fs::write(out.join("trigger.json"), crate::licence::dumps(&res)).unwrap();
    eprintln!(
        "trigger: {}/{} ({})",
        res["passed"], res["total"], res["precision_recall"]
    );
    res
}

fn cmd_output(s: &Skill, a: &Opts, runs: usize, timeout: u64, out: &Path) -> Value {
    let cases = s.json("evals.json")["evals"].as_array().unwrap().clone();
    let cases: Vec<Value> = match &a.ids {
        Some(ids) => cases
            .into_iter()
            .filter(|c| ids.split(',').any(|i| c["id"] == i))
            .collect(),
        None => cases,
    };
    let jobs: Vec<(usize, bool, usize)> = (0..cases.len())
        .flat_map(|c| {
            [true, false]
                .into_iter()
                .flat_map(move |w| (0..runs).map(move |i| (c, w, i)))
        })
        .collect();
    let results = par_map(&jobs, a.workers, |(ci, w, i)| {
        let c = &cases[*ci];
        let d = out
            .join(c["name"].as_str().unwrap_or(""))
            .join(if *w { "with_skill" } else { "without_skill" })
            .join(format!("run-{}", i + 1));
        let r = output_once(s, c, *w, &d, a.model.as_deref(), timeout);
        eprintln!(
            "  {:<20} {:<14} {}/{}",
            c["name"].as_str().unwrap_or(""),
            r["config"].as_str().unwrap_or(""),
            r["passed"],
            r["total"]
        );
        r
    });
    let mut summary = Map::new();
    for cfg in ["with_skill", "without_skill"] {
        let rs: Vec<&Value> = results.iter().filter(|r| r["config"] == cfg).collect();
        let col = |k: &str| stats(&rs.iter().map(|r| r[k].as_f64()).collect::<Vec<_>>());
        let mut models: Vec<String> = rs
            .iter()
            .flat_map(|r| {
                r["models"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|m| m.as_str().map(String::from))
            })
            .collect();
        models.sort();
        models.dedup();
        summary.insert(cfg.into(), json!({
            "pass_rate": col("pass_rate"),
            "skill_loaded": format!("{}/{}", rs.iter().filter(|r| r["skill_loaded"] == true).count(), rs.len()),
            "seconds": col("seconds"), "cost_usd": col("cost_usd"), "output_tokens": col("output_tokens"),
            "models": models,
        }));
    }
    let res = json!({ "runs": runs, "summary": summary, "results": results });
    fs::write(out.join("output.json"), crate::licence::dumps(&res)).unwrap();
    res
}

/// (year, month, day, hour, minute, second) in UTC.
fn utc_now() -> (i64, u32, u32, u64, u64, u64) {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let z = (secs / 86400) as i64 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    let t = secs % 86400;
    (y, m, d, t / 3600, t % 3600 / 60, t % 60)
}

fn markdown(trig: Option<&Value>, outp: Option<&Value>) -> String {
    let (y, mo, d, h, mi, _) = utc_now();
    let version = Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    let mut lines = vec![format!(
        "Run {y}-{mo:02}-{d:02} {h:02}:{mi:02} UTC with `claude` {version}."
    )];
    if let Some(t) = trig {
        let pr = &t["precision_recall"];
        lines.push(String::new());
        lines.push(format!(
            "**Trigger:** {}/{} queries right ({} run(s) each; {} missed, {} false triggers).",
            t["passed"], t["total"], t["runs"], pr["false_negative"], pr["false_positive"]
        ));
        lines.extend([
            "".into(),
            "| Query | Should | Rate |".into(),
            "|---|---|---|".into(),
        ]);
        for r in t["cases"].as_array().unwrap() {
            lines.push(format!(
                "| {} | {} | {:.2}{} |",
                r["query"].as_str().unwrap_or(""),
                if r["should_trigger"] == true {
                    "yes"
                } else {
                    "no"
                },
                r["trigger_rate"].as_f64().unwrap_or(0.0),
                if r["pass"] == true { "" } else { " ✗" }
            ));
        }
    }
    if let Some(o) = outp {
        let sm = &o["summary"];
        let mut models: Vec<&str> = ["with_skill", "without_skill"]
            .iter()
            .flat_map(|c| {
                sm[c]["models"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(Value::as_str)
            })
            .collect();
        models.sort();
        models.dedup();
        lines.extend([
            String::new(),
            format!(
                "**Output** ({} run(s) per case and configuration; models: {}):",
                o["runs"],
                if models.is_empty() {
                    "unknown".to_string()
                } else {
                    models.join(", ")
                }
            ),
            String::new(),
            "| Configuration | Pass rate | Skill loaded | Seconds | Cost (USD) | Output tokens |"
                .into(),
            "|---|---|---|---|---|---|".into(),
        ]);
        for cfg in ["with_skill", "without_skill"] {
            let x = &sm[cfg];
            let g = |k: &str| x[k].as_str().unwrap_or("").to_string();
            lines.push(format!(
                "| {cfg} | {} | {} | {} | {} | {} |",
                g("pass_rate"),
                g("skill_loaded"),
                g("seconds"),
                g("cost_usd"),
                g("output_tokens")
            ));
        }
        lines.extend([
            String::new(),
            "| Case | With skill | Without skill |".into(),
            "|---|---|---|".into(),
        ]);
        let results = o["results"].as_array().unwrap();
        let mut names: Vec<&str> = vec![];
        for r in results {
            let n = r["eval"].as_str().unwrap_or("");
            if !names.contains(&n) {
                names.push(n);
            }
        }
        for name in names {
            let cell = |cfg: &str| {
                results
                    .iter()
                    .filter(|r| r["eval"] == name && r["config"] == cfg)
                    .map(|r| format!("{}/{}", r["passed"], r["total"]))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            lines.push(format!(
                "| {name} | {} | {} |",
                cell("with_skill"),
                cell("without_skill")
            ));
        }
        let fails: Vec<String> = results
            .iter()
            .filter(|r| r["config"] == "with_skill")
            .flat_map(|r| {
                r["items"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|i| i["passed"] != true)
                    .map(move |i| {
                        format!(
                            "- {}: {}",
                            r["eval"].as_str().unwrap_or(""),
                            i["text"].as_str().unwrap_or("")
                        )
                    })
            })
            .collect();
        if !fails.is_empty() {
            lines.extend([String::new(), "Failed with the skill:".into()]);
            lines.extend(fails);
        }
    }
    lines.join("\n") + "\n"
}

fn parse(args: &[String]) -> Result<Opts, String> {
    let mut o = Opts {
        mode: String::new(),
        runs: None,
        workers: 4,
        timeout: None,
        ids: None,
        model: None,
        out: None,
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let (flag, inline) = match a.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f, Some(v.to_string())),
            _ => (a.as_str(), None),
        };
        if flag.starts_with("--") {
            let v = inline
                .or_else(|| it.next().cloned())
                .ok_or(format!("{flag} needs a value"))?;
            let num = |v: &str| {
                v.parse::<u64>()
                    .map_err(|_| format!("{flag}: invalid int value: '{v}'"))
            };
            match flag {
                "--runs" => o.runs = Some(num(&v)? as usize),
                "--workers" => o.workers = num(&v)? as usize,
                "--timeout" => o.timeout = Some(num(&v)?),
                "--ids" => o.ids = Some(v),
                "--model" => o.model = Some(v),
                "--out" => o.out = Some(PathBuf::from(v)),
                _ => return Err(format!("unrecognized arguments: {flag}")),
            }
        } else if ["trigger", "output", "all"].contains(&a.as_str()) && o.mode.is_empty() {
            o.mode = a.clone();
        } else {
            return Err(format!(
                "invalid choice: '{a}' (choose from trigger, output, all)"
            ));
        }
    }
    if o.mode.is_empty() {
        Err("give trigger, output or all".into())
    } else {
        Ok(o)
    }
}

pub fn main(args: &[String]) -> i32 {
    let a = match parse(args) {
        Ok(a) => a,
        Err(e) => {
            eprintln!(
                "usage: sharingan bench trigger|output|all [--runs N] [--workers N] [--timeout S] [--ids 1,3] [--model M] [--out DIR]\nerror: {e}"
            );
            return 2;
        }
    };
    if Command::new("claude")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_err()
    {
        eprintln!("the claude CLI is not on PATH");
        return 1;
    }
    let s = Skill::from_env();
    let out = a.out.clone().unwrap_or_else(|| {
        let (y, mo, d, h, mi, se) = utc_now();
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home)
            .join(".cache")
            .join(&s.name)
            .join("bench")
            .join(format!("{y}{mo:02}{d:02}T{h:02}{mi:02}{se:02}Z"))
    });
    fs::create_dir_all(&out).unwrap();
    let trig = ["trigger", "all"]
        .contains(&a.mode.as_str())
        .then(|| cmd_trigger(&s, &a, a.runs.unwrap_or(3), a.timeout.unwrap_or(90), &out));
    let outp = ["output", "all"]
        .contains(&a.mode.as_str())
        .then(|| cmd_output(&s, &a, a.runs.unwrap_or(2), a.timeout.unwrap_or(1200), &out));
    let md = markdown(trig.as_ref(), outp.as_ref());
    fs::write(out.join("benchmark.md"), &md).unwrap();
    println!("{md}");
    eprintln!("results: {}", out.display());
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn skill() -> Skill {
        Skill {
            dir: Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../.claude/skills/ouroboric-sharingan")
                .canonicalize()
                .unwrap(),
            name: "ouroboric-sharingan".into(),
        }
    }

    #[test]
    fn eval_files_are_consistent() {
        let s = skill();
        let ev = s.json("evals.json");
        assert_eq!(ev["skill_name"], s.name.as_str());
        let evals = ev["evals"].as_array().unwrap();
        let mut ids: Vec<String> = evals.iter().map(|e| e["id"].to_string()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), evals.len(), "duplicate eval ids");
        for e in evals {
            for k in ["id", "name", "prompt", "expected_output", "expectations"] {
                assert!(e.get(k).is_some(), "eval {}: missing {k}", e["id"]);
            }
            for f in e["files"].as_array().into_iter().flatten() {
                assert!(
                    s.evals().join(f.as_str().unwrap()).exists(),
                    "eval {}: missing fixture {f}",
                    e["id"]
                );
            }
            for c in e["checks"].as_array().into_iter().flatten() {
                assert!(
                    ["diff_empty", "exists", "unchanged", "not_identical"]
                        .contains(&c["type"].as_str().unwrap()),
                    "unknown check {}",
                    c["type"]
                );
            }
        }
        let trig = s.json("trigger_evals.json");
        let trig = trig.as_array().unwrap();
        let pos = trig.iter().filter(|t| t["should_trigger"] == true).count();
        assert!(
            pos > 0 && trig.len() > pos,
            "trigger set needs positive and negative queries"
        );
        let mut q: Vec<&str> = trig.iter().map(|t| t["query"].as_str().unwrap()).collect();
        q.sort();
        q.dedup();
        assert_eq!(q.len(), trig.len(), "duplicate trigger queries");
    }

    #[test]
    fn workspace_and_checks() {
        let s = skill();
        let tmp = TempDir::new("sharingan-test-");
        let (dest, shas) = build_workspace(&s, &tmp.0, true);
        let fixtures: BTreeMap<String, ()> = fs::read_dir(s.evals().join("files/acme"))
            .unwrap()
            .flatten()
            .map(|e| (e.file_name().to_string_lossy().into_owned(), ()))
            .collect();
        assert_eq!(
            shas.keys().cloned().collect::<Vec<_>>(),
            fixtures.into_keys().collect::<Vec<_>>(),
            "every fixture repo gets a commit"
        );
        assert!(
            dest.join(".claude/skills")
                .join(&s.name)
                .join("SKILL.md")
                .exists(),
            "skill installed in the destination"
        );
        assert!(
            !dest
                .join(".claude/skills")
                .join(&s.name)
                .join("evals")
                .exists(),
            "evals must not be installed with the skill"
        );
        let checks = [
            json!({"type": "diff_empty"}),
            json!({"type": "unchanged", "path": "REUSE.toml"}),
            json!({"type": "not_identical", "path": ".github/workflows/release.yml", "to": "../acme/acme-ci/.github/workflows/release.yml"}),
        ];
        let base = run_checks(&dest, &checks);
        assert!(
            base.iter().all(|c| c["passed"] == true),
            "clean workspace passes its checks: {base:?}"
        );
        fs::write(dest.join("REUSE.toml"), "changed\n").unwrap();
        fs::create_dir_all(dest.join(".github/workflows")).unwrap();
        fs::copy(
            dest.parent()
                .unwrap()
                .join("acme/acme-ci/.github/workflows/release.yml"),
            dest.join(".github/workflows/release.yml"),
        )
        .unwrap();
        let after = run_checks(&dest, &checks);
        assert!(
            after.iter().all(|c| c["passed"] != true),
            "changed workspace fails its checks: {after:?}"
        );
    }

    #[test]
    fn stats_format() {
        assert_eq!(stats(&[]), "–");
        assert_eq!(stats(&[Some(1.0)]), "1.00");
        assert_eq!(stats(&[Some(1.0), Some(3.0), None]), "2.00 ± 1.41");
    }
}
