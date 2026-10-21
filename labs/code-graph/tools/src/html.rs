//! graph.html: a layout computed here, drawn as inline SVG, plus a small embedded viewer script.
use crate::graph::Graph;
use serde_json::json;
use std::collections::BTreeMap;
use std::fmt::Write;

const VIEWER: &str = include_str!("../../assets/viewer.js");
const ITER: usize = 120;
const RELS: &[&str] = &[
    "calls",
    "imports_from",
    "imports",
    "contains",
    "method",
    "defines",
    "inherits",
    "implements",
    "references",
];

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Fruchterman-Reingold from a fixed start (communities on a spiral, members in a phyllotaxis),
/// repulsion on a grid, a fixed number of steps: one graph, one layout.
fn layout(g: &Graph, w: f64, h: f64) -> Vec<(f64, f64)> {
    let n = g.nodes.len();
    let deg = g.degree();
    let ncomm = g.nodes.iter().map(|x| x.community + 1).max().unwrap_or(1);
    let mut members: Vec<Vec<usize>> = vec![Vec::new(); ncomm];
    for (i, x) in g.nodes.iter().enumerate() {
        members[x.community].push(i);
    }
    let k = (w * h / n.max(1) as f64).sqrt() * 0.7;
    let mut pos = vec![(0.0, 0.0); n];
    let mut centre = vec![(0.0, 0.0); ncomm];
    let mut acc = 0.0;
    for (c, m) in members.iter().enumerate() {
        acc += m.len() as f64;
        let r = (acc - m.len() as f64 / 2.0).sqrt() / (n as f64).sqrt() * 0.45;
        let a = c as f64 * 2.399963;
        centre[c] = (w / 2.0 + r * w * a.cos(), h / 2.0 + r * h * a.sin());
    }
    for (c, m) in members.iter().enumerate() {
        let mut order = m.clone();
        order.sort_by_key(|&i| (std::cmp::Reverse(deg[i]), i));
        for (j, &i) in order.iter().enumerate() {
            let (r, a) = (k * 0.8 * (j as f64 + 0.5).sqrt(), j as f64 * 2.399963);
            pos[i] = (centre[c].0 + r * a.cos(), centre[c].1 + r * a.sin());
        }
    }
    let cs = k * 3.0;
    let (gx, gy) = ((w / cs) as usize + 2, (h / cs) as usize + 2);
    for it in 0..ITER {
        let temp = k * 2.0 * (1.0 - it as f64 / ITER as f64) + 0.2;
        let mut disp = vec![(0.0f64, 0.0f64); n];
        let mut grid: Vec<Vec<usize>> = vec![Vec::new(); gx * gy];
        for (i, p) in pos.iter().enumerate() {
            grid[(p.1 / cs).clamp(0.0, gy as f64 - 1.0) as usize * gx
                + (p.0 / cs).clamp(0.0, gx as f64 - 1.0) as usize]
                .push(i);
        }
        for i in 0..n {
            let (cx, cy) = (
                (pos[i].0 / cs).clamp(0.0, gx as f64 - 1.0) as usize,
                (pos[i].1 / cs).clamp(0.0, gy as f64 - 1.0) as usize,
            );
            for yy in cy.saturating_sub(1)..(cy + 2).min(gy) {
                for xx in cx.saturating_sub(1)..(cx + 2).min(gx) {
                    for &j in &grid[yy * gx + xx] {
                        if j == i {
                            continue;
                        }
                        let (dx, dy) = (pos[i].0 - pos[j].0, pos[i].1 - pos[j].1);
                        let d2 = (dx * dx + dy * dy).max(0.01);
                        let f = k * k / d2 * (1.0 - d2.sqrt() / cs).max(0.0);
                        disp[i].0 += dx * f;
                        disp[i].1 += dy * f;
                    }
                }
            }
            let c = centre[g.nodes[i].community];
            disp[i].0 += (c.0 - pos[i].0) * 0.15 + (w / 2.0 - pos[i].0) * 0.05;
            disp[i].1 += (c.1 - pos[i].1) * 0.15 + (h / 2.0 - pos[i].1) * 0.05;
        }
        for e in &g.edges {
            if e.s == e.t {
                continue;
            }
            let (dx, dy) = (pos[e.s].0 - pos[e.t].0, pos[e.s].1 - pos[e.t].1);
            let d = (dx * dx + dy * dy).sqrt().max(0.01);
            let f = d * d / k * if e.rel == "contains" { 0.5 } else { 1.0 };
            let (fx, fy) = (dx / d * f, dy / d * f);
            disp[e.s].0 -= fx;
            disp[e.s].1 -= fy;
            disp[e.t].0 += fx;
            disp[e.t].1 += fy;
        }
        for i in 0..n {
            let d = (disp[i].0.powi(2) + disp[i].1.powi(2)).sqrt().max(1e-9);
            let m = d.min(temp);
            pos[i].0 = (pos[i].0 + disp[i].0 / d * m).clamp(0.0, w);
            pos[i].1 = (pos[i].1 + disp[i].1 / d * m).clamp(0.0, h);
        }
    }
    // fit to the page: the 1st to 99th percentile sets the frame, outliers are pinned to its edge
    let pct = |v: &mut Vec<f64>, q: f64| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[((v.len() - 1) as f64 * q) as usize]
    };
    let (mut xs, mut ys): (Vec<f64>, Vec<f64>) = pos.iter().cloned().unzip();
    let (x0, x1, y0, y1) = (
        pct(&mut xs.clone(), 0.01),
        pct(&mut xs, 0.99),
        pct(&mut ys.clone(), 0.01),
        pct(&mut ys, 0.99),
    );
    let s = ((w - 80.0) / (x1 - x0).max(1.0)).min((h - 80.0) / (y1 - y0).max(1.0));
    pos.iter()
        .map(|p| {
            (
                (((p.0.clamp(x0, x1)) - x0) * s + 40.0).round(),
                (((p.1.clamp(y0, y1)) - y0) * s + 40.0).round(),
            )
        })
        .collect()
}

pub fn render(g: &Graph, title: &str) -> String {
    let n = g.nodes.len();
    let scale = (n as f64 / 1500.0).sqrt().max(1.0);
    let (w, h) = ((1600.0 * scale).round(), (1000.0 * scale).round());
    let pos = if n == 0 { vec![] } else { layout(g, w, h) };
    let deg = g.degree();
    let mut o = String::new();
    let _ = write!(o, "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title>\n<style>{CSS}</style></head><body>\n", esc(title));
    let _ = write!(o, "<header><h1>{}</h1><input id=\"q\" type=\"search\" placeholder=\"Search a label or a file\" aria-label=\"Search\"><button id=\"fit\" type=\"button\">Fit</button><span>{} nodes, {} edges</span></header>\n<main>\n", esc(title), n, g.edges.len());
    let _ = write!(o, "<svg id=\"g\" viewBox=\"0 0 {w} {h}\" role=\"img\" aria-label=\"Code graph of {}\"><g id=\"edges\">", esc(title));
    for e in &g.edges {
        let rel = if RELS.contains(&e.rel.as_str()) {
            e.rel.as_str()
        } else {
            "other"
        };
        let _ = write!(
            o,
            "<line class=\"e {rel}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>",
            pos[e.s].0, pos[e.s].1, pos[e.t].0, pos[e.t].1
        );
    }
    o += "</g><g id=\"nodes\">";
    for (i, nd) in g.nodes.iter().enumerate() {
        let r = 3.0 + (1.0 + deg[i] as f64).ln().min(4.0) * 1.6;
        let ext = if nd.external { " x" } else { "" };
        let _ = write!(o, "<circle class=\"c{}{ext}\" cx=\"{}\" cy=\"{}\" r=\"{r:.1}\"><title>{}</title></circle>", nd.community % 12, pos[i].0, pos[i].1, esc(&nd.label));
    }
    o += "</g><g id=\"lab\">";
    // labels for the best-connected nodes, skipping any that would overlap one already placed
    let mut order: Vec<usize> = (0..n).filter(|&i| !g.nodes[i].external).collect();
    order.sort_by_key(|&i| (std::cmp::Reverse(deg[i]), i));
    let mut boxes: Vec<(f64, f64, f64, f64)> = Vec::new();
    for i in order.into_iter().take(400) {
        let (x, y) = (pos[i].0 + 8.0, pos[i].1 + 4.0);
        let b = (
            x,
            y - 13.0,
            x + 7.6 * g.nodes[i].label.chars().count() as f64,
            y + 3.0,
        );
        if boxes.len() >= 70
            || boxes
                .iter()
                .any(|q| b.0 < q.2 && q.0 < b.2 && b.1 < q.3 && q.1 < b.3)
        {
            continue;
        }
        boxes.push(b);
        let _ = write!(
            o,
            "<text x=\"{x}\" y=\"{y}\">{}</text>",
            esc(&g.nodes[i].label)
        );
    }
    o += "</g><g id=\"hl\"></g></svg>\n<aside><div id=\"info\">Click a node.</div><h2>Communities</h2><ul id=\"legend\">";
    let mut comms: BTreeMap<usize, (usize, &str)> = BTreeMap::new();
    for nd in &g.nodes {
        comms
            .entry(nd.community)
            .or_insert((0, nd.cname.as_str()))
            .0 += 1;
    }
    for (c, (cnt, name)) in comms.iter().take(60) {
        let _ = write!(
            o,
            "<li data-c=\"{c}\"><i class=\"s{}\"></i>{} <b>{cnt}</b></li>",
            c % 12,
            esc(name)
        );
    }
    if comms.len() > 60 {
        let _ = write!(
            o,
            "<li class=\"more\">{} smaller communities not listed</li>",
            comms.len() - 60
        );
    }
    o += "</ul><h2>Relations</h2><ul class=\"rels\">";
    for r in RELS {
        let _ = write!(o, "<li><i class=\"l {r}\"></i>{r}</li>");
    }
    o += "</ul></aside>\n</main>\n";
    let data = json!({
        "n": g.nodes.iter().map(|x| json!([x.label, x.file, if x.line > 0 { format!("L{}", x.line) } else { String::new() }, x.community, x.cname])).collect::<Vec<_>>(),
        "e": g.edges.iter().map(|e| json!([e.s, e.t, e.rel])).collect::<Vec<_>>(),
        "v": [0, 0, w, h],
    });
    // "</" would end the script early in an HTML parser
    let data = data.to_string().replace("</", "<\\/");
    let _ = write!(o, "<script id=\"d\" type=\"application/json\">{data}</script>\n<script>{VIEWER}</script>\n</body></html>\n");
    o
}

const CSS: &str = r#"
:root{--bg:#fafaf7;--fg:#1d1d1b;--mut:#6b6b66;--pan:#f0efe9;--line:#d8d6cc;
--c0:#2f6fb0;--c1:#c8602b;--c2:#3c8f5a;--c3:#8a4fa8;--c4:#b8962e;--c5:#2b9aa0;--c6:#b8426b;--c7:#6b7b2c;--c8:#5a5fb8;--c9:#a8603c;--c10:#4a8a8a;--c11:#8a6a9a}
@media (prefers-color-scheme:dark){:root{--bg:#16171a;--fg:#e8e6df;--mut:#a09e96;--pan:#202226;--line:#3a3c42;
--c0:#6aa6e6;--c1:#f0905a;--c2:#6fcf8f;--c3:#c08ae0;--c4:#e0c060;--c5:#5fd0d6;--c6:#ee7fa4;--c7:#a8bc58;--c8:#8c92ee;--c9:#e09a70;--c10:#78c0c0;--c11:#c0a0d0}}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--fg);font:14px/1.4 system-ui,sans-serif;display:flex;flex-direction:column;height:100vh}
header{display:flex;gap:12px;align-items:center;padding:8px 16px;border-bottom:1px solid var(--line);flex-wrap:wrap}
h1{font-size:16px;margin:0}h2{font-size:12px;text-transform:uppercase;letter-spacing:.06em;color:var(--mut);margin:16px 0 6px}
header span{color:var(--mut);margin-left:auto}
input,button{font:inherit;color:inherit;background:var(--pan);border:1px solid var(--line);border-radius:6px;padding:4px 8px}input{min-width:220px}
main{flex:1;display:flex;min-height:0}svg{flex:1;min-width:0;cursor:grab;touch-action:none}
aside{width:300px;overflow-y:auto;overflow-x:hidden;padding:12px 16px;border-left:1px solid var(--line);background:var(--pan)}
#info .t{font-weight:600;word-break:break-all}#info div{overflow-wrap:anywhere}
ul{list-style:none;margin:0;padding:0}#legend li{display:flex;gap:8px;align-items:center;padding:2px 0;cursor:pointer}#legend li:hover{text-decoration:underline}#legend span{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}#legend b{margin-left:auto;color:var(--mut);font-weight:400}
.rels li{display:flex;gap:8px;align-items:center;color:var(--mut)}
i.s0,.c0{fill:var(--c0);background:var(--c0)}i.s1,.c1{fill:var(--c1);background:var(--c1)}i.s2,.c2{fill:var(--c2);background:var(--c2)}i.s3,.c3{fill:var(--c3);background:var(--c3)}
i.s4,.c4{fill:var(--c4);background:var(--c4)}i.s5,.c5{fill:var(--c5);background:var(--c5)}i.s6,.c6{fill:var(--c6);background:var(--c6)}i.s7,.c7{fill:var(--c7);background:var(--c7)}
i.s8,.c8{fill:var(--c8);background:var(--c8)}i.s9,.c9{fill:var(--c9);background:var(--c9)}i.s10,.c10{fill:var(--c10);background:var(--c10)}i.s11,.c11{fill:var(--c11);background:var(--c11)}
i{display:inline-block;width:10px;height:10px;border-radius:50%;flex:none}svg circle{background:none;stroke:var(--bg);stroke-width:1}svg circle.x{opacity:.55}
.e{stroke-width:1;stroke:var(--mut);opacity:.35}.e.contains,.e.method,.e.defines{opacity:.18}.e.calls{stroke:var(--c1);opacity:.6}.e.imports_from,.e.imports{stroke:var(--c0);opacity:.5}.e.inherits,.e.implements{stroke:var(--c3);opacity:.7}.e.references{stroke:var(--c2);opacity:.4}
i.l{width:16px;height:3px;border-radius:0;background:var(--mut)}i.l.calls{background:var(--c1)}i.l.imports_from,i.l.imports{background:var(--c0)}i.l.inherits,i.l.implements{background:var(--c3)}i.l.references{background:var(--c2)}
text{font-size:13px;fill:var(--fg);paint-order:stroke;stroke:var(--bg);stroke-width:3px;pointer-events:none}
svg.f circle{opacity:.1}svg.f .e{opacity:.03}svg.f circle.on{opacity:1}svg.f .e.on{opacity:.9;stroke-width:1.6}svg.f #lab{display:none}svg circle.pick{stroke:var(--fg);stroke-width:2}
@media (max-width:760px){main{flex-direction:column}aside{width:auto;max-height:40vh;border-left:0;border-top:1px solid var(--line)}}
"#;
