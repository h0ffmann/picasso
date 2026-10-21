//! code-graph: the code-only graph of a repository, written as graphify's graph.json.
mod extract;
mod graph;
mod html;
mod lang;
mod link;
mod query;

use graph::Graph;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "code-graph: the code graph of a repository, in graphify's graph.json schema

usage:
  code-graph extract DIR [--out OUTDIR] [--no-viz] [--exclude PATH]... [--title TEXT]
                         [--link NAME=PATH/graph.json[@URL]]...
                                                     parse DIR, write graph.json, GRAPH_REPORT.md, graph.html
  code-graph update DIR [--out OUTDIR] [--no-viz]    same as extract (nothing is cached)
  code-graph cluster-only DIR [--graph FILE]         cluster an existing graph.json again
  code-graph html [--graph FILE] [--out FILE] [--title TEXT]        draw graph.html from a graph.json
  code-graph query \"QUESTION\" [--budget N] [--depth N] [--graph FILE]
  code-graph path A B [--graph FILE]
  code-graph explain X [--graph FILE]
A directory below DIR that has its own .git (a submodule, a nested repository) is skipped.
--exclude PATH skips one more path, relative to DIR; repeat it. --title names the page (default: the directory name).
--link NAME=GRAPH[@URL] adds the nodes of another repository's graph.json that DIR refers to (a flake
?dir=PATH, a GitHub tree/blob URL, lab=PATH) as external nodes; clicking one opens URL (default NAME.html).
(--code-only and --no-label are accepted and ignored: this tool only does code, with no model)
Default OUTDIR and graph: graphify-out/ (graph.json).
";

struct Args {
    pos: Vec<String>,
    out: Option<PathBuf>,
    graph: Option<PathBuf>,
    budget: usize,
    depth: usize,
    viz: bool,
    exclude: Vec<String>,
    title: Option<String>,
    link: Vec<String>,
}

fn parse(v: Vec<String>) -> Result<Args, String> {
    let mut a = Args {
        pos: vec![],
        out: None,
        graph: None,
        budget: 2000,
        depth: 2,
        viz: true,
        exclude: vec![],
        title: None,
        link: vec![],
    };
    let mut it = v.into_iter();
    while let Some(x) = it.next() {
        let mut val = |name: &str| it.next().ok_or(format!("{name} needs a value"));
        match x.as_str() {
            "--out" => a.out = Some(val("--out")?.into()),
            "--graph" => a.graph = Some(val("--graph")?.into()),
            "--budget" => {
                a.budget = val("--budget")?
                    .parse()
                    .map_err(|_| "--budget needs a number")?
            }
            "--depth" => {
                a.depth = val("--depth")?
                    .parse()
                    .map_err(|_| "--depth needs a number")?
            }
            "--no-viz" => a.viz = false,
            "--exclude" => a.exclude.push(val("--exclude")?),
            "--link" => a.link.push(val("--link")?),
            "--title" => a.title = Some(val("--title")?),
            "--code-only" | "--no-label" => {}
            _ => a.pos.push(x),
        }
    }
    Ok(a)
}

fn load(p: &Path) -> Result<Graph, String> {
    let s = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
    Graph::from_json(&serde_json::from_str(&s).map_err(|e| format!("{}: {e}", p.display()))?)
}

fn report(g: &Graph, name: &str) -> String {
    let deg = g.degree();
    let mut o = format!(
        "# Graph Report - {name}\n\n## Summary\n- {} nodes, {} edges, {} communities\n",
        g.nodes.len(),
        g.edges.len(),
        g.nodes.iter().map(|n| n.community + 1).max().unwrap_or(0)
    );
    let (mut ext, mut inf) = (0, 0);
    for e in &g.edges {
        if e.conf == "INFERRED" {
            inf += 1
        } else {
            ext += 1
        }
    }
    o += &format!("- Extraction: {ext} EXTRACTED, {inf} INFERRED. Clustering: Louvain (not graphify's Leiden).\n\n");
    let mut idx: Vec<usize> = (0..g.nodes.len())
        .filter(|&i| !g.nodes[i].external && g.nodes[i].label.ends_with(')'))
        .collect();
    idx.sort_by_key(|&i| (std::cmp::Reverse(deg[i]), i));
    o += "## God Nodes (most connected)\n";
    for (k, &i) in idx.iter().take(10).enumerate() {
        o += &format!(
            "{}. `{}` - {} edges ({})\n",
            k + 1,
            g.nodes[i].label,
            deg[i],
            g.nodes[i].file
        );
    }
    o += "\n## Cross-Community Calls\n";
    let mut shown = 0;
    for e in g
        .edges
        .iter()
        .filter(|e| e.rel == "calls" && g.nodes[e.s].community != g.nodes[e.t].community)
    {
        if shown == 10 {
            break;
        }
        shown += 1;
        o += &format!(
            "- `{}` --calls--> `{}` [{}]\n",
            g.nodes[e.s].label, g.nodes[e.t].label, e.conf
        );
    }
    if shown == 0 {
        o += "- None.\n";
    }
    let mut by: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
    for (i, n) in g.nodes.iter().enumerate() {
        by.entry(n.community).or_default().push(i);
    }
    o += &format!("\n## Communities ({} total)\n", by.len());
    for (c, mut m) in by {
        m.sort_by_key(|&i| (std::cmp::Reverse(deg[i]), i));
        let names: Vec<&str> = m
            .iter()
            .take(6)
            .map(|&i| g.nodes[i].label.as_str())
            .collect();
        o += &format!(
            "- **{c} {}** ({} nodes): {}\n",
            g.nodes[m[0]].label,
            m.len(),
            names.join(", ")
        );
    }
    o
}

fn write(p: &Path, s: &str) -> Result<(), String> {
    std::fs::write(p, s).map_err(|e| format!("{}: {e}", p.display()))
}

fn emit(g: &Graph, dir: &Path, name: &str, viz: bool) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&g.to_json()).map_err(|e| e.to_string())?;
    write(&dir.join("graph.json"), &(json + "\n"))?;
    write(&dir.join("GRAPH_REPORT.md"), &report(g, name))?;
    if viz {
        write(&dir.join("graph.html"), &html::render(g, name))?;
    }
    eprintln!(
        "[code-graph] wrote {}: {} nodes, {} edges, {} communities",
        dir.join("graph.json").display(),
        g.nodes.len(),
        g.edges.len(),
        g.nodes.iter().map(|n| n.community + 1).max().unwrap_or(0)
    );
    Ok(())
}

fn run() -> Result<(), String> {
    let mut argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.is_empty() || argv[0] == "--help" || argv[0] == "-h" {
        print!("{USAGE}");
        return Ok(());
    }
    let cmd = argv.remove(0);
    let a = parse(argv)?;
    let gpath = a
        .graph
        .clone()
        .unwrap_or_else(|| "graphify-out/graph.json".into());
    let dir_arg = || {
        a.pos
            .first()
            .map(PathBuf::from)
            .ok_or("a directory is needed".to_string())
    };
    match cmd.as_str() {
        "extract" | "update" => {
            let root = dir_arg()?;
            let canon = root
                .canonicalize()
                .map_err(|e| format!("{}: {e}", root.display()))?;
            let name = canon
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "graph".into());
            let mut g = extract::extract(&canon, &a.exclude);
            if !a.link.is_empty() {
                let mut links = vec![];
                for spec in &a.link {
                    let (name, path, url) = link::parse_spec(spec)?;
                    links.push(link::Link {
                        name,
                        graph: load(&path)?,
                        url,
                    });
                }
                let files: Vec<(String, String)> = extract::text_files(&canon, &a.exclude)
                    .into_iter()
                    .filter_map(|p| Some((std::fs::read_to_string(canon.join(&p)).ok()?, p)))
                    .map(|(t, p)| (p, t))
                    .collect();
                link::apply(&mut g, &files, &links);
            }
            emit(
                &g,
                &a.out.clone().unwrap_or_else(|| root.join("graphify-out")),
                a.title.as_deref().unwrap_or(&name),
                a.viz,
            )
        }
        "cluster-only" => {
            let root = dir_arg()?;
            let p = a
                .graph
                .clone()
                .unwrap_or_else(|| root.join("graphify-out/graph.json"));
            let mut g = load(&p)?;
            g.cluster();
            emit(
                &g,
                p.parent().unwrap_or(Path::new(".")),
                a.title.as_deref().unwrap_or("graph"),
                a.viz,
            )
        }
        "html" => {
            let g = load(&gpath)?;
            write(
                &a.out.clone().unwrap_or_else(|| "graph.html".into()),
                &html::render(&g, a.title.as_deref().unwrap_or("graph")),
            )
        }
        "query" => {
            let q = a.pos.first().ok_or("a question is needed")?;
            let g = load(&gpath)?;
            print!(
                "{}",
                query::Q::new(&g).query(q, a.budget, a.depth, &gpath.display().to_string())
            );
            Ok(())
        }
        "path" => {
            let (x, y) = (
                a.pos.first().ok_or("two names are needed")?,
                a.pos.get(1).ok_or("two names are needed")?,
            );
            print!("{}", query::Q::new(&load(&gpath)?).path(x, y)?);
            Ok(())
        }
        "explain" => {
            print!(
                "{}",
                query::Q::new(&load(&gpath)?).explain(a.pos.first().ok_or("a name is needed")?)?
            );
            Ok(())
        }
        _ => Err(format!("unknown command {cmd}\n\n{USAGE}")),
    }
}

fn main() -> ExitCode {
    // trees can be deep: parse and walk on a thread with a big stack
    let h = std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(run)
        .expect("thread");
    match h.join().expect("worker") {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("code-graph: {e}");
            ExitCode::FAILURE
        }
    }
}
