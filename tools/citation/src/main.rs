//! citation — .zenodo.json is the one hand-edited citation file; CITATION.cff is generated from it.
//!
//! Zenodo reads only .zenodo.json when both exist, and GitHub's "Cite this repository" reads only
//! CITATION.cff, so the CFF is derived here, with the BibTeX, APA and ABNT references between the
//! READMEs' citation markers, and --check fails when any of them drifts. Ported from the Python
//! scripts/citation.py (itself copied from marola-dev/marola, MIP-0079); only the constants differ.
//! Paths are relative to the current directory, which must be the repository root.

use regex::Regex;
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::LazyLock;

const START: &str = "<!-- citation:start -->";
const END: &str = "<!-- citation:end -->";

// The concept DOI resolves to the latest release; v0.1.1 minted it (v0.1.0 is not on Zenodo).
const CONCEPT_DOI: &str = "10.5281/zenodo.23311208";
const NAME: &str = "picasso";
const HOMEPAGE: &str = "https://github.com/h0ffmann/picasso";
const YEAR: &str = "2026";

// developers.zenodo.org, deposit metadata, checked 2026-10-07.
const RELATIONS: &[&str] = &[
    "isCitedBy",
    "cites",
    "isSupplementTo",
    "isSupplementedBy",
    "isContinuedBy",
    "continues",
    "isDescribedBy",
    "describes",
    "hasMetadata",
    "isMetadataFor",
    "isNewVersionOf",
    "isPreviousVersionOf",
    "isPartOf",
    "hasPart",
    "isReferencedBy",
    "references",
    "isDocumentedBy",
    "documents",
    "isCompiledBy",
    "compiles",
    "isVariantFormOf",
    "isOriginalFormof",
    "isIdenticalTo",
    "isAlternateIdentifier",
    "isReviewedBy",
    "reviews",
    "isDerivedFrom",
    "isSourceOf",
    "requires",
    "isRequiredBy",
    "isObsoletedBy",
    "obsoletes",
];
const CONTRIBUTOR_TYPES: &[&str] = &[
    "ContactPerson",
    "DataCollector",
    "DataCurator",
    "DataManager",
    "Distributor",
    "Editor",
    "HostingInstitution",
    "Producer",
    "ProjectLeader",
    "ProjectManager",
    "ProjectMember",
    "RegistrationAgency",
    "RegistrationAuthority",
    "RelatedPerson",
    "Researcher",
    "ResearchGroup",
    "RightsHolder",
    "Supervisor",
    "Sponsor",
    "WorkPackageLeader",
    "Other",
];
const SPDX: &[(&str, &str)] = &[("mit", "MIT"), ("agpl-3.0-only", "AGPL-3.0-only")];
// Zenodo sets these itself (version from the tag, date from the release, DOI minted); a value
// here would override or break that.
const FORBIDDEN: &[&str] = &["version", "publication_date", "doi", "prereserve_doi"];

static NAME_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[^,]+, [^,]+$").unwrap());
static ORCID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\d{4}-\d{4}-\d{4}-\d{3}[\dX]$").unwrap());
static DOI_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^10\.\d{4,9}/\S+$").unwrap());

type Meta = Map<String, Value>;
/// name, orcid, affiliation, contributor type, author
type AddArgs = (Option<String>, Option<String>, Option<String>, String, bool);

/// Python's `dict.get(key)` truthiness: missing, null, "", [] and 0 are falsy.
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

fn s<'a>(m: &'a Map<String, Value>, k: &str) -> &'a str {
    m.get(k).and_then(Value::as_str).unwrap_or("")
}

/// Python's repr() of an optional JSON string, for the error messages.
fn repr(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => "None".into(),
        Some(Value::String(t)) if t.contains('\'') && !t.contains('"') => format!("\"{t}\""),
        Some(Value::String(t)) => format!("'{}'", t.replace('\\', "\\\\").replace('\'', "\\'")),
        Some(other) => other.to_string(),
    }
}

fn objs<'a>(meta: &'a Meta, key: &str) -> Vec<&'a Map<String, Value>> {
    meta.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_object).collect())
        .unwrap_or_default()
}

fn orcid_ok(orcid: &str) -> bool {
    if !ORCID_RE.is_match(orcid) {
        return false;
    }
    let digits: Vec<char> = orcid.chars().filter(|c| *c != '-').collect();
    let mut total = 0u32;
    for ch in &digits[..digits.len() - 1] {
        total = (total + ch.to_digit(10).unwrap()) * 2;
    }
    let check = (12 - total % 11) % 11;
    let want = if check == 10 {
        'X'
    } else {
        char::from_digit(check, 10).unwrap()
    };
    digits[digits.len() - 1] == want
}

fn person_errors(p: &Map<String, Value>, wher: &str) -> Vec<String> {
    let mut errs = vec![];
    if !NAME_RE.is_match(s(p, "name")) {
        errs.push(format!(
            "{wher}: name {} is not 'Family, Given'",
            repr(p.get("name"))
        ));
    }
    if let Some(o) = p.get("orcid")
        && !orcid_ok(o.as_str().unwrap_or(""))
    {
        errs.push(format!(
            "{wher}: orcid {} is not a valid bare ORCID iD",
            repr(Some(o))
        ));
    }
    errs
}

fn validate(meta: &Meta) -> Vec<String> {
    let mut errs = vec![];
    for key in [
        "title",
        "description",
        "creators",
        "license",
        "keywords",
        "language",
    ] {
        if !truthy(meta.get(key)) {
            errs.push(format!("missing {key}"));
        }
    }
    if !matches!(s(meta, "upload_type"), "software" | "dataset") {
        errs.push("upload_type must be software or dataset".into());
    }
    if s(meta, "access_right") != "open" {
        errs.push("access_right must be open".into());
    }
    if !SPDX.iter().any(|(k, _)| *k == s(meta, "license")) {
        errs.push(format!(
            "license {} has no SPDX mapping here",
            repr(meta.get("license"))
        ));
    }
    let lang = s(meta, "language");
    if lang.chars().count() != 3 || !lang.chars().all(|c| c.is_ascii_lowercase()) {
        errs.push("language must be an ISO 639-3 code such as eng".into());
    }
    for k in FORBIDDEN {
        if meta.contains_key(*k) {
            errs.push(format!("{k} is set by Zenodo; remove it"));
        }
    }
    for (i, p) in objs(meta, "creators").into_iter().enumerate() {
        errs.extend(person_errors(p, &format!("creators[{i}]")));
    }
    for (i, p) in objs(meta, "contributors").into_iter().enumerate() {
        errs.extend(person_errors(p, &format!("contributors[{i}]")));
        if !CONTRIBUTOR_TYPES.contains(&s(p, "type")) {
            errs.push(format!(
                "contributors[{i}]: type {} is not Zenodo's",
                repr(p.get("type"))
            ));
        }
    }
    for (i, r) in objs(meta, "related_identifiers").into_iter().enumerate() {
        if !RELATIONS.contains(&s(r, "relation")) {
            errs.push(format!(
                "related_identifiers[{i}]: relation {} unknown",
                repr(r.get("relation"))
            ));
        }
        let ident = s(r, "identifier");
        match s(r, "scheme") {
            "doi" if !DOI_RE.is_match(ident) => errs.push(format!(
                "related_identifiers[{i}]: {} is not a bare DOI",
                repr(r.get("identifier").or(Some(&json!(""))))
            )),
            "url" if !ident.starts_with("https://") => errs.push(format!(
                "related_identifiers[{i}]: {} is not an https URL",
                repr(r.get("identifier").or(Some(&json!(""))))
            )),
            "doi" | "url" => {}
            _ => errs.push(format!(
                "related_identifiers[{i}]: scheme must be doi or url"
            )),
        }
    }
    errs
}

fn q(t: &str) -> String {
    serde_json::to_string(t).unwrap()
}

/// shortcut: html.unescape knows ~2200 named entities, this knows the five XML ones and numeric
/// references; upgrade if a description ever needs more.
fn unescape(t: &str) -> String {
    static ENT: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"&(#[xX][0-9a-fA-F]+|#[0-9]+|amp|lt|gt|quot|apos);").unwrap());
    ENT.replace_all(t, |c: &regex::Captures| {
        let e = &c[1];
        let ch = match e {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if e[1..].starts_with(['x', 'X']) => u32::from_str_radix(&e[2..], 16)
                .ok()
                .and_then(char::from_u32),
            _ => e[1..].parse().ok().and_then(char::from_u32),
        };
        ch.map(String::from).unwrap_or_else(|| c[0].to_string())
    })
    .into_owned()
}

fn plain(description: &str) -> String {
    static PP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"</p>\s*<p>").unwrap());
    static TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]+>").unwrap());
    let text = PP.replace_all(description, "\n\n");
    unescape(&TAG.replace_all(&text, "")).trim().to_string()
}

fn repo(meta: &Meta) -> Result<String, String> {
    objs(meta, "related_identifiers")
        .into_iter()
        .find(|r| {
            s(r, "relation") == "isSupplementTo"
                && s(r, "identifier").starts_with("https://github.com/")
        })
        .map(|r| s(r, "identifier").to_string())
        .ok_or_else(|| "no isSupplementTo related identifier pointing at github.com".to_string())
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let (mut lines, mut line) = (vec![], String::new());
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            lines.push(std::mem::take(&mut line));
            line = word.to_string();
        } else if line.is_empty() {
            line = word.to_string();
        } else {
            line = format!("{line} {word}");
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn families(meta: &Meta) -> Vec<(String, String)> {
    objs(meta, "creators")
        .into_iter()
        .map(|p| {
            let (f, g) = s(p, "name").split_once(", ").unwrap_or((s(p, "name"), ""));
            (f.to_string(), g.to_string())
        })
        .collect()
}

fn render_cff(meta: &Meta, concept_doi: &str) -> Result<String, String> {
    let repo = repo(meta)?;
    let mut out = vec![
        "# Generated from .zenodo.json by scripts/citation.py; edit that file, then `just citation`.".to_string(),
        "cff-version: 1.2.0".into(),
        format!("message: \"If you use {NAME}, please cite it using the metadata below.\""),
        format!("type: {}", s(meta, "upload_type")),
        format!("title: {}", q(s(meta, "title"))),
        "abstract: >-".into(),
    ];
    for para in plain(s(meta, "description")).split("\n\n") {
        out.extend(wrap(para, 96).into_iter().map(|l| format!("  {l}")));
        out.push(String::new());
    }
    out.pop();
    out.push("authors:".into());
    for p in objs(meta, "creators") {
        let (family, given) = s(p, "name").split_once(", ").unwrap_or((s(p, "name"), ""));
        out.push(format!("  - family-names: {}", q(family)));
        out.push(format!("    given-names: {}", q(given)));
        if p.contains_key("orcid") {
            out.push(format!(
                "    orcid: \"https://orcid.org/{}\"",
                s(p, "orcid")
            ));
        }
        if p.contains_key("affiliation") {
            out.push(format!("    affiliation: {}", q(s(p, "affiliation"))));
        }
    }
    out.push(format!("repository-code: \"{repo}\""));
    out.push(format!("url: \"{HOMEPAGE}\""));
    if !concept_doi.is_empty() {
        out.extend([
            format!("doi: {concept_doi}"),
            "identifiers:".into(),
            "  - type: doi".into(),
            format!("    value: {concept_doi}"),
            "    description: Concept DOI, resolves to the latest release".into(),
        ]);
    }
    let spdx = SPDX
        .iter()
        .find(|(k, _)| *k == s(meta, "license"))
        .map_or("", |(_, v)| v);
    out.push(format!("license: {spdx}"));
    out.push("keywords:".into());
    for k in meta
        .get("keywords")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        out.push(format!("  - {}", q(k.as_str().unwrap_or(""))));
    }
    Ok(out.join("\n") + "\n")
}

fn add_person(
    meta: &mut Meta,
    name: &str,
    orcid: Option<&str>,
    affiliation: Option<&str>,
    ctype: &str,
    author: bool,
) -> Result<(), String> {
    let mut person = Map::new();
    person.insert("name".into(), json!(name));
    if let Some(a) = affiliation.filter(|a| !a.is_empty()) {
        person.insert("affiliation".into(), json!(a));
    }
    if let Some(o) = orcid.filter(|o| !o.is_empty()) {
        person.insert("orcid".into(), json!(o));
    }
    let key = if author { "creators" } else { "contributors" };
    if !author {
        person.insert("type".into(), json!(ctype));
    }
    let listed = objs(meta, "creators")
        .into_iter()
        .chain(objs(meta, "contributors"));
    if listed.into_iter().any(|p| s(p, "name") == name) {
        return Err(format!("{name} is already listed"));
    }
    match meta.entry(key).or_insert_with(|| json!([])) {
        Value::Array(a) => a.push(Value::Object(person)),
        _ => return Err(format!("{key} is not a list")),
    }
    Ok(())
}

fn write_json(path: &Path, meta: &Meta) -> Result<(), String> {
    let text = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    std::fs::write(path, text + "\n").map_err(|e| format!("{}: {e}", path.display()))
}

fn render_refs(meta: &Meta, concept_doi: &str) -> Result<String, String> {
    let people = families(meta);
    let repo = repo(meta)?;
    let title = s(meta, "title");
    let (short, sub) = match title.split_once(": ") {
        Some((a, b)) => (a, Some(b)),
        None => (title, None),
    };
    let slug: String = short
        .to_lowercase()
        .chars()
        .filter(|c| matches!(c, 'a'..='z' | '0'..='9'))
        .collect();
    let mut bib = vec![
        format!("@software{{{}_{YEAR}_{slug},", people[0].0.to_lowercase()),
        format!(
            "  author    = {{{}}},",
            people
                .iter()
                .map(|(f, g)| format!("{f}, {g}"))
                .collect::<Vec<_>>()
                .join(" and ")
        ),
        format!("  title     = {{{{{title}}}}},"),
        format!("  year      = {{{YEAR}}},"),
    ];
    if !concept_doi.is_empty() {
        bib.push("  publisher = {Zenodo},".into());
        bib.push(format!("  doi       = {{{concept_doi}}},"));
    }
    bib.push(format!("  url       = {{{repo}}}"));
    bib.push("}".into());

    let initials = |g: &str| {
        g.split_whitespace()
            .map(|p| format!("{}.", p.chars().next().unwrap()))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let apa_names: Vec<String> = people
        .iter()
        .map(|(f, g)| format!("{f}, {}", initials(g)))
        .collect();
    let apa_authors = match apa_names.split_last() {
        Some((last, rest)) if !rest.is_empty() => format!("{}, & {last}", rest.join(", ")),
        _ => apa_names[0].clone(),
    };
    let wher = if concept_doi.is_empty() {
        repo.clone()
    } else {
        format!("Zenodo. https://doi.org/{concept_doi}")
    };
    let apa = format!("{apa_authors} ({YEAR}). *{title}* [Computer software]. {wher}");
    let abnt_authors = people
        .iter()
        .map(|(f, g)| format!("{}, {g}", f.to_uppercase()))
        .collect::<Vec<_>>()
        .join("; ");
    let abnt_title = match sub {
        Some(sub) => format!("**{short}**: {sub}"),
        None => format!("**{title}**"),
    };
    let abnt = if concept_doi.is_empty() {
        format!("{abnt_authors}. {abnt_title}. [S. l.]: GitHub, {YEAR}. Disponível em: {repo}.")
    } else {
        format!("{abnt_authors}. {abnt_title}. [S. l.]: Zenodo, {YEAR}. DOI {concept_doi}.")
    };
    let mut lines = vec![
        START.to_string(),
        String::new(),
        "BibTeX:".into(),
        String::new(),
        "```bibtex".into(),
    ];
    lines.extend(bib);
    lines.extend([
        "```".to_string(),
        String::new(),
        "APA:".into(),
        String::new(),
        format!("> {apa}"),
        String::new(),
        "ABNT (NBR 6023):".into(),
        String::new(),
        format!("> {abnt}"),
        String::new(),
        END.into(),
    ]);
    Ok(lines.join("\n"))
}

fn splice(text: &str, block: &str) -> Result<String, String> {
    let err = || format!("no {START} ... {END} block");
    let (head, rest) = text.split_once(START).ok_or_else(err)?;
    let (_, tail) = rest.split_once(END).ok_or_else(err)?;
    Ok(format!("{head}{block}{tail}"))
}

fn read_meta(path: &Path) -> Result<Meta, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    match serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))? {
        Value::Object(m) => Ok(m),
        _ => Err(format!("{}: not a JSON object", path.display())),
    }
}

fn name(p: &Path) -> String {
    p.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

fn run(zenodo: &Path, cff: &Path, check: bool, readmes: &[PathBuf]) -> u8 {
    let meta = match read_meta(zenodo) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("citation: {e}");
            return 1;
        }
    };
    let errs = validate(&meta);
    if !errs.is_empty() {
        eprintln!(
            "citation: .zenodo.json is invalid:\n  {}",
            errs.join("\n  ")
        );
        return 1;
    }
    let build = || -> Result<Vec<(PathBuf, String)>, String> {
        let block = render_refs(&meta, CONCEPT_DOI)?;
        let mut wanted = vec![(cff.to_path_buf(), render_cff(&meta, CONCEPT_DOI)?)];
        for readme in readmes {
            let text =
                std::fs::read_to_string(readme).map_err(|e| format!("{}: {e}", name(readme)))?;
            let spliced = splice(&text, &block).map_err(|e| format!("{}: {e}", name(readme)))?;
            wanted.push((readme.clone(), spliced));
        }
        Ok(wanted)
    };
    let wanted = match build() {
        Ok(w) => w,
        Err(e) => {
            eprintln!("citation: {e}");
            return 1;
        }
    };
    let stale: Vec<&(PathBuf, String)> = wanted
        .iter()
        .filter(|(p, text)| std::fs::read_to_string(p).map_or(true, |t| &t != text))
        .collect();
    if check {
        if !stale.is_empty() {
            let names: Vec<String> = stale.iter().map(|(p, _)| name(p)).collect();
            eprintln!("citation: stale: {}; run `just citation`", names.join(", "));
            return 1;
        }
        eprintln!("citation: .zenodo.json valid, generated text current");
        return 0;
    }
    for (p, text) in stale {
        if let Err(e) = std::fs::write(p, text) {
            eprintln!("citation: {}: {e}", p.display());
            return 1;
        }
        eprintln!("citation: wrote {}", name(p));
    }
    0
}

/// The original script's `--self-test`; `cargo test` runs the same function.
pub fn self_test() -> Result<(), String> {
    let mut good: Meta = serde_json::from_value(json!({
        "title": "t",
        "upload_type": "software",
        "description": "<p>One &amp; two.</p><p>Three.</p>",
        "creators": [{"name": "Santos, Matheus Hoffmann Fernandes", "orcid": "0009-0009-1056-7661"}],
        "license": "mit",
        "access_right": "open",
        "language": "eng",
        "keywords": ["nix"],
        "related_identifiers": [
            {"identifier": "https://github.com/x/y", "relation": "isSupplementTo", "scheme": "url"},
            {"identifier": "10.5281/zenodo.23221352", "relation": "references", "scheme": "doi"},
        ],
    }))
    .unwrap();
    macro_rules! check {
        ($c:expr) => {
            if !$c {
                return Err(format!("assertion failed: {}", stringify!($c)));
            }
        };
    }
    check!(validate(&good).is_empty());
    check!(orcid_ok("0000-0002-1825-0097") && !orcid_ok("0000-0002-1825-0098"));
    let mut bad = good.clone();
    bad.insert("version".into(), json!("v1"));
    bad["creators"][0]["name"] = json!("Matheus Santos");
    bad["related_identifiers"][1]["relation"] = json!("isRelatedTo");
    check!(validate(&bad).len() == 3);
    let cff = render_cff(&good, "")?;
    check!(
        cff.contains("family-names: \"Santos\"") && cff.contains("orcid.org/0009-0009-1056-7661")
    );
    check!(cff.contains("  One & two.\n\n  Three.\n") && !cff.contains("doi:"));
    check!(render_cff(&good, "10.5281/zenodo.1")?.contains("doi: 10.5281/zenodo.1\n"));
    add_person(
        &mut good,
        "Valério, Bruno",
        None,
        None,
        "ProjectMember",
        false,
    )?;
    check!(good["contributors"] == json!([{"name": "Valério, Bruno", "type": "ProjectMember"}]));
    check!(validate(&good).is_empty());
    check!(add_person(&mut good, "Valério, Bruno", None, None, "Other", true).is_err());
    let refs = render_refs(&good, "")?;
    check!(
        refs.contains("author    = {Santos, Matheus Hoffmann Fernandes}") && !refs.contains("doi")
    );
    check!(
        refs.contains("> Santos, M. H. F. (2026). *t* [Computer software]. https://github.com/x/y")
    );
    good["creators"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name": "Valério, Bruno"}));
    good.insert("title".into(), json!("m: a sub"));
    let refs = render_refs(&good, "10.5281/zenodo.1")?;
    check!(
        refs.contains("Santos, M. H. F., & Valério, B. (2026)")
            && refs.contains("doi       = {10.5281/zenodo.1}")
    );
    check!(refs.contains("SANTOS, Matheus Hoffmann Fernandes; VALÉRIO, Bruno. **m**: a sub."));
    check!(refs.contains("DOI 10.5281/zenodo.1.") && refs.contains("@software{santos_2026_m,"));
    check!(splice(&format!("a\n{START}\nold\n{END}\nb"), "NEW")? == "a\nNEW\nb");
    let d = std::env::temp_dir().join(format!("citation-selftest-{}", std::process::id()));
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let (z, c, r) = (
        d.join(".zenodo.json"),
        d.join("CITATION.cff"),
        d.join("README.md"),
    );
    let rs = [r.clone()];
    let res = (|| {
        write_json(&z, &good)?;
        std::fs::write(&r, format!("# x\n\n{START}\n{END}\n\n## y\n"))
            .map_err(|e| e.to_string())?;
        check!(run(&z, &c, true, &rs) == 1);
        check!(run(&z, &c, false, &rs) == 0 && run(&z, &c, true, &rs) == 0);
        let text = std::fs::read_to_string(&r).map_err(|e| e.to_string())?;
        check!(text.contains("BibTeX:") && text.ends_with("\n\n## y\n"));
        std::fs::write(&r, "no markers\n").map_err(|e| e.to_string())?;
        check!(run(&z, &c, true, &rs) == 1);
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&d);
    res?;
    eprintln!("citation: self-test ok");
    Ok(())
}

const USAGE: &str = "usage: citation [--check] [--self-test] [add --name \"Family, Given\" [--orcid ID] [--affiliation TEXT] [--type TYPE | --author]]";

fn usage_error(msg: &str) -> ExitCode {
    eprintln!("{USAGE}\ncitation: error: {msg}");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let (zenodo, cff, readmes) = (
        PathBuf::from(".zenodo.json"),
        PathBuf::from("CITATION.cff"),
        [PathBuf::from("README.md")],
    );
    let mut args = std::env::args().skip(1);
    let (mut check, mut self_t) = (false, false);
    let mut add: Option<AddArgs> = None;
    while let Some(a) = args.next() {
        match (a.as_str(), add.as_mut()) {
            ("-h" | "--help", _) => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            ("--check", None) => check = true,
            ("--self-test", None) => self_t = true,
            ("add", None) => add = Some((None, None, None, "ProjectMember".into(), false)),
            ("--author", Some(x)) => x.4 = true,
            (opt @ ("--name" | "--orcid" | "--affiliation" | "--type"), Some(x)) => {
                let Some(v) = args.next() else {
                    return usage_error(&format!("argument {opt}: expected one argument"));
                };
                match opt {
                    "--name" => x.0 = Some(v),
                    "--orcid" => x.1 = Some(v),
                    "--affiliation" => x.2 = Some(v),
                    _ if CONTRIBUTOR_TYPES.contains(&v.as_str()) => x.3 = v,
                    _ => return usage_error(&format!("argument --type: invalid choice: {v:?}")),
                }
            }
            _ => return usage_error(&format!("unrecognized arguments: {a}")),
        }
    }
    if self_t {
        return match self_test() {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("citation: self-test failed: {e}");
                ExitCode::FAILURE
            }
        };
    }
    if let Some((name, orcid, affiliation, ctype, author)) = add {
        let Some(name) = name else {
            return usage_error("the following arguments are required: --name");
        };
        let go = || -> Result<(), String> {
            let mut meta = read_meta(&zenodo)?;
            add_person(
                &mut meta,
                &name,
                orcid.as_deref(),
                affiliation.as_deref(),
                &ctype,
                author,
            )?;
            let errs = validate(&meta);
            if !errs.is_empty() {
                return Err(format!("not added:\n  {}", errs.join("\n  ")));
            }
            write_json(&zenodo, &meta)
        };
        if let Err(e) = go() {
            eprintln!("citation: {e}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::from(run(&zenodo, &cff, check, &readmes))
}

#[cfg(test)]
mod tests {
    #[test]
    fn self_test() {
        super::self_test().unwrap();
    }

    #[test]
    fn unescape_numeric() {
        assert_eq!(super::unescape("a &#38; b &#x26; &lt;"), "a & b & <");
    }
}
