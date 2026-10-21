//! Front matter, captions, alt texts and the plan of a Threads post from a post README.
//!
//! The front matter of each post README:
//!
//!     ---
//!     status: POSTED                                  # DESIGN | REVIEW | POSTED
//!     instagram: https://www.instagram.com/p/<id>/    # required when POSTED
//!     caption: pt-BR                                  # which "## Caption (<lang>)" block was posted
//!     ---

use crate::threads::{self, Api, Params, Threads};
use regex::Regex;
use std::{
    collections::BTreeMap,
    fmt,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

#[derive(Debug)]
pub struct PostError(pub String);
pub type Result<T> = std::result::Result<T, PostError>;

impl fmt::Display for PostError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}

fn io<T>(r: std::io::Result<T>, what: &Path) -> Result<T> {
    r.map_err(|e| PostError(format!("{}: {e}", what.display())))
}

const REPO: &str = "h0ffmann/picasso";
const STATUSES: [&str; 3] = ["DESIGN", "REVIEW", "POSTED"];
// Threads counts an emoji by its UTF-8 length, so bytes is the safe measure.
pub const TEXT_LIMIT: usize = 500;
const CAROUSEL_MAX: usize = 20;
const CONCEPT_DOI: &str = "10.5281/zenodo.23311208";

pub fn root() -> PathBuf {
    let top = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success());
    top.map_or_else(
        || PathBuf::from("."),
        |o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()),
    )
}

fn social(root: &Path) -> PathBuf {
    root.join("docs").join("social")
}

pub fn front_matter(text: &str) -> BTreeMap<String, String> {
    let re = Regex::new(r"(?s)\A---\n(.*?)\n---\n").unwrap();
    let Some(m) = re.captures(text) else {
        return BTreeMap::new();
    };
    m[1].lines()
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

pub fn problems(text: &str) -> Vec<String> {
    let fm = front_matter(text);
    if fm.is_empty() {
        return vec!["no front matter (--- status: ... ---) at the top".into()];
    }
    let get = |k: &str| fm.get(k).map_or("", String::as_str);
    let mut out = vec![];
    let status = get("status");
    if !STATUSES.contains(&status) {
        out.push(format!(
            "status is {status:?}, not one of {}",
            STATUSES.join(", ")
        ));
    }
    if status == "POSTED" {
        if !Regex::new(r"\Ahttps://www\.instagram\.com/p/[\w-]+/?\z")
            .unwrap()
            .is_match(get("instagram"))
        {
            out.push("POSTED needs instagram: https://www.instagram.com/p/<id>/".into());
        }
        let lang = get("caption");
        match (!lang.is_empty()).then(|| caption(text, lang)).flatten() {
            None => out.push(format!(
                "caption: {lang:?} names no '## Caption ({lang})' text block"
            )),
            Some(c) if c.len() > TEXT_LIMIT => out.push(format!(
                "the {lang} caption is longer than Threads' {TEXT_LIMIT} bytes"
            )),
            Some(_) => {}
        }
    }
    out
}

pub fn caption(text: &str, lang: &str) -> Option<String> {
    let re = Regex::new(&format!(
        r"(?ms)^## Caption \({}\)\n+```text\n(.*?)\n```",
        regex::escape(lang)
    ))
    .unwrap();
    re.captures(text).map(|m| m[1].trim().to_string())
}

pub fn alt_texts(text: &str) -> BTreeMap<usize, String> {
    let mut out = BTreeMap::new();
    let Some(start) = Regex::new(r"(?m)^## Alt text\n").unwrap().find(text) else {
        return out;
    };
    // the section runs to the next "## " line
    let body = &text[start.end()..];
    let end = Regex::new(r"(?m)^## ")
        .unwrap()
        .find(body)
        .map_or(body.len(), |m| m.start());
    for m in Regex::new(r"(?m)^(\d+)\. (.+)$")
        .unwrap()
        .captures_iter(&body[..end])
    {
        if let Ok(n) = m[1].parse() {
            out.insert(n, m[2].trim().to_string());
        }
    }
    out
}

pub fn slides(post: &Path) -> Vec<PathBuf> {
    let num = |p: &PathBuf| -> u64 {
        let d: String = p
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .chars()
            .filter(char::is_ascii_digit)
            .collect();
        d.parse().unwrap_or(0)
    };
    let mut v: Vec<PathBuf> = std::fs::read_dir(post)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().unwrap_or_default().to_string_lossy();
            n.starts_with("slide-") && n.ends_with(".png")
        })
        .collect();
    v.sort_by_key(|p| (num(p), p.clone()));
    v
}

pub fn posts(root: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(social(root))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("instagram_"))
        })
        .collect();
    v.sort();
    v
}

fn readme(post: &Path) -> Result<String> {
    let p = post.join("README.md");
    io(std::fs::read_to_string(&p), &p)
}

fn git_show(root: &Path, rev: &str, path: &str) -> String {
    let r = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["show", &format!("{rev}:{path}")])
        .output();
    r.ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

pub fn became_posted(before: &str, after: &str) -> bool {
    let status = |t: &str| front_matter(t).get("status").cloned();
    status(after).as_deref() == Some("POSTED") && status(before).as_deref() != Some("POSTED")
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

pub fn changed(root: &Path, before: &str, after: &str) -> Result<Vec<String>> {
    if before.is_empty() || before.chars().all(|c| c == '0') {
        eprintln!(
            "social: no previous commit to compare with; post by hand with workflow_dispatch"
        );
        return Ok(vec![]);
    }
    Ok(posts(root)
        .iter()
        .map(|p| rel(root, p))
        .filter(|r| {
            became_posted(
                &git_show(root, before, &format!("{r}/README.md")),
                &git_show(root, after, &format!("{r}/README.md")),
            )
        })
        .collect())
}

pub fn latest(root: &Path) -> Result<Option<String>> {
    let mut done = None;
    for p in posts(root) {
        if front_matter(&readme(&p)?)
            .get("status")
            .is_some_and(|s| s == "POSTED")
        {
            done = Some(rel(root, &p));
        }
    }
    Ok(done)
}

pub type Images = Vec<(String, String)>;

pub fn plan(root: &Path, post: &Path, sha: &str) -> Result<(String, Images)> {
    let name = post
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let text = readme(post)?;
    let bad = problems(&text);
    if !bad.is_empty() {
        return Err(PostError(format!("{name}: {}", bad.join("; "))));
    }
    let fm = front_matter(&text);
    if fm["status"] != "POSTED" {
        return Err(PostError(format!("{name}: not POSTED on Instagram yet")));
    }
    let alts = alt_texts(&text);
    let dir = rel(root, post);
    let files = slides(post);
    if files.len() > CAROUSEL_MAX {
        return Err(PostError(format!(
            "{name}: {} slides, Threads takes at most {CAROUSEL_MAX}",
            files.len()
        )));
    }
    let images = files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let url = format!(
                "https://raw.githubusercontent.com/{REPO}/{sha}/{dir}/{}",
                f.file_name().unwrap().to_string_lossy()
            );
            (
                url,
                alts.get(&(i + 1))
                    .map_or(String::new(), |a| a.chars().take(1000).collect()),
            )
        })
        .collect();
    Ok((caption(&text, &fm["caption"]).unwrap_or_default(), images))
}

/// `serde_json` would escape non-ASCII less than Python's ensure_ascii=False; this matches it.
pub fn post_cmd(root: &Path, post: &Path, sha: &str, dry_run: bool) -> Result<i32> {
    let (text, images) = plan(root, post, sha)?;
    let name = post
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    if dry_run {
        let imgs: Vec<_> = images
            .iter()
            .map(|(u, a)| serde_json::json!([u, a]))
            .collect();
        let v = serde_json::json!({"post": name, "text": text, "images": imgs});
        println!("{}", serde_json::to_string_pretty(&v).unwrap());
        return Ok(0);
    }
    let link = threads::client()?.publish(&text, &images, "")?;
    println!("{name} on Threads: {link}");
    Ok(0)
}

pub fn release_text(tag: &str) -> Result<(String, String)> {
    if !Regex::new(r"\Av\d+\.\d+\.\d+\z").unwrap().is_match(tag) {
        return Err(PostError(format!(
            "{tag:?} is not a vMAJOR.MINOR.PATCH tag"
        )));
    }
    let url = format!("https://github.com/{REPO}/releases/tag/{tag}");
    let text = format!(
        "picasso {tag} saiu. Labs Nix que fixam a toolchain como qualquer dependência.\n\n\
         Notas: {url}\nDOI: https://doi.org/{CONCEPT_DOI}\nSite: https://h0ffmann.github.io/picasso\n\n\
         #nix #nixos #reproducibility #opensource"
    );
    Ok((text, url))
}

pub fn release_cmd(tag: &str, dry_run: bool) -> Result<i32> {
    let (text, url) = release_text(tag)?;
    if dry_run {
        println!("{text}");
        return Ok(0);
    }
    println!(
        "picasso {tag} on Threads: {}",
        threads::client()?.publish(&text, &[], &url)?
    );
    Ok(0)
}

pub fn check(root: &Path) -> Result<i32> {
    let mut bad = 0;
    let all = posts(root);
    for post in &all {
        for p in problems(&readme(post)?) {
            eprintln!("{}/README.md: {p}", rel(root, post));
            bad += 1;
        }
    }
    println!("social: {} posts checked, {bad} problems", all.len());
    Ok(i32::from(bad > 0))
}

const SAMPLE: &str = "---
status: POSTED
instagram: https://www.instagram.com/p/AbC_1-x/
caption: pt-BR
---

# sample

## Caption (pt-BR)

```text
Olá, Threads ✨
```

## Alt text

1. First slide.
2. Second slide.

## Render again
";

type Call = (String, String, Params);

/// A fake Threads API that records every call.
struct Fake<'a> {
    calls: &'a std::cell::RefCell<Vec<Call>>,
    broken: bool,
}

impl Api for Fake<'_> {
    fn call(&mut self, method: &str, path: &str, p: &Params) -> Result<serde_json::Value> {
        let mut calls = self.calls.borrow_mut();
        calls.push((method.into(), path.into(), p.clone()));
        let n = calls.len();
        let fields = p.get("fields").map(String::as_str);
        Ok(if self.broken && fields == Some("status,error_message") {
            serde_json::json!({"status": "ERROR", "error_message": "bad image"})
        } else if path.ends_with("/threads") {
            serde_json::json!({"id": format!("c{n}")})
        } else if path.ends_with("/threads_publish") {
            serde_json::json!({"id": "m1"})
        } else if fields == Some("permalink") {
            serde_json::json!({"permalink": "https://www.threads.com/@x/post/1"})
        } else {
            serde_json::json!({"status": "FINISHED"})
        })
    }
}

/// The assertions of the original `--self-test`, shared by the flag and `cargo test`.
pub fn self_test(root: &Path) -> Result<()> {
    assert_eq!(front_matter(SAMPLE)["status"], "POSTED");
    assert!(problems(SAMPLE).is_empty(), "{:?}", problems(SAMPLE));
    assert_eq!(caption(SAMPLE, "pt-BR").as_deref(), Some("Olá, Threads ✨"));
    assert_eq!(
        alt_texts(SAMPLE),
        BTreeMap::from([
            (1, "First slide.".to_string()),
            (2, "Second slide.".to_string())
        ])
    );
    assert!(!problems("# no front matter\n").is_empty());
    assert!(!problems(&SAMPLE.replace("POSTED", "DONE")).is_empty());
    assert!(!problems(&SAMPLE.replace("https://www.instagram.com/p/AbC_1-x/", "")).is_empty());
    assert!(!problems(&SAMPLE.replace("caption: pt-BR", "caption: en")).is_empty());
    assert!(!problems(&SAMPLE.replace("Olá, Threads ✨", &"é".repeat(251))).is_empty());
    assert!(
        problems(
            &SAMPLE
                .replace("status: POSTED", "status: DESIGN")
                .replace("instagram: ", "x: ")
        )
        .is_empty()
    );
    assert!(became_posted("", SAMPLE));
    assert!(became_posted(&SAMPLE.replace("POSTED", "REVIEW"), SAMPLE));
    assert!(!became_posted(SAMPLE, SAMPLE));
    assert!(!became_posted("", &SAMPLE.replace("POSTED", "REVIEW")));

    let d = std::env::temp_dir().join(format!("social-selftest-{}", std::process::id()));
    let post = d.join("instagram_009");
    io(std::fs::create_dir_all(&post), &post)?;
    io(std::fs::write(post.join("README.md"), SAMPLE), &post)?;
    for n in [10, 2, 1] {
        io(
            std::fs::write(post.join(format!("slide-{n}.png")), b""),
            &post,
        )?;
    }
    let names: Vec<_> = slides(&post)
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["slide-1.png", "slide-2.png", "slide-10.png"]);
    let (text, images) = plan(&d, &post, "abc")?;
    assert_eq!(text, "Olá, Threads ✨");
    assert_eq!(
        images[2],
        (
            "https://raw.githubusercontent.com/h0ffmann/picasso/abc/instagram_009/slide-10.png"
                .into(),
            String::new()
        )
    );
    assert_eq!(images[1].1, "Second slide.");
    std::fs::remove_dir_all(&d).ok();

    let calls = std::cell::RefCell::new(vec![]);
    let fake = |broken| {
        Threads::new(
            Fake {
                calls: &calls,
                broken,
            },
            "42",
            Duration::ZERO,
        )
    };
    let imgs = [
        ("u1".to_string(), "a1".to_string()),
        ("u2".to_string(), "a2".to_string()),
    ];
    let link = fake(false).publish("hi", &imgs, "")?;
    assert_eq!(link, "https://www.threads.com/@x/post/1");
    {
        let c = calls.borrow();
        let made: Vec<_> = c
            .iter()
            .enumerate()
            .filter(|(_, c)| c.1 == "42/threads")
            .collect();
        let kinds: Vec<_> = made
            .iter()
            .map(|(_, c)| c.2["media_type"].as_str())
            .collect();
        assert_eq!(kinds, ["IMAGE", "IMAGE", "CAROUSEL"]);
        assert!(
            made[..2]
                .iter()
                .all(|(_, c)| c.2.get("is_carousel_item").is_some_and(|v| v == "true"))
        );
        assert!(made[2].1.2["text"] == "hi" && made[2].1.2["children"].matches(',').count() == 1);
        let carousel_id = format!("c{}", made[2].0 + 1);
        let published: Vec<_> = c
            .iter()
            .filter(|c| c.1 == "42/threads_publish")
            .map(|c| c.2["creation_id"].clone())
            .collect();
        assert_eq!(published, [carousel_id]);
    }

    calls.borrow_mut().clear();
    fake(false).publish("only text", &[], "")?;
    assert_eq!(
        calls.borrow()[0].2,
        threads::params([("media_type", "TEXT"), ("text", "only text")])
    );

    match fake(true).publish("x", &[("u".into(), "a".into())], "") {
        Err(PostError(e)) => assert!(e.contains("bad image"), "{e}"),
        Ok(_) => panic!("an ERROR container must stop the post"),
    }

    let (text, url) = release_text("v1.2.3")?;
    assert!(url.ends_with("/releases/tag/v1.2.3") && text.len() <= TEXT_LIMIT);
    assert!(text.matches("https://").count() <= 5); // Threads refuses more than five distinct links
    assert!(
        release_text("1.2").is_err(),
        "a tag that is not vMAJOR.MINOR.PATCH must be refused"
    );
    calls.borrow_mut().clear();
    fake(false).publish("rel", &[], "https://x")?;
    assert_eq!(calls.borrow()[0].2["link_attachment"], "https://x");

    assert_eq!(check(root)?, 0, "a post README in this repo is invalid");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::threads::{Http, refresh_at};
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn self_test_passes() {
        self_test(&root()).unwrap();
    }

    /// A one-shot HTTP stub on localhost: records the raw request, answers `reply` as JSON.
    fn stub(status: &str, reply: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", l.local_addr().unwrap());
        let status = status.to_string();
        let h = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut req = vec![0u8; 8192];
            let mut n = 0;
            // read until the head and (if any) the body are in
            loop {
                n += s.read(&mut req[n..]).unwrap();
                let t = String::from_utf8_lossy(&req[..n]).into_owned();
                if let Some((head, body)) = t.split_once("\r\n\r\n") {
                    let len = head
                        .to_lowercase()
                        .split("content-length: ")
                        .nth(1)
                        .map_or(0, |x| x.split("\r\n").next().unwrap().parse().unwrap());
                    if body.len() >= len {
                        break;
                    }
                }
            }
            write!(s, "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}", reply.len()).unwrap();
            String::from_utf8_lossy(&req[..n]).into_owned()
        });
        (base, h)
    }

    #[test]
    fn post_goes_as_a_form_with_the_token_in_the_body() {
        let (base, h) = stub("200 OK", r#"{"id":"c1"}"#);
        let r = Http::new(&base, "SECRET")
            .call(
                "POST",
                "42/threads",
                &threads::params([("media_type", "TEXT"), ("text", "oi")]),
            )
            .unwrap();
        assert_eq!(r["id"], "c1");
        let req = h.join().unwrap();
        assert!(req.starts_with("POST /42/threads "), "{req}");
        assert!(
            req.contains("access_token=SECRET") && req.contains("media_type=TEXT"),
            "{req}"
        );
    }

    #[test]
    fn get_goes_as_a_query() {
        let (base, h) = stub("200 OK", r#"{"status":"FINISHED"}"#);
        Http::new(&base, "SECRET")
            .call(
                "GET",
                "c1",
                &threads::params([("fields", "status,error_message")]),
            )
            .unwrap();
        let req = h.join().unwrap();
        assert!(req.starts_with("GET /c1?"), "{req}");
        assert!(
            req.contains("access_token=SECRET") && req.contains("fields=status%2Cerror_message"),
            "{req}"
        );
    }

    #[test]
    fn http_error_shows_the_body_not_the_token() {
        let (base, h) = stub("400 Bad Request", r#"{"error":"expired SECRET"}"#);
        let e = Http::new(&base, "SECRET")
            .call("POST", "42/threads", &Params::new())
            .unwrap_err()
            .0;
        h.join().unwrap();
        assert!(
            e.contains("HTTP 400") && e.contains("expired") && !e.contains("SECRET"),
            "{e}"
        );
    }

    #[test]
    fn refresh_reads_token_and_days() {
        let (base, h) = stub(
            "200 OK",
            r#"{"access_token":"NEW","token_type":"bearer","expires_in":5184000}"#,
        );
        assert_eq!(refresh_at(&base, "OLD").unwrap(), ("NEW".to_string(), 60));
        let req = h.join().unwrap();
        assert!(
            req.starts_with("GET /refresh_access_token?")
                && req.contains("grant_type=th_refresh_token")
                && req.contains("access_token=OLD"),
            "{req}"
        );
    }
}
