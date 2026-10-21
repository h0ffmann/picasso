//! `render page <in.html> <out.pdf> --footer TEXT [--pad MM] [--color C] [--doi FILE]`
//! `render ste <repo-root>` prints WHITEPAPER.md (pandoc) to WHITEPAPER.pdf.
//! The browser is `$CHROMIUM` or `chromium` on PATH (Nix: `nix shell nixpkgs#chromium`).
//! Chromium talks DevTools over fds 3/4 (`--remote-debugging-pipe`); `sh` maps them to
//! our pipes, so no websocket and no unsafe code are needed.
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

type R<T> = Result<T, String>;

struct Cdp<W: Write, B: BufRead> {
    w: W,
    r: B,
    id: u64,
}

impl<W: Write, B: BufRead> Cdp<W, B> {
    /// Sends one command and waits for its reply; events are skipped, except that
    /// `wait_event` (if set) is also returned once seen.
    fn call(&mut self, method: &str, params: Value, session: Option<&str>) -> R<Value> {
        self.id += 1;
        let mut msg = json!({"id": self.id, "method": method, "params": params});
        if let Some(s) = session {
            msg["sessionId"] = json!(s);
        }
        self.w
            .write_all(format!("{msg}\0").as_bytes())
            .and_then(|_| self.w.flush())
            .map_err(|e| e.to_string())?;
        loop {
            let v = self.next()?;
            if v["id"] == self.id {
                return match v.get("error") {
                    Some(e) => Err(format!("{method}: {e}")),
                    None => Ok(v["result"].clone()),
                };
            }
        }
    }

    fn next(&mut self) -> R<Value> {
        let mut buf = Vec::new();
        self.r.read_until(0, &mut buf).map_err(|e| e.to_string())?;
        if buf.pop() != Some(0) {
            return Err("browser closed the pipe".into());
        }
        serde_json::from_slice(&buf).map_err(|e| e.to_string())
    }

    fn wait_event(&mut self, name: &str) -> R<()> {
        while self.next()?["method"] != name {}
        Ok(())
    }
}

struct Opts {
    footer: String,
    pad: f64,
    color: String,
    doi: Option<PathBuf>,
}

fn print(html: &Path, out: &Path, o: &Opts) -> R<()> {
    let html = html
        .canonicalize()
        .map_err(|e| format!("{}: {e}", html.display()))?;
    let browser = std::env::var("CHROMIUM").unwrap_or_else(|_| "chromium".into());
    let mut child = Command::new("sh")
        .args(["-c", "exec \"$0\" --headless --disable-gpu --no-sandbox --remote-debugging-pipe about:blank 3<&0 4>&1 >/dev/null", &browser])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{browser}: {e}"))?;
    let mut c = Cdp {
        w: child.stdin.take().unwrap(),
        r: BufReader::new(child.stdout.take().unwrap()),
        id: 0,
    };
    let pdf = (|| -> R<Vec<u8>> {
        let t = c.call("Target.createTarget", json!({"url": "about:blank"}), None)?;
        let a = c.call(
            "Target.attachToTarget",
            json!({"targetId": t["targetId"], "flatten": true}),
            None,
        )?;
        let s = a["sessionId"].as_str().ok_or("no session")?.to_string();
        let s = Some(s.as_str());
        c.call("Page.enable", json!({}), s)?;
        c.call(
            "Page.navigate",
            json!({"url": format!("file://{}", html.display())}),
            s,
        )?;
        c.wait_event("Page.loadEventFired")?;
        let eval = |c: &mut Cdp<_, _>, e: &str| {
            c.call(
                "Runtime.evaluate",
                json!({"expression": e, "awaitPromise": true}),
                s,
            )
        };
        eval(&mut c, "document.fonts.ready.then(() => 0)")?;
        if let Some(f) = &o.doi {
            // doi.txt is written by `zenodo.py draft` once Zenodo has reserved the DOI.
            if let Ok(d) = std::fs::read_to_string(f) {
                let e = format!(
                    "document.querySelector('[data-doi]').textContent = {}",
                    json!(format!("https://doi.org/{}", d.trim()))
                );
                eval(&mut c, &e)?;
            }
        }
        let style = format!(
            "font:7.5pt Inter,sans-serif;color:{};width:100%;padding:0 {}mm;display:flex;justify-content:space-between",
            o.color, o.pad
        );
        let footer = format!(
            "<div style=\"{style}\"><span>{}</span><span><span class=\"pageNumber\"></span> / <span class=\"totalPages\"></span></span></div>",
            o.footer
        );
        let r = c.call(
            "Page.printToPDF",
            json!({"paperWidth": 8.27, "paperHeight": 11.69, "printBackground": true, "preferCSSPageSize": true,
                   "displayHeaderFooter": true, "headerTemplate": "<span></span>", "footerTemplate": footer}),
            s,
        )?;
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(r["data"].as_str().ok_or("no pdf data")?)
            .map_err(|e| e.to_string())
    })();
    let _ = c.call("Browser.close", json!({}), None);
    let _ = child.wait();
    std::fs::write(out, pdf?).map_err(|e| format!("{}: {e}", out.display()))
}

/// Same look as the former Node script: pandoc body in a plain manual style.
fn ste(root: &Path) -> R<()> {
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let md = Command::new("pandoc")
        .args([
            root.join("WHITEPAPER.md").as_os_str(),
            "-f".as_ref(),
            "gfm".as_ref(),
            "-t".as_ref(),
            "html5".as_ref(),
        ])
        .output()
        .map_err(|e| format!("pandoc: {e}"))?;
    if !md.status.success() {
        return Err("pandoc failed".into());
    }
    let mut body = String::new();
    md.stdout
        .as_slice()
        .read_to_string(&mut body)
        .map_err(|e| e.to_string())?;
    let html = include_str!("ste.html")
        .replace("{ROOT}", &format!("file://{}/", root.display()))
        .replace("{BODY}", &body);
    let dir = std::env::temp_dir().join(format!("ste-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tmp = dir.join("whitepaper.html");
    std::fs::write(&tmp, html).map_err(|e| e.to_string())?;
    let o = Opts {
        footer: "h0ffmann/picasso · same inputs, same factory · ASD-STE100 text".into(),
        pad: 20.0,
        color: "#555".into(),
        doi: None,
    };
    let r = print(&tmp, &root.join("WHITEPAPER.pdf"), &o);
    let _ = std::fs::remove_dir_all(dir);
    r
}

fn run(args: &[String]) -> R<()> {
    match args.first().map(String::as_str) {
        Some("ste") => ste(Path::new(args.get(1).ok_or("usage: render ste <repo-root>")?)),
        Some("page") if args.len() >= 3 => {
            let mut o = Opts { footer: String::new(), pad: 17.0, color: "#4F5868".into(), doi: None };
            let mut it = args[3..].iter();
            while let Some(k) = it.next() {
                let v = it.next().ok_or(format!("{k} needs a value"))?;
                match k.as_str() {
                    "--footer" => o.footer = v.clone(),
                    "--pad" => o.pad = v.parse().map_err(|_| "bad --pad")?,
                    "--color" => o.color = v.clone(),
                    "--doi" => o.doi = Some(v.into()),
                    _ => return Err(format!("unknown flag {k}")),
                }
            }
            print(Path::new(&args[1]), Path::new(&args[2]), &o)
        }
        _ => Err("usage: render page <in.html> <out.pdf> --footer TEXT [--pad MM] [--color C] [--doi FILE] | render ste <repo-root>".into()),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = run(&args) {
        eprintln!("render: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_usage() {
        assert!(run(&[]).is_err());
        assert!(
            run(&[
                "page".into(),
                "a".into(),
                "b".into(),
                "--x".into(),
                "1".into()
            ])
            .is_err()
        );
    }

    #[test]
    fn frames_are_nul_terminated_and_replies_matched() {
        let replies = b"{\"method\":\"E\"}\0{\"id\":1,\"result\":{\"ok\":1}}\0".to_vec();
        let mut c = Cdp {
            w: Vec::new(),
            r: &replies[..],
            id: 0,
        };
        assert_eq!(c.call("M", json!({}), None).unwrap()["ok"], 1);
        assert!(c.w.ends_with(b"}\0"));
    }
}
