//! Code-only extraction: walk a tree, parse each file with tree-sitter, resolve names across files.
use crate::graph::{Edge, Graph, Node};
use crate::lang::{self, Call, Imp, Lang, Role};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

const SKIP_DIRS: &[&str] = &[
    "target",
    "node_modules",
    "graphify-out",
    "result",
    "dist",
    "build",
    "_build",
    "__pycache__",
    "vendor",
];
const MAX_BYTES: u64 = 2 * 1024 * 1024;

/// graphify's node-id recipe, minus NFKC: lowercase, runs of non-word characters become `_`.
pub fn make_id(parts: &[&str]) -> String {
    let joined: Vec<String> = parts
        .iter()
        .map(|p| p.trim_matches(|c| c == '_' || c == '.').to_string())
        .filter(|p| !p.is_empty())
        .collect();
    let mut out = String::new();
    for ch in joined.join("_").to_lowercase().chars() {
        if ch.is_alphanumeric() || ch == '_' {
            if !(ch == '_' && out.ends_with('_')) {
                out.push(ch);
            }
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    File,
    Class,
    Func,
}

struct Sym {
    id: String,
    name: String,
    label: String,
    file: usize,
    line: u32,
    kind: Kind,
    parent: Option<usize>,
}

struct PEdge {
    s: String,
    t: String,
    rel: &'static str,
    score: f64,
    ctx: &'static str,
    file: usize,
    line: u32,
}

struct File {
    path: String,
    lang: Lang,
    sym: usize,
    segs: Vec<String>,
}

struct PCall {
    caller: usize,
    call: Call,
    line: u32,
}

struct Cx<'a> {
    src: &'a [u8],
    lang: Lang,
    file: usize,
    fsym: usize,
    stem: String,
    syms: &'a mut Vec<Sym>,
    ids: &'a mut HashMap<String, usize>,
    edges: &'a mut Vec<PEdge>,
    calls: Vec<PCall>,
    bases: Vec<(usize, &'static str, String, u32)>,
    imps: Vec<Imp>,
}

impl Cx<'_> {
    #[allow(clippy::too_many_arguments)] // one call shape for classes and functions
    fn add(
        &mut self,
        id: String,
        name: &str,
        label: String,
        line: u32,
        kind: Kind,
        parent: Option<usize>,
        rel: &'static str,
        from: usize,
    ) -> usize {
        if let Some(&i) = self.ids.get(&id) {
            return i;
        }
        let i = self.syms.len();
        self.ids.insert(id.clone(), i);
        let from_id = self.syms[from].id.clone();
        self.edges.push(PEdge {
            s: from_id,
            t: id.clone(),
            rel,
            score: 1.0,
            ctx: "",
            file: self.file,
            line,
        });
        self.syms.push(Sym {
            id,
            name: name.to_string(),
            label,
            file: self.file,
            line,
            kind,
            parent,
        });
        i
    }

    fn walk(&mut self, n: tree_sitter::Node, class: Option<usize>, func: Option<usize>) {
        let (mut class, mut func) = (class, func);
        let line = n.start_position().row as u32 + 1;
        let file_sym = self.fsym;
        let fortran = self.lang == Lang::Fortran;
        match lang::role(self.lang, n, self.src) {
            Role::Class(name) => {
                let i = self.add(
                    make_id(&[&self.stem, &name]),
                    &name,
                    name.clone(),
                    line,
                    Kind::Class,
                    None,
                    if fortran { "defines" } else { "contains" },
                    file_sym,
                );
                for (rel, b) in lang::bases(self.lang, n, self.src) {
                    self.bases.push((i, rel, b, line));
                }
                class = Some(i);
                func = None;
            }
            Role::Impl { ty, tr } => {
                let found = self
                    .syms
                    .iter()
                    .position(|s| s.file == self.file && s.kind == Kind::Class && s.name == ty);
                let i = found.unwrap_or_else(|| {
                    self.add(
                        make_id(&[&self.stem, &ty]),
                        &ty,
                        ty.clone(),
                        line,
                        Kind::Class,
                        None,
                        "contains",
                        file_sym,
                    )
                });
                if let Some(tr) = tr {
                    self.bases.push((i, "implements", tr, line));
                }
                class = Some(i);
                func = None;
            }
            Role::Func(name) => {
                let (id, label, rel, from) = match class {
                    Some(c) if fortran => (
                        make_id(&[&self.stem, &name]),
                        format!("{name}()"),
                        "defines",
                        c,
                    ),
                    Some(c) => (
                        make_id(&[&self.stem, &self.syms[c].name, &name]),
                        format!(".{name}()"),
                        if fortran { "defines" } else { "method" },
                        c,
                    ),
                    None => (
                        make_id(&[&self.stem, &name]),
                        format!("{name}()"),
                        "contains",
                        file_sym,
                    ),
                };
                let i = self.add(id, &name, label, line, Kind::Func, class, rel, from);
                func = Some(i);
                class = None;
            }
            Role::None => {}
        }
        if let Some(c) = lang::call(self.lang, n, self.src) {
            let caller = func.unwrap_or(file_sym);
            self.calls.push(PCall {
                caller,
                call: c,
                line,
            });
        }
        self.imps.extend(lang::imports(self.lang, n, self.src));
        let mut cur = n.walk();
        let kids: Vec<_> = n.children(&mut cur).collect();
        for k in kids {
            self.walk(k, class, func);
        }
    }
}

fn collect(dir: &Path, rel: &str, skip: &[String], out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut ents: Vec<_> = rd.filter_map(Result::ok).collect();
    ents.sort_by_key(|e| e.file_name());
    for e in ents {
        let name = e.file_name().to_string_lossy().to_string();
        let Ok(ft) = e.file_type() else { continue };
        let path = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        if ft.is_dir() {
            // a directory with its own .git (file or directory) is another repository
            if !name.starts_with('.')
                && !SKIP_DIRS.contains(&name.as_str())
                && !name.starts_with("result-")
                && !skip.contains(&path)
                && !e.path().join(".git").exists()
            {
                collect(&e.path(), &path, skip, out);
            }
        } else if ft.is_file()
            && Lang::of_path(&name).is_some()
            && e.metadata().map(|m| m.len() <= MAX_BYTES).unwrap_or(false)
        {
            out.push(path);
        }
    }
}

fn normalize(p: &str) -> String {
    let mut v: Vec<&str> = Vec::new();
    for s in p.split('/') {
        match s {
            "" | "." => {}
            ".." => {
                v.pop();
            }
            x => v.push(x),
        }
    }
    v.join("/")
}

fn dir_of(p: &str) -> &str {
    p.rsplit_once('/').map(|x| x.0).unwrap_or("")
}

fn common(a: &str, b: &str) -> usize {
    a.split('/')
        .zip(b.split('/'))
        .take_while(|(x, y)| x == y)
        .count()
}

struct Resolver<'a> {
    files: &'a [File],
    by_path: HashMap<&'a str, usize>,
}

impl Resolver<'_> {
    fn suffix(&self, from: usize, segs: &[String]) -> Option<usize> {
        if segs.is_empty() {
            return None;
        }
        let fam = self.files[from].lang.family();
        self.files
            .iter()
            .enumerate()
            .filter(|(i, f)| *i != from && f.lang.family() == fam && f.segs.ends_with(segs))
            .max_by_key(|(i, f)| {
                (
                    common(dir_of(&f.path), dir_of(&self.files[from].path)),
                    std::cmp::Reverse(f.path.clone()),
                    *i,
                )
            })
            .map(|x| x.0)
    }

    fn resolve(&self, from: usize, imp: &Imp) -> Option<usize> {
        let fp = &self.files[from];
        if let Some(p) = &imp.path {
            if !p.starts_with('.') {
                return None;
            }
            let base = normalize(&format!("{}/{}", dir_of(&fp.path), p));
            let exts: &[&str] = if fp.lang == Lang::Nix {
                &["", ".nix", "/default.nix"]
            } else {
                &[
                    "",
                    ".ts",
                    ".tsx",
                    ".js",
                    ".jsx",
                    ".mjs",
                    ".cjs",
                    ".mts",
                    ".cts",
                    "/index.ts",
                    "/index.tsx",
                    "/index.js",
                ]
            };
            return exts
                .iter()
                .find_map(|e| self.by_path.get(format!("{base}{e}").as_str()).copied())
                .filter(|i| *i != from);
        }
        let mut segs = imp.segs.clone();
        match fp.lang {
            Lang::Rust => {
                while matches!(
                    segs.first().map(String::as_str),
                    Some("crate" | "self" | "super")
                ) {
                    segs.remove(0);
                }
                if matches!(
                    segs.first().map(String::as_str),
                    Some("std" | "core" | "alloc")
                ) {
                    return None;
                }
                if let [n] = imp.names.as_slice() {
                    let mut full = segs.clone();
                    full.push(n.clone());
                    if let Some(i) = self.suffix(from, &full) {
                        return Some(i);
                    }
                }
                self.suffix(from, &segs)
            }
            Lang::Python if imp.dots > 0 => {
                let mut d = dir_of(&fp.path).to_string();
                for _ in 1..imp.dots {
                    d = dir_of(&d).to_string();
                }
                let base = format!("{d}/{}", segs.join("/"));
                let base = normalize(&base);
                for e in [".py", "/__init__.py"] {
                    if let Some(&i) = self.by_path.get(format!("{base}{e}").as_str()) {
                        return Some(i);
                    }
                }
                // `from . import x` names a sibling module
                imp.names
                    .iter()
                    .find_map(|n| self.by_path.get(format!("{d}/{n}.py").as_str()).copied())
            }
            _ => self.suffix(from, &segs),
        }
    }
}

pub fn extract(root: &Path, exclude: &[String]) -> Graph {
    let mut paths = Vec::new();
    let skip: Vec<String> = exclude.iter().map(|x| normalize(x)).collect();
    collect(root, "", &skip, &mut paths);
    let mut syms: Vec<Sym> = Vec::new();
    let mut ids: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<PEdge> = Vec::new();
    let mut files: Vec<File> = Vec::new();
    let (mut all_calls, mut all_bases, mut all_imps) = (Vec::new(), Vec::new(), Vec::new());
    for p in &paths {
        let lang = Lang::of_path(p).unwrap();
        let Ok(src) = std::fs::read(root.join(p)) else {
            continue;
        };
        let mut parser = tree_sitter::Parser::new();
        if parser.set_language(&lang.grammar()).is_err() {
            continue;
        }
        let Some(tree) = parser.parse(&src, None) else {
            continue;
        };
        let fi = files.len();
        let no_ext = p.rsplit_once('.').map(|x| x.0).unwrap_or(p);
        let mut stem = make_id(&[no_ext]);
        if ids.contains_key(&stem) {
            stem = make_id(&[p]); // same path, other extension
        }
        let mut segs: Vec<String> = no_ext.split('/').map(String::from).collect();
        if matches!(
            segs.last().map(String::as_str),
            Some("mod" | "__init__" | "index" | "default")
        ) && segs.len() > 1
        {
            segs.pop();
        }
        let sym = syms.len();
        ids.insert(stem.clone(), sym);
        syms.push(Sym {
            id: stem.clone(),
            name: p.rsplit('/').next().unwrap().to_string(),
            label: p.rsplit('/').next().unwrap().to_string(),
            file: fi,
            line: 1,
            kind: Kind::File,
            parent: None,
        });
        files.push(File {
            path: p.clone(),
            lang,
            sym,
            segs,
        });
        let mut cx = Cx {
            src: &src,
            lang,
            file: fi,
            fsym: sym,
            stem,
            syms: &mut syms,
            ids: &mut ids,
            edges: &mut edges,
            calls: vec![],
            bases: vec![],
            imps: vec![],
        };
        cx.walk(tree.root_node(), None, None);
        let (c, b, i) = (cx.calls, cx.bases, cx.imps);
        all_calls.extend(c.into_iter().map(|x| (fi, x)));
        all_bases.extend(b.into_iter().map(|x| (fi, x)));
        all_imps.extend(i.into_iter().map(|x| (fi, x)));
    }

    let rs = Resolver {
        files: &files,
        by_path: files
            .iter()
            .enumerate()
            .map(|(i, f)| (f.path.as_str(), i))
            .collect(),
    };
    let ci = |f: usize, s: &str| {
        if files[f].lang == Lang::Fortran {
            s.to_lowercase()
        } else {
            s.to_string()
        }
    };
    let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
    let mut module_by_name: HashMap<String, usize> = HashMap::new();
    for (i, s) in syms.iter().enumerate() {
        if s.kind != Kind::File {
            by_name.entry(ci(s.file, &s.name)).or_default().push(i);
            if files[s.file].lang == Lang::Fortran && s.kind == Kind::Class {
                module_by_name
                    .entry(s.name.to_lowercase())
                    .or_insert(s.file);
            }
        }
    }
    let mut externals: BTreeMap<String, String> = BTreeMap::new();
    let mut imported: Vec<HashSet<usize>> = vec![HashSet::new(); files.len()];
    let mut seen: HashSet<(String, String, &'static str)> = HashSet::new();
    let mut out: Vec<PEdge> = Vec::new();
    for e in edges {
        if seen.insert((e.s.clone(), e.t.clone(), e.rel)) {
            out.push(e);
        }
    }
    let mut push = |out: &mut Vec<PEdge>, e: PEdge| {
        if seen.insert((e.s.clone(), e.t.clone(), e.rel)) {
            out.push(e);
        }
    };

    // imports: real ones first, so a `mod x;` line never hides the `use` that names the same file
    all_imps.sort_by_key(|(_, i): &(usize, Imp)| i.weak);
    for (fi, imp) in &all_imps {
        let fsym = &syms[files[*fi].sym].id;
        let lang = files[*fi].lang;
        let target = if lang == Lang::Fortran {
            imp.segs
                .first()
                .and_then(|m| module_by_name.get(&m.to_lowercase()).copied())
                .filter(|t| t != fi)
        } else {
            rs.resolve(*fi, imp)
        };
        match target {
            Some(t) => {
                imported[*fi].insert(t);
                push(
                    &mut out,
                    PEdge {
                        s: fsym.clone(),
                        t: syms[files[t].sym].id.clone(),
                        rel: "imports_from",
                        score: 1.0,
                        ctx: "import",
                        file: *fi,
                        line: imp.line,
                    },
                );
                for n in &imp.names {
                    if let Some(&x) = by_name.get(&ci(t, n)).and_then(|v| {
                        v.iter()
                            .find(|&&x| syms[x].file == t && syms[x].parent.is_none())
                    }) {
                        push(
                            &mut out,
                            PEdge {
                                s: fsym.clone(),
                                t: syms[x].id.clone(),
                                rel: "imports",
                                score: 1.0,
                                ctx: "import",
                                file: *fi,
                                line: imp.line,
                            },
                        );
                    }
                }
            }
            None => {
                let local = imp.weak
                    || lang == Lang::Nix
                    || imp.dots > 0
                    || imp
                        .path
                        .as_deref()
                        .map(|p| p.starts_with('.'))
                        .unwrap_or(false)
                    || (lang == Lang::Rust
                        && matches!(
                            imp.segs.first().map(String::as_str),
                            Some("crate" | "self" | "super")
                        ))
                    || lang == Lang::Fortran;
                if local {
                    continue;
                }
                let name = match lang {
                    Lang::Rust => imp.names.last().or(imp.segs.last()).cloned(),
                    Lang::Gleam => imp.segs.last().cloned(),
                    _ => imp.path.clone().or_else(|| imp.segs.first().cloned()),
                };
                let Some(name) = name else { continue };
                let name = name.trim_start_matches("node:").to_lowercase();
                let mut id = make_id(&[&name]);
                if id.is_empty() {
                    continue;
                }
                if ids.contains_key(&id) {
                    id = format!("external_{id}");
                }
                externals.insert(id.clone(), name);
                push(
                    &mut out,
                    PEdge {
                        s: fsym.clone(),
                        t: id,
                        rel: "imports_from",
                        score: 1.0,
                        ctx: "import",
                        file: *fi,
                        line: imp.line,
                    },
                );
            }
        }
    }

    let is_self = |q: &str| matches!(q, "self" | "Self" | "this" | "cls");
    // Candidates for a name seen in file `fi`, from symbol `from`: (target, exact)
    let pick = |fi: usize,
                from: usize,
                name: &str,
                qual: Option<&str>,
                recv: bool,
                want_class: bool|
     -> Option<(usize, bool)> {
        let cands: Vec<usize> = by_name
            .get(&ci(fi, name))?
            .iter()
            .copied()
            .filter(|&c| c != from)
            .filter(|&c| !want_class || syms[c].kind == Kind::Class)
            .collect();
        let mut cands = cands;
        let mut by_class = false;
        if let Some(q) = qual.filter(|q| !is_self(q)) {
            let m: Vec<usize> = cands
                .iter()
                .copied()
                .filter(|&c| syms[c].parent.map(|p| syms[p].name == q).unwrap_or(false))
                .collect();
            if !m.is_empty() {
                cands = m;
                by_class = true;
            }
        }
        let own_class = syms[from].parent.or(Some(from));
        let same_class: Vec<usize> = cands
            .iter()
            .copied()
            .filter(|&c| syms[c].parent.is_some() && syms[c].parent == own_class)
            .collect();
        if same_class.len() == 1 {
            return Some((same_class[0], true));
        }
        let same: Vec<usize> = cands
            .iter()
            .copied()
            .filter(|&c| syms[c].file == fi)
            .collect();
        if same.len() == 1 {
            return Some((same[0], true));
        }
        if !same.is_empty() {
            return None;
        }
        let via: Vec<usize> = cands
            .iter()
            .copied()
            .filter(|&c| imported[fi].contains(&syms[c].file))
            .collect();
        if via.len() == 1 {
            return Some((via[0], true));
        }
        // `x.join()` on an unknown receiver is usually a std method: no guess across files
        if cands.len() == 1 && (!recv || by_class) {
            return Some((cands[0], false));
        }
        None
    };

    for (fi, (cls, rel, name, line)) in &all_bases {
        if let Some((t, exact)) = pick(*fi, *cls, name, None, false, true) {
            push(
                &mut out,
                PEdge {
                    s: syms[*cls].id.clone(),
                    t: syms[t].id.clone(),
                    rel,
                    score: if exact { 1.0 } else { 0.8 },
                    ctx: "",
                    file: *fi,
                    line: *line,
                },
            );
        }
    }
    for (fi, c) in &all_calls {
        if let Some((t, exact)) = pick(
            *fi,
            c.caller,
            &c.call.name,
            c.call.qual.as_deref(),
            c.call.recv,
            false,
        ) {
            let src_id = &syms[c.caller].id;
            if *src_id != syms[t].id {
                push(
                    &mut out,
                    PEdge {
                        s: src_id.clone(),
                        t: syms[t].id.clone(),
                        rel: "calls",
                        score: if exact { 1.0 } else { 0.8 },
                        ctx: "call",
                        file: *fi,
                        line: c.line,
                    },
                );
            }
        }
    }

    // assemble, sorted by id
    let mut nodes: Vec<Node> = syms
        .iter()
        .map(|s| Node {
            id: s.id.clone(),
            label: s.label.clone(),
            file: files[s.file].path.clone(),
            line: s.line,
            ..Default::default()
        })
        .collect();
    for (id, name) in externals {
        nodes.push(Node {
            id,
            label: name,
            external: true,
            ..Default::default()
        });
    }
    nodes.sort_by(|a, b| a.id.cmp(&b.id));
    let ix: HashMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let mut es: Vec<Edge> = out
        .iter()
        .filter_map(|e| {
            Some(Edge {
                s: *ix.get(e.s.as_str())?,
                t: *ix.get(e.t.as_str())?,
                rel: e.rel.to_string(),
                conf: if e.score < 1.0 {
                    "INFERRED"
                } else {
                    "EXTRACTED"
                }
                .to_string(),
                score: e.score,
                ctx: e.ctx.to_string(),
                file: files[e.file].path.clone(),
                line: e.line,
            })
        })
        .collect();
    es.sort_by(|a, b| (a.s, a.t, &a.rel, a.line).cmp(&(b.s, b.t, &b.rel, b.line)));
    let mut g = Graph { nodes, edges: es };
    g.cluster();
    g
}
