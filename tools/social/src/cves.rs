//! Counts GitHub-reviewed advisories with a CVE id per month for npm and PyPI, from OSV's dumps.
//!
//!     curl -O https://osv-vulnerabilities.storage.googleapis.com/npm/all.zip   (and PyPI/all.zip)
//!     social cves <dir with npm.zip and PyPI.zip> > cves.json
//!
//! OSV carries the GitHub Advisory Database as GHSA-* records; "published" is GitHub's date.

use crate::post::{PostError, Result};
use serde_json::{Map, Value, json, ser::PrettyFormatter};
use std::{collections::BTreeMap, io::Read, time::SystemTime};

/// Today in UTC as YYYY-MM-DD (days-since-epoch to civil date, H. Hinnant's algorithm).
fn today() -> String {
    let days = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() / 86400) as i64;
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

fn count(zip_path: &str) -> Result<BTreeMap<String, u64>> {
    let err = |e: &dyn std::fmt::Display| PostError(format!("{zip_path}: {e}"));
    let mut z = zip::ZipArchive::new(std::fs::File::open(zip_path).map_err(|e| err(&e))?)
        .map_err(|e| err(&e))?;
    let mut months = BTreeMap::new();
    for i in 0..z.len() {
        let mut buf = String::new();
        z.by_index(i)
            .map_err(|e| err(&e))?
            .read_to_string(&mut buf)
            .map_err(|e| err(&e))?;
        let v: Value = serde_json::from_str(&buf).map_err(|e| err(&e))?;
        let ghsa = v["id"].as_str().is_some_and(|s| s.starts_with("GHSA-"));
        let cve = v["aliases"].as_array().is_some_and(|a| {
            a.iter()
                .any(|x| x.as_str().is_some_and(|s| s.starts_with("CVE-")))
        });
        if ghsa && cve {
            let p = v["published"]
                .as_str()
                .ok_or_else(|| err(&"record without published"))?;
            *months.entry(p.chars().take(7).collect()).or_insert(0) += 1;
        }
    }
    Ok(months)
}

pub fn build(dir: &str, fetched: &str) -> Result<Value> {
    let mut monthly = Map::new();
    for eco in ["npm", "PyPI"] {
        monthly.insert(eco.into(), json!(count(&format!("{dir}/{eco}.zip"))?));
    }
    Ok(json!({
        "source": "OSV dumps of the GitHub Advisory Database (GHSA-* records with a CVE alias)",
        "fetched": fetched,
        "monthly": monthly,
    }))
}

/// json.dump(indent=1) of the original: one-space indent.
pub fn dump(v: &Value) -> String {
    let mut out = Vec::new();
    let mut ser =
        serde_json::Serializer::with_formatter(&mut out, PrettyFormatter::with_indent(b" "));
    serde::Serialize::serialize(v, &mut ser).expect("a Value always serializes");
    String::from_utf8(out).expect("JSON is UTF-8") + "\n"
}

pub fn run(dir: &str) -> Result<i32> {
    print!("{}", dump(&build(dir, &today())?));
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn counts_and_dumps_like_the_python() {
        let d = std::env::temp_dir().join(format!("social-cves-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        for (eco, recs) in [
            (
                "npm",
                vec![
                    r#"{"id":"GHSA-1","aliases":["CVE-1"],"published":"2020-01-05T00:00:00Z"}"#,
                    r#"{"id":"GHSA-2","aliases":["CVE-2"],"published":"2020-01-20T00:00:00Z"}"#,
                    r#"{"id":"GHSA-3","aliases":[],"published":"2020-01-20T00:00:00Z"}"#,
                    r#"{"id":"MAL-4","aliases":["CVE-4"],"published":"2020-01-20T00:00:00Z"}"#,
                    r#"{"id":"GHSA-5","published":"2019-12-01T00:00:00Z"}"#,
                ],
            ),
            (
                "PyPI",
                vec![r#"{"id":"GHSA-6","aliases":["CVE-6"],"published":"2019-03-01T00:00:00Z"}"#],
            ),
        ] {
            let mut w =
                zip::ZipWriter::new(std::fs::File::create(d.join(format!("{eco}.zip"))).unwrap());
            for (i, r) in recs.iter().enumerate() {
                w.start_file(
                    format!("{i}.json"),
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
                w.write_all(r.as_bytes()).unwrap();
            }
            w.finish().unwrap();
        }
        let out = dump(&build(d.to_str().unwrap(), "2026-10-12").unwrap());
        assert_eq!(
            out,
            "{\n \"source\": \"OSV dumps of the GitHub Advisory Database (GHSA-* records with a CVE alias)\",\n \"fetched\": \"2026-10-12\",\n \"monthly\": {\n  \"npm\": {\n   \"2020-01\": 2\n  },\n  \"PyPI\": {\n   \"2019-03\": 1\n  }\n }\n}\n"
        );
        std::fs::remove_dir_all(d).ok();
    }

    #[test]
    fn today_is_a_date() {
        let t = today();
        assert!(t.starts_with("20") && t.len() == 10, "{t}");
    }
}
