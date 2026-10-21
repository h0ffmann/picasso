//! CI guard for the site's hard rules (AGENTS.md), run on the built page (`just build`): the
//! Marola and Ouroboric logos are always on it, loaded from their own repositories, Gabriel Volpe
//! stays credited with a link, and the header links the hosting plan.
//!
//!   site-check [site-dir]     # default: the current directory (site/)

use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const LOGOS: [(&str, &str); 2] = [
    (
        "marola",
        "https://raw.githubusercontent.com/marola-dev/marola/",
    ),
    (
        "ouroboric",
        "https://raw.githubusercontent.com/ouroboric-dev/ouroboric/",
    ),
];

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        out.push(p.clone());
        // like pathlib's rglob: do not follow symlinked directories
        if e.file_type().is_ok_and(|t| t.is_dir()) {
            walk(&p, out);
        }
    }
}

pub fn check(site: &Path) -> Vec<String> {
    let dist = site.join("dist");
    let mut errors = vec![];
    let html = std::fs::read_to_string(dist.join("index.html")).unwrap_or_default();
    if html.is_empty() {
        errors.push("dist/index.html is missing: run `just build` first".to_string());
    }
    for (name, origin) in LOGOS {
        let block = Regex::new(&format!(
            r#"(?s)<a\b[^>]*\bdata-logo="{name}"[^>]*>.*?</a>"#
        ))
        .unwrap();
        let img = Regex::new(&format!(
            r#"<img\b[^>]*\bsrc="{}[^"]+\.svg""#,
            regex::escape(origin)
        ))
        .unwrap();
        if !block.find(&html).is_some_and(|b| img.is_match(b.as_str())) {
            errors.push(format!(
                "the {name} logo is missing, or not loaded from {origin}"
            ));
        }
    }
    if !html.contains(r#"href="https://github.com/gvolpe/nix-config""#) {
        errors.push("the credit to Gabriel Volpe (gvolpe/nix-config) is missing".to_string());
    }
    if !html.contains(r#"href="plan/hosting-plan.pdf""#)
        || !dist.join("plan/hosting-plan.pdf").exists()
    {
        errors.push(
            "the hosting plan (plan/hosting-plan.pdf) is not linked or not published".to_string(),
        );
    }
    let copy = Regex::new(r"(?i)(marola|ouroboric).*logo|logo.*(marola|ouroboric)").unwrap();
    let mut all = vec![];
    walk(site, &mut all);
    for p in all {
        let rel = p.strip_prefix(site).unwrap();
        if rel
            .components()
            .next()
            .is_some_and(|c| c.as_os_str() == "build")
        {
            continue;
        }
        if copy.is_match(&p.file_name().unwrap().to_string_lossy()) {
            errors.push(format!(
                "{}: trademark logos are hotlinked, never copied here",
                rel.display()
            ));
        }
    }
    errors
}

fn main() -> ExitCode {
    let site = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    let errors = check(&site);
    for e in &errors {
        eprintln!("site-check: {e}");
    }
    if errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAROLA: &str = r#"<a href="https://x" data-logo="marola"><img src="https://raw.githubusercontent.com/marola-dev/marola/abc/logo.svg"></a>"#;
    const OUROBORIC: &str = r#"<a data-logo="ouroboric" href="y"><img alt="" src="https://raw.githubusercontent.com/ouroboric-dev/ouroboric/abc/l.svg"></a>"#;
    const CREDIT: &str = r#"<a href="https://github.com/gvolpe/nix-config">gvolpe</a>"#;
    const PLAN: &str = r#"<a href="plan/hosting-plan.pdf">plan</a>"#;

    fn site(name: &str, html: &str, plan: bool) -> PathBuf {
        let s = std::env::temp_dir().join(format!("site-check-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&s);
        std::fs::create_dir_all(s.join("dist/plan")).unwrap();
        std::fs::write(s.join("dist/index.html"), html).unwrap();
        if plan {
            std::fs::write(s.join("dist/plan/hosting-plan.pdf"), "").unwrap();
        }
        s
    }

    fn page(parts: &[&str]) -> String {
        parts.join("\n")
    }

    #[test]
    fn complete_page_passes() {
        let s = site("ok", &page(&[MAROLA, OUROBORIC, CREDIT, PLAN]), true);
        assert_eq!(check(&s), Vec::<String>::new());
    }

    #[test]
    fn missing_marola_logo_fails() {
        let s = site("nomarola", &page(&[OUROBORIC, CREDIT, PLAN]), true);
        assert_eq!(check(&s).len(), 1);
        assert!(check(&s)[0].starts_with("the marola logo is missing"));
    }

    #[test]
    fn missing_ouroboric_logo_fails() {
        let s = site("noouro", &page(&[MAROLA, CREDIT, PLAN]), true);
        assert!(check(&s)[0].starts_with("the ouroboric logo is missing"));
    }

    #[test]
    fn logo_not_from_own_repo_fails() {
        let bad = MAROLA.replace(
            "raw.githubusercontent.com/marola-dev/marola",
            "example.com/marola",
        );
        let s = site("badorigin", &page(&[&bad, OUROBORIC, CREDIT, PLAN]), true);
        assert!(check(&s)[0].starts_with("the marola logo"));
    }

    #[test]
    fn logo_that_is_not_an_svg_fails() {
        let bad = MAROLA.replace(".svg", ".png");
        let s = site("png", &page(&[&bad, OUROBORIC, CREDIT, PLAN]), true);
        assert_eq!(check(&s).len(), 1);
    }

    #[test]
    fn missing_credit_fails() {
        let s = site("nocredit", &page(&[MAROLA, OUROBORIC, PLAN]), true);
        assert_eq!(
            check(&s),
            ["the credit to Gabriel Volpe (gvolpe/nix-config) is missing"]
        );
    }

    #[test]
    fn missing_plan_link_or_file_fails() {
        let s = site("noplanlink", &page(&[MAROLA, OUROBORIC, CREDIT]), true);
        assert_eq!(check(&s).len(), 1);
        let s = site(
            "noplanfile",
            &page(&[MAROLA, OUROBORIC, CREDIT, PLAN]),
            false,
        );
        assert_eq!(check(&s).len(), 1);
    }

    #[test]
    fn missing_page_fails() {
        let s = site("nopage", "", true);
        assert!(check(&s)[0].starts_with("dist/index.html is missing"));
    }

    #[test]
    fn copied_logo_file_fails_but_build_dir_is_skipped() {
        let s = site("copy", &page(&[MAROLA, OUROBORIC, CREDIT, PLAN]), true);
        std::fs::create_dir_all(s.join("build")).unwrap();
        std::fs::write(s.join("build/Marola-Logo.svg"), "").unwrap();
        assert_eq!(check(&s), Vec::<String>::new());
        std::fs::create_dir_all(s.join("static")).unwrap();
        std::fs::write(s.join("static/ouroboric_logo.svg"), "").unwrap();
        assert_eq!(
            check(&s),
            ["static/ouroboric_logo.svg: trademark logos are hotlinked, never copied here"]
        );
    }
}
