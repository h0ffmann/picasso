//! viz: the code graph of a repository with graphify, the marola-devkit way: code-only, no API key,
//! no model, written under ~/.cache and never into the checkout. `viz --help` lists the commands.
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};

// Baked in at build time by the lab's flake; from `cargo run` the variable is unset and PATH is used.
const GRAPHIFY: Option<&str> = option_env!("GRAPHIFY");

const USAGE: &str = "\
viz — the code graph of a repository with graphify (code-only, keyless, offline)
  viz build [dir]                         extract and cluster <dir> (default .)
  viz update [dir]                        the same, plus the Markdown: pages, headings and links
  viz query \"<question>\"                  answer from the graph, capped at --budget 400
  viz path <a> <b> | explain <x> | ...    any other graphify command, on the same graph
  viz where                               the output directory (graph.json, graph.html)
VIZ_OUT overrides the output directory (default ~/.cache/marola-graph/<repo>).
";

fn var(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|v| !v.is_empty())
}

fn out_dir(dir: &str) -> PathBuf {
    let git = Command::new("git").args(["-C", dir, "rev-parse", "--show-toplevel"]).stderr(Stdio::null()).output();
    let root = git
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
        .unwrap_or_else(|| std::fs::canonicalize(dir).unwrap_or_else(|_| PathBuf::from(dir)));
    if let Some(o) = var("VIZ_OUT") {
        return o.into();
    }
    let cache = var("XDG_CACHE_HOME").unwrap_or_else(|| format!("{}/.cache", var("HOME").unwrap_or_default()));
    Path::new(&cache).join("marola-graph").join(root.file_name().unwrap_or_default())
}

/// env -i: graphify reads provider keys from the environment, and the LLM pass is what costs tokens.
/// The discard port makes any model call fail at once instead of reaching a local Ollama.
fn graphify(bin: &str, out: &Path, args: &[String]) -> Command {
    let mut c = Command::new(bin);
    c.env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", out)
        .env("GRAPHIFY_OUT", out)
        .env("GRAPHIFY_NO_AUTO_REFRESH", "1")
        .env("PYTHONHASHSEED", "0")
        .env("OLLAMA_BASE_URL", "http://127.0.0.1:9")
        .args(args);
    c
}

/// The graphify invocations one viz command makes, in order. `graph` is `<out>/graph.json`.
fn plan(cmd: &str, rest: &[String], graph: &str) -> Vec<Vec<String>> {
    let s = |a: &[&str]| a.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    let dir = rest.first().map_or(".", String::as_str);
    match cmd {
        "build" => vec![s(&["extract", dir, "--code-only", "--no-label"]), s(&["cluster-only", dir, "--no-label", "--graph", graph])],
        // graphify's update takes no --graph: GRAPHIFY_OUT alone says where it writes.
        "update" => vec![s(&["update", dir])],
        "query" => vec![[&["query".to_string()], rest, &s(&["--graph", graph, "--budget", "400"])].concat()],
        _ => vec![[&[cmd.to_string()], rest, &s(&["--graph", graph])].concat()],
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let cmd = args.next().unwrap_or_else(|| "build".into());
    let rest: Vec<String> = args.collect();
    match cmd.as_str() {
        "-h" | "--help" | "help" => return print!("{USAGE}"),
        "where" => return println!("{}", out_dir(".").display()),
        _ => {}
    }
    let bin = var("GRAPHIFY").or(GRAPHIFY.map(String::from)).unwrap_or_else(|| "graphify".into());
    // build and update graph the directory they are given; the rest use the repository we are in.
    let dir = if matches!(cmd.as_str(), "build" | "update") { rest.first().map_or(".", String::as_str) } else { "." };
    let out = out_dir(dir);
    let graph = out.join("graph.json").display().to_string();
    if let Err(e) = std::fs::create_dir_all(&out) {
        eprintln!("viz: {}: {e}", out.display());
        exit(1);
    }
    for call in plan(&cmd, &rest, &graph) {
        match graphify(&bin, &out, &call).status() {
            Ok(s) if s.success() => {}
            Ok(s) => exit(s.code().unwrap_or(1)),
            Err(e) => {
                eprintln!("viz: {bin}: {e}");
                exit(127);
            }
        }
    }
    if matches!(cmd.as_str(), "build" | "update") {
        println!("viz: {}/graph.html", out.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn build_extracts_code_only_then_clusters_into_graph_json() {
        assert_eq!(
            plan("build", &v(&["."]), "/o/graph.json"),
            [v(&["extract", ".", "--code-only", "--no-label"]), v(&["cluster-only", ".", "--no-label", "--graph", "/o/graph.json"])]
        );
    }

    #[test]
    fn update_passes_no_graph() {
        assert_eq!(plan("update", &v(&["."]), "/o/graph.json"), [v(&["update", "."])]);
    }

    #[test]
    fn query_is_capped_and_other_commands_pass_through() {
        assert_eq!(plan("query", &v(&["where is x"]), "/o/g"), [v(&["query", "where is x", "--graph", "/o/g", "--budget", "400"])]);
        assert_eq!(plan("path", &v(&["a", "b"]), "/o/g"), [v(&["path", "a", "b", "--graph", "/o/g"])]);
    }

    #[test]
    fn graphify_runs_with_a_scrubbed_environment() {
        std::env::set_var("ANTHROPIC_API_KEY", "secret");
        let text = graphify("env", Path::new("/o"), &[]).output().unwrap().stdout;
        let text = String::from_utf8_lossy(&text);
        let mut lines: Vec<&str> = text.lines().filter(|l| !l.starts_with("PATH=")).collect();
        lines.sort();
        assert_eq!(lines, ["GRAPHIFY_NO_AUTO_REFRESH=1", "GRAPHIFY_OUT=/o", "HOME=/o", "OLLAMA_BASE_URL=http://127.0.0.1:9", "PYTHONHASHSEED=0"], "no provider key leaks");
    }

    #[test]
    fn where_defaults_to_the_cache() {
        let t = std::env::temp_dir().join(format!("viz-{}", std::process::id())).join("repo");
        std::fs::create_dir_all(&t).unwrap();
        std::env::remove_var("VIZ_OUT");
        std::env::set_var("XDG_CACHE_HOME", "/c");
        assert_eq!(out_dir(t.to_str().unwrap()), Path::new("/c/marola-graph/repo"));
        std::env::set_var("VIZ_OUT", "/x");
        assert_eq!(out_dir(t.to_str().unwrap()), Path::new("/x"));
    }
}
