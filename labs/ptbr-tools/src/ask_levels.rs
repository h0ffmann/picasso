//! ptbr-ask-levels: the UserPromptSubmit hook's logic. A 1:1 port of ask_levels.py.
//!
//! A request that starts with `ptbr` (`ptbr: ...`, `ptbr ...`, `/ptbr`, `/ptbr-writer:...`) gets one
//! instruction for Claude; any other request gets nothing, so the hook costs no context.
//!
//! The choices live in ptbr-writer.toml, read in this order, each overriding the one before:
//! the plugin's ptbr-writer.default.toml, ~/.config/ptbr-writer/ptbr-writer.toml, then
//! ./ptbr-writer.toml; the request itself overrides all. With no config file, Claude asks the
//! levels, the formats and the folder once and writes ./ptbr-writer.toml.
//!
//! ```text
//! ptbr-ask-levels SKILL.md TO_PDF DEFAULTS [READING_TIME] < hook-input.json
//! ptbr-ask-levels --self-test
//! ```

use crate::json::{self, J};
use crate::text::{fold, has_word, is_word, join, py_repr, py_space, read_text};
use crate::toml::{self, Value};
use std::io::Read;

const COMPOSES: &str = "smart-note"; // combines with another level: the `notes` format, not one of the levels
const MAX: usize = 4; // AskUserQuestion takes at most 4 options; "Other" is added by itself
const FORMATS: [&str; 4] = ["txt", "md", "pdf", "notes"];
const LANGUAGES: [&str; 2] = ["pt-BR", "asd-ste100"]; // pt-BR is the default; ASD-STE100 is Simplified Technical English
const NAME: &str = "ptbr-writer-<AAAAMMDDThhmmZ>-<objetivo>-<nível>.<ext>";

fn lower(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// `(?<=...)` helper: does `s` (already a char slice) start with `word`, ignoring case?
fn starts_ci(s: &[char], word: &str) -> Option<usize> {
    let w: Vec<char> = word.chars().collect();
    (s.len() >= w.len() && s.iter().zip(&w).all(|(a, b)| lower(*a) == *b)).then_some(w.len())
}

fn after_ws_slash(prompt: &str) -> Vec<char> {
    let c: Vec<char> = prompt.chars().collect();
    let mut i = 0;
    while i < c.len() && py_space(c[i]) {
        i += 1;
    }
    if c.get(i) == Some(&'/') {
        i += 1;
    }
    c[i..].to_vec()
}

/// `^\s*/?ptbr(-writer)?(:\S*)?(?=[\s:,.]|$)`, ignoring case.
fn is_trigger(prompt: &str) -> bool {
    let c = after_ws_slash(prompt);
    let Some(n) = starts_ci(&c, "ptbr") else {
        return false;
    };
    let tail = |r: &[char]| {
        r.first()
            .is_none_or(|&x| py_space(x) || matches!(x, ':' | ',' | '.'))
    };
    let rest = &c[n..];
    starts_ci(rest, "-writer").is_some_and(|w| tail(&rest[w..])) || tail(rest)
}

/// `^\s*/?ptbr(-writer)?\s*:?\s*(reconfigur\w*|config\w*)\b`, ignoring case.
fn is_reconfig(prompt: &str) -> bool {
    let c = after_ws_slash(prompt);
    let Some(n) = starts_ci(&c, "ptbr") else {
        return false;
    };
    let rest = &c[n..];
    let tail = |r: &[char]| {
        let mut i = 0;
        while r.get(i).is_some_and(|&x| py_space(x)) {
            i += 1;
        }
        if r.get(i) == Some(&':') {
            i += 1;
        }
        while r.get(i).is_some_and(|&x| py_space(x)) {
            i += 1;
        }
        // `\w*` takes the rest of the word, so `\b` always holds after it.
        starts_ci(&r[i..], "reconfigur")
            .or_else(|| starts_ci(&r[i..], "config"))
            .is_some()
    };
    starts_ci(rest, "-writer").is_some_and(|w| tail(&rest[w..])) || tail(rest)
}

/// `(?<!\w)(asd-?)?ste-?100(?!\w)`, ignoring case.
fn mentions_ste(prompt: &str) -> bool {
    let c: Vec<char> = prompt.chars().collect();
    (0..c.len()).any(|i| {
        if i > 0 && is_word(c[i - 1]) {
            return false;
        }
        let try_from = |mut j: usize| -> bool {
            let Some(n) = starts_ci(&c[j..], "ste") else {
                return false;
            };
            j += n;
            if c.get(j) == Some(&'-') {
                j += 1;
            }
            let Some(n) = starts_ci(&c[j..], "100") else {
                return false;
            };
            j += n;
            c.get(j).is_none_or(|&x| !is_word(x))
        };
        let with_asd = starts_ci(&c[i..], "asd").is_some_and(|n| {
            try_from(i + n) || (c.get(i + n) == Some(&'-') && try_from(i + n + 1))
        });
        with_asd || try_from(i)
    })
}

/// The "## Níveis de rigor" list of SKILL.md: (name, what it is, without the final dots).
pub fn skill_levels(text: &str) -> Vec<(String, String)> {
    let section = text.split_once("## Níveis de rigor").map_or(text, |x| x.1);
    let section = section.split("\n## ").next().unwrap_or(section);
    section
        .split('\n')
        .filter_map(|l| {
            let r = l.strip_prefix("- `")?;
            let (name, after) = r.split_once('`')?;
            let u = after.strip_prefix(": ")?;
            (!name.is_empty() && !u.is_empty())
                .then(|| (name.to_string(), u.trim_end_matches('.').to_string()))
        })
        .collect()
}

#[derive(Clone, Default)]
struct Cfg {
    levels: Option<Vec<String>>,
    formats: Option<Vec<String>>,
    folder: Option<String>,
    print: Option<Vec<String>>,
    languages: Option<Vec<String>>,
}

impl Cfg {
    fn apply(&mut self, o: Cfg) {
        macro_rules! over {
            ($($f:ident),*) => { $(if o.$f.is_some() { self.$f = o.$f; })* };
        }
        over!(levels, formats, folder, print, languages);
    }
}

/// The table in the file: empty when absent, `None` when it cannot be read.
fn load(path: &str) -> Option<toml::Doc> {
    if !std::path::Path::new(path).is_file() {
        return Some(toml::Doc::default());
    }
    toml::parse(&read_text(path).ok()?).ok()
}

/// A value as Python's `str()` and `repr()` show it inside an f-string.
fn show(v: &Value, repr: bool) -> String {
    match v {
        Value::Str(s) if repr => py_repr(s),
        Value::Str(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Arr(a) => format!(
            "[{}]",
            a.iter()
                .map(|x| show(x, true))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Table => "{}".to_string(),
        Value::Other(t) => t.clone(),
    }
}

/// Keep only values the plugin knows; a folder must stay inside the session's root.
fn clean(
    doc: &toml::Doc,
    levels: &[(String, String)],
    problems: &mut Vec<String>,
    where_: &str,
) -> Cfg {
    let mut out = Cfg::default();
    let names: Vec<&str> = levels
        .iter()
        .map(|l| l.0.as_str())
        .filter(|n| *n != COMPOSES)
        .collect();
    for (key, allowed) in [
        ("levels", &names[..]),
        ("formats", &FORMATS[..]),
        ("print", &FORMATS[..]),
        ("languages", &LANGUAGES[..]),
    ] {
        let Some(v) = doc.get(key) else { continue };
        let (vals, bad): (Vec<String>, Vec<&Value>) = match v {
            Value::Arr(a) => {
                let ok = |x: &&Value| matches!(x, Value::Str(s) if allowed.contains(&s.as_str()));
                (
                    a.iter().filter(ok).map(|x| show(x, false)).collect(),
                    a.iter().filter(|x| !ok(x)).collect(),
                )
            }
            _ => (vec![], vec![]),
        };
        if !bad.is_empty() || !matches!(v, Value::Arr(_)) {
            let shown = if bad.is_empty() {
                show(v, false)
            } else {
                format!(
                    "[{}]",
                    bad.iter()
                        .map(|x| show(x, true))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            problems.push(format!(
                "{where_}: `{key}` tem valores desconhecidos {shown}; ignorados"
            ));
        }
        match key {
            "levels" => out.levels = Some(vals),
            "formats" => out.formats = Some(vals),
            "print" => out.print = Some(vals),
            _ => out.languages = Some(vals),
        }
    }
    if let Some(f) = doc.get("folder") {
        match f {
            Value::Str(s)
                if !s.is_empty()
                    && !s.starts_with('/')
                    && !s.replace('\\', "/").split('/').any(|p| p == "..") =>
            {
                out.folder = Some(s.clone())
            }
            _ => problems.push(format!(
                "{where_}: `folder` precisa ser relativa à raiz, sem `..`; ignorada"
            )),
        }
    }
    out
}

fn list_or_null(v: &Option<Vec<String>>) -> String {
    v.as_deref().map_or("null".to_string(), json::list)
}

/// The additionalContext text, or `None` when the request is not for the plugin.
pub fn instruction(
    prompt: &str,
    skill_md: &str,
    to_pdf: &str,
    defaults_path: &str,
    home: &str,
    cwd: &str,
    reading_time: &str,
) -> Option<String> {
    if !is_trigger(prompt) {
        return None;
    }
    let levels = skill_levels(&read_text(skill_md).ok()?);
    if levels.is_empty() {
        return None;
    }
    // A Python path is run by the interpreter; a binary is run as it is.
    let rt = if reading_time.ends_with(".py") {
        format!("python3 -I {reading_time}")
    } else {
        reading_time.to_string()
    };
    let project = join(cwd, "ptbr-writer.toml");
    let user = join(
        &join(&join(home, ".config"), "ptbr-writer"),
        "ptbr-writer.toml",
    );
    let (mut problems, mut cfg, mut found) = (Vec::new(), Cfg::default(), Vec::<String>::new());
    for (path, where_) in [
        (defaults_path, "padrões"),
        (user.as_str(), user.as_str()),
        (project.as_str(), "./ptbr-writer.toml"),
    ] {
        let Some(doc) = load(path) else {
            problems.push(format!(
                "{where_}: não consegui ler; leia o arquivo você mesmo e siga-o"
            ));
            if path != defaults_path {
                found.push(where_.to_string());
            }
            continue;
        };
        if !doc.top.is_empty() && path != defaults_path {
            found.push(where_.to_string());
        }
        cfg.apply(clean(&doc, &levels, &mut problems, where_));
    }
    let folded = fold(prompt);
    let named: Vec<&str> = levels
        .iter()
        .map(|l| l.0.as_str())
        .filter(|n| has_word(&folded, &fold(n)))
        .collect();
    let ste = mentions_ste(prompt);

    let opts: Vec<&(String, String)> = levels
        .iter()
        .filter(|l| l.0 != COMPOSES)
        .take(MAX)
        .collect();
    let notes = levels.iter().any(|l| l.0 == COMPOSES);
    let mut fmts = vec![
        "`txt` (texto puro)".to_string(),
        "`md` (Markdown)".to_string(),
        format!("`pdf` (do .md, com `bash {to_pdf} <arquivo>.md`)"),
    ];
    if notes {
        fmts.push(format!(
            "`notes` (o nível {COMPOSES}: notas indexáveis, uma ideia por nota, com tags e links)"
        ));
    }
    let write = format!(
        "Grave cada versão em cada formato, na pasta configurada (crie se faltar), com o nome `{NAME}`: \
         o carimbo em UTC, o mesmo para todos os arquivos do pedido; o objetivo em 2 a 4 palavras, minúsculas sem acento, \
         unidas por hífen; o nível sem acento; a versão em ASD-STE100 leva `-ste100` antes da extensão{}\
         . Uma versão por nível em cada idioma de `languages`: `pt-BR` pelas regras da skill; `asd-ste100` em inglês técnico \
         simplificado (ASD-STE100: vocabulário controlado, voz ativa, frases curtas, uma instrução por frase), com o mesmo conteúdo, \
         sem inventar nada. Na resposta, cada versão sob `## <nível>` (ou `## <nível> · ASD-STE100`) e a lista dos arquivos gravados; o conteúdo dos formatos em `print` \
         aparece inteiro na resposta, num bloco ```text ou ```markdown, nunca só o caminho. Se o script do pdf sair com 3, \
         ele precisa baixar ferramentas: diga o tamanho que ele informa, pergunte se pode baixar e, com o sim, rode de novo \
         com `--download`; sem o sim, ou se falhar, entregue o .md e diga por quê. \
         Tempo de leitura: cada versão da resposta termina com a linha `Leitura estimada: ~N min (...)`, e cada `.txt` e `.md` começa com ela \
         (a versão ASD-STE100 usa `Estimated reading time: ...`); o `.pdf` herda a linha do `.md`; em `notes`, ponha `reading_time` \
         no front matter de cada nota. Calcule-a com `{rt} [--lang en] <arquivo>` (grave o arquivo sem a linha, \
         calcule e então insira a linha), nunca de cabeça.",
        if notes { format!("; com `notes`, as notas pelas regras de {COMPOSES} (`{COMPOSES} + <nível>`)") } else { String::new() }
    );
    let print = cfg.print.as_ref().map_or_else(
        || json::list(&["txt".to_string(), "md".to_string()]),
        |p| json::list(p),
    );
    let ask = [
        "Chame AskUserQuestion com quatro perguntas numa chamada só:".to_string(),
        format!(
            "1. header \"Nível\", multiSelect, \"Em quais níveis?\", opções: {}.",
            opts.iter().map(|(n, u)| format!("{n} ({u})")).collect::<Vec<_>>().join("; ")
        ),
        format!("2. header \"Formato\", multiSelect, \"Em quais formatos?\", opções: {}.", fmts.join("; ")),
        "3. header \"Pasta\", uma escolha, \"Onde gravar os arquivos?\", opções nesta ordem: raiz (Recommended), \
         a pasta da sessão, o padrão; docs/. Outra pasta vem pelo Other."
            .to_string(),
        "4. header \"Idioma\", multiSelect, \"Em quais idiomas?\", opções nesta ordem: pt-BR (Recommended), o padrão; \
         ASD-STE100 (inglês técnico simplificado). Sem marcação, pt-BR."
            .to_string(),
        format!(
            "Depois grave as respostas em ./ptbr-writer.toml, com `levels`, `formats`, `folder` (\".\" para a raiz), \
             `languages` (\"pt-BR\" e/ou \"asd-ste100\") e `print = {print}`, uma linha de comentário dizendo que o arquivo \
             pode ser editado ou refeito com `ptbr reconfigurar`, e diga isso à pessoa numa frase."
        ),
    ];
    let no_ask = "Sem AskUserQuestion (sessão não interativa): não pergunte nem grave arquivo; use o nível que a skill classificar, declare-o e responda sob `## <nível>`.";
    let mut lines: Vec<String> =
        vec!["ptbr-writer: um pedido de escrita com o plugin.".to_string()];
    if is_reconfig(prompt) {
        lines.push("A pessoa pediu para reconfigurar.".to_string());
        lines.extend(ask);
        lines.push("Não escreva texto agora, só a configuração.".to_string());
        lines.push(no_ask.to_string());
    } else if !found.is_empty() || !named.is_empty() || ste {
        let mut used = cfg.clone();
        if !named.is_empty() {
            used.levels = Some(
                named
                    .iter()
                    .filter(|n| **n != COMPOSES)
                    .map(|n| n.to_string())
                    .collect(),
            );
            let f = used.formats.get_or_insert_with(Vec::new);
            if named.contains(&COMPOSES) && !f.iter().any(|x| x == "notes") {
                f.push("notes".to_string());
            }
        }
        if ste
            && !used
                .languages
                .as_ref()
                .is_some_and(|l| l.iter().any(|x| x == "asd-ste100"))
        {
            let mut l = used
                .languages
                .take()
                .unwrap_or_else(|| vec!["pt-BR".to_string()]);
            l.push("asd-ste100".to_string());
            used.languages = Some(l);
        }
        let shown =
            format!(
            "{{\"levels\": {}, \"formats\": {}, \"folder\": {}, \"print\": {}, \"languages\": {}}}",
            list_or_null(&used.levels),
            list_or_null(&used.formats),
            used.folder.as_deref().map_or("null".to_string(), json::quote),
            list_or_null(&used.print),
            list_or_null(&used.languages)
        );
        lines.push(format!(
            "Configuração em uso ({}; o pedido vence): {shown}. Não pergunte nada disso.",
            if found.is_empty() {
                "padrões".to_string()
            } else {
                found.join(", ")
            }
        ));
        let tail = if used.formats.as_ref().is_none_or(Vec::is_empty) {
            format!(
                "Sem formatos: só a resposta, sem arquivo, mas cada versão termina com a linha `Leitura estimada: ~N min (...)` (`Estimated reading time: ...` em inglês), calculada com `{rt} [--lang en] -` lendo o texto da entrada padrão; com `asd-ste100`, a versão em inglês técnico simplificado sob `## <nível> · ASD-STE100`. "
            )
        } else {
            write
        };
        lines.push(format!(
            "Carregue a skill ptbr-writer e escreva uma versão por nível de `levels` (vazio: a skill classifica e declara o nível) em cada idioma de `languages`. {tail}"
        ));
    } else {
        lines
            .push("Não há ptbr-writer.toml ainda: esta é a única vez que se pergunta.".to_string());
        lines.extend(ask);
        lines.push(format!(
            "Então carregue a skill ptbr-writer e escreva uma versão por nível marcado. {write}"
        ));
        lines.push(no_ask.to_string());
    }
    lines.extend(
        problems
            .iter()
            .map(|p| format!("Aviso da configuração: {p}.")),
    );
    Some(format!(
        "{{\"hookSpecificOutput\": {{\"hookEventName\": \"UserPromptSubmit\", \"additionalContext\": {}}}}}",
        json::quote(&lines.join("\n"))
    ))
}

/// The self-test of ask_levels.py: (name, ok).
pub fn self_test_cases() -> Vec<(String, bool)> {
    use std::fs;
    let tmp = std::env::temp_dir().join(format!("ptbr-ask-levels-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    let s = |p: &std::path::Path| p.to_string_lossy().into_owned();
    let (home, cwd) = (tmp.join("home"), tmp.join("proj"));
    let user_dir = home.join(".config").join("ptbr-writer");
    fs::create_dir_all(&user_dir).expect("temp dir");
    fs::create_dir_all(&cwd).expect("temp dir");
    let skill = s(&tmp.join("SKILL.md"));
    fs::write(&skill, "# t\n\n## Níveis de rigor\n\n- `estrito`: runbooks.\n- `leve`: comunicados.\n- `científico`: artigos.\n- `smart-note`: notas.\n\n## Regras\n").expect("write");
    let defaults = s(&tmp.join("defaults.toml"));
    fs::write(&defaults, "levels = []\nformats = []\nfolder = \".\"\nprint = [\"txt\", \"md\"]\nlanguages = [\"pt-BR\"]\n").expect("write");
    let (home_s, cwd_s) = (s(&home), s(&cwd));
    let ask = |p: &str| -> Option<String> {
        let out = instruction(
            p,
            &skill,
            "/p/to-pdf.sh",
            &defaults,
            &home_s,
            &cwd_s,
            "/p/reading_time.py",
        )?;
        match json::parse(&out)?
            .get("hookSpecificOutput")?
            .get("additionalContext")?
        {
            J::Str(t) => Some(t.clone()),
            _ => None,
        }
    };
    let project = |t: &str| fs::write(cwd.join("ptbr-writer.toml"), t).expect("write");
    let mut checks: Vec<(String, bool)> = Vec::new();
    let mut check = |name: &str, ok: bool| checks.push((name.to_string(), ok));
    let has = |o: &Option<String>, subs: &[&str]| {
        o.as_ref()
            .is_some_and(|t| subs.iter().all(|x| t.contains(x)))
    };
    let lacks = |o: &Option<String>, sub: &str| o.as_ref().is_some_and(|t| !t.contains(sub));

    let out = ask("ptbr: reescreva este texto");
    check(
        "no config: asks once, four questions",
        has(&out, &["única vez", "quatro perguntas"]),
    );
    check(
        "no config: writes ./ptbr-writer.toml",
        has(&out, &["grave as respostas em ./ptbr-writer.toml"]),
    );
    check(
        "notes is a format, smart-note is not a level option",
        has(&out, &["`notes` (o nível smart-note"]) && lacks(&out, "smart-note (notas)"),
    );
    check(
        "pdf points at to-pdf",
        has(&out, &["bash /p/to-pdf.sh <arquivo>.md"]),
    );
    check(
        "folder question: raiz first, docs/ offered",
        has(&out, &["raiz (Recommended)", "docs/"]),
    );
    check(
        "language question: pt-BR default, ASD-STE100 offered, multiSelect",
        has(
            &out,
            &[
                "quatro perguntas",
                "header \"Idioma\", multiSelect",
                "pt-BR (Recommended)",
                "ASD-STE100 (inglês técnico simplificado)",
            ],
        ),
    );
    check("languages saved in the config", has(&out, &["`languages`"]));
    check(
        "reading time: line in the answer, .txt/.md start with it, pdf inherits, notes front matter",
        has(&out, &["termina com a linha `Leitura estimada", "começa com ela", "`.pdf` herda", "`reading_time` no front matter"]),
    );
    check(
        "reading time: the script's absolute path, English line for ASD-STE100",
        has(
            &out,
            &["python3 -I /p/reading_time.py", "Estimated reading time"],
        ),
    );
    check(
        "non-interactive fallback",
        has(&out, &["Sem AskUserQuestion"]),
    );
    for p in [
        "/ptbr revise isto",
        "/ptbr-writer:ptbr revise",
        "PTBR reescreva",
        "ptbr",
    ] {
        check(&format!("trigger: {}", py_repr(p)), ask(p).is_some());
    }
    for p in [
        "for ptbr-writer",
        "abra skills/ptbr/SKILL.md",
        "rode o ptbr-routing",
        "reescreva este runbook",
        "o plugin ptbr é bom",
    ] {
        check(&format!("no trigger: {}", py_repr(p)), ask(p).is_none());
    }
    check(
        "bad input is silent",
        instruction(
            "",
            &skill,
            "x",
            &defaults,
            &home_s,
            &cwd_s,
            "reading_time.py",
        )
        .is_none(),
    );

    project("levels = [\"estrito\"]\nformats = [\"md\", \"pdf\"]\nfolder = \"docs\"\n");
    let out = ask("ptbr: reescreva");
    check(
        "config: no question",
        lacks(&out, "AskUserQuestion com quatro") && has(&out, &["Não pergunte"]),
    );
    check(
        "config: values in use",
        has(&out, &["\"levels\": [\"estrito\"]", "\"folder\": \"docs\""]),
    );
    check(
        "config: pt-BR is the default language",
        has(&out, &["\"languages\": [\"pt-BR\"]"]),
    );
    let out = ask("ptbr: reescreva");
    check(
        "config: the reading-time instruction is there too",
        has(&out, &["python3 -I /p/reading_time.py", "começa com ela"]),
    );
    project("levels = [\"estrito\"]\nformats = []\n");
    let out = ask("ptbr: reescreva");
    check(
        "no files: the answer still ends with the reading-time line",
        has(
            &out,
            &["sem arquivo, mas cada versão termina com a linha `Leitura estimada"],
        ),
    );
    project("levels = [\"estrito\"]\nformats = [\"md\", \"pdf\"]\nfolder = \"docs\"\n");
    let out = ask("ptbr ste100: reescreva");
    check(
        "ste100 in the request adds ASD-STE100",
        has(
            &out,
            &["\"languages\": [\"pt-BR\", \"asd-ste100\"]", "-ste100"],
        ),
    );
    check("config: file name pattern", has(&out, &[NAME]));
    let out = ask("ptbr leve: reescreva");
    check(
        "the request overrides the config",
        has(&out, &["\"levels\": [\"leve\"]"]),
    );
    let out = ask("ptbr reconfigurar");
    check(
        "reconfigure asks again",
        has(&out, &["reconfigurar", "quatro perguntas"]),
    );

    fs::write(
        user_dir.join("ptbr-writer.toml"),
        "formats = [\"txt\"]\nfolder = \"notas\"\n",
    )
    .expect("write");
    project("folder = \"docs\"\n");
    let out = ask("ptbr: x");
    check(
        "project overrides user, user overrides defaults",
        has(&out, &["\"folder\": \"docs\"", "\"formats\": [\"txt\"]"]),
    );
    project("folder = \"../fora\"\nformats = [\"exe\"]\n");
    let out = ask("ptbr: x");
    check(
        "a folder outside the root is refused",
        has(&out, &["sem `..`", "\"folder\": \"notas\""]),
    );
    check(
        "an unknown format is dropped",
        has(&out, &["`formats` tem valores desconhecidos"]),
    );
    project("levels = [\n");
    let out = ask("ptbr: x");
    check("broken TOML is reported", has(&out, &["não consegui ler"]));
    let out = instruction(
        "ptbr",
        &skill,
        "x",
        &defaults,
        &home_s,
        &cwd_s,
        "reading_time.py",
    );
    check(
        "valid hook JSON",
        out.and_then(|o| json::parse(&o)).and_then(|j| {
            match j.get("hookSpecificOutput")?.get("hookEventName")? {
                J::Str(s) => Some(s == "UserPromptSubmit"),
                _ => None,
            }
        }) == Some(true),
    );
    let _ = fs::remove_dir_all(&tmp);
    checks
}

fn self_test() -> i32 {
    let checks = self_test_cases();
    let fails = checks.iter().filter(|c| !c.1).count();
    for (n, ok) in &checks {
        println!("{}{n}", if *ok { "  ok   " } else { "  FAIL " });
    }
    if fails == 0 {
        println!("ask-levels self-test: ok");
    } else {
        eprintln!("ask-levels self-test: {fails} failure(s)");
    }
    (fails > 0) as i32
}

pub fn main(argv: Vec<String>) -> i32 {
    if argv.len() == 2 && argv[1] == "--self-test" {
        return self_test();
    }
    let mut buf = Vec::new();
    // Python: an unreadable or non-JSON stdin is a silent success.
    let Some(input) = std::io::stdin()
        .read_to_end(&mut buf)
        .ok()
        .and_then(|_| String::from_utf8(buf).ok())
        .and_then(|s| json::parse(&s))
    else {
        return 0;
    };
    let prompt = match &input {
        J::Obj(_) => match input.get("prompt") {
            None => String::new(),
            Some(J::Str(s)) => s.clone(),
            Some(_) => return 1,
        },
        _ => return 1,
    };
    if argv.len() < 4 {
        eprintln!(
            "usage: ptbr-ask-levels SKILL.md TO_PDF DEFAULTS [READING_TIME] < hook-input.json"
        );
        return 1;
    }
    let home = std::env::var("HOME").map_or("~".to_string(), |h| {
        let t = h.trim_end_matches('/');
        if t.is_empty() {
            "/".to_string()
        } else {
            t.to_string()
        }
    });
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let rt = argv.get(4).map_or("reading_time.py", String::as_str);
    if let Some(out) = instruction(&prompt, &argv[1], &argv[2], &argv[3], &home, &cwd, rt) {
        println!("{out}");
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_test_cases_pass() {
        for (name, ok) in self_test_cases() {
            assert!(ok, "{name}");
        }
    }

    #[test]
    fn trigger_and_reconfig_edges() {
        for p in [
            "ptbr:x",
            "  /PTBR, x",
            "ptbr-writer",
            "ptbr-writer:y",
            "ptbr.",
        ] {
            assert!(is_trigger(p), "{p}");
        }
        for p in ["ptbrx", "ptbr-writerx", "x ptbr", "", "ptbr-"] {
            assert!(!is_trigger(p), "{p}");
        }
        assert!(is_reconfig("ptbr: Reconfigurar"));
        assert!(is_reconfig("/ptbr-writer configuração"));
        assert!(!is_reconfig("ptbr reescreva"));
    }

    #[test]
    fn ste_and_accent_fold() {
        for p in [
            "ptbr ste100",
            "ptbr ASD-STE-100 x",
            "asdste100",
            "(ste-100)",
        ] {
            assert!(mentions_ste(p), "{p}");
        }
        for p in ["xste100", "ste1000", "ste10"] {
            assert!(!mentions_ste(p), "{p}");
        }
        assert!(has_word(&fold("Científico"), "cientifico"));
        assert!(has_word(&fold("CIENTI\u{301}FICO"), "cientifico"));
    }
}
