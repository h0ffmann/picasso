//! to-pdf texto.md [saida.pdf]: Markdown to PDF for pt-BR text, Typst as a library.
use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use std::{fmt::Write as _, path::PathBuf, process::ExitCode};
use typst_as_lib::TypstEngine;
use typst_layout::PagedDocument;

/// Only the body and code faces ptbr-writter needs; ⚠ (U+26A0) comes from DejaVu Sans Mono.
const FONTS: [&[u8]; 6] = [
    include_bytes!("../fonts/LibertinusSerif-Regular.otf"),
    include_bytes!("../fonts/LibertinusSerif-Bold.otf"),
    include_bytes!("../fonts/LibertinusSerif-Italic.otf"),
    include_bytes!("../fonts/LibertinusSerif-BoldItalic.otf"),
    include_bytes!("../fonts/DejaVuSansMono.ttf"),
    include_bytes!("../fonts/DejaVuSansMono-Bold.ttf"),
];

const PREAMBLE: &str = r##"#set text(font: "Libertinus Serif", lang: "pt", region: "br")
#set page(margin: 2.5cm)
#set table(stroke: 0.5pt + luma(150), inset: 6pt)
#show link: set text(fill: rgb("#1a5fb4"))
#show raw: set text(font: "DejaVu Sans Mono")
#show raw.where(block: true): block.with(fill: luma(240), inset: 8pt, radius: 3pt, width: 100%)
"##;

/// Escapes every character that is markup in Typst, so any prose compiles as plain text.
/// Escaped everywhere, not only at line start: `\-` and `\=` render as the plain character.
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        let special = matches!(
            c,
            '\\' | '#'
                | '*'
                | '_'
                | '`'
                | '$'
                | '<'
                | '>'
                | '@'
                | '['
                | ']'
                | '='
                | '-'
                | '+'
                | '/'
                | '~'
                | '.'
                | '('
        ); // "." and "(": `#emph[x].y` and `#emph[x](y)` would continue the call
        if special {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Fence language to the name Typst's bundled syntaxes know. Unknown names pass through:
/// Typst then renders the block as plain monospace, it does not fail.
fn lang_alias(l: &str) -> String {
    let l = l
        .split(|c: char| c.is_whitespace() || c == ',' || c == '{')
        .next()
        .unwrap_or("")
        .to_lowercase();
    match l.as_str() {
        "sh" | "shell" | "console" | "zsh" | "shell-session" => "bash",
        "yml" => "yaml",
        "js" => "javascript",
        "ts" => "typescript",
        "py" => "python",
        "rs" => "rust",
        "golang" => "go",
        "c++" | "cxx" => "cpp",
        "md" => "markdown",
        other => return other.to_string(),
    }
    .to_string()
}

/// A Typst string literal.
fn lit(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

enum Kind {
    Root,
    Para,
    Heading(u8),
    Wrap(&'static str),
    Link(String),
    List(Option<u64>),
    Item,
    Table(Vec<Alignment>, usize),
    Cell,
    Code(Option<String>),
}

struct Frame {
    kind: Kind,
    buf: String,
    items: Vec<String>,
}

fn frame(kind: Kind) -> Frame {
    Frame {
        kind,
        buf: String::new(),
        items: Vec::new(),
    }
}

fn render(f: Frame) -> String {
    let Frame { kind, buf, items } = f;
    let args = |items: &[String]| {
        items
            .iter()
            .map(|i| format!("[{i}]"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    match kind {
        Kind::Root | Kind::Item | Kind::Cell => buf,
        Kind::Para => format!("{buf}\n\n"),
        Kind::Heading(n) => format!("{} {buf}\n\n", "=".repeat(n as usize)),
        Kind::Wrap("quote") => format!("#quote(block: true)[{buf}]\n\n"),
        Kind::Wrap(name) => format!("#{name}[{buf}]"),
        Kind::Link(url) => format!("#link({})[{buf}]", lit(&url)),
        Kind::List(None) => format!("#list({})\n\n", args(&items)),
        Kind::List(Some(start)) => format!("#enum(start: {start}, {})\n\n", args(&items)),
        Kind::Code(lang) => {
            let lang = lang
                .filter(|l| !l.is_empty())
                .map(|l| format!("lang: {}, ", lit(&l)))
                .unwrap_or_default();
            format!(
                "#raw(block: true, {lang}{})\n\n",
                lit(buf.trim_end_matches('\n'))
            )
        }
        Kind::Table(aligns, header) => {
            let al = aligns
                .iter()
                .map(|a| match a {
                    Alignment::Left => "left",
                    Alignment::Center => "center",
                    Alignment::Right => "right",
                    Alignment::None => "auto",
                })
                .collect::<Vec<_>>()
                .join(", ");
            let (head, body) = items.split_at(header.min(items.len()));
            format!(
                "#table(columns: {n}, align: ({al},), table.header({}), {})\n\n",
                args(head),
                args(body),
                n = aligns.len().max(1),
            )
        }
    }
}

/// Markdown to Typst markup (without the preamble).
pub fn to_typst(md: &str) -> String {
    let opts = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH;
    let mut st = vec![frame(Kind::Root)];
    for ev in Parser::new_ext(md, opts) {
        match ev {
            Event::Start(tag) => st.push(frame(match tag {
                Tag::Paragraph => Kind::Para,
                Tag::Heading { level, .. } => Kind::Heading(level as u8),
                Tag::BlockQuote(_) => Kind::Wrap("quote"),
                Tag::CodeBlock(CodeBlockKind::Fenced(l)) => Kind::Code(Some(lang_alias(&l))),
                Tag::CodeBlock(_) => Kind::Code(None),
                Tag::List(start) => Kind::List(start),
                Tag::Item => Kind::Item,
                Tag::Table(a) => Kind::Table(a, 0),
                Tag::TableHead | Tag::TableRow => continue, // cells go straight to the table
                Tag::TableCell => Kind::Cell,
                Tag::Emphasis => Kind::Wrap("emph"),
                Tag::Strong => Kind::Wrap("strong"),
                Tag::Strikethrough => Kind::Wrap("strike"),
                Tag::Link { dest_url, .. } => Kind::Link(dest_url.to_string()),
                // shortcut: images and other tags keep only their text, upgrade when ptbr-writter emits images
                _ => Kind::Wrap("box"),
            })),
            Event::End(TagEnd::TableHead) => {
                let t = st.last_mut().unwrap();
                let n = t.items.len();
                if let Kind::Table(_, h) = &mut t.kind {
                    *h = n;
                }
            }
            Event::End(TagEnd::TableRow) => {}
            Event::End(_) => {
                let f = st.pop().unwrap();
                let is_cell = matches!(f.kind, Kind::Cell | Kind::Item);
                let out = render(f);
                let parent = st.last_mut().unwrap();
                if is_cell {
                    parent.items.push(out);
                } else {
                    parent.buf.push_str(&out);
                }
            }
            Event::Text(t) => {
                let top = st.last_mut().unwrap();
                if matches!(top.kind, Kind::Code(_)) {
                    top.buf.push_str(&t);
                } else {
                    top.buf.push_str(&esc(&t));
                }
            }
            Event::Code(c) => {
                let _ = write!(st.last_mut().unwrap().buf, "#raw({})", lit(&c));
            }
            Event::SoftBreak => st.last_mut().unwrap().buf.push('\n'),
            Event::HardBreak => st.last_mut().unwrap().buf.push_str(" \\\n"),
            Event::Rule => st
                .last_mut()
                .unwrap()
                .buf
                .push_str("#line(length: 100%)\n\n"),
            Event::Html(h) | Event::InlineHtml(h) => st.last_mut().unwrap().buf.push_str(&esc(&h)),
            _ => {}
        }
    }
    render(st.pop().unwrap())
}

/// Markdown to PDF bytes. Fonts are embedded (typst-assets) and nothing touches the network.
pub fn to_pdf(md: &str) -> Result<Vec<u8>, String> {
    let src = format!("{PREAMBLE}\n{}", to_typst(md));
    let engine = TypstEngine::builder().main_file(src).fonts(FONTS).build();
    let doc: PagedDocument = engine.compile().output.map_err(|e| format!("typst: {e}"))?;
    typst_pdf::pdf(&doc, &Default::default()).map_err(|e| format!("pdf: {e:?}"))
}

fn run() -> Result<PathBuf, String> {
    let mut args = std::env::args_os().skip(1);
    let usage = "uso: to-pdf texto.md [saida.pdf]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    let out = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| input.with_extension("pdf"));
    if args.next().is_some() {
        return Err(usage.into());
    }
    let md = std::fs::read_to_string(&input).map_err(|e| format!("{}: {e}", input.display()))?;
    let pdf = to_pdf(&md)?;
    std::fs::write(&out, pdf).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(out)
}

fn main() -> ExitCode {
    match run() {
        Ok(out) => {
            println!("{}", out.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("to-pdf: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_each_special_char() {
        for c in "\\#*_`$<>@[]=-+/~.(".chars() {
            assert_eq!(esc(&format!("a{c}b")), format!("a\\{c}b"), "{c}");
        }
        assert_eq!(esc("1. x"), "1\\. x");
        assert_eq!(esc("ação"), "ação");
    }

    #[test]
    fn heading_and_paragraph() {
        assert_eq!(
            to_typst("## Título\n\nTexto *a* **b** ~~c~~"),
            "== Título\n\nTexto #emph[a] #strong[b] #strike[c]\n\n"
        );
    }

    #[test]
    fn code_inline_and_block() {
        assert_eq!(to_typst("`a\"b`"), "#raw(\"a\\\"b\")\n\n");
        assert_eq!(
            to_typst("```sh\nls\n```"),
            "#raw(block: true, lang: \"bash\", \"ls\")\n\n"
        );
    }

    #[test]
    fn lists_nested() {
        assert_eq!(
            to_typst("- a\n  - b\n- c"),
            "#list([a#list([b])\n\n], [c])\n\n"
        );
        assert_eq!(to_typst("3. a\n4. b"), "#enum(start: 3, [a], [b])\n\n");
    }

    #[test]
    fn table() {
        let t = to_typst("| A | B |\n|:--|--:|\n| 1 | 2 |");
        assert_eq!(
            t,
            "#table(columns: 2, align: (left, right,), table.header([A], [B]), [1], [2])\n\n"
        );
    }

    #[test]
    fn link_quote_rule() {
        assert_eq!(
            to_typst("[x](https://a.b/c)"),
            "#link(\"https://a.b/c\")[x]\n\n"
        );
        assert_eq!(to_typst("> q"), "#quote(block: true)[q\n\n]\n\n");
        assert_eq!(to_typst("---"), "#line(length: 100%)\n\n");
    }

    #[test]
    fn hostile_prose_compiles() {
        let md =
            "# a_b *c* #d $e <f> @g [h] = - + / // /* \\ ~ 1. x\n\n**n**.x e *i*(j) e **k**[l]\n";
        assert!(to_pdf(md).unwrap().starts_with(b"%PDF"));
    }

    #[test]
    fn lang_kept_and_aliased() {
        for (from, to) in [
            ("sh", "bash"),
            ("console", "bash"),
            ("zsh", "bash"),
            ("yml", "yaml"),
            ("js", "javascript"),
            ("ts", "typescript"),
            ("py", "python"),
            ("rs", "rust"),
            ("golang", "go"),
            ("c++", "cpp"),
            ("cxx", "cpp"),
            ("md", "markdown"),
            ("SQL", "sql"),
            ("scala", "scala"),
        ] {
            assert_eq!(
                to_typst(&format!("```{from}\nx\n```")),
                format!("#raw(block: true, lang: \"{to}\", \"x\")\n\n")
            );
        }
    }

    #[test]
    fn snapshots() {
        use insta::assert_snapshot as snap;
        snap!(
            "accents",
            to_typst("# Ação e coração\n\nÀ vista, pão, é, ü — “aspas”.")
        );
        snap!("warning", to_typst("Reinicie o serviço ⚠ agora."));
        snap!(
            "escaping",
            to_typst("a # b $ c @ d * e _ f [ g ] h < i > j")
        );
        snap!(
            "table",
            to_typst("| Passo | Prazo |\n|:--|--:|\n| ler | 2 min |")
        );
        for (lang, src) in [
            ("json", "{\"a\": 1}"),
            ("python", "print(\"#x\")"),
            ("sql", "SELECT 1;"),
            ("rust", "fn main() {}"),
            ("scala", "object A"),
        ] {
            insta::with_settings!({snapshot_suffix => lang}, {
                snap!("code", to_typst(&format!("```{lang}\n{src}\n```")));
            });
        }
        snap!(
            "nested",
            to_typst("1. a\n   - b\n   - c\n2. d\n\n> q [x](https://a.b)")
        );
    }

    #[test]
    fn sample_end_to_end() {
        let pdf = to_pdf(include_str!("../example/smoke.md")).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
    }

    /// A fence is highlighted when its PDF differs from the same code under an unknown language.
    #[test]
    fn required_languages_are_highlighted() {
        let code = [
            ("json", "{\"a\": [1, true, null]}"),
            ("python", "def f(x):\n    return \"a\" + str(x)  # c"),
            ("sql", "SELECT a, COUNT(*) FROM t WHERE b = 'x' GROUP BY a;"),
            ("rust", "fn main() { let x: u8 = 1; println!(\"{x}\"); }"),
            ("scala", "object A { def f(x: Int): Int = x + 1 }"),
        ];
        for (lang, src) in code {
            let hi = to_pdf(&format!("```{lang}\n{src}\n```")).unwrap();
            let plain = to_pdf(&format!("```nao-existe\n{src}\n```")).unwrap();
            assert_ne!(hi, plain, "{lang} is not highlighted");
        }
    }
}
