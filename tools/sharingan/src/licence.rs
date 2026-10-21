//! Licence and tooling inventory of local checkouts, for ouroboric-sharingan.
//!
//!     sharingan licence-scan [--into SPDX-ID] DIR...   one JSON object per checkout, on stdout
//!     sharingan licence-scan --matrix                  the verdict matrix as the Markdown table
//!
//! It reads files only. It never runs, installs or imports anything from the checkouts, so it is
//! safe on an untrusted clone. (`git` is run read-only for the commit and the remote.)

use regex::Regex;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::LazyLock;

// Split so that `reuse lint` does not read these strings as this file's own licence tags.
pub const TAG_LICENSE: &str = concat!("SPDX-", "License-Identifier");
pub const TAG_COPYRIGHT: &str = concat!("SPDX-", "FileCopyrightText");

const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "vendor/bundle",
    ".venv",
    "venv",
    "target",
    "dist",
    "build",
    "result",
    "__pycache__",
    ".direnv",
    "_build",
];
const MAX_BYTES: u64 = 256 * 1024;

pub const CLASSES: [&str; 14] = [
    "permissive",
    "Apache-2.0",
    "MPL-2.0",
    "LGPL",
    "GPL-2.0-only",
    "GPL-2.0-or-later",
    "GPL-3.0",
    "AGPL-3.0",
    "CC-BY-4.0",
    "CC-BY-SA-4.0",
    "CC-NC-ND",
    "source-available",
    "none",
    "unknown",
];
const DESTS: [&str; 5] = [
    "permissive",
    "Apache-2.0",
    "GPL-3.0",
    "AGPL-3.0",
    "CC-BY-4.0",
];

// reuse: copy it, keep the notice. reuse-file: copy it, but that file keeps its own licence and
// notice. borrow: re-implement the idea in your own words and code, with a credit line; copy
// nothing. cite: link and describe only. references/licences.md carries this table verbatim; a
// test fails if they differ.
pub const MATRIX: [[&str; 5]; 14] = [
    ["reuse", "reuse", "reuse", "reuse", "reuse-file"],
    ["reuse-file", "reuse", "reuse", "reuse", "reuse-file"],
    [
        "reuse-file",
        "reuse-file",
        "reuse-file",
        "reuse-file",
        "reuse-file",
    ],
    ["borrow", "borrow", "reuse-file", "reuse-file", "borrow"],
    ["borrow", "borrow", "borrow", "borrow", "borrow"],
    ["borrow", "borrow", "reuse", "reuse", "borrow"],
    ["borrow", "borrow", "reuse", "reuse", "borrow"],
    ["borrow", "borrow", "reuse-file", "reuse", "borrow"],
    [
        "reuse-file",
        "reuse-file",
        "reuse-file",
        "reuse-file",
        "reuse",
    ],
    [
        "reuse-file",
        "reuse-file",
        "reuse-file",
        "reuse-file",
        "reuse-file",
    ],
    ["cite", "cite", "cite", "cite", "cite"],
    ["cite", "cite", "cite", "cite", "cite"],
    ["cite", "cite", "cite", "cite", "cite"],
    ["cite", "cite", "cite", "cite", "cite"],
];

fn rank(v: &str) -> u8 {
    match v {
        "cite" => 0,
        "borrow" => 1,
        "reuse-file" => 2,
        _ => 3,
    }
}

fn row(class: &str) -> &'static [&'static str; 5] {
    &MATRIX[CLASSES.iter().position(|c| *c == class).unwrap()]
}

const PERMISSIVE: &[&str] = &[
    "MIT",
    "MIT-0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "0BSD",
    "Zlib",
    "Unlicense",
    "CC0-1.0",
    "BSL-1.0",
    "PSF-2.0",
    "Python-2.0",
    "WTFPL",
    "X11",
    "Artistic-2.0",
];
const SOURCE_AVAILABLE: &[&str] = &[
    "BUSL-1.1",
    "SSPL-1.0",
    "Elastic-2.0",
    "Commons-Clause",
    "PolyForm",
    "LicenseRef-Commons-Clause",
];

fn re(p: &str) -> Regex {
    Regex::new(p).unwrap()
}

static CC_NC: LazyLock<Regex> = LazyLock::new(|| re(r"^CC-BY-(NC|ND|NC-SA|NC-ND)-[\d.]+$"));
static BASE_ID: LazyLock<Regex> = LazyLock::new(|| re(r"(-only|-or-later|\+)$"));

/// One SPDX id (no operators) to its class.
pub fn classify(spdx: &str) -> &'static str {
    if spdx.is_empty() {
        return "none";
    }
    let t = spdx.trim();
    let s = t.strip_suffix('+').unwrap_or(t);
    if PERMISSIVE.contains(&s) {
        "permissive"
    } else if s == "Apache-2.0" {
        "Apache-2.0"
    } else if s.starts_with("MPL-2.0") {
        "MPL-2.0"
    } else if s.starts_with("LGPL-") {
        "LGPL"
    } else if s.starts_with("AGPL-3.0") {
        "AGPL-3.0"
    } else if s.starts_with("GPL-3.0") {
        "GPL-3.0"
    } else if s == "GPL-2.0-or-later" || t == "GPL-2.0+" {
        "GPL-2.0-or-later"
    } else if s.starts_with("GPL-2.0") {
        "GPL-2.0-only" // a bare GPL-2.0 text says nothing about "or later": assume -only
    } else if CC_NC.is_match(s) {
        "CC-NC-ND"
    } else if s.starts_with("CC-BY-SA-4.0") {
        "CC-BY-SA-4.0"
    } else if s.starts_with("CC-BY-4.0") {
        "CC-BY-4.0"
    } else if SOURCE_AVAILABLE.contains(&s) || s.starts_with("PolyForm") || s.starts_with("BUSL") {
        "source-available"
    } else if s == "NONE" {
        "none"
    } else {
        "unknown"
    }
}

/// A licence text cannot say -only or -or-later; compare ids without it.
pub fn base_id(spdx: &str) -> String {
    BASE_ID.replace(spdx.trim(), "").into_owned()
}

pub fn dest_column(spdx: &str) -> Result<usize, String> {
    match classify(spdx) {
        "permissive" => Ok(0),
        c @ ("Apache-2.0" | "GPL-3.0" | "AGPL-3.0" | "CC-BY-4.0") => {
            Ok(DESTS.iter().position(|d| *d == c).unwrap())
        }
        _ => Err(format!(
            "--into {spdx}: no column in the matrix; decide by hand (references/licences.md)"
        )),
    }
}

static WITH: LazyLock<Regex> = LazyLock::new(|| re(r"\s+WITH\s+\S+"));
static OR: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\s+OR\s+"));
static AND: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\s+AND\s+"));

/// Verdict for an SPDX expression. OR: the best alternative. AND: the worst part.
pub fn verdict(expr: Option<&str>, col: usize) -> &'static str {
    let Some(expr) = expr.filter(|e| !e.is_empty()) else {
        return row("none")[col];
    };
    let expr = WITH.replace_all(expr.trim().trim_matches(['(', ')']), "");
    let mut best = "cite";
    for alt in OR.split(&expr) {
        let worst = AND
            .split(alt)
            .map(|p| row(classify(p.trim_matches(['(', ')', ' '])))[col])
            .min_by_key(|v| rank(v))
            .unwrap();
        if rank(worst) > rank(best) {
            best = worst;
        }
    }
    best
}

pub fn matrix_markdown() -> String {
    let mut rows = vec![
        format!("| from \\ into | {} |", DESTS.join(" | ")),
        format!("{}|", "|---".repeat(DESTS.len() + 1)),
    ];
    for (c, r) in CLASSES.iter().zip(MATRIX.iter()) {
        rows.push(format!("| {c} | {} |", r.join(" | ")));
    }
    rows.join("\n")
}

// --- detection ------------------------------------------------------------------------------

static WS: LazyLock<Regex> = LazyLock::new(|| re(r"\s+"));

/// SPDX id of a licence file's text, from its distinctive phrases.
pub fn detect_text(text: &str) -> &'static str {
    let head: String = text.chars().take(20000).collect();
    let t = WS.replace_all(&head, " ").to_lowercase();
    let has = |s: &str| t.contains(s);
    if has("gnu affero general public license") {
        return "AGPL-3.0";
    }
    if has("gnu lesser general public license") || has("gnu library general public license") {
        return if has("version 2.1") || has("version 2,") {
            "LGPL-2.1"
        } else {
            "LGPL-3.0"
        };
    }
    if has("gnu general public license") {
        if has("version 3") {
            return "GPL-3.0";
        }
        if has("version 2") {
            let later = has("version 2 of the license, or (at your option) any later version");
            return if later { "GPL-2.0-or-later" } else { "GPL-2.0" };
        }
    }
    if has("mozilla public license") && has("2.0") {
        return "MPL-2.0";
    }
    if has("apache license") && has("version 2.0") {
        return "Apache-2.0";
    }
    if has("business source license") {
        return "BUSL-1.1";
    }
    if has("server side public license") {
        return "SSPL-1.0";
    }
    if has("elastic license 2.0") {
        return "Elastic-2.0";
    }
    if has("polyform") {
        return "PolyForm";
    }
    if has("commons clause") {
        return "Commons-Clause";
    }
    if has("creative commons") || (has("attribution") && has("international")) {
        for (name, spdx) in [
            ("attribution-noncommercial-noderivatives", "CC-BY-NC-ND-4.0"),
            ("attribution-noncommercial-sharealike", "CC-BY-NC-SA-4.0"),
            ("attribution-noncommercial", "CC-BY-NC-4.0"),
            ("attribution-noderivatives", "CC-BY-ND-4.0"),
            ("attribution-sharealike 4.0", "CC-BY-SA-4.0"),
            ("attribution 4.0 international", "CC-BY-4.0"),
            ("cc0 1.0", "CC0-1.0"),
        ] {
            if has(name) {
                return spdx;
            }
        }
    }
    if has("cc0 1.0 universal") {
        return "CC0-1.0";
    }
    if has("this is free and unencumbered software released into the public domain") {
        return "Unlicense";
    }
    if has("boost software license") {
        return "BSL-1.0";
    }
    if has("permission is hereby granted, free of charge") {
        return "MIT";
    }
    if has("redistribution and use in source and binary forms") {
        return if has("neither the name") || has("names of its contributors") {
            "BSD-3-Clause"
        } else {
            "BSD-2-Clause"
        };
    }
    if has("permission to use, copy, modify, and/or distribute this software for any purpose") {
        return if has("provided that the above copyright notice") {
            "ISC"
        } else {
            "0BSD"
        };
    }
    if has("this software is provided 'as-is'") && has("permission is granted to anyone") {
        return "Zlib";
    }
    "unknown"
}

static LICENCE_FILE: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^(LICEN[CS]E|COPYING|UNLICENSE)([.-].*)?$"));
static NOTICE_FILE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^NOTICE([.-].*)?$"));
static TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(&format!(
        "{}[ \\t]*:[ \\t]*([^\\n*#<>\"']+)",
        regex::escape(TAG_LICENSE)
    ))
});
static DOI: LazyLock<Regex> = LazyLock::new(|| re(r#"(?m)^\s*doi:\s*['"]?([^'"\s]+)"#));
static SETUP_LIC: LazyLock<Regex> = LazyLock::new(|| {
    re(r#"(?m)^\s*(?:license|s\.license|spec\.license)\s*[=:]\s*['"]?([^'"\n]+)"#)
});
static CLASSIFIER: LazyLock<Regex> = LazyLock::new(|| re(r"^License :: OSI Approved :: (.+)"));

const TOOLING: &[(&str, &[&str])] = &[
    ("skill", &["**/SKILL.md"]),
    (
        "agent",
        &[".claude/agents/*.md", "agents/*.md", ".opencode/agent/*.md"],
    ),
    ("command", &[".claude/commands/*.md", "commands/*.md"]),
    (
        "agent-rules",
        &[
            "AGENTS.md",
            "CLAUDE.md",
            ".cursorrules",
            ".cursor/rules/*",
            ".github/copilot-instructions.md",
        ],
    ),
    (
        "hook-or-settings",
        &[
            ".claude/settings.json",
            ".claude/hooks/*",
            ".mcp.json",
            ".pre-commit-config.yaml",
            ".githooks/*",
            ".husky/*",
        ],
    ),
    (
        "plugin",
        &[".claude-plugin/*.json", "**/.claude-plugin/plugin.json"],
    ),
    (
        "workflow",
        &[
            ".github/workflows/*.yml",
            ".github/workflows/*.yaml",
            "**/action.yml",
            "**/action.yaml",
        ],
    ),
    (
        "task-runner",
        &[
            "justfile",
            "**/justfile",
            "Justfile",
            "Makefile",
            "**/Makefile",
            "Taskfile.yml",
        ],
    ),
    (
        "nix",
        &["flake.nix", "**/flake.nix", "shell.nix", "default.nix"],
    ),
    (
        "container",
        &[
            "Dockerfile",
            "**/Dockerfile",
            ".devcontainer/*",
            "docker-compose.yml",
            "compose.yaml",
        ],
    ),
    ("script", &["scripts/*", "bin/*", "tools/*", "**/*.sh"]),
    (
        "prompt",
        &["prompts/*", "**/*.prompt.md", "**/prompts/*.md"],
    ),
    (
        "docs-standard",
        &[
            "CONTRIBUTING.md",
            "**/*style-guide*.md",
            "**/STYLE*.md",
            "**/*writing-style*.md",
        ],
    ),
    (
        "lint-config",
        &[
            "ruff.toml",
            ".editorconfig",
            ".shellcheckrc",
            ".golangci.yml",
            "eslint.config.*",
            ".prettierrc*",
            "biome.json",
        ],
    ),
    (
        "template",
        &[
            ".github/ISSUE_TEMPLATE/*",
            ".github/PULL_REQUEST_TEMPLATE.md",
            ".github/pull_request_template.md",
            "templates/**",
        ],
    ),
    (
        "citation",
        &["CITATION.cff", ".zenodo.json", "codemeta.json"],
    ),
];

/// Python's fnmatch: `*` crosses '/', `?` is one char. shortcut: no `[seq]`, none of TOOLING uses it.
fn fnmatch_re(g: &str) -> Regex {
    let body: String = g
        .chars()
        .map(|c| match c {
            '*' => ".*".to_string(),
            '?' => ".".to_string(),
            c => regex::escape(&c.to_string()),
        })
        .collect();
    re(&format!("(?s)^{body}$"))
}

type KindGlobs = (&'static str, Vec<(&'static str, Regex)>);
static TOOLING_RE: LazyLock<Vec<KindGlobs>> = LazyLock::new(|| {
    TOOLING
        .iter()
        .map(|(k, gs)| (*k, gs.iter().map(|g| (*g, fnmatch_re(g))).collect()))
        .collect()
});

/// Relative paths (with '/'), a directory's files first, then its subdirectories, all sorted.
fn walk(root: &Path, rel: &str, out: &mut Vec<String>) {
    let Ok(rd) = fs::read_dir(root.join(rel)) else {
        return;
    };
    let (mut dirs, mut files) = (vec![], vec![]);
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        let Ok(ft) = e.file_type() else { continue };
        // os.walk lists a symlink to a directory as a directory and does not follow it.
        let (is_dir, follow) = if ft.is_symlink() {
            (
                fs::metadata(e.path()).map(|m| m.is_dir()).unwrap_or(false),
                false,
            )
        } else {
            (ft.is_dir(), true)
        };
        if is_dir {
            if follow && !SKIP_DIRS.contains(&name.as_str()) {
                dirs.push(name);
            }
        } else {
            files.push(name);
        }
    }
    dirs.sort();
    files.sort();
    let join = |n: &String| {
        if rel.is_empty() {
            n.clone()
        } else {
            format!("{rel}/{n}")
        }
    };
    out.extend(files.iter().map(join));
    for d in &dirs {
        walk(root, &join(d), out);
    }
}

fn read(p: &Path) -> String {
    match fs::metadata(p) {
        Ok(m) if m.len() <= MAX_BYTES => fs::read(p)
            .map(|b| {
                String::from_utf8_lossy(&b)
                    .replace("\r\n", "\n")
                    .replace('\r', "\n")
            })
            .unwrap_or_default(),
        _ => String::new(),
    }
}

/// REUSE.toml glob: * does not cross '/', ** does.
fn reuse_glob(pattern: &str) -> Regex {
    let (mut out, mut rest) = (String::new(), pattern);
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix("**/") {
            out += "(?:.*/)?";
            rest = r;
        } else if let Some(r) = rest.strip_prefix("**") {
            out += ".*";
            rest = r;
        } else if let Some(r) = rest.strip_prefix('*') {
            out += "[^/]*";
            rest = r;
        } else {
            let c = rest.chars().next().unwrap();
            out += &regex::escape(&c.to_string());
            rest = &rest[c.len_utf8()..];
        }
    }
    re(&format!("^{out}$"))
}

struct Annotation {
    globs: Vec<Regex>,
    paths: Vec<String>,
    licence: Option<String>,
    precedence: String,
    copyright: Vec<String>,
}

fn strings(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(a)) => a
            .iter()
            .filter_map(|x| x.as_str().map(String::from))
            .collect(),
        _ => vec![],
    }
}

fn parse_toml(text: &str) -> Result<Value, String> {
    let v: toml::Value = toml::from_str(text).map_err(|e| e.to_string())?;
    serde_json::to_value(v).map_err(|e| e.to_string())
}

fn read_reuse_toml(root: &Path) -> (Vec<Annotation>, Option<String>) {
    let p = root.join("REUSE.toml");
    if !p.exists() {
        return (vec![], None);
    }
    let data = match parse_toml(&read(&p)) {
        Ok(d) => d,
        Err(e) => return (vec![], Some(format!("REUSE.toml does not parse: {e}"))),
    };
    let anns = data
        .get("annotations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|a| {
            let paths = strings(a.get("path"));
            Annotation {
                globs: paths.iter().map(|g| reuse_glob(g)).collect(),
                paths,
                licence: a.get(TAG_LICENSE).and_then(Value::as_str).map(String::from),
                precedence: a
                    .get("precedence")
                    .and_then(Value::as_str)
                    .unwrap_or("closest")
                    .into(),
                copyright: strings(a.get(TAG_COPYRIGHT)),
            }
        })
        .collect();
    (anns, None)
}

fn truthy(v: Option<&Value>) -> Option<&Value> {
    v.filter(|v| match v {
        Value::Null | Value::Bool(false) => false,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
        Value::Number(n) => n.as_f64() != Some(0.0),
        _ => true,
    })
}

fn manifest_licences(root: &Path, rel: &str) -> Vec<String> {
    let name = rel.rsplit('/').next().unwrap();
    let text = read(&root.join(rel));
    if text.is_empty() {
        return vec![];
    }
    let unparsed = || vec!["unparsed".to_string()];
    match name {
        "package.json" | "composer.json" => {
            let Ok(d) = serde_json::from_str::<Value>(&text) else {
                return unparsed();
            };
            let Some(d) = d.as_object() else {
                return unparsed();
            };
            match truthy(d.get("license")).or_else(|| truthy(d.get("licenses"))) {
                Some(Value::Array(a)) => a
                    .iter()
                    .filter_map(|x| match x {
                        Value::Object(o) => o.get("type").and_then(Value::as_str),
                        x => x.as_str(),
                    })
                    .map(String::from)
                    .collect(),
                Some(Value::String(s)) => vec![s.clone()],
                _ => vec![],
            }
        }
        "pyproject.toml" | "Cargo.toml" | "gleam.toml" => {
            let Ok(d) = parse_toml(&text) else {
                return unparsed();
            };
            let sect = truthy(d.get("project"))
                .or_else(|| truthy(d.get("package")))
                .or_else(|| truthy(d.pointer("/tool/poetry")))
                .unwrap_or(&d);
            let Some(sect_o) = sect.as_object() else {
                return unparsed();
            };
            let lic = truthy(sect_o.get("license"))
                .or_else(|| truthy(sect_o.get("licences")))
                .or_else(|| truthy(d.get("licences")));
            let mut out = match lic {
                Some(Value::Object(o)) => {
                    match truthy(o.get("text")).or_else(|| truthy(o.get("file"))) {
                        Some(Value::String(s)) => vec![s.clone()],
                        _ => vec![],
                    }
                }
                Some(Value::Array(a)) => a
                    .iter()
                    .filter_map(|x| x.as_str().map(String::from))
                    .collect(),
                Some(Value::String(s)) => vec![s.clone()],
                _ => vec![],
            };
            for c in strings(sect_o.get("classifiers")) {
                if let Some(m) = CLASSIFIER.captures(&c) {
                    out.push(format!("classifier: {}", &m[1]));
                }
            }
            out
        }
        _ if name == "setup.cfg" || name.ends_with(".gemspec") => SETUP_LIC
            .captures(&text)
            .map(|m| vec![m[1].trim().to_string()])
            .unwrap_or_default(),
        _ => vec![],
    }
}

fn git(root: &Path, args: &[&str]) -> Value {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .map_or(Value::Null, Value::String)
}

/// Python's `suffix` and `stem` of a file name.
fn suffix_stem(name: &str) -> (&str, &str) {
    match name.rfind('.') {
        Some(i) if i > 0 && i < name.len() - 1 => (&name[i..], &name[..i]),
        _ => ("", name),
    }
}

fn parent(p: &str) -> &str {
    p.rsplit_once('/').map_or(".", |(a, _)| a)
}

fn py_list(v: &[String]) -> String {
    format!(
        "[{}]",
        v.iter()
            .map(|s| format!("'{s}'"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub fn scan(root: &Path, into: Option<(&str, usize)>) -> Value {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut files = vec![];
    walk(&root, "", &mut files);
    let mut licence_files: Vec<(String, String)> = vec![];
    let (mut notices, mut tooling): (Vec<String>, Vec<(String, &str)>) = (vec![], vec![]);
    let mut tags: Vec<(String, String)> = vec![];
    let mut manifests: Vec<(String, Vec<String>)> = vec![];
    let mut citation = Map::new();
    for rel in &files {
        let name = rel.rsplit('/').next().unwrap();
        let (suffix, stem) = suffix_stem(name);
        let parent_name = rel.rsplit('/').nth(1).unwrap_or("");
        if LICENCE_FILE.is_match(name)
            || (parent_name == "LICENSES" && (suffix == ".txt" || suffix.is_empty()))
        {
            let id = if parent_name == "LICENSES" {
                stem.to_string()
            } else {
                detect_text(&read(&root.join(rel))).to_string()
            };
            licence_files.push((rel.clone(), id));
        } else if NOTICE_FILE.is_match(name) {
            notices.push(rel.clone());
        }
        if matches!(
            name,
            "package.json"
                | "composer.json"
                | "pyproject.toml"
                | "Cargo.toml"
                | "gleam.toml"
                | "setup.cfg"
        ) || name.ends_with(".gemspec")
        {
            let lic = manifest_licences(&root, rel);
            if !lic.is_empty() {
                manifests.push((rel.clone(), lic));
            }
        }
        let text = if [".png", ".jpg", ".pdf", ".svg", ".zip", ".gz"].contains(&suffix) {
            String::new()
        } else {
            read(&root.join(rel))
        };
        let head: String = text.chars().take(4000).collect();
        if let Some(m) = TAG_RE.captures(&head) {
            tags.push((
                rel.clone(),
                m[1].trim().trim_end_matches('-').trim().to_string(),
            ));
        }
        if name == "CITATION.cff" {
            let doi = DOI.captures(&text).map(|m| m[1].to_string());
            citation.insert(rel.clone(), json!({ "doi": doi }));
        } else if name == ".zenodo.json" || name == "codemeta.json" {
            citation.insert(rel.clone(), json!({ "doi": null }));
        }
        for (kind, globs) in TOOLING_RE.iter() {
            if globs
                .iter()
                .any(|(g, r)| r.is_match(rel) || (!g.contains('/') && name == *g))
            {
                tooling.push((rel.clone(), kind));
                break;
            }
        }
    }

    let (anns, reuse_problem) = read_reuse_toml(&root);
    let root_ids: Vec<String> = licence_files
        .iter()
        .filter(|(k, _)| !k.contains('/'))
        .map(|(_, v)| v.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let lic_dirs: BTreeMap<&str, &str> = licence_files
        .iter()
        .filter(|(k, _)| !k.starts_with("LICENSES/"))
        .map(|(k, v)| (parent(k), v.as_str()))
        .collect();

    // Most specific licence: REUSE override > file tag > REUSE closest > nearest LICENSE.
    let licence_of = |rel: &str| -> (Option<String>, String) {
        let hit = anns
            .iter()
            .rev()
            .find(|a| a.globs.iter().any(|g| g.is_match(rel)));
        if let Some(a) = hit.filter(|a| a.precedence == "override") {
            return (a.licence.clone(), "REUSE.toml (override)".into());
        }
        if let Some((_, t)) = tags.iter().find(|(k, _)| k == rel) {
            return (Some(t.clone()), "file tag".into());
        }
        if let Some(a) = hit {
            return (a.licence.clone(), "REUSE.toml".into());
        }
        let mut d = parent(rel);
        loop {
            if let Some(l) = lic_dirs.get(d) {
                let from = if d != "." {
                    format!("{d}/LICENSE")
                } else {
                    "LICENSE".into()
                };
                return (Some(l.to_string()), from);
            }
            if d == "." {
                return (None, "no licence found".into());
            }
            d = parent(d);
        }
    };

    let tooling_json: Vec<Value> = tooling
        .iter()
        .map(|(path, kind)| {
            let (lic, from) = licence_of(path);
            let mut t = json!({ "path": path, "kind": kind, "licence": lic, "licence_from": from });
            if let Some((_, col)) = into {
                t["verdict"] = verdict(lic.as_deref(), col).into();
            }
            t
        })
        .collect();

    let mut flags: Vec<String> = vec![];
    if licence_files.is_empty() && tags.is_empty() && anns.is_empty() {
        flags.push("no-licence: all rights reserved; cite only, copy nothing".into());
    }
    let mut every: BTreeSet<&str> = licence_files.iter().map(|(_, v)| classify(v)).collect();
    every.extend(tags.iter().map(|(_, v)| classify(v)));
    every.extend(
        anns.iter()
            .filter_map(|a| a.licence.as_deref())
            .filter(|l| !l.is_empty() && !l.contains(' '))
            .map(classify),
    );
    if every.iter().filter(|c| **c != "none").count() > 1 {
        flags.push(
            "mixed: licences differ by path; use each item's own licence, not the repo's".into(),
        );
    }
    for (path, lic) in &manifests {
        let ids: BTreeSet<String> = lic
            .iter()
            .filter(|x| !x.starts_with("classifier"))
            .map(|x| base_id(x))
            .collect();
        let roots: BTreeSet<String> = root_ids.iter().map(|r| base_id(r)).collect();
        if !root_ids.is_empty() && !ids.is_empty() && ids.is_disjoint(&roots) {
            let ids: Vec<String> = ids.into_iter().collect();
            flags.push(format!(
                "manifest-mismatch: {path} says {}, LICENSE says {}",
                py_list(&ids),
                py_list(&root_ids)
            ));
        }
    }
    if !notices.is_empty() {
        flags.push("notice: keep NOTICE text with anything reused (Apache-2.0 section 4d)".into());
    }
    if every.contains("unknown") {
        flags.push("unknown-licence: a licence file was not recognised; a human reads it".into());
    }
    flags.extend(reuse_problem);

    let pairs = |v: &[(String, String)]| -> Value {
        Value::Object(v.iter().map(|(k, s)| (k.clone(), json!(s))).collect())
    };
    let mut out = json!({
        "path": root.to_string_lossy(),
        "remote": git(&root, &["remote", "get-url", "origin"]),
        "commit": git(&root, &["rev-parse", "HEAD"]),
        "licence_files": pairs(&licence_files),
        "root_licences": root_ids,
        "notices": notices,
        "file_tags": pairs(&tags),
        "reuse_toml": anns.iter().map(|a| json!({
            "paths": a.paths, "licence": a.licence, "precedence": a.precedence, "copyright": a.copyright,
        })).collect::<Vec<_>>(),
        "manifests": Value::Object(manifests.iter().map(|(k, v)| (k.clone(), json!(v))).collect()),
        "citation": citation,
        "tooling": tooling_json,
        "flags": flags,
    });
    if let Some((id, col)) = into {
        let expr = (!root_ids.is_empty()).then(|| root_ids.join(" AND "));
        out["into"] = id.into();
        out["repo_verdict"] = verdict(expr.as_deref(), col).into();
    }
    out
}

/// `json.dumps(v, indent=1)`: one-space indent, non-ASCII escaped.
pub fn dumps(v: &Value) -> String {
    let mut buf = vec![];
    let f = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, f);
    serde::Serialize::serialize(v, &mut ser).unwrap();
    let mut out = String::new();
    for c in String::from_utf8(buf).unwrap().chars() {
        if c.is_ascii() {
            out.push(c);
        } else {
            let mut u = [0u16; 2];
            for unit in c.encode_utf16(&mut u) {
                out += &format!("\\u{unit:04x}");
            }
        }
    }
    out
}

/// CLI entry: returns the process exit code.
pub fn main(args: &[String]) -> i32 {
    let (mut dirs, mut into, mut matrix) = (vec![], None, false);
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--matrix" => matrix = true,
            "--into" => match it.next() {
                Some(v) => into = Some(v.clone()),
                None => return usage("--into needs a value"),
            },
            s if s.starts_with("--into=") => into = Some(s["--into=".len()..].to_string()),
            "-h" | "--help" => {
                println!(
                    "usage: sharingan licence-scan [--into SPDX-ID] DIR...\n       sharingan licence-scan --matrix"
                );
                return 0;
            }
            s if s.starts_with("--") => return usage(&format!("unrecognized arguments: {s}")),
            s => dirs.push(s.to_string()),
        }
    }
    if matrix {
        println!("{}", matrix_markdown());
        return 0;
    }
    if dirs.is_empty() {
        return usage("give at least one checkout directory");
    }
    let col = match into
        .as_deref()
        .map(|i| dest_column(i).map(|c| (i, c)))
        .transpose()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    for d in dirs {
        println!("{}", dumps(&scan(Path::new(&d), col)));
    }
    0
}

fn usage(msg: &str) -> i32 {
    eprintln!("usage: sharingan licence-scan [--into SPDX-ID] DIR... | --matrix\nerror: {msg}");
    2
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIT_TEXT: &str = "MIT License\n\nCopyright (c) 2026 Example\n\nPermission is hereby granted, free of charge, to any person obtaining a copy of this software...\n";

    fn col(into: &str) -> usize {
        dest_column(into).unwrap()
    }

    #[test]
    fn detect() {
        for (text, want) in [
            (MIT_TEXT, "MIT"),
            ("Apache License\nVersion 2.0, January 2004", "Apache-2.0"),
            (
                "GNU GENERAL PUBLIC LICENSE\nVersion 2, June 1991",
                "GPL-2.0",
            ),
            (
                "GNU GENERAL PUBLIC LICENSE\nVersion 3, 29 June 2007",
                "GPL-3.0",
            ),
            ("GNU AFFERO GENERAL PUBLIC LICENSE\nVersion 3", "AGPL-3.0"),
            ("GNU LESSER GENERAL PUBLIC LICENSE\nVersion 3", "LGPL-3.0"),
            ("Mozilla Public License Version 2.0", "MPL-2.0"),
            (
                "Attribution-ShareAlike 4.0 International\nCreative Commons",
                "CC-BY-SA-4.0",
            ),
            (
                "Attribution-NonCommercial 4.0 International\nCreative Commons",
                "CC-BY-NC-4.0",
            ),
            (
                "Attribution 4.0 International\nCreative Commons",
                "CC-BY-4.0",
            ),
            ("Business Source License 1.1", "BUSL-1.1"),
            (
                "Redistribution and use in source and binary forms... Neither the name",
                "BSD-3-Clause",
            ),
            (
                "This is free and unencumbered software released into the public domain.",
                "Unlicense",
            ),
            ("All rights reserved.", "unknown"),
        ] {
            assert_eq!(detect_text(text), want, "detect {want}");
        }
    }

    #[test]
    fn verdicts() {
        for (expr, into, want) in [
            (Some("MIT"), "AGPL-3.0-only", "reuse"),
            (Some("GPL-2.0"), "AGPL-3.0-only", "borrow"),
            (Some("GPL-2.0-or-later"), "AGPL-3.0-only", "reuse"),
            (Some("GPL-3.0-only"), "MIT", "borrow"),
            (Some("MIT OR GPL-3.0-only"), "MIT", "reuse"),
            (Some("MIT AND GPL-2.0-only"), "AGPL-3.0-only", "borrow"),
            (Some("Apache-2.0"), "MIT", "reuse-file"),
            (Some("CC-BY-SA-4.0"), "CC-BY-4.0", "reuse-file"),
            (Some("CC-BY-NC-4.0"), "CC-BY-4.0", "cite"),
            (Some("BUSL-1.1"), "AGPL-3.0-only", "cite"),
            (None, "MIT", "cite"),
            (
                Some("GPL-2.0-only WITH Classpath-exception-2.0"),
                "AGPL-3.0-only",
                "borrow",
            ),
        ] {
            assert_eq!(
                verdict(expr, col(into)),
                want,
                "verdict {expr:?} into {into}"
            );
        }
        assert!(dest_column("GPL-2.0-only").is_err());
    }

    #[test]
    fn globs() {
        assert!(
            reuse_glob("**/*.md").is_match("AGENTS.md"),
            "REUSE **/ matches the root"
        );
        assert!(
            !reuse_glob("docs/*").is_match("docs/a/b.md"),
            "REUSE * stays in one directory"
        );
    }

    #[test]
    fn matrix_matches_the_reference_doc() {
        let doc = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.claude/skills/ouroboric-sharingan/references/licences.md");
        let text = fs::read_to_string(doc).unwrap();
        let a = text.split("<!-- matrix:start -->\n").nth(1).unwrap();
        let m = a.split("\n<!-- matrix:end -->").next().unwrap();
        assert_eq!(m.trim(), matrix_markdown());
    }

    #[test]
    fn scans_a_synthetic_checkout() {
        let tmp = std::env::temp_dir().join(format!("sharingan-lic-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let w = |p: &str, t: &str| {
            let f = tmp.join("repo").join(p);
            fs::create_dir_all(f.parent().unwrap()).unwrap();
            fs::write(f, t).unwrap();
        };
        w("LICENSE", MIT_TEXT);
        w("NOTICE", "Example\n");
        w(".claude/skills/demo/SKILL.md", "---\nname: demo\n---\n");
        w(
            "vendor/gpl/LICENSE",
            "GNU GENERAL PUBLIC LICENSE\nVersion 3, 29 June 2007\n",
        );
        w("vendor/gpl/justfile", "default:\n\techo hi\n");
        w(
            "scripts/tool.sh",
            &format!("#!/bin/sh\n# {TAG_LICENSE}: Apache-2.0\n"),
        );
        w("package.json", r#"{"license": "ISC"}"#);
        w(
            "CITATION.cff",
            "cff-version: 1.2.0\ndoi: 10.5281/zenodo.1\n",
        );
        let r = tmp.join("repo");
        let res = scan(&r, Some(("AGPL-3.0-only", col("AGPL-3.0-only"))));
        let tool = |res: &Value, p: &str| {
            res["tooling"]
                .as_array()
                .unwrap()
                .iter()
                .find(|t| t["path"] == p)
                .unwrap()
                .clone()
        };
        let skill = tool(&res, ".claude/skills/demo/SKILL.md");
        assert_eq!(skill["kind"], "skill");
        assert_eq!(skill["licence"], "MIT");
        assert_eq!(tool(&res, "vendor/gpl/justfile")["licence"], "GPL-3.0");
        assert_eq!(tool(&res, "scripts/tool.sh")["licence"], "Apache-2.0");
        assert_eq!(res["repo_verdict"], "reuse");
        assert_eq!(res["citation"]["CITATION.cff"]["doi"], "10.5281/zenodo.1");
        let flags = res["flags"].to_string();
        for f in ["mixed", "manifest-mismatch", "notice"] {
            assert!(flags.contains(f), "flag {f}");
        }

        w(
            "REUSE.toml",
            &format!(
                "version = 1\n[[annotations]]\npath = [\"scripts/**\"]\nprecedence = \"override\"\n{TAG_COPYRIGHT} = \"2026 X\"\n{TAG_LICENSE} = \"MPL-2.0\"\n"
            ),
        );
        let res = scan(&r, Some(("MIT", col("MIT"))));
        let t = tool(&res, "scripts/tool.sh");
        assert_eq!(t["licence"], "MPL-2.0", "REUSE override beats file tag");
        assert_eq!(t["verdict"], "reuse-file");

        w("../bare/prompts/x.md", "hello\n");
        w("../bare/AGENTS.md", "rules\n");
        let res = scan(&tmp.join("bare"), Some(("CC-BY-4.0", col("CC-BY-4.0"))));
        assert!(
            res["flags"]
                .as_array()
                .unwrap()
                .iter()
                .any(|f| f.as_str().unwrap().starts_with("no-licence"))
        );
        assert_eq!(res["repo_verdict"], "cite");
        let _ = fs::remove_dir_all(&tmp);
    }
}
