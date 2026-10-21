//! Write src/picasso_site_releases.mjs from the repo's v* tags: the latest version, what is
//! unreleased on HEAD, a changelog per release grouped by commit type, and the Zenodo DOIs. Run by
//! `just build` and `just test` before Gleam compiles, so the page carries this data in its bundle
//! and its prerendered HTML and the browser fetches none of it. Generated, never committed, because
//! a release must not need a commit after its tag. A clone without tags (CI's shallow checkout)
//! gets no version and an empty changelog.
//!
//! The concept DOI comes from ../CITATION.cff. Zenodo mints a version DOI some time after the tag,
//! so the version DOIs are asked from its API at build time (with curl); when Zenodo cannot be
//! reached the build keeps the concept DOI and shows no version DOI.
//!
//!   site-releases [out]      # run in site/; default src/picasso_site_releases.mjs

use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

const REPO: &str = "https://github.com/h0ffmann/picasso";
const ZENODO: &str = "https://zenodo.org/api/records";
const GROUPS: [(&str, &str); 3] = [
    ("feat", "Features"),
    ("fix", "Fixes"),
    ("docs", "Documentation"),
];
const OTHER: &str = "Other changes";

#[derive(Serialize, Debug, PartialEq)]
struct Entry {
    subject: String,
    pr: Option<u64>,
    breaking: bool,
}

#[derive(Serialize)]
struct Group {
    title: &'static str,
    entries: Vec<Entry>,
}

#[derive(Serialize)]
struct Release {
    tag: String,
    date: String,
    previous: Option<String>,
    groups: Vec<Group>,
    doi: Option<String>,
}

#[derive(Serialize)]
struct Data {
    repo: &'static str,
    version: Option<String>,
    doi: Option<String>,
    unreleased: Vec<Group>,
    releases: Vec<Release>,
}

fn git(args: &[&str]) -> String {
    Command::new("git")
        .args(["-C", "."])
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

fn is_semver(tag: &str) -> bool {
    Regex::new(r"^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$")
        .unwrap()
        .is_match(tag)
}

/// `feat(site)!: x (#92)` -> ("Features", {subject: "site: x", pr: 92, breaking: true}).
fn entry(subject: &str) -> (&'static str, Entry) {
    let pr = Regex::new(r" \(#(\d+)\)$").unwrap().captures(subject);
    let text = pr
        .as_ref()
        .map_or(subject, |c| &subject[..c.get(0).unwrap().start()]);
    let m = Regex::new(r"^(\w+)(?:\(([^)]*)\))?(!)?: (.*)$")
        .unwrap()
        .captures(text);
    let (kind, scope, bang, text) = match &m {
        Some(m) => (
            m.get(1).unwrap().as_str(),
            m.get(2).map(|s| s.as_str()),
            m.get(3).is_some(),
            m.get(4).unwrap().as_str(),
        ),
        None => ("", None, false, text),
    };
    let title = GROUPS
        .iter()
        .find(|(k, _)| *k == kind)
        .map_or(OTHER, |(_, t)| t);
    let subject = match scope {
        Some(s) if !s.is_empty() => format!("{s}: {text}"),
        _ => text.to_string(),
    };
    (
        title,
        Entry {
            subject,
            pr: pr.map(|c| c[1].parse().unwrap()),
            breaking: bang,
        },
    )
}

fn grouped(rev_range: &str) -> Vec<Group> {
    let out = git(&["log", "--first-parent", "--format=%an%x1f%s", rev_range]);
    let mut by_title: HashMap<&str, Vec<Entry>> = HashMap::new();
    for line in out.split('\n').filter(|l| !l.is_empty()) {
        let (author, subject) = line.split_once('\x1f').unwrap_or((line, ""));
        if author.ends_with("[bot]") {
            continue;
        }
        let (title, e) = entry(subject);
        by_title.entry(title).or_default().push(e);
    }
    GROUPS
        .iter()
        .map(|(_, t)| *t)
        .chain([OTHER])
        .filter_map(|title| {
            by_title
                .remove(title)
                .map(|entries| Group { title, entries })
        })
        .collect()
}

fn concept_doi(cff: &str) -> Option<String> {
    let re = Regex::new(r#"(?m)^doi:\s*['"]?(10\.5281/zenodo\.\d+)"#).unwrap();
    re.captures(cff).map(|c| c[1].to_string())
}

/// {"v0.1.1": "10.5281/zenodo.23311209", ...} from Zenodo's records of the concept DOI: the 25
/// newest, the most its API gives an anonymous caller in one page.
fn version_dois(
    concept: &str,
    fetch: &dyn Fn(&str) -> Result<String, String>,
) -> Result<HashMap<String, String>, String> {
    let recid = concept.rsplit('.').next().unwrap();
    let url = format!("{ZENODO}?q=conceptrecid%3A{recid}&all_versions=true&sort=newest&size=25");
    let body: serde_json::Value = serde_json::from_str(&fetch(&url)?).map_err(|e| e.to_string())?;
    let hits = body["hits"]["hits"].as_array().ok_or("no hits")?;
    Ok(hits
        .iter()
        .filter_map(|h| {
            let version = h["metadata"]["version"]
                .as_str()
                .filter(|s| !s.is_empty())?;
            let doi = h["doi"].as_str().filter(|s| !s.is_empty())?;
            (h["conceptdoi"].as_str() == Some(concept))
                .then(|| (version.to_string(), doi.to_string()))
        })
        .collect())
}

fn fetch(url: &str) -> Result<String, String> {
    let out = Command::new("curl")
        .args(["-fsS", "--max-time", "20", url])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn build() -> Data {
    let refs = git(&[
        "for-each-ref",
        "--sort=-v:refname",
        "--format=%(refname:short) %(creatordate:short)",
        "refs/tags/v*",
    ]);
    let tags: Vec<(&str, &str)> = refs
        .lines()
        .filter_map(|l| l.split_once(' '))
        .filter(|(t, _)| is_semver(t))
        .collect();
    let releases_raw: Vec<Release> = tags
        .iter()
        .enumerate()
        .map(|(i, (tag, date))| {
            let previous = tags.get(i + 1).map(|p| p.0.to_string());
            let range = previous
                .as_ref()
                .map_or(tag.to_string(), |p| format!("{p}..{tag}"));
            Release {
                tag: tag.to_string(),
                date: date.to_string(),
                previous,
                groups: grouped(&range),
                doi: None,
            }
        })
        .collect();
    let version = tags.first().map(|t| t.0.to_string());
    let unreleased = version
        .as_ref()
        .map_or(vec![], |v| grouped(&format!("{v}..HEAD")));
    let doi = std::fs::read_to_string(Path::new("..").join("CITATION.cff"))
        .ok()
        .and_then(|c| concept_doi(&c));
    let mut dois = HashMap::new();
    if let Some(d) = &doi {
        match version_dois(d, &fetch) {
            Ok(m) => dois = m,
            Err(e) => eprintln!("site-releases: no version DOIs, Zenodo not reached ({e})"),
        }
    }
    let releases = releases_raw
        .into_iter()
        .map(|mut r| {
            r.doi = dois.get(&r.tag).cloned();
            r
        })
        .collect();
    Data {
        repo: REPO,
        version,
        doi,
        unreleased,
        releases,
    }
}

fn module(data: &impl Serialize) -> String {
    // The JSON as a string literal: Gleam decodes it with the same decoder as any other source.
    let json = serde_json::to_string(data).unwrap();
    format!(
        "// Generated by site-releases; never edit or commit.\nexport const releases = () => {};\n",
        serde_json::to_string(&json).unwrap()
    )
}

fn main() {
    let out = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "src/picasso_site_releases.mjs".into());
    let data = build();
    std::fs::write(&out, module(&data)).unwrap_or_else(|e| {
        eprintln!("site-releases: {out}: {e}");
        std::process::exit(1);
    });
    println!(
        "{}: version {}, {} releases, DOI {}",
        Path::new(&out).file_name().unwrap().to_string_lossy(),
        data.version.as_deref().unwrap_or("none"),
        data.releases.len(),
        data.doi.as_deref().unwrap_or("none")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(s: &str, pr: Option<u64>, breaking: bool) -> Entry {
        Entry {
            subject: s.into(),
            pr,
            breaking,
        }
    }

    #[test]
    fn entries() {
        assert_eq!(
            entry("feat(site): port the site to Gleam and Lustre (#92)"),
            (
                "Features",
                e("site: port the site to Gleam and Lustre", Some(92), false)
            )
        );
        assert_eq!(
            entry("feat(lint)!: drop ruff"),
            ("Features", e("lint: drop ruff", None, true))
        );
        assert_eq!(entry("docs: a logo (#83)").0, "Documentation");
        assert_eq!(entry("chore: bump (#68)").0, OTHER);
        assert_eq!(
            entry("picasso: the site (#81)"),
            (OTHER, e("the site", Some(81), false))
        );
        assert_eq!(entry("Merge pull request #5 from x").0, OTHER);
    }

    #[test]
    fn semver() {
        assert!(["v0.1.0", "v10.0.3"].iter().all(|t| is_semver(t)));
        assert!(
            !["v0.1", "v01.0.0", "v1.0.0-rc1"]
                .iter()
                .any(|t| is_semver(t))
        );
    }

    #[test]
    fn concept() {
        assert_eq!(
            concept_doi("cff-version: 1.2.0\ndoi: 10.5281/zenodo.23311208\n").as_deref(),
            Some("10.5281/zenodo.23311208")
        );
        assert_eq!(
            concept_doi("identifiers:\n  - value: 10.5281/zenodo.1\n"),
            None
        );
    }

    #[test]
    fn zenodo() {
        let body = r#"{"hits":{"hits":[
          {"doi":"10.5281/zenodo.9","conceptdoi":"10.5281/zenodo.8","metadata":{"version":"v0.1.1"}},
          {"doi":"10.5281/zenodo.7","conceptdoi":"10.5281/zenodo.6","metadata":{"version":"v9.9.9"}}]}}"#;
        let asked = std::cell::RefCell::new(vec![]);
        let dois = version_dois("10.5281/zenodo.8", &|u| {
            asked.borrow_mut().push(u.to_string());
            Ok(body.into())
        })
        .unwrap();
        assert_eq!(
            dois,
            HashMap::from([("v0.1.1".to_string(), "10.5281/zenodo.9".to_string())])
        );
        assert!(asked.borrow()[0].contains("conceptrecid%3A8"));
        assert!(version_dois("10.5281/zenodo.8", &|_| Err("down".into())).is_err());
    }

    #[test]
    fn module_roundtrip() {
        let js =
            module(&serde_json::json!({"doi": "10.5281/zenodo.8", "s": "a \"quote\" </script>"}));
        let lit = js.split_once(" => ").unwrap().1.trim_end_matches(";\n");
        let inner: String = serde_json::from_str(lit).unwrap();
        let v: serde_json::Value = serde_json::from_str(&inner).unwrap();
        assert_eq!(v["s"], "a \"quote\" </script>");
    }
}
