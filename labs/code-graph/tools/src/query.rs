//! query, path and explain over a graph.json: label matching, a bounded BFS, shortest path.
use crate::graph::Graph;
use std::collections::{HashMap, VecDeque};

const STOP: &[&str] = &[
    "a", "an", "the", "is", "are", "of", "in", "on", "to", "and", "or", "how", "what", "where",
    "which", "does", "do", "for", "with", "from", "it", "this", "that",
];

fn tokens(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|x| !x.is_empty())
        .map(String::from)
        .collect()
}

pub struct Q<'a> {
    pub g: &'a Graph,
    adj: Vec<Vec<usize>>, // edge indexes, in edge order
    deg: Vec<usize>,
}

impl<'a> Q<'a> {
    pub fn new(g: &'a Graph) -> Self {
        let mut adj = vec![Vec::new(); g.nodes.len()];
        for (i, e) in g.edges.iter().enumerate() {
            adj[e.s].push(i);
            if e.s != e.t {
                adj[e.t].push(i);
            }
        }
        let deg = adj.iter().map(Vec::len).collect();
        Q { g, adj, deg }
    }

    fn other(&self, e: usize, n: usize) -> usize {
        let ed = &self.g.edges[e];
        if ed.s == n {
            ed.t
        } else {
            ed.s
        }
    }

    /// Nodes best first. A term scores most as the whole label, less as a prefix or a token, least as a substring.
    pub fn score(&self, question: &str) -> Vec<(f64, usize)> {
        let raw = tokens(question);
        let mut terms: Vec<String> = raw
            .iter()
            .filter(|t| !STOP.contains(&t.as_str()))
            .cloned()
            .collect();
        if terms.is_empty() {
            terms = raw;
        }
        terms.dedup();
        if terms.is_empty() {
            return vec![];
        }
        let labels: Vec<String> = self
            .g
            .nodes
            .iter()
            .map(|n| n.label.to_lowercase())
            .collect();
        let n = self.g.nodes.len() as f64;
        let idf: Vec<f64> = terms
            .iter()
            .map(|t| {
                (1.0 + n / (1.0 + labels.iter().filter(|l| l.contains(t.as_str())).count() as f64))
                    .ln()
            })
            .collect();
        let joined = terms.join(" ");
        let mut out = Vec::new();
        for (i, nd) in self.g.nodes.iter().enumerate() {
            let bare = labels[i].trim_end_matches("()").trim_start_matches('.');
            let lt = tokens(&nd.label);
            let src = nd.file.to_lowercase();
            let (mut sc, mut matched) = (0.0, 0);
            if joined == bare || joined == nd.id {
                sc += 100.0 * idf.iter().cloned().fold(0.0, f64::max);
            }
            for (t, w) in terms.iter().zip(&idf) {
                let tier = if t == bare {
                    10.0
                } else if bare.starts_with(t.as_str()) || lt.contains(t) {
                    5.0
                } else if labels[i].contains(t.as_str()) {
                    2.0
                } else {
                    0.0
                };
                if tier > 0.0 {
                    matched += 1;
                }
                sc += tier * *w + if src.contains(t.as_str()) { *w } else { 0.0 };
            }
            if matched > 0 {
                sc *= (matched as f64 / terms.len() as f64).powi(2);
            }
            if sc > 0.0 {
                out.push((sc, i));
            }
        }
        out.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap()
                .then(
                    self.g.nodes[a.1]
                        .label
                        .len()
                        .cmp(&self.g.nodes[b.1].label.len()),
                )
                .then(self.g.nodes[a.1].id.cmp(&self.g.nodes[b.1].id))
        });
        out
    }

    fn find(&self, name: &str) -> Option<usize> {
        self.score(name).first().map(|x| x.1)
    }

    fn node_line(&self, i: usize) -> String {
        let n = &self.g.nodes[i];
        format!(
            "NODE {} [src={} loc={} community={}]",
            n.label,
            n.file,
            if n.line > 0 {
                format!("L{}", n.line)
            } else {
                String::new()
            },
            n.cname
        )
    }

    fn edge_line(&self, i: usize) -> String {
        let e = &self.g.edges[i];
        let ctx = if e.ctx.is_empty() {
            String::new()
        } else {
            format!(" context={}", e.ctx)
        };
        let at = if e.line > 0 {
            format!(" at={}:L{}", e.file, e.line)
        } else {
            String::new()
        };
        format!(
            "EDGE {} --{} [{}{}]--> {}{}",
            self.g.nodes[e.s].label, e.rel, e.conf, ctx, self.g.nodes[e.t].label, at
        )
    }

    pub fn query(&self, question: &str, budget: usize, depth: usize, path: &str) -> String {
        let scored = self.score(question);
        let Some(&(top, _)) = scored.first() else {
            return format!("No node matches {question:?}.\n");
        };
        let mut seeds: Vec<usize> = Vec::new();
        for &(s, i) in &scored {
            if seeds.len() >= 3 || s < top * 0.2 {
                break;
            }
            if !seeds
                .iter()
                .any(|&j| self.g.nodes[j].label == self.g.nodes[i].label)
            {
                seeds.push(i);
            }
        }
        let mut sorted = self.deg.clone();
        sorted.sort_unstable();
        let hub = sorted
            .get(sorted.len() * 99 / 100)
            .copied()
            .unwrap_or(0)
            .max(50);
        let mut dist: HashMap<usize, usize> = seeds.iter().map(|&s| (s, 0)).collect();
        let mut frontier = seeds.clone();
        for hop in 1..=depth {
            let mut next = Vec::new();
            for &n in &frontier {
                if !seeds.contains(&n) && self.deg[n] >= hub {
                    continue;
                }
                for &e in &self.adj[n] {
                    let m = self.other(e, n);
                    if let std::collections::hash_map::Entry::Vacant(v) = dist.entry(m) {
                        v.insert(hop);
                        next.push(m);
                    }
                }
            }
            frontier = next;
        }
        let mut nodes: Vec<usize> = dist.keys().copied().collect();
        nodes.sort_by(|&a, &b| {
            (
                dist[&a],
                std::cmp::Reverse(self.deg[a]),
                &self.g.nodes[a].label,
                a,
            )
                .cmp(&(
                    dist[&b],
                    std::cmp::Reverse(self.deg[b]),
                    &self.g.nodes[b].label,
                    b,
                ))
        });
        let mut edges: Vec<usize> = (0..self.g.edges.len())
            .filter(|&i| {
                dist.contains_key(&self.g.edges[i].s) && dist.contains_key(&self.g.edges[i].t)
            })
            .collect();
        edges.sort_by_key(|&i| {
            (
                self.g.edges[i].rel != "calls",
                self.g.edges[i].rel.clone(),
                i,
            )
        });
        let start: Vec<String> = seeds
            .iter()
            .map(|&s| format!("{:?}", self.g.nodes[s].label).replace('"', "'"))
            .collect();
        let mut out = format!("Graph: {path} ({} nodes) | Traversal: BFS depth={depth} | Start: [{}] | {} nodes found\n\n", self.g.nodes.len(), start.join(", "), nodes.len());
        for &i in &nodes {
            out += &self.node_line(i);
            out.push('\n');
        }
        for &i in &edges {
            out += &self.edge_line(i);
            out.push('\n');
        }
        let cap = budget * 3;
        if out.len() > cap {
            let mut cut = cap;
            while !out.is_char_boundary(cut) {
                cut -= 1;
            }
            let cut = out[..cut].rfind('\n').filter(|&c| c > 0).unwrap_or(cut);
            out.truncate(cut);
            out += &format!("\n... (cut at about {budget} tokens)\n");
        }
        out
    }

    pub fn path(&self, a: &str, b: &str) -> Result<String, String> {
        let s = self.find(a).ok_or(format!("No node matches {a:?}."))?;
        let t = self.find(b).ok_or(format!("No node matches {b:?}."))?;
        let mut prev: HashMap<usize, (usize, usize)> = HashMap::new();
        let mut q = VecDeque::from([s]);
        while let Some(n) = q.pop_front() {
            if n == t {
                break;
            }
            for &e in &self.adj[n] {
                let m = self.other(e, n);
                if m != s && !prev.contains_key(&m) {
                    prev.insert(m, (n, e));
                    q.push_back(m);
                }
            }
        }
        if s != t && !prev.contains_key(&t) {
            return Err(format!(
                "No path between {} and {}.",
                self.g.nodes[s].label, self.g.nodes[t].label
            ));
        }
        let mut hops = Vec::new();
        let mut cur = t;
        while cur != s {
            let (p, e) = prev[&cur];
            hops.push((p, e, cur));
            cur = p;
        }
        hops.reverse();
        let mut out = format!("Shortest path ({} hops):\n", hops.len());
        for (p, e, c) in hops {
            let ed = &self.g.edges[e];
            let (l, r) = (&self.g.nodes[p].label, &self.g.nodes[c].label);
            if ed.s == p {
                out += &format!("  {l} --{} [{}]--> {r}\n", ed.rel, ed.conf);
            } else {
                out += &format!("  {l} <--{} [{}]-- {r}\n", ed.rel, ed.conf);
            }
        }
        Ok(out)
    }

    pub fn explain(&self, x: &str) -> Result<String, String> {
        let i = self.find(x).ok_or(format!("No node matches {x:?}."))?;
        let n = &self.g.nodes[i];
        let mut out = format!(
            "Node: {}\n  ID:        {}\n  Source:    {}{}\n  Community: {}\n  Degree:    {}\n\n",
            n.label,
            n.id,
            n.file,
            if n.line > 0 {
                format!(" L{}", n.line)
            } else {
                String::new()
            },
            n.cname,
            self.deg[i]
        );
        let mut rows: Vec<(String, String, String)> = self.adj[i]
            .iter()
            .map(|&e| {
                let ed = &self.g.edges[e];
                let (arrow, m) = if ed.s == i {
                    ("-->", ed.t)
                } else {
                    ("<--", ed.s)
                };
                let at = if ed.line > 0 {
                    format!(" {}:L{}", ed.file, ed.line)
                } else {
                    String::new()
                };
                (
                    ed.rel.clone(),
                    self.g.nodes[m].label.clone(),
                    format!(
                        "  {arrow} {} [{}] [{}]{at}",
                        self.g.nodes[m].label, ed.rel, ed.conf
                    ),
                )
            })
            .collect();
        rows.sort();
        out += &format!("Connections ({}):\n", rows.len());
        for r in rows {
            out += &r.2;
            out.push('\n');
        }
        Ok(out)
    }
}
