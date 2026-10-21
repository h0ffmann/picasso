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
