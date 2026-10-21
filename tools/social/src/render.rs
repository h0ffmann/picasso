//! Renders the five 1080x1350 slides of the first Instagram post (docs/social/instagram_001).
//! The logo and the drawings are read from the repo, so a change to them shows on the next render;
//! the page is drawn by headless Chromium (CHROMIUM=<path> picks the binary).

use crate::post::{PostError, Result};
use regex::Regex;
use serde_json::Value;
use std::{path::Path, process::Command};

const CREAM: &str = "#F3EDE2";
const INK: &str = "#1d1b19";
const BLUE: &str = "#5277C3";
const ROSE: &str = "#E8A87C";
const ACCENT: &str = "#d9480f";
const MUTED: &str = "#6b645b";
const MONO: &str = "'DejaVu Sans Mono',monospace";

fn fail<E: std::fmt::Display>(what: &str) -> impl FnOnce(E) -> PostError + '_ {
    move |e| PostError(format!("{what}: {e}"))
}

/// The files carry their own light/dark <style>; the slides set one palette for all of them.
fn svg(root: &Path, p: &str) -> Result<String> {
    let s = std::fs::read_to_string(root.join(p)).map_err(fail(p))?;
    Ok(Regex::new(r"(?s)<style>.*?</style>")
        .unwrap()
        .replace(&s, "")
        .into_owned())
}

fn fmt(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

type Monthly = Vec<(String, Vec<(String, u64)>)>;

fn monthly(cves: &Value) -> Result<Monthly> {
    let obj = cves["monthly"]
        .as_object()
        .ok_or_else(|| PostError("cves.json has no monthly".into()))?;
    Ok(obj
        .iter()
        .map(|(eco, by)| {
            (
                eco.clone(),
                by.as_object()
                    .into_iter()
                    .flatten()
                    .map(|(k, v)| (k.clone(), v.as_u64().unwrap_or(0)))
                    .collect(),
            )
        })
        .collect())
}

fn total(m: &Monthly, eco: &str, from: &str) -> u64 {
    m.iter()
        .filter(|(e, _)| e == eco)
        .flat_map(|(_, by)| by)
        .filter(|(k, _)| k.as_str() >= from)
        .map(|(_, v)| v)
        .sum()
}

/// Cumulative CVE advisories per ecosystem, drawn like a GitHub star-history chart.
fn cve_chart(m: &Monthly, w: f64, h: f64) -> String {
    let months: Vec<String> = (2018..=2026)
        .flat_map(|y| (1..=12).map(move |mo| format!("{y}-{mo:02}")))
        .collect();
    let last = m
        .iter()
        .flat_map(|(_, by)| by.iter().map(|(k, _)| k.clone()))
        .max()
        .unwrap_or_default();
    let xs: Vec<&String> = months.iter().filter(|x| **x <= last).collect();
    let cum: Vec<(&str, Vec<u64>)> = m
        .iter()
        .map(|(eco, by)| {
            let mut t: u64 = by.iter().filter(|(k, _)| k < xs[0]).map(|(_, v)| v).sum();
            let c = xs.iter().map(|x| {
                t += by.iter().find(|(k, _)| k == *x).map_or(0, |(_, v)| *v);
                t
            });
            (eco.as_str(), c.collect())
        })
        .collect();
    let top = cum.iter().flat_map(|(_, c)| c).copied().max().unwrap_or(0) as f64;
    let max = (top / 1000.0).ceil() * 1000.0;
    let (l, b) = (70.0, 40.0);
    let x = |i: usize| l + (i as f64 / (xs.len() - 1) as f64) * (w - l - 10.0);
    let y = |v: f64| h - b - (v / max) * (h - b - 10.0);
    let mut g = String::new();
    let mut v = 0.0;
    while v <= max {
        let label = if v > 0.0 {
            format!("{}k", v / 1000.0)
        } else {
            "0".into()
        };
        g += &format!(
            r##"<line x1="{l}" x2="{}" y1="{y0}" y2="{y0}" stroke="#d9d1c3" stroke-width="1.5"/><text x="{}" y="{}" text-anchor="end">{label}</text>"##,
            w - 10.0,
            l - 12.0,
            y(v) + 7.0,
            y0 = y(v)
        );
        v += 2000.0;
    }
    for yr in (2018..=2026).step_by(2) {
        if let Some(i) = xs.iter().position(|m| **m == format!("{yr}-01")) {
            g += &format!(
                r#"<text x="{}" y="{}" text-anchor="middle">{yr}</text>"#,
                x(i),
                h - 6.0
            );
        }
    }
    for (eco, col) in [("npm", ACCENT), ("PyPI", BLUE)] {
        let Some((_, c)) = cum.iter().find(|(e, _)| *e == eco) else {
            continue;
        };
        let pts: Vec<String> = c
            .iter()
            .enumerate()
            .map(|(i, v)| format!("{:.1},{:.1}", x(i), y(*v as f64)))
            .collect();
        g += &format!(
            r#"<polyline points="{}" fill="none" stroke="{col}" stroke-width="6" stroke-linejoin="round" stroke-linecap="round"/>"#,
            pts.join(" ")
        );
        g += &format!(
            r#"<circle cx="{}" cy="{}" r="9" fill="{col}"/>"#,
            x(xs.len() - 1),
            y(*c.last().unwrap() as f64)
        );
    }
    format!(
        r#"<svg width="{w}" height="{h}" viewBox="0 0 {w} {h}" style="font-family:{MONO};font-size:22px;fill:{MUTED};overflow:visible">{g}</svg>"#
    )
}

fn css() -> String {
    format!(
        "
*{{margin:0;padding:0;box-sizing:border-box}}
body{{width:1080px;height:1350px;font-family:Inter,sans-serif;color:{INK};background:{CREAM};position:relative;overflow:hidden}}
.ink,.l{{fill:none;stroke:{INK};stroke-linecap:round;stroke-linejoin:round}}
.ink{{stroke-width:6.5}}.l{{stroke-width:3.2}}.pupil,.d{{fill:{INK}}}
.blue{{fill:{BLUE}}}.rose{{fill:{ROSE}}}
.a{{fill:none;stroke:{ACCENT};stroke-width:3.2;stroke-linecap:round}}
.pad{{position:absolute;inset:96px 96px 160px}}
.mono{{font-family:{MONO}}}
.kicker{{font-family:{MONO};font-size:28px;letter-spacing:2px;color:{BLUE}}}
h1{{font-family:'Inter Display',Inter;font-weight:700;font-size:112px;line-height:1;letter-spacing:-3px}}
h2{{font-family:'Inter Display',Inter;font-weight:700;font-size:72px;line-height:1.05;letter-spacing:-2px}}
p{{font-size:36px;line-height:1.35}}
.num{{font-family:'Inter Display',Inter;font-weight:700;font-size:120px;line-height:1;letter-spacing:-4px;color:{BLUE}}}
.src{{font-family:{MONO};font-size:20px;line-height:1.4;color:{MUTED};margin-top:10px}}
.mark{{background:linear-gradient(transparent 62%, {ROSE} 62%);padding:0 4px}}
blockquote{{border-left:8px solid {ROSE};padding:4px 0 4px 28px;font-size:32px;line-height:1.35;font-style:italic}}
.foot{{position:absolute;left:96px;right:96px;bottom:64px;display:flex;justify-content:space-between;align-items:center;font-family:{MONO};font-size:24px;color:{MUTED}}}
.page{{color:{ACCENT}}}
"
    )
}

fn slides(root: &Path, dir: &Path) -> Result<Vec<String>> {
    let logo = svg(root, "docs/media/logo.svg")?;
    let sized = |s: &str, w: u32| {
        s.replace(
            r#"width="240" height="240""#,
            &format!(r#"width="{w}" height="{w}""#),
        )
    };
    let draw = |n: &str, w: u32| -> Result<String> {
        Ok(
            svg(root, &format!("site/static/drawings/{n}.svg"))?.replace(
                r#"width="360" height="240""#,
                &format!(
                    r#"width="{w}" height="{}""#,
                    (f64::from(w) * 2.0 / 3.0).round()
                ),
            ),
        )
    };
    let cves_path = dir.join("cves.json");
    let cves: Value =
        serde_json::from_str(&std::fs::read_to_string(&cves_path).map_err(fail("cves.json"))?)
            .map_err(fail("cves.json"))?;
    let m = monthly(&cves)?;
    let fetched = cves["fetched"].as_str().unwrap_or("");
    let foot = |n: u32| {
        format!(
            r#"<div class="foot"><span>{}</span><span>h0ffmann.github.io/picasso</span><span class="page">{n}/5</span></div>"#,
            sized(&logo, 64)
        )
    };
    let (npm, pypi) = (total(&m, "npm", "0"), total(&m, "PyPI", "0"));
    let y2026 = total(&m, "npm", "2026") + total(&m, "PyPI", "2026");
    let chart = cve_chart(&m, 888.0, 470.0);
    let (owl, guitar, dove) = (draw("owl", 360)?, draw("guitar", 440)?, draw("dove", 520)?);
    let big_logo = sized(&logo, 500);
    Ok(vec![
        format!(
            r#"<div class="pad">
    <div class="kicker">v0.1.1 · first release on zenodo</div>
    <div style="margin:56px 0 40px;display:flex;justify-content:center">{big_logo}</div>
    <h1>picasso</h1>
    <p style="margin-top:28px;font-size:44px">A toolchain is a dependency,<br><span class="mark">so pin it like one.</span></p>
    <div class="src" style="margin-top:28px">doi:10.5281/zenodo.23311208</div>
  </div>{}"#,
            foot(1)
        ),
        format!(
            r#"<div class="pad">
    <div class="kicker">the problem</div>
    <h2 style="margin-top:36px">Python and JavaScript:<br><span class="mark">two cans of worms.</span></h2>
    <div style="display:flex;gap:56px;margin-top:56px">
      <div style="flex:1"><div class="num">4.03%</div>
        <p style="margin-top:16px;font-size:30px">of 1.4M Jupyter notebooks gave their own results again.</p>
        <div class="src">Pimentel et al., MSR 2019</div></div>
      <div style="flex:1"><div class="num">174</div>
        <p style="margin-top:16px;font-size:30px">malicious packages from real attacks, shipped via registries like npm and PyPI.</p>
        <div class="src">Ohm et al., DIMVA 2020</div></div>
    </div>
    <blockquote style="margin-top:64px">“The only historical exception to that rule was JavaScript, and that’s a can of worms that doesn’t fit in this article.”</blockquote>
    <div class="src" style="padding-left:36px">Fabio Akita, akitaonrails.com, 19 Sep 2026</div>
    <div style="margin-top:8px;margin-left:auto;width:360px">{owl}</div>
  </div>{}"#,
            foot(2)
        ),
        format!(
            r#"<div class="pad">
    <div class="kicker">the evidence</div>
    <h2 style="margin-top:36px">CVEs in npm and PyPI, <span class="mark">every month, piling up.</span></h2>
    <div style="margin-top:40px;display:flex;gap:40px;font-size:30px">
      <div><span style="color:{ACCENT};font-weight:700">● npm</span> {}</div>
      <div><span style="color:{BLUE};font-weight:700">● PyPI</span> {}</div>
    </div>
    <div style="margin-top:28px">{chart}</div>
    <p style="margin-top:36px;font-size:30px">{} of them published in 2026 alone, more than in 2023, 2024 and 2025 together.</p>
    <div class="src">GitHub Advisory Database (reviewed, with a CVE id), via OSV, {fetched}</div>
  </div>{}"#,
            fmt(npm),
            fmt(pypi),
            fmt(y2026),
            foot(3)
        ),
        format!(
            r#"<div class="pad">
    <div class="kicker">the idea</div>
    <h2 style="margin-top:36px">If the generator is stochastic, <span class="mark">the rest must be deterministic.</span></h2>
    <div style="margin-top:48px"><span class="num" style="font-size:96px">80</span>
      <p style="display:inline;font-size:30px;margin-left:12px">different answers from 1,000 identical LLM requests at temperature 0.</p>
      <div class="src">He / Thinking Machines Lab, 2025</div></div>
    <div style="margin-top:36px"><span class="num" style="font-size:96px">&gt;99%</span>
      <p style="display:inline;font-size:30px;margin-left:12px">of 709,816 nixpkgs packages rebuilt from their recorded inputs.</p>
      <div class="src">Malka, Zacchiroli, Zimmermann, arXiv:2501.15919</div></div>
    <div style="margin-top:24px;margin-left:auto;width:440px">{guitar}</div>
  </div>{}"#,
            foot(4)
        ),
        format!(
            r#"<div class="pad">
    <div class="kicker">read · cite · borrow</div>
    <h2 style="margin-top:36px">Same inputs,<br>same factory.</h2>
    <p style="margin-top:28px">The whitepaper on why determinism matters in 2026, and six Nix labs you can pin from your own repo.</p>
    <div class="src">Hoffmann, M. (2026). Same inputs, same factory: why determinism matters in 2026. v1.0.</div>
    <div class="mono" style="margin-top:44px;font-size:32px;line-height:1.7">
      <div><span style="color:{BLUE}">doi</span> 10.5281/zenodo.23311208</div>
      <div><span style="color:{BLUE}">web</span> h0ffmann.github.io/picasso</div>
      <div><span style="color:{BLUE}">src</span> github.com/h0ffmann/picasso</div>
    </div>
    <div style="margin-top:24px;margin-left:auto;width:520px">{dove}</div>
  </div>{}"#,
            foot(5)
        ),
    ])
}

fn browser() -> Result<String> {
    if let Ok(p) = std::env::var("CHROMIUM") {
        return Ok(p);
    }
    [
        "chromium",
        "chromium-browser",
        "google-chrome",
        "google-chrome-stable",
    ]
    .into_iter()
    .find(|b| {
        Command::new(b)
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success())
    })
    .map(String::from)
    .ok_or_else(|| {
        PostError("no Chromium found: put chromium on PATH or set CHROMIUM=<path>".into())
    })
}

pub fn run(root: &Path) -> Result<i32> {
    let dir = root.join("docs/social/instagram_001");
    let bin = browser()?;
    let html = std::env::temp_dir().join(format!("social-render-{}.html", std::process::id()));
    let css = css();
    for (i, s) in slides(root, &dir)?.iter().enumerate() {
        std::fs::write(&html, format!(r#"<!doctype html><html><head><meta charset="utf-8"><style>{css}</style></head><body>{s}</body></html>"#))
            .map_err(fail("write html"))?;
        let out = dir.join(format!("slide-{}.png", i + 1));
        let st = Command::new(&bin)
            .args([
                "--headless",
                "--no-sandbox",
                "--disable-gpu",
                "--hide-scrollbars",
                "--force-device-scale-factor=1",
            ])
            .args(["--window-size=1080,1350", "--virtual-time-budget=5000"])
            .arg(format!("--screenshot={}", out.display()))
            .arg(format!("file://{}", html.display()))
            .output()
            .map_err(fail(&bin))?;
        if !st.status.success() || !out.exists() {
            std::fs::remove_file(&html).ok();
            return Err(PostError(format!(
                "{bin} failed on slide {}: {}",
                i + 1,
                String::from_utf8_lossy(&st.stderr)
            )));
        }
        println!("social: {}", out.display());
    }
    std::fs::remove_file(&html).ok();
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_separator() {
        assert_eq!(
            [fmt(0), fmt(999), fmt(1000), fmt(12345), fmt(1234567)],
            ["0", "999", "1,000", "12,345", "1,234,567"]
        );
    }

    #[test]
    fn totals_and_chart_of_the_real_data() {
        let root = crate::post::root();
        let v: Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("docs/social/instagram_001/cves.json")).unwrap(),
        )
        .unwrap();
        let m = monthly(&v).unwrap();
        // the figures written in the post README (alt text of slide 3)
        assert_eq!(
            (total(&m, "npm", "0"), total(&m, "PyPI", "0")),
            (6390, 6118)
        );
        assert_eq!(total(&m, "npm", "2026") + total(&m, "PyPI", "2026"), 5078);
        let svg = cve_chart(&m, 888.0, 470.0);
        assert!(
            svg.starts_with("<svg")
                && svg.matches("<polyline").count() == 2
                && svg.contains("2026</text>")
        );
    }
}
