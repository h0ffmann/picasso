//! `--link NAME=PATH/graph.json[@URL]`: boundary nodes for another repository's graph.
//!
//! The text files of DIR are scanned for references to the repository NAME. A reference names a
//! path in that repository; the path resolves to one node of its graph, which is added here as an
//! external node (`NAME:<id>`, `href` = URL#node=<id>) with a `depends_on` edge from the file that
//! holds the reference. The linked graph is not merged: only the nodes that are referenced enter.
//!
//! A reference is, on one line (precision over recall):
//! 1. `github:OWNER/NAME...?dir=PATH` or `...&dir=PATH`, a flake URL, ended by a quote or the line;
//! 2. `github.com/OWNER/NAME/tree/REV/PATH` or `.../blob/REV/PATH`, REV one segment;
//! 3. `lab=PATH` at the start of a word (a lock file's key);
//! 4. a word `labs/X`, in a file whose text contains NAME (any case).
//!
//! PATH resolves to the linked graph's best node under it: PATH/flake.nix if there is one, then
//! the node with most edges, then the lowest id. A PATH with no node is dropped.
use crate::graph::{Edge, Graph, Node};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub struct Link {
    pub name: String,
    pub graph: Graph,
    pub url: String,
}

/// `NAME=PATH[@URL]` into (NAME, PATH, URL); the URL is NAME.html when absent.
pub fn parse_spec(s: &str) -> Result<(String, PathBuf, String), String> {
    let (name, rest) = s
        .split_once('=')
        .ok_or("--link needs NAME=PATH/graph.json[@URL]")?;
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || "_.-".contains(c))
    {
        return Err(format!("--link: bad name {name:?}"));
    }
    let (path, url) = match rest.rsplit_once('@') {
        Some((p, u)) => (p, u.to_string()),
        None => (rest, format!("{name}.html")),
    };
    // the URL ends up in a link the page follows: relative, http or https only
    if path.is_empty() || url.is_empty() || (url.contains(':') && !url.starts_with("http")) {
        return Err(format!("--link: bad path or URL in {s:?}"));
    }
    Ok((name.to_string(), path.into(), url))
}

fn path_at(s: &str) -> &str {
    let end = s
        .find(|c: char| !(c.is_alphanumeric() || "_-./".contains(c)))
        .unwrap_or(s.len());
    s[..end].trim_end_matches(['/', '.'])
}

fn word_starts(l: &str, key: &str) -> Vec<usize> {
    l.match_indices(key)
        .map(|(i, _)| i)
        .filter(|&i| {
            l[..i]
                .chars()
                .next_back()
                .is_none_or(|c| !(c.is_alphanumeric() || "_-./=".contains(c)))
        })
        .collect()
}

/// The (PATH, line) references to the repository `name` in one file's text.
pub fn refs(name: &str, text: &str) -> Vec<(String, u32)> {
    let names_it = text.to_lowercase().contains(&name.to_lowercase());
    let mut out = Vec::new();
    for (k, l) in text.lines().enumerate() {
        let n = k as u32 + 1;
        let mut add = |p: &str| {
            if !p.is_empty() {
                out.push((p.to_string(), n));
            }
        };
        for (i, _) in l.match_indices("github:") {
            let rest = &l[i + 7..];
            if rest.split('/').nth(1).map(|s| s.split(['?', '#']).next()) == Some(Some(name)) {
                let url = &rest[..rest.find(['"', '\'', '`']).unwrap_or(rest.len())];
                if let Some(d) = ["?dir=", "&dir="].iter().find_map(|k| url.find(k)) {
                    add(path_at(&url[d + 5..]));
                }
            }
        }
        for (i, _) in l.match_indices("github.com/") {
            let seg: Vec<&str> = l[i + 11..].splitn(5, '/').collect();
            if seg.len() == 5 && seg[1] == name && (seg[2] == "tree" || seg[2] == "blob") {
                add(path_at(seg[4]));
            }
        }
        for i in word_starts(l, "lab=") {
            add(path_at(&l[i + 4..]));
        }
        if names_it {
            for i in word_starts(l, "labs/") {
                let p = path_at(&l[i..]);
                add(&p.splitn(3, '/').take(2).collect::<Vec<_>>().join("/"));
            }
        }
    }
    out
}

/// The node of `g` that stands for the directory or file `path`.
pub fn resolve(g: &Graph, path: &str) -> Option<usize> {
    let deg = g.degree();
    let flake = format!("{path}/flake.nix");
    g.nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| !n.external && (n.file == path || n.file.starts_with(&format!("{path}/"))))
        .min_by_key(|(i, n)| (n.file != flake, std::cmp::Reverse(deg[*i]), n.id.as_str()))
        .map(|(i, _)| i)
}

fn enc(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Adds the external nodes and `depends_on` edges for `files` (path, text), after clustering.
pub fn apply(g: &mut Graph, files: &[(String, String)], links: &[Link]) {
    // (referencing file, link, target node in the linked graph) -> first line
    let mut found: BTreeMap<(&str, usize, usize), u32> = BTreeMap::new();
    for (path, text) in files {
        for (li, l) in links.iter().enumerate() {
            for (p, line) in refs(&l.name, text) {
                if let Some(t) = resolve(&l.graph, &p) {
                    found.entry((path.as_str(), li, t)).or_insert(line);
                }
            }
        }
    }
    let mut next = g.nodes.iter().map(|n| n.community + 1).max().unwrap_or(0);
    let file_comm = next;
    let mut ext: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    let mut src: BTreeMap<&str, usize> = BTreeMap::new();
    let mut fresh = BTreeSet::new();
    for &(path, li, t) in found.keys() {
        let base = path.rsplit('/').next().unwrap_or(path);
        let at = g
            .nodes
            .iter()
            .position(|n| n.file == path && n.label == base);
        let s = match (at, src.get(path)) {
            (Some(i), _) => i,
            (None, Some(&i)) => i,
            (None, None) => {
                let mut id = crate::extract::make_id(&[path]);
                while g.nodes.iter().any(|n| n.id == id) {
                    id += "_file";
                }
                g.nodes.push(Node {
                    id,
                    label: base.to_string(),
                    file: path.to_string(),
                    line: 1,
                    community: file_comm,
                    cname: "files that link to other repositories".into(),
                    ..Default::default()
                });
                fresh.insert(g.nodes.len() - 1);
                src.insert(path, g.nodes.len() - 1);
                g.nodes.len() - 1
            }
        };
        let l = &links[li];
        let x = *ext.entry((li, t)).or_insert_with(|| {
            let o = &l.graph.nodes[t];
            g.nodes.push(Node {
                id: format!("{}:{}", l.name, o.id),
                label: format!("{}: {}", l.name, o.label),
                file: format!("{}:{}", l.name, o.file),
                line: o.line,
                external: true,
                repo: l.name.clone(),
                href: format!("{}#node={}", l.url, enc(&o.id)),
                ..Default::default()
            });
            g.nodes.len() - 1
        });
        g.edges.push(Edge {
            s,
            t: x,
            rel: "depends_on".into(),
            conf: "EXTRACTED".into(),
            score: 1.0,
            ctx: "link".into(),
            file: path.to_string(),
            line: found[&(path, li, t)],
        });
    }
    // one community per linked repository, after the files' one
    if !fresh.is_empty() {
        next += 1;
    }
    for (li, l) in links.iter().enumerate() {
        if ext.keys().any(|&(i, _)| i == li) {
            for (&(i, _), &x) in &ext {
                if i == li {
                    g.nodes[x].community = next;
                    g.nodes[x].cname = l.name.clone();
                }
            }
            next += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(t: &str) -> Vec<(String, u32)> {
        refs("picasso", t)
    }

    #[test]
    fn spec() {
        let (n, p, u) = parse_spec("picasso=a/graph.json@picasso.html").unwrap();
        assert_eq!(
            (n.as_str(), p.to_str().unwrap(), u.as_str()),
            ("picasso", "a/graph.json", "picasso.html")
        );
        assert_eq!(parse_spec("x=g.json").unwrap().2, "x.html");
        assert_eq!(
            parse_spec("x=g.json@https://h/x.html").unwrap().2,
            "https://h/x.html"
        );
        assert!(parse_spec("x=g.json@javascript:alert(1)").is_err());
        assert!(parse_spec("g.json").is_err() && parse_spec("=g.json").is_err());
    }

    #[test]
    fn flake_urls() {
        let t = "a.url = \"github:h0ffmann/picasso?dir=labs/x\";\nb = \"github:h0ffmann/picasso/{{ rev }}?ref=m&dir=labs/y#to-pdf\";\nc = \"github:h0ffmann/other?dir=labs/z\";\nd = \"github:h0ffmann/picasso\";";
        assert_eq!(r(t), [("labs/x".into(), 1), ("labs/y".into(), 2)]);
    }

    #[test]
    fn github_urls() {
        let t = "[l](https://github.com/h0ffmann/picasso/tree/main/labs/ptbr-tools) and https://github.com/h0ffmann/picasso/blob/abc/labs/q/flake.nix.\nhttps://github.com/h0ffmann/picasso/issues/3 https://github.com/h0ffmann/other/tree/main/labs/no";
        let p: Vec<_> = r(t).into_iter().filter(|x| x.1 == 1).map(|x| x.0).collect();
        assert!(
            p.contains(&"labs/ptbr-tools".to_string())
                && p.contains(&"labs/q/flake.nix".to_string())
        );
        assert!(!r(t).iter().any(|x| x.1 == 2));
    }

    #[test]
    fn lab_key_and_bare_paths() {
        assert_eq!(
            r("rev=1\nlab=labs/ptbr-tools\n"),
            [("labs/ptbr-tools".into(), 2)]
        );
        assert!(r("collab=labs/x").is_empty());
        // a bare labs/X needs the file to name the repository
        assert!(refs("picasso", "see labs/ptbr-pdf").is_empty());
        assert_eq!(
            r("picasso: see `labs/ptbr-pdf/src/main.rs`"),
            [("labs/ptbr-pdf".into(), 1)]
        );
        assert!(r("picasso: ../labs/x and xlabs/y").is_empty());
    }
}
