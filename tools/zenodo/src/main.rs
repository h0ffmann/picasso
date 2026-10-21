//! Deposit the whitepaper on Zenodo, as a record of its own (not the picasso repository).
//!
//! Run from docs/whitepapers/determinism-2026 (paths are relative to the current directory):
//!
//!   ZENODO_TOKEN=... zenodo draft     # create the draft from zenodo.json, reserve a DOI into doi.txt
//!   just render paper                   # re-render: the cover's "Cite as" now carries the DOI
//!   ZENODO_TOKEN=... zenodo upload    # attach ../../../PICASSOPAPER.pdf to the draft
//!   zenodo --self-test                # checks zenodo.json and the files, no network
//!
//! It never publishes: a published DOI cannot be withdrawn, so the author reviews the draft on
//! zenodo.org and presses Publish. --sandbox targets sandbox.zenodo.org for a rehearsal (its own
//! token). The token needs the deposit:write scope; keep it in the environment, never in a file.

use serde_json::{Value, json};
use std::path::Path;
use std::process::ExitCode;

const META: &str = "zenodo.json";
const PDF: &str = "../../../PICASSOPAPER.pdf";
const STATE: &str = ".zenodo-draft";
const DOI: &str = "doi.txt";

fn api(
    base: &str,
    method: &str,
    path: &str,
    token: &str,
    body: Option<Value>,
    raw: Option<Vec<u8>>,
) -> Result<Value, String> {
    let req = ureq::request(method, &format!("{base}{path}"))
        .set("Authorization", &format!("Bearer {token}"));
    let res = match (raw, body) {
        (Some(bytes), _) => req
            .set("Content-Type", "application/octet-stream")
            .send_bytes(&bytes),
        (None, Some(b)) => req
            .set("Content-Type", "application/json")
            .send_string(&b.to_string()),
        (None, None) => req.set("Content-Type", "application/json").call(),
    };
    // the error text never includes the request headers, so the token stays out of the logs
    let text = res
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;
    if text.trim().is_empty() {
        Ok(json!({}))
    } else {
        serde_json::from_str(&text).map_err(|e| e.to_string())
    }
}

fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
        Some(Value::Number(n)) => n.as_f64() != Some(0.0),
    }
}

fn check() -> Result<Value, String> {
    let text = std::fs::read_to_string(META).map_err(|e| format!("{META}: {e}"))?;
    let m: Value = serde_json::from_str(&text).map_err(|e| format!("{META}: {e}"))?;
    for k in [
        "title",
        "upload_type",
        "publication_type",
        "description",
        "creators",
        "license",
        "access_right",
    ] {
        if !truthy(m.get(k)) {
            return Err(format!("zenodo.json: missing {k}"));
        }
    }
    if m.get("doi").is_some() || m.get("publication_date").is_some() {
        return Err("zenodo.json: Zenodo sets doi and publication_date".into());
    }
    let creators = m["creators"]
        .as_array()
        .ok_or("zenodo.json: creators is not a list")?;
    if !creators.iter().all(|c| truthy(c.get("orcid"))) {
        return Err("zenodo.json: every creator needs an ORCID".into());
    }
    if m.get("references").and_then(Value::as_array).map(Vec::len) != Some(30) {
        return Err("zenodo.json: references out of step with the paper".into());
    }
    if !Path::new(PDF).exists() {
        return Err("PICASSOPAPER.pdf missing: run just render paper".into());
    }
    Ok(m)
}

fn run(args: &[String]) -> Result<(), String> {
    if args.iter().any(|a| a == "--self-test") {
        check()?;
        println!("zenodo.json ok");
        return Ok(());
    }
    let base = if args.iter().any(|a| a == "--sandbox") {
        "https://sandbox.zenodo.org/api"
    } else {
        "https://zenodo.org/api"
    };
    let token = std::env::var("ZENODO_TOKEN")
        .ok()
        .filter(|t| !t.is_empty())
        .ok_or("ZENODO_TOKEN is not set")?;
    let m = check()?;
    if args.iter().any(|a| a == "draft") {
        if let Ok(st) = std::fs::read_to_string(STATE) {
            return Err(format!(
                "a draft already exists ({}); delete {STATE} to start over",
                st.trim()
            ));
        }
        let mut meta = m;
        meta["prereserve_doi"] = json!(true);
        let d = api(
            base,
            "POST",
            "/deposit/depositions",
            &token,
            Some(json!({ "metadata": meta })),
            None,
        )?;
        let id = d["id"].to_string();
        std::fs::write(STATE, format!("{base} {id}\n")).map_err(|e| e.to_string())?;
        let doi = d["metadata"]["prereserve_doi"]["doi"]
            .as_str()
            .ok_or("no reserved DOI in the response")?;
        std::fs::write(DOI, format!("{doi}\n")).map_err(|e| e.to_string())?;
        println!(
            "draft {id}: {}\nreserved DOI {doi}; now run: just render paper",
            d["links"]["html"].as_str().unwrap_or("")
        );
    } else if args.iter().any(|a| a == "upload") {
        let st = std::fs::read_to_string(STATE).map_err(|e| format!("{STATE}: {e}"))?;
        let (base, dep) = st
            .split_once(char::is_whitespace)
            .ok_or("malformed .zenodo-draft")?;
        let (base, dep) = (base.trim(), dep.trim());
        let d = api(
            base,
            "GET",
            &format!("/deposit/depositions/{dep}"),
            &token,
            None,
            None,
        )?;
        let bucket = d["links"]["bucket"]
            .as_str()
            .ok_or("no bucket link in the response")?;
        let (_, bucket_path) = bucket.split_once("/api").ok_or("unexpected bucket link")?;
        let pdf_name = Path::new(PDF)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("PICASSOPAPER.pdf");
        let bytes = std::fs::read(PDF).map_err(|e| format!("{PDF}: {e}"))?;
        api(
            base,
            "PUT",
            &format!("{bucket_path}/{pdf_name}"),
            &token,
            None,
            Some(bytes),
        )?;
        println!(
            "uploaded {pdf_name}; review and publish at {}",
            d["links"]["html"].as_str().unwrap_or("")
        );
    } else {
        return Err(include_str!("usage.txt").trim_end().to_string());
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_metadata_passes_the_check() {
        let dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/whitepapers/determinism-2026");
        std::env::set_current_dir(dir).unwrap();
        // the PDF is a release artefact: only its absence may fail the check
        match check() {
            Ok(_) => {}
            Err(e) => assert!(e.starts_with("PICASSOPAPER.pdf missing"), "{e}"),
        }
    }

    #[test]
    fn truthiness_follows_python() {
        assert!(
            !truthy(Some(&json!(""))) && !truthy(Some(&json!([]))) && truthy(Some(&json!("x")))
        );
    }
}
