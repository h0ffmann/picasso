//! ptbr-reading-time: an honest, deterministic estimate of how long a person takes to read a text.
//! A 1:1 port of reading_time.py; the regular expressions are written out by hand.
//!
//! ```text
//! ptbr-reading-time [--lang pt-BR|en] [--json] [FILE]     (stdin when no FILE)
//! ptbr-reading-time --self-test
//! ```
//!
//! Formula (seconds, then rounded UP to whole minutes; 60 s or less prints "< 1 min"):
//! `seconds = words * complexity / 238 * 60 + code_lines * 10 + links * 5`
//! 238 words per minute is Brysbaert (2019), silent reading of English non-fiction, DOI
//! 10.1016/j.jml.2019.104047; pt-BR uses the same rate, unscaled (unverified).

use crate::text::{is_word, os_error, py_space, splitlines, universal_newlines};
use std::io::Read;

const WPM: f64 = 238.0;
const SECONDS_PER_CODE_LINE: f64 = 10.0;
const SECONDS_PER_LINK: f64 = 5.0;
const LONG_WORD: usize = 10;

pub struct Report {
    pub minutes: u64,
    pub seconds: i64,
    pub words: usize,
    pub links: usize,
    pub code_blocks: usize,
    pub code_lines: usize,
    pub complexity: f64,
    pub mean_sentence_words: f64,
    pub long_word_share: f64,
}

type Chars = Vec<char>;

fn first(c: &[char], from: usize, x: char) -> Option<usize> {
    c[from..].iter().position(|&y| y == x).map(|p| p + from)
}

/// `re.sub(r"\A---\n.*?\n---\n", "", text, flags=re.S)`
fn front_matter(t: &str) -> &str {
    match t
        .strip_prefix("---\n")
        .and_then(|r| r.find("\n---\n").map(|p| &r[p + 5..]))
    {
        Some(rest) => rest,
        None => t,
    }
}

/// `LINE.sub("", text)`: the reading-time line itself, one per line start.
fn drop_line(t: &str) -> String {
    t.split('\n')
        .map(|l| {
            if ["Leitura estimada:", "Estimated reading time:"]
                .iter()
                .any(|p| l.starts_with(p))
            {
                ""
            } else {
                l
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Fenced blocks: `^ {0,3}(```|~~~).*?^ {0,3}\1[ \t]*$` (multiline, dotall). Returns the text
/// without the blocks and the blocks.
fn fences(t: &str) -> (String, Vec<String>) {
    fn marker(l: &str) -> Option<&'static str> {
        let r = l.trim_start_matches(' ');
        if l.len() - r.len() > 3 {
            return None;
        }
        if r.starts_with("```") {
            Some("```")
        } else if r.starts_with("~~~") {
            Some("~~~")
        } else {
            None
        }
    }
    fn closer(l: &str, m: &str) -> bool {
        let r = l.trim_start_matches(' ');
        l.len() - r.len() <= 3
            && r.strip_prefix(m)
                .is_some_and(|x| x.chars().all(|c| c == ' ' || c == '\t'))
    }
    let lines: Vec<&str> = t.split('\n').collect();
    let (mut out, mut blocks, mut i) = (Vec::new(), Vec::new(), 0);
    while i < lines.len() {
        let end =
            marker(lines[i]).and_then(|m| (i + 1..lines.len()).find(|&j| closer(lines[j], m)));
        if let Some(j) = end {
            blocks.push(lines[i..=j].join("\n"));
            // the match leaves the line break that follows it: an empty piece in place of the block
            out.push("");
            i = j + 1;
        } else {
            out.push(lines[i]);
            i += 1;
        }
    }
    (out.join("\n"), blocks)
}

/// `(?<!!)\[[^\]]*\]\([^)\s]+[^)]*\)` or `<https?://[^>]+>`: the number of non-overlapping matches.
fn count_links(c: &[char]) -> usize {
    let (mut n, mut i) = (0, 0);
    while i < c.len() {
        let mut end = None;
        if c[i] == '[' && (i == 0 || c[i - 1] != '!') {
            if let Some(j) = first(c, i + 1, ']').filter(|&j| c.get(j + 1) == Some(&'(')) {
                if let Some(m) = first(c, j + 2, ')') {
                    if m > j + 2 && !py_space(c[j + 2]) {
                        end = Some(m + 1);
                    }
                }
            }
        } else if c[i] == '<' {
            end = angle_url(c, i);
        }
        match end {
            Some(e) => {
                n += 1;
                i = e;
            }
            None => i += 1,
        }
    }
    n
}

/// `<https?://[^>]+>` at i: the end of the match.
fn angle_url(c: &[char], i: usize) -> Option<usize> {
    let rest = &c[i + 1..];
    let skip = if rest.starts_with(&['h', 't', 't', 'p', 's', ':', '/', '/']) {
        8
    } else if rest.starts_with(&['h', 't', 't', 'p', ':', '/', '/']) {
        7
    } else {
        return None;
    };
    let body = i + 1 + skip;
    first(c, body, '>').filter(|&g| g > body).map(|g| g + 1)
}

/// `re.sub(r"<https?://[^>]+>", "", text)`
fn drop_angle_urls(c: &[char]) -> Chars {
    let (mut o, mut i) = (Vec::new(), 0);
    while i < c.len() {
        if c[i] == '<' {
            if let Some(e) = angle_url(c, i) {
                i = e;
                continue;
            }
        }
        o.push(c[i]);
        i += 1;
    }
    o
}

/// `re.sub(r"!\[[^\]]*\]\([^)]*\)", " ", t)` (image) and `re.sub(r"\[([^\]]*)\]\([^)]*\)", r"\1", t)`
/// (link: keep its text).
fn markdown_links(c: &[char], image: bool) -> Chars {
    let (mut o, mut i) = (Vec::new(), 0);
    while i < c.len() {
        let at = if image {
            c[i] == '!' && c.get(i + 1) == Some(&'[')
        } else {
            c[i] == '['
        };
        if at {
            let open = i + if image { 2 } else { 1 };
            if let Some(j) = first(c, open, ']').filter(|&j| c.get(j + 1) == Some(&'(')) {
                if let Some(m) = first(c, j + 2, ')') {
                    if image {
                        o.push(' ');
                    } else {
                        o.extend_from_slice(&c[open..j]);
                    }
                    i = m + 1;
                    continue;
                }
            }
        }
        o.push(c[i]);
        i += 1;
    }
    o
}

/// `URL.sub(" ", t)` with `https?://[^\s)>\]]+|www\.[^\s)>\]]+`; returns the text and the count.
fn urls(c: &[char]) -> (Chars, usize) {
    let (mut o, mut n, mut i) = (Vec::new(), 0, 0);
    let stop = |x: char| py_space(x) || x == ')' || x == '>' || x == ']';
    while i < c.len() {
        let rest = &c[i..];
        let skip = if rest.starts_with(&['h', 't', 't', 'p', 's', ':', '/', '/']) {
            8
        } else if rest.starts_with(&['h', 't', 't', 'p', ':', '/', '/']) {
            7
        } else if rest.starts_with(&['w', 'w', 'w', '.']) {
            4
        } else {
            0
        };
        if skip > 0 && rest.get(skip).is_some_and(|&x| !stop(x)) {
            let mut j = i + skip;
            while j < c.len() && !stop(c[j]) {
                j += 1;
            }
            o.push(' ');
            n += 1;
            i = j;
        } else {
            o.push(c[i]);
            i += 1;
        }
    }
    (o, n)
}

fn is_class(x: char) -> bool {
    matches!(x, '-' | ':' | '|' | ' ')
}

/// One match of `^\s*(\|?[-:| ]+\|?)\s*$` (multiline) at the line start p, with the engine's
/// backtracking order; returns where it ends.
fn rule_at(t: &[char], p: usize) -> Option<usize> {
    let n = t.len();
    let mut q = p;
    while q < n && py_space(t[q]) {
        q += 1;
    }
    for s1 in (p..=q).rev() {
        for lead in [true, false] {
            let mut g = s1;
            if lead {
                if t.get(g) == Some(&'|') {
                    g += 1;
                } else {
                    continue;
                }
            }
            let mut r = g;
            while r < n && is_class(t[r]) {
                r += 1;
            }
            for c_end in (g + 1..=r).rev() {
                for trail in [true, false] {
                    let mut h = c_end;
                    if trail {
                        if t.get(h) == Some(&'|') {
                            h += 1;
                        } else {
                            continue;
                        }
                    }
                    let mut z = h;
                    while z < n && py_space(t[z]) {
                        z += 1;
                    }
                    if let Some(e) = (h..=z).rev().find(|&e| e == n || t[e] == '\n') {
                        return Some(e);
                    }
                }
            }
        }
    }
    None
}

fn drop_rules(t: &[char]) -> Chars {
    let (mut o, mut i) = (Vec::new(), 0);
    while i < t.len() {
        if i == 0 || t[i - 1] == '\n' {
            if let Some(e) = rule_at(t, i) {
                o.push(' ');
                i = e;
                continue;
            }
        }
        o.push(t[i]);
        i += 1;
    }
    o
}

/// `WORD.findall`: `[^\W_][\w'’-]*`
fn words(t: &[char]) -> Vec<&[char]> {
    let (mut v, mut i) = (Vec::new(), 0);
    while i < t.len() {
        if t[i].is_alphanumeric() {
            let st = i;
            i += 1;
            while i < t.len() && (is_word(t[i]) || matches!(t[i], '\'' | '’' | '-')) {
                i += 1;
            }
            v.push(&t[st..i]);
        } else {
            i += 1;
        }
    }
    v
}

/// The end of a match of `[.!?]+(?:\s|$)|\n\s*\n|\n\s*(?:[-*]|\d+\.)\s` at i.
fn sentence_break(t: &[char], i: usize) -> Option<usize> {
    let n = t.len();
    if matches!(t[i], '.' | '!' | '?') {
        let mut j = i;
        while j < n && matches!(t[j], '.' | '!' | '?') {
            j += 1;
        }
        if j == n {
            return Some(j);
        }
        if py_space(t[j]) {
            return Some(j + 1);
        }
    }
    if t[i] == '\n' {
        let mut q = i + 1;
        while q < n && py_space(t[q]) {
            q += 1;
        }
        if let Some(k) = (i + 1..q).rev().find(|&k| t[k] == '\n') {
            return Some(k + 1);
        }
        let mut k = q;
        if k < n && matches!(t[k], '-' | '*') {
            k += 1;
        } else {
            let d = k;
            while k < n && t[k].is_ascii_digit() {
                k += 1;
            }
            if k == d || t.get(k) != Some(&'.') {
                return None;
            }
            k += 1;
        }
        if k < n && py_space(t[k]) {
            return Some(k + 1);
        }
    }
    None
}

fn sentences(t: &[char]) -> usize {
    let (mut count, mut last, mut i) = (0, 0, 0);
    let has_word = |p: &[char]| p.iter().any(|c| c.is_alphanumeric());
    while i < t.len() {
        match sentence_break(t, i) {
            Some(e) if e > i => {
                count += has_word(&t[last..i]) as usize;
                last = e;
                i = e;
            }
            _ => i += 1,
        }
    }
    count + has_word(&t[last..]) as usize
}

/// Python's `round(x, nd)` for a float, printed back as `repr` does.
fn round_to(x: f64, nd: usize) -> f64 {
    format!("{x:.nd$}").parse().unwrap_or(x)
}

pub fn analyse(text: &str) -> Report {
    let text = text.replace("\r\n", "\n");
    let text = drop_line(front_matter(&text));
    let (text, blocks) = fences(&text);
    let code_lines = blocks
        .iter()
        .map(|b| {
            let l = splitlines(b);
            l.get(1..l.len().saturating_sub(1))
                .unwrap_or(&[])
                .iter()
                .filter(|x| !x.trim_matches(py_space).is_empty())
                .count()
        })
        .sum::<usize>();
    let t: Chars = text.chars().collect();
    let mut links = count_links(&t);
    let t = markdown_links(&markdown_links(&t, true), false);
    links += urls(&drop_angle_urls(&t)).1;
    let (t, _) = urls(&t);
    let t = drop_rules(&t);
    let w = words(&t);
    let sent = sentences(&t);
    let mean_len = if sent > 0 {
        w.len() as f64 / sent as f64
    } else {
        0.0
    };
    let long_share = if w.is_empty() {
        0.0
    } else {
        w.iter()
            .filter(|x| x.iter().filter(|c| c.is_alphabetic()).count() >= LONG_WORD)
            .count() as f64
            / w.len() as f64
    };
    let complexity =
        1.0 + if mean_len > 20.0 { 0.15 } else { 0.0 } + if long_share > 0.25 { 0.15 } else { 0.0 };
    let seconds = w.len() as f64 * complexity / WPM * 60.0
        + code_lines as f64 * SECONDS_PER_CODE_LINE
        + links as f64 * SECONDS_PER_LINK;
    Report {
        minutes: if seconds <= 60.0 {
            0
        } else {
            (seconds / 60.0).ceil() as u64
        },
        seconds: seconds.round_ties_even() as i64,
        words: w.len(),
        links,
        code_blocks: blocks.len(),
        code_lines,
        complexity,
        mean_sentence_words: round_to(mean_len, 1),
        long_word_share: round_to(long_share, 2),
    }
}

/// The words of the line, per language: label, "< 1 min", word(s), link(s), code block(s).
fn labels(lang: &str) -> (&'static str, [&'static str; 6]) {
    if lang == "en" {
        (
            "Estimated reading time",
            [
                "word",
                "words",
                "link",
                "links",
                "code block",
                "code blocks",
            ],
        )
    } else {
        (
            "Leitura estimada",
            [
                "palavra",
                "palavras",
                "link",
                "links",
                "bloco de código",
                "blocos de código",
            ],
        )
    }
}

pub fn line(r: &Report, lang: &str) -> String {
    let (label, [w1, wn, l1, ln, b1, bn]) = labels(lang);
    let n = |k: usize, one: &str, many: &str| format!("{k} {}", if k == 1 { one } else { many });
    let mut parts = vec![n(r.words, w1, wn), n(r.links, l1, ln)];
    if r.code_blocks > 0 {
        parts.push(n(r.code_blocks, b1, bn));
    }
    let t = if r.minutes > 0 {
        format!("~{} min", r.minutes)
    } else {
        "< 1 min".to_string()
    };
    format!("{label}: {t} ({})", parts.join(", "))
}

fn json(r: &Report, lang: &str) -> String {
    format!(
        "{{\"minutes\": {}, \"seconds\": {}, \"words\": {}, \"links\": {}, \"code_blocks\": {}, \"code_lines\": {}, \
         \"complexity\": {:?}, \"mean_sentence_words\": {}, \"long_word_share\": {}, \"line\": {}}}",
        r.minutes,
        r.seconds,
        r.words,
        r.links,
        r.code_blocks,
        r.code_lines,
        r.complexity,
        // Python's `round(0, n)` of the int 0 (no words) prints `0`, not `0.0`
        if r.words == 0 { "0".to_string() } else { format!("{:?}", r.mean_sentence_words) },
        if r.words == 0 { "0".to_string() } else { format!("{:?}", r.long_word_share) },
        crate::json::quote(&line(r, lang))
    )
}

/// The self-test cases of reading_time.py: (name, ok, detail).
pub fn self_test_cases() -> Vec<(String, bool, String)> {
    let long = ("responsabilidades ".repeat(99) + "agora. ").repeat(10);
    let cases: Vec<(&str, String, &str, &str)> = vec![
        (
            "empty",
            String::new(),
            "pt-BR",
            "Leitura estimada: < 1 min (0 palavras, 0 links)",
        ),
        (
            "short",
            "Reinicie o serviço.".into(),
            "pt-BR",
            "Leitura estimada: < 1 min (3 palavras, 0 links)",
        ),
        (
            "short en",
            "Restart the service.".into(),
            "en",
            "Estimated reading time: < 1 min (3 words, 0 links)",
        ),
        (
            "long: 2400 words = 10.1 min -> 11",
            "Reinicie o serviço agora. ".repeat(600),
            "pt-BR",
            "Leitura estimada: ~11 min (2400 palavras, 0 links)",
        ),
        (
            "links cost 5 s, url and target not read",
            "Veja [a doc](https://example.com/muito/longa) e https://x.org agora. ".to_string()
                + &"a ".repeat(300),
            "pt-BR",
            "Leitura estimada: ~2 min (305 palavras, 2 links)",
        ),
        (
            "code: 29 lines = 290 s -> 5 min",
            "Rode:\n\n```bash\n".to_string() + &"make test\n".repeat(29) + "```\n",
            "pt-BR",
            "Leitura estimada: ~5 min (1 palavra, 0 links, 1 bloco de código)",
        ),
        (
            "code blocks (en)",
            "Run:\n\n```\na\n```\n\n```\nb\n```\n".into(),
            "en",
            "Estimated reading time: < 1 min (1 word, 0 links, 2 code blocks)",
        ),
        (
            "inline code is words; the line itself is ignored",
            "Leitura estimada: ~9 min (1 palavra, 0 links)\n\nUse `git commit` hoje.".into(),
            "pt-BR",
            "Leitura estimada: < 1 min (4 palavras, 0 links)",
        ),
        (
            "long sentences and long words: x1.3, 6 min instead of 5",
            long,
            "pt-BR",
            "Leitura estimada: ~6 min (1000 palavras, 0 links)",
        ),
    ];
    let mut out: Vec<_> = cases
        .into_iter()
        .map(|(name, text, lang, want)| {
            let got = line(&analyse(&text), lang);
            (
                name.to_string(),
                got == want,
                format!("got {got:?}, want {want:?}: "),
            )
        })
        .collect();
    let r = analyse("a [b](http://x.y) c");
    out.push((
        "json parts".into(),
        r.links == 1 && r.words == 3,
        String::new(),
    ));
    out
}

fn self_test() -> i32 {
    let mut bad = 0;
    for (name, ok, detail) in self_test_cases() {
        bad += !ok as i32;
        println!(
            "{}{name}",
            if ok {
                "  ok   ".to_string()
            } else {
                format!("  FAIL {detail}")
            }
        );
    }
    println!(
        "reading-time self-test: {}",
        if bad == 0 {
            "ok".to_string()
        } else {
            format!("{bad} failure(s)")
        }
    );
    (bad > 0) as i32
}

const USAGE: &str = "    ptbr-reading-time [--lang pt-BR|en] [--json] [FILE]     (stdin when no FILE)\n    ptbr-reading-time --self-test";

pub fn main(argv: Vec<String>) -> i32 {
    let mut args: Vec<String> = argv.into_iter().skip(1).collect();
    if args == ["--self-test"] {
        return self_test();
    }
    let (mut lang, mut as_json, mut path) = ("pt-BR".to_string(), false, None);
    args.reverse();
    while let Some(a) = args.pop() {
        if a == "--json" {
            as_json = true;
        } else if a == "--lang" && args.last().is_some_and(|l| l == "pt-BR" || l == "en") {
            lang = args.pop().unwrap_or_default();
        } else if (a == "-" || !a.starts_with('-')) && path.is_none() {
            path = Some(a);
        } else {
            eprintln!("{USAGE}");
            return 2;
        }
    }
    // A file is read in Python's text mode (universal newlines); stdin on POSIX is not translated.
    let text = match path.as_deref() {
        Some(p) if p != "-" => std::fs::read(p)
            .map(|b| (b, true))
            .map_err(|e| os_error(&e, p)),
        _ => {
            let mut b = Vec::new();
            std::io::stdin()
                .read_to_end(&mut b)
                .map(|_| (b, false))
                .map_err(|e| e.to_string())
        }
    };
    let text = match text.map(|(b, file)| {
        String::from_utf8(b).map(|t| if file { universal_newlines(&t) } else { t })
    }) {
        Ok(Ok(t)) => t,
        Ok(Err(e)) => {
            eprintln!("reading_time: {e}");
            return 1;
        }
        Err(e) => {
            eprintln!("reading_time: {e}");
            return 1;
        }
    };
    let r = analyse(&text);
    println!(
        "{}",
        if as_json {
            json(&r, &lang)
        } else {
            line(&r, &lang)
        }
    );
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_test_cases_pass() {
        for (name, ok, detail) in self_test_cases() {
            assert!(ok, "{detail}{name}");
        }
    }

    #[test]
    fn front_matter_and_rules_are_not_words() {
        let r = analyse("---\ntitle: x y z\n---\nUm dois.\n\n|---|---|\n\n---\n\ntrês");
        assert_eq!(r.words, 3);
    }

    #[test]
    fn tilde_fences_and_unclosed_fences() {
        assert_eq!(analyse("a\n~~~\nx\ny\n~~~\nb").code_lines, 2);
        assert_eq!(analyse("a\n```\nx\nb").code_blocks, 0);
    }
}
