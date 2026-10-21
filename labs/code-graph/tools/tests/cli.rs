use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_code-graph"))
}

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("code-graph-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn example() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../example")
}

fn extract(src: &Path, out: &Path) -> Value {
    let st = bin()
        .arg("extract")
        .arg(src)
        .arg("--out")
        .arg(out)
        .status()
        .unwrap();
    assert!(st.success());
    serde_json::from_str(&std::fs::read_to_string(out.join("graph.json")).unwrap()).unwrap()
}

fn has_edge(g: &Value, s: &str, rel: &str, t: &str) -> bool {
    g["links"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["source"] == s && e["target"] == t && e["relation"] == rel)
}

#[test]
fn example_has_the_call_and_the_import() {
    let g = extract(&example(), &scratch("ex"));
    assert!(has_edge(&g, "greet_greet", "calls", "shout_shout"));
    assert!(has_edge(&g, "greet", "imports_from", "shout"));
    assert!(has_edge(&g, "greet", "contains", "greet_main"));
    assert_eq!(g["directed"], false);
    assert!(g["nodes"][0]["label"].is_string());
}

#[test]
fn same_tree_gives_the_same_bytes() {
    // this crate's own sources: Rust, a few hundred nodes
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let (a, b) = (scratch("det-a"), scratch("det-b"));
    extract(&src, &a);
    extract(&src, &b);
    for f in ["graph.json", "graph.html", "GRAPH_REPORT.md"] {
        assert_eq!(
            std::fs::read(a.join(f)).unwrap(),
            std::fs::read(b.join(f)).unwrap(),
            "{f} differs between runs"
        );
    }
}

#[test]
fn html_is_self_contained() {
    for src in [example(), Path::new(env!("CARGO_MANIFEST_DIR")).join("src")] {
        let out = scratch("html");
        extract(&src, &out);
        let h = std::fs::read_to_string(out.join("graph.html")).unwrap();
        assert!(h.starts_with("<!doctype html>") && h.contains("<svg") && h.contains("<circle"));
        for bad in [
            "http://", "https://", "src=", "href=", "@import", "url(", "<link",
        ] {
            assert!(!h.contains(bad), "graph.html contains {bad}");
        }
        assert!(h.contains("prefers-color-scheme:dark"));
    }
}

#[test]
fn html_from_graph_json_equals_html_from_extract() {
    let out = scratch("html-eq");
    extract(&example(), &out);
    let st = bin()
        .arg("html")
        .arg("--graph")
        .arg(out.join("graph.json"))
        .arg("--out")
        .arg(out.join("again.html"))
        .status()
        .unwrap();
    assert!(st.success());
    let (a, b) = (
        std::fs::read_to_string(out.join("graph.html")).unwrap(),
        std::fs::read_to_string(out.join("again.html")).unwrap(),
    );
    // the title is the directory name for extract and "graph" for html; the drawing is the same
    let strip = |s: &str| {
        s.split("<svg")
            .nth(1)
            .unwrap()
            .split("</svg>")
            .next()
            .unwrap()
            .replace("Code graph of example", "")
            .replace("Code graph of graph", "")
    };
    assert_eq!(strip(&a), strip(&b));
}

#[test]
fn polyglot_tree() {
    let d = scratch("poly");
    let w = |p: &str, s: &str| {
        let f = d.join(p);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, s).unwrap();
    };
    w("pkg/a.py", "from pkg.b import Base, make\n\nclass Child(Base):\n    def go(self):\n        return make(1)\n");
    w(
        "pkg/b.py",
        "class Base:\n    def hi(self): pass\n\ndef make(x): return Base()\n",
    );
    w("web.ts", "import { foo } from \"./lib\";\nexport class A { m() { return foo(); } }\nexport function run() { new A().m(); }\n");
    w("lib.js", "export function foo() { return 1; }\n");
    w("S.scala", "object S { def f(): Int = g(); def g(): Int = 1 }\nclass C extends T { def h() = S.f() }\ntrait T\n");
    w("m.f90", "module mm\ncontains\nsubroutine s1()\n  call s2()\nend subroutine\nsubroutine s2()\nend subroutine\nend module\n");
    w(
        "app.gleam",
        "import util\npub fn main() { util.helper() }\n",
    );
    w("util.gleam", "pub fn helper() { 1 }\n");
    w("flake.nix", "{ a = 1; }\n");
    w("nlib.nix", "let f = x: g x; g = y: y; in { inherit f; }\n");
    w("default.nix", "import ./nlib.nix\n");
    let g = extract(&d, &d.join("out"));
    assert!(has_edge(&g, "pkg_a_child", "inherits", "pkg_b_base"));
    assert!(has_edge(&g, "pkg_a_child_go", "calls", "pkg_b_make"));
    assert!(has_edge(&g, "pkg_a", "imports_from", "pkg_b"));
    assert!(has_edge(&g, "pkg_a_child", "method", "pkg_a_child_go"));
    assert!(has_edge(&g, "web_run", "calls", "web_a_m"));
    assert!(has_edge(&g, "web", "imports_from", "lib"));
    assert!(has_edge(&g, "s_c_h", "calls", "s_s_f"));
    assert!(has_edge(&g, "s_c", "inherits", "s_t"));
    assert!(has_edge(&g, "m_s1", "calls", "m_s2"));
    assert!(has_edge(&g, "m", "defines", "m_mm"));
    assert!(has_edge(&g, "app_main", "calls", "util_helper"));
    assert!(has_edge(&g, "app", "imports_from", "util"));
    assert!(has_edge(&g, "nlib_f", "calls", "nlib_g"));
    assert!(has_edge(&g, "default", "imports_from", "nlib"));
    assert!(g["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["id"] == "flake"));
}

#[test]
fn query_path_explain() {
    let out = scratch("q");
    extract(&example(), &out);
    let g = out.join("graph.json");
    let run = |args: &[&str]| {
        let o = bin().args(args).arg("--graph").arg(&g).output().unwrap();
        assert!(o.status.success(), "{args:?}");
        String::from_utf8(o.stdout).unwrap()
    };
    let q = run(&["query", "greet"]);
    assert!(q.contains("NODE greet()") && q.contains("EDGE greet() --calls"));
    assert!(run(&["query", "greet", "--budget", "20"]).len() < q.len());
    assert!(run(&["path", "greet()", "shout()"]).contains("greet() --calls [EXTRACTED]--> shout()"));
    assert!(run(&["explain", "shout()"]).contains("<-- greet() [calls]"));
}

#[test]
fn reads_graphifys_own_graph_json() {
    let d = scratch("gfy");
    let j = r#"{"directed":false,"multigraph":false,"graph":{},"nodes":[{"id":"a","label":"a()","source_file":"a.py","source_location":"L1","community":0},{"id":"b","label":"b()","source_file":"a.py","source_location":"L3","community":0}],"edges":[{"source":"a","target":"b","relation":"calls","confidence":"EXTRACTED"}]}"#;
    std::fs::write(d.join("graph.json"), j).unwrap();
    let o = bin()
        .args(["path", "a()", "b()", "--graph"])
        .arg(d.join("graph.json"))
        .output()
        .unwrap();
    assert!(String::from_utf8(o.stdout)
        .unwrap()
        .contains("a() --calls [EXTRACTED]--> b()"));
}

#[test]
fn nested_repositories_and_excluded_paths_are_skipped() {
    let d = scratch("skip");
    let w = |p: &str, s: &str| {
        let f = d.join(p);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, s).unwrap();
    };
    w("own.py", "def own(): pass\n");
    w("sub/.git", "gitdir: ../.git/modules/sub\n"); // a submodule has a .git file
    w("sub/theirs.py", "def theirs(): pass\n");
    w("nested/.git/HEAD", "ref: refs/heads/main\n"); // a nested repository has a .git directory
    w("nested/theirs2.py", "def theirs2(): pass\n");
    w("extra/lib.py", "def vend(): pass\n");
    let ids = |g: &Value| -> Vec<String> {
        g["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n["id"].as_str().unwrap().to_string())
            .collect()
    };
    let g = extract(&d, &d.join("o1"));
    let all = ids(&g);
    assert!(all.contains(&"own_own".to_string()) && all.contains(&"extra_lib".to_string()));
    assert!(!all
        .iter()
        .any(|i| i.starts_with("sub") || i.starts_with("nested")));
    let st = bin()
        .arg("extract")
        .arg(&d)
        .args(["--exclude", "extra/", "--title", "My page", "--out"])
        .arg(d.join("o2"))
        .status()
        .unwrap();
    assert!(st.success());
    let g: Value =
        serde_json::from_str(&std::fs::read_to_string(d.join("o2/graph.json")).unwrap()).unwrap();
    assert!(!ids(&g).iter().any(|i| i.starts_with("extra")));
    let h = std::fs::read_to_string(d.join("o2/graph.html")).unwrap();
    assert!(h.contains("<title>My page</title>") && h.contains("<h1>My page</h1>"));
}

fn write_all(d: &Path, files: &[(&str, &str)]) {
    for (p, s) in files {
        let f = d.join(p);
        std::fs::create_dir_all(f.parent().unwrap()).unwrap();
        std::fs::write(f, s).unwrap();
    }
}

fn linked(src: &Path, out: &Path, link: &str) -> Value {
    let st = bin()
        .arg("extract")
        .arg(src)
        .args(["--link", link, "--out"])
        .arg(out)
        .status()
        .unwrap();
    assert!(st.success());
    serde_json::from_str(&std::fs::read_to_string(out.join("graph.json")).unwrap()).unwrap()
}

fn node<'a>(g: &'a Value, id: &str) -> &'a Value {
    g["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == id)
        .unwrap_or_else(|| panic!("no node {id}"))
}

/// Repo `a` refers to repo `b` by a flake `?dir=` URL and by a `lab=` line.
fn two_repos(name: &str) -> (PathBuf, PathBuf) {
    let d = scratch(name);
    let (a, b) = (d.join("a"), d.join("b"));
    write_all(
        &b,
        &[
            ("labs/x/flake.nix", "{ description = \"x\"; }\n"),
            ("labs/x/src/lib.rs", "pub fn f() { g(); }\nfn g() {}\n"),
            ("labs/y/main.rs", "fn main() { h(); }\nfn h() {}\n"),
            ("labs/z/other.rs", "fn z() {}\n"),
        ],
    );
    write_all(
        &a,
        &[
            (
                "flake.nix",
                "{ inputs.b.url = \"github:o/b?dir=labs/x\"; inputs.c.url = \"github:o/c?dir=labs/z\"; }\n",
            ),
            ("tools.lock", "rev=1\nlab=labs/y\nlab=labs/gone\n"),
            ("a.rs", "fn a() {}\n"),
        ],
    );
    extract(&b, &d.join("b-out"));
    (a, d.join("b-out/graph.json"))
}

#[test]
fn link_adds_the_referenced_nodes_of_another_repository() {
    let (a, bj) = two_repos("link");
    let spec = format!("b={}@b.html", bj.display());
    let out = a.parent().unwrap().join("a-out");
    let g = linked(&a, &out, &spec);
    let x = node(&g, "b:labs_x_flake");
    assert_eq!(x["external"], "b");
    assert_eq!(x["label"], "b: labs/x");
    assert_eq!(x["linked_label"], "flake.nix");
    assert_eq!(x["href"], "b.html#node=labs_x_flake");
    assert_eq!(x["source_file"], "b:labs/x/flake.nix");
    assert_eq!(x["community_name"], "b");
    assert_eq!(node(&g, "b:labs_y_main")["community_name"], "b");
    // the file that holds the reference is the edge's source; a lock file gets a node of its own
    assert!(has_edge(&g, "flake", "depends_on", "b:labs_x_flake"));
    assert!(has_edge(&g, "tools_lock", "depends_on", "b:labs_y_main"));
    assert_eq!(node(&g, "tools_lock")["source_file"], "tools.lock");
    let e = g["links"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["source"] == "tools_lock")
        .unwrap();
    assert_eq!(e["source_location"], "L2");
    // only the referenced nodes enter: not labs/z (another repo's URL), not labs/gone, not the symbols
    let ext: Vec<_> = g["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|n| n["id"].as_str().unwrap().starts_with("b:"))
        .collect();
    assert_eq!(ext.len(), 2);
    // the page: one relative link per external node, no network, no href attribute
    let h = std::fs::read_to_string(out.join("graph.html")).unwrap();
    assert!(h.contains("b.html#node=labs_x_flake") && h.contains("external: b"));
    // the relations legend lists what the graph has
    assert!(h.contains("<i class=\"l depends_on\">") && !h.contains("<i class=\"l references\">"));
    for bad in [
        "http://", "https://", "src=", "href=", "@import", "url(", "<link",
    ] {
        assert!(!h.contains(bad), "graph.html contains {bad}");
    }
}

#[test]
fn link_output_is_deterministic_and_off_by_default() {
    let (a, bj) = two_repos("link-det");
    let spec = format!("b={}", bj.display());
    let (o1, o2) = (
        a.parent().unwrap().join("o1"),
        a.parent().unwrap().join("o2"),
    );
    linked(&a, &o1, &spec);
    linked(&a, &o2, &spec);
    for f in ["graph.json", "graph.html", "GRAPH_REPORT.md"] {
        assert_eq!(
            std::fs::read(o1.join(f)).unwrap(),
            std::fs::read(o2.join(f)).unwrap(),
            "{f} differs between runs"
        );
    }
    // the URL defaults to NAME.html
    let g: Value =
        serde_json::from_str(&std::fs::read_to_string(o1.join("graph.json")).unwrap()).unwrap();
    assert_eq!(
        node(&g, "b:labs_x_flake")["href"],
        "b.html#node=labs_x_flake"
    );
    // a link to a graph that has none of the referenced paths leaves graph.json as without --link
    let plain = a.parent().unwrap().join("plain");
    extract(&a, &plain);
    let none = a.parent().unwrap().join("none");
    linked(
        &a,
        &none,
        &format!("nomatch={}", plain.join("graph.json").display()),
    );
    assert!(
        std::fs::read(plain.join("graph.json")).unwrap()
            == std::fs::read(none.join("graph.json")).unwrap()
    );
    let st = bin()
        .arg("extract")
        .arg(&a)
        .args(["--link", "b=x.json@javascript:alert(1)", "--out"])
        .arg(a.parent().unwrap().join("bad"))
        .status()
        .unwrap();
    assert!(!st.success());
}
