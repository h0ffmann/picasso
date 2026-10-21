//! The graph in memory, its graph.json form (graphify's schema) and the clustering.
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Debug, Default)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub file: String,
    pub line: u32,
    pub external: bool,
    pub community: usize,
    pub cname: String,
    /// the short name of the linked repository this node stands for (`--link`), else empty
    pub repo: String,
    /// a linked node's own label, its `label` being the PATH it was found by (`--link`)
    pub orig: String,
    /// where that repository's graph.html is, focused on the node (`--link`), else empty
    pub href: String,
}

#[derive(Clone, Debug, Default)]
pub struct Edge {
    pub s: usize,
    pub t: usize,
    pub rel: String,
    pub conf: String,
    pub score: f64,
    pub ctx: String,
    pub file: String,
    pub line: u32,
}

#[derive(Default)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

fn loc(line: u32) -> Value {
    if line == 0 {
        Value::Null
    } else {
        json!(format!("L{line}"))
    }
}

impl Graph {
    pub fn to_json(&self) -> Value {
        let nodes: Vec<Value> = self
            .nodes
            .iter()
            .map(|n| {
                let mut m = Map::new();
                m.insert("id".into(), json!(n.id));
                m.insert("label".into(), json!(n.label));
                m.insert("norm_label".into(), json!(n.label.to_lowercase()));
                m.insert("_origin".into(), json!("ast"));
                m.insert("file_type".into(), json!("code"));
                m.insert("source_file".into(), json!(n.file));
                m.insert("source_location".into(), loc(n.line));
                m.insert("community".into(), json!(n.community));
                m.insert("community_name".into(), json!(n.cname));
                if n.external {
                    // a node of a linked repository carries its name, any other external `true`
                    let v = if n.repo.is_empty() {
                        json!(true)
                    } else {
                        json!(n.repo)
                    };
                    m.insert("external".into(), v);
                    m.insert("type".into(), json!("external"));
                }
                if !n.orig.is_empty() {
                    m.insert("linked_label".into(), json!(n.orig));
                }
                if !n.href.is_empty() {
                    m.insert("href".into(), json!(n.href));
                }
                Value::Object(m)
            })
            .collect();
        let links: Vec<Value> = self
            .edges
            .iter()
            .map(|e| {
                let mut m = Map::new();
                m.insert("source".into(), json!(self.nodes[e.s].id));
                m.insert("target".into(), json!(self.nodes[e.t].id));
                m.insert("relation".into(), json!(e.rel));
                m.insert("_origin".into(), json!("ast"));
                m.insert("confidence".into(), json!(e.conf));
                m.insert("confidence_score".into(), json!(e.score));
                if !e.ctx.is_empty() {
                    m.insert("context".into(), json!(e.ctx));
                }
                m.insert("source_file".into(), json!(e.file));
                m.insert("source_location".into(), loc(e.line));
                m.insert("weight".into(), json!(1.0));
                Value::Object(m)
            })
            .collect();
        json!({"directed": false, "multigraph": false, "graph": {}, "nodes": nodes, "links": links, "hyperedges": []})
    }

    /// Reads graphify's own graph.json too: `links` or `edges`, any extra field ignored.
    pub fn from_json(v: &Value) -> Result<Graph, String> {
        let s = |x: &Value, k: &str| x.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        let ln = |x: &Value| {
            x.get("source_location")
                .and_then(Value::as_str)
                .and_then(|l| l.trim_start_matches('L').split('-').next()?.parse().ok())
                .unwrap_or(0)
        };
        let mut g = Graph::default();
        let mut ix = HashMap::new();
        for n in v
            .get("nodes")
            .and_then(Value::as_array)
            .ok_or("no nodes array")?
        {
            let id = s(n, "id");
            ix.insert(id.clone(), g.nodes.len());
            let cname = match n.get("community_name").and_then(Value::as_str) {
                Some(c) => c.to_string(),
                None => n
                    .get("community")
                    .map(|c| c.to_string())
                    .unwrap_or_default(),
            };
            g.nodes.push(Node {
                label: if s(n, "label").is_empty() {
                    id.clone()
                } else {
                    s(n, "label")
                },
                id,
                file: s(n, "source_file"),
                line: ln(n),
                external: match n.get("external") {
                    Some(Value::String(s)) => !s.is_empty(),
                    Some(x) => x.as_bool().unwrap_or(false),
                    None => false,
                },
                repo: n
                    .get("external")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                href: s(n, "href"),
                orig: s(n, "linked_label"),
                community: n.get("community").and_then(Value::as_u64).unwrap_or(0) as usize,
                cname,
            });
        }
        let links = v
            .get("links")
            .or_else(|| v.get("edges"))
            .and_then(Value::as_array);
        for e in links.into_iter().flatten() {
            let (Some(&a), Some(&b)) = (ix.get(&s(e, "source")), ix.get(&s(e, "target"))) else {
                continue;
            };
            g.edges.push(Edge {
                s: a,
                t: b,
                rel: s(e, "relation"),
                conf: s(e, "confidence"),
                score: e
                    .get("confidence_score")
                    .and_then(Value::as_f64)
                    .unwrap_or(1.0),
                ctx: s(e, "context"),
                file: s(e, "source_file"),
                line: ln(e),
            });
        }
        Ok(g)
    }

    pub fn degree(&self) -> Vec<usize> {
        let mut d = vec![0; self.nodes.len()];
        for e in &self.edges {
            d[e.s] += 1;
            d[e.t] += 1;
        }
        d
    }

    /// Louvain communities, then each node's community and name (its best-connected member).
    pub fn cluster(&mut self) {
        let n = self.nodes.len();
        let mut pair: BTreeMap<(usize, usize), f64> = BTreeMap::new();
        for e in &self.edges {
            if e.s != e.t {
                *pair.entry((e.s.min(e.t), e.s.max(e.t))).or_default() += 1.0;
            }
        }
        let mut adj = vec![Vec::new(); n];
        for (&(a, b), &w) in &pair {
            adj[a].push((b, w));
            adj[b].push((a, w));
        }
        let comm = louvain(adj);
        // community 0 is the largest; a tie goes to the community with the smallest node index
        let mut size: BTreeMap<usize, (usize, usize)> = BTreeMap::new();
        for (i, &c) in comm.iter().enumerate() {
            let e = size.entry(c).or_insert((0, i));
            e.0 += 1;
        }
        let mut order: Vec<_> = size.into_iter().collect();
        order.sort_by_key(|&(_, (sz, first))| (std::cmp::Reverse(sz), first));
        let rank: HashMap<usize, usize> = order
            .iter()
            .enumerate()
            .map(|(r, &(c, _))| (c, r))
            .collect();
        let deg = self.degree();
        let mut hub: HashMap<usize, usize> = HashMap::new();
        for (i, &c) in comm.iter().enumerate() {
            let r = rank[&c];
            self.nodes[i].community = r;
            let best = hub.entry(r).or_insert(i);
            if deg[i] > deg[*best] {
                *best = i;
            }
        }
        // two communities with one hub label get the hub's file too
        let mut count: HashMap<String, usize> = HashMap::new();
        for h in hub.values() {
            *count.entry(self.nodes[*h].label.clone()).or_default() += 1;
        }
        for i in 0..n {
            let h = &self.nodes[hub[&self.nodes[i].community]];
            self.nodes[i].cname = if count[&h.label] > 1 && !h.file.is_empty() {
                format!("{} ({})", h.label, h.file)
            } else {
                h.label.clone()
            };
        }
    }
}

/// Louvain modularity optimisation. Nodes are visited in index order and ties go to the lowest
/// community id, so one input always gives one partition.
fn louvain(mut adj: Vec<Vec<(usize, f64)>>) -> Vec<usize> {
    let n0 = adj.len();
    let mut map: Vec<usize> = (0..n0).collect(); // original node -> current super-node
    loop {
        let n = adj.len();
        let k: Vec<f64> = adj.iter().map(|r| r.iter().map(|x| x.1).sum()).collect();
        let m2: f64 = k.iter().sum();
        if m2 == 0.0 {
            return map;
        }
        let mut comm: Vec<usize> = (0..n).collect();
        let mut tot = k.clone();
        let mut moved_any = false;
        for _ in 0..100 {
            let mut moved = false;
            for i in 0..n {
                let mut w: BTreeMap<usize, f64> = BTreeMap::new();
                for &(j, x) in &adj[i] {
                    if j != i {
                        *w.entry(comm[j]).or_default() += x;
                    }
                }
                let old = comm[i];
                tot[old] -= k[i];
                let gain = |c: usize, wc: f64| wc - tot[c] * k[i] / m2;
                let mut best = old;
                let mut best_gain = gain(old, w.get(&old).copied().unwrap_or(0.0));
                for (&c, &wc) in &w {
                    let g = gain(c, wc);
                    if g > best_gain + 1e-12 {
                        best = c;
                        best_gain = g;
                    }
                }
                tot[best] += k[i];
                if best != old {
                    comm[i] = best;
                    moved = true;
                }
            }
            moved_any |= moved;
            if !moved {
                break;
            }
        }
        if !moved_any {
            return map;
        }
        // renumber communities by their first member, then aggregate
        let mut ren: BTreeMap<usize, usize> = BTreeMap::new();
        let mut next = 0;
        for &c in &comm {
            ren.entry(c).or_insert_with(|| {
                next += 1;
                next - 1
            });
        }
        let mut agg: Vec<BTreeMap<usize, f64>> = vec![BTreeMap::new(); next];
        for (i, row) in adj.iter().enumerate() {
            for &(j, x) in row {
                *agg[ren[&comm[i]]].entry(ren[&comm[j]]).or_default() += x;
            }
        }
        for m in map.iter_mut() {
            *m = ren[&comm[*m]];
        }
        adj = agg.into_iter().map(|r| r.into_iter().collect()).collect();
        if adj.len() == n {
            return map;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clique(g: &mut Graph, base: usize, n: usize) {
        for i in 0..n {
            for j in i + 1..n {
                g.edges.push(Edge {
                    s: base + i,
                    t: base + j,
                    rel: "calls".into(),
                    ..Default::default()
                });
            }
        }
    }

    #[test]
    fn two_cliques_and_a_bridge_make_two_communities() {
        let mut g = Graph::default();
        for i in 0..10 {
            g.nodes.push(Node {
                id: format!("n{i}"),
                label: format!("n{i}()"),
                ..Default::default()
            });
        }
        clique(&mut g, 0, 5);
        clique(&mut g, 5, 5);
        g.edges.push(Edge {
            s: 0,
            t: 5,
            rel: "calls".into(),
            ..Default::default()
        });
        g.cluster();
        let c: Vec<usize> = g.nodes.iter().map(|n| n.community).collect();
        assert!(
            c[..5].iter().all(|&x| x == c[0]) && c[5..].iter().all(|&x| x == c[5]) && c[0] != c[5]
        );
        let mut again = Graph {
            nodes: g.nodes.clone(),
            edges: g.edges.clone(),
        };
        again.cluster();
        assert_eq!(
            c,
            again.nodes.iter().map(|n| n.community).collect::<Vec<_>>()
        );
    }

    #[test]
    fn ids_follow_graphifys_recipe() {
        assert_eq!(
            crate::extract::make_id(&["src/lib", "Greeter", "new"]),
            "src_lib_greeter_new"
        );
        assert_eq!(crate::extract::make_id(&["a-b.c"]), "a_b_c");
    }
}
