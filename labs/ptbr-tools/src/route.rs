//! ptbr-route: pick the skill and the level for a writing request, before any skill is loaded.
//!
//! ```text
//! ptbr-route "<pedido>"     one line: `<skill> · <nível>`, from routes.tsv beside the skill
//! ptbr-route --check        routes.tsv against SKILL.md: one row per level, every skill exists
//! ptbr-route --self-test    the answers for a few known requests
//! ```
//!
//! The binary sits in `<skill>/scripts/`, as route.py did: routes.tsv is in `<skill>/`, SKILL.md
//! two directories above that (the plugin root).

use crate::text::{fold, has_word, os_error, read_text, splitlines};
use std::path::PathBuf;

const COMPOSES: &str = "smart-note";

pub type Row = (String, String, Vec<String>);

pub fn routes(tsv: &str) -> Result<Vec<Row>, String> {
    let mut rows = Vec::new();
    for line in splitlines(tsv).into_iter().skip(1) {
        if line.trim_matches(crate::text::py_space).is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let [skill, level, words] = f[..] else {
            return Err(format!(
                "routes.tsv: a row needs 3 tab-separated fields: {line:?}"
            ));
        };
        let words = words
            .split(',')
            .map(|w| w.trim_matches(crate::text::py_space))
            .filter(|w| !w.is_empty())
            .map(fold)
            .collect();
        rows.push((skill.to_string(), level.to_string(), words));
    }
    Ok(rows)
}

fn score(request: &str, words: &[String]) -> usize {
    let text = fold(request);
    words.iter().filter(|w| has_word(&text, w)).count()
}

pub fn route(request: &str, rows: &[Row]) -> String {
    let scored: Vec<(usize, &str, &str)> = rows
        .iter()
        .map(|(s, lv, w)| (score(request, w), s.as_str(), lv.as_str()))
        .collect();
    let note = scored.iter().any(|&(n, _, lv)| n > 0 && lv == COMPOSES);
    let rest: Vec<_> = scored
        .iter()
        .filter(|&&(n, _, lv)| lv != COMPOSES && n > 0)
        .collect();
    let best = rest.iter().map(|x| x.0).max().unwrap_or(0);
    let top: Vec<_> = rest.iter().filter(|x| x.0 == best).collect();
    if top.is_empty() {
        return if note {
            "ptbr-writer · smart-note"
        } else {
            "ptbr-writer · sem nível: a skill classifica e declara"
        }
        .to_string();
    }
    let pre = if note {
        format!("{COMPOSES} + ")
    } else {
        String::new()
    };
    let line = top
        .iter()
        .map(|x| format!("{} · {pre}{}", x.1, x.2))
        .collect::<Vec<_>>()
        .join(" | ");
    line + if top.len() > 1 {
        " (empate: escolha um e declare)"
    } else {
        ""
    }
}

/// `re.findall(r"^- `([^`]+)`:", section, re.M)` over the "## Níveis de rigor" section.
pub fn levels(skill_md: &str) -> Vec<String> {
    let section = skill_md
        .split_once("## Níveis de rigor")
        .map_or(skill_md, |x| x.1);
    let section = section.split("\n## ").next().unwrap_or(section);
    section
        .split('\n')
        .filter_map(|l| {
            let r = l.strip_prefix("- `")?;
            let (name, after) = r.split_once('`')?;
            (!name.is_empty() && after.starts_with(':')).then(|| name.to_string())
        })
        .collect()
}

pub fn check_with(rows: &[Row], skill_md: &str, skills: &[String]) -> Vec<String> {
    let have: Vec<&str> = rows.iter().map(|r| r.1.as_str()).collect();
    let want = levels(skill_md);
    let mut errors: Vec<String> = want
        .iter()
        .filter(|lv| !have.contains(&lv.as_str()))
        .map(|lv| format!("routes.tsv: no row for level {lv}"))
        .collect();
    errors.extend(
        have.iter()
            .filter(|lv| !want.iter().any(|w| w == **lv))
            .map(|lv| format!("routes.tsv: {lv} is not a level in SKILL.md")),
    );
    let mut seen: Vec<&str> = Vec::new();
    for lv in &have {
        let n = have.iter().filter(|x| *x == lv).count();
        if n > 1 && !seen.contains(lv) {
            errors.push(format!("routes.tsv: {lv} appears {n} times"));
            seen.push(lv);
        }
    }
    errors.extend(
        rows.iter()
            .filter(|r| !skills.contains(&r.0))
            .map(|r| format!("routes.tsv: no skill {}", r.0)),
    );
    errors
}

fn here() -> Result<(PathBuf, PathBuf), String> {
    // argv[0] when it is a path (a symlink to the binary keeps its place, as route.py's
    // `__file__` did not resolve at the script level), else where the binary is.
    let exe = match std::env::args().next().filter(|a| a.contains('/')) {
        Some(a) => std::env::current_dir().map_err(|e| e.to_string())?.join(a),
        None => std::env::current_exe().map_err(|e| e.to_string())?,
    };
    let here = exe
        .parent()
        .and_then(|p| p.parent())
        .ok_or("cannot find the skill directory")?
        .to_path_buf();
    let root = here
        .parent()
        .and_then(|p| p.parent())
        .ok_or("cannot find the plugin root")?
        .to_path_buf();
    Ok((here, root))
}

fn read(p: &std::path::Path) -> Result<String, String> {
    let p = p.to_string_lossy();
    read_text(&p).map_err(|e| os_error(&e, &p))
}

fn check() -> Result<Vec<String>, String> {
    let (here, root) = here()?;
    let rows = routes(&read(&here.join("routes.tsv"))?)?;
    let mut skills = vec!["ptbr-writer".to_string()];
    if let Ok(dir) = std::fs::read_dir(root.join("skills")) {
        let mut found: Vec<String> = dir
            .flatten()
            .filter(|d| d.path().join("SKILL.md").is_file())
            .map(|d| d.file_name().to_string_lossy().into_owned())
            .collect();
        found.sort();
        skills.extend(found);
    }
    Ok(check_with(&rows, &read(&root.join("SKILL.md"))?, &skills))
}

/// The assertions of route.py's self_test; the first failure is returned.
pub fn self_test_with(rows: &[Row]) -> Result<(), String> {
    let eq = |got: String, want: &str| {
        if got == want {
            Ok(())
        } else {
            Err(format!("got {got:?}, want {want:?}"))
        }
    };
    eq(
        route("reescreva este runbook de incidente", rows),
        "ptbr-writer · estrito",
    )?;
    eq(
        route("revise o RELATORIO e as citações", rows),
        "cientifico · científico",
    )?;
    eq(
        route("transforme em notas do Obsidian este comunicado", rows),
        "ptbr-writer · smart-note + leve",
    )?;
    eq(
        route("guarde no meu segundo cérebro", rows),
        "ptbr-writer · smart-note",
    )?;
    let r = route("melhore isto", rows);
    if !r.starts_with("ptbr-writer · sem nível") {
        return Err(r);
    }
    let r = route("o comunicado sobre o runbook", rows);
    if !r.ends_with("(empate: escolha um e declare)") {
        return Err(r);
    }
    let r = route("um postmortem", rows);
    if !r.starts_with("ptbr-writer · sem nível") {
        return Err(format!("\"post\" is a word, not a prefix: {r}"));
    }
    if levels("## Níveis de rigor\n\n- `a`: x.\n- `b`: y.\n\n## Regras\n- `c`: z.\n") != ["a", "b"]
    {
        return Err("levels".into());
    }
    Ok(())
}

const USAGE: &str =
    "ptbr-route \"<pedido>\"     one line: `<skill> · <nível>`, from routes.tsv beside the skill\n\
ptbr-route --check        routes.tsv against SKILL.md: one row per level, every skill exists\n\
ptbr-route --self-test    the answers for a few known requests";

fn run(argv: &[String]) -> Result<i32, String> {
    if argv == ["--self-test"] {
        self_test_with(&routes(&read(&here()?.0.join("routes.tsv"))?)?)?;
        println!("route self-test: ok");
        return Ok(0);
    }
    if argv == ["--check"] {
        let errors = check()?;
        if errors.is_empty() {
            println!("route check: ok");
            return Ok(0);
        }
        eprintln!("{}", errors.join("\n"));
        return Ok(1);
    }
    if argv.is_empty() || argv[0].starts_with('-') {
        eprintln!("{USAGE}");
        return Ok(2);
    }
    println!(
        "{}",
        route(
            &argv.join(" "),
            &routes(&read(&here()?.0.join("routes.tsv"))?)?
        )
    );
    Ok(0)
}

pub fn main(argv: Vec<String>) -> i32 {
    run(&argv).unwrap_or_else(|e| {
        eprintln!("ptbr-route: {e}");
        1
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<Row> {
        routes(include_str!("../tests/fixtures/routes.tsv")).unwrap()
    }

    #[test]
    fn self_test() {
        self_test_with(&rows()).unwrap();
    }

    #[test]
    fn check_finds_drift() {
        let skills = vec!["ptbr-writer".to_string(), "cientifico".to_string()];
        let md = "## Níveis de rigor\n\n- `estrito`: a.\n- `descritivo`: b.\n- `leve`: c.\n- `científico`: d.\n- `smart-note`: e.\n";
        assert!(check_with(&rows(), md, &skills).is_empty());
        let md2 = md.replace("- `leve`: c.\n", "- `novo`: c.\n");
        assert_eq!(
            check_with(&rows(), &md2, &skills),
            [
                "routes.tsv: no row for level novo",
                "routes.tsv: leve is not a level in SKILL.md"
            ]
        );
        assert_eq!(
            check_with(&rows(), md, &skills[..1]),
            ["routes.tsv: no skill cientifico"]
        );
    }
}
