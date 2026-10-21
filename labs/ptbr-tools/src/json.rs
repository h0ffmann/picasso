//! Minimal JSON: a strict parser for the hook input (what Python's `json.load` accepts) and an
//! escaper that writes what `json.dumps(..., ensure_ascii=False)` writes.

pub enum J {
    Null,
    Bool(bool),
    Num,
    Str(String),
    Arr,
    Obj(Vec<(String, J)>),
}

impl J {
    /// The last value of a key, as `dict` keeps it.
    pub fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(v) => v.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
}

pub fn parse(s: &str) -> Option<J> {
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    let v = value(&c, &mut i)?;
    ws(&c, &mut i);
    (i == c.len()).then_some(v)
}

fn ws(c: &[char], i: &mut usize) {
    while matches!(c.get(*i), Some(' ' | '\t' | '\n' | '\r')) {
        *i += 1;
    }
}

fn lit(c: &[char], i: &mut usize, word: &str) -> bool {
    let w: Vec<char> = word.chars().collect();
    if c[*i..].starts_with(&w) {
        *i += w.len();
        true
    } else {
        false
    }
}

fn value(c: &[char], i: &mut usize) -> Option<J> {
    ws(c, i);
    match *c.get(*i)? {
        '"' => string(c, i).map(J::Str),
        '{' => {
            *i += 1;
            let mut o = Vec::new();
            ws(c, i);
            if c.get(*i) == Some(&'}') {
                *i += 1;
                return Some(J::Obj(o));
            }
            loop {
                ws(c, i);
                if c.get(*i) != Some(&'"') {
                    return None;
                }
                let k = string(c, i)?;
                ws(c, i);
                if c.get(*i) != Some(&':') {
                    return None;
                }
                *i += 1;
                o.push((k, value(c, i)?));
                ws(c, i);
                match c.get(*i)? {
                    ',' => *i += 1,
                    '}' => {
                        *i += 1;
                        return Some(J::Obj(o));
                    }
                    _ => return None,
                }
            }
        }
        '[' => {
            *i += 1;
            ws(c, i);
            if c.get(*i) == Some(&']') {
                *i += 1;
                return Some(J::Arr);
            }
            loop {
                value(c, i)?;
                ws(c, i);
                match c.get(*i)? {
                    ',' => *i += 1,
                    ']' => {
                        *i += 1;
                        return Some(J::Arr);
                    }
                    _ => return None,
                }
            }
        }
        'n' => lit(c, i, "null").then_some(J::Null),
        't' => lit(c, i, "true").then_some(J::Bool(true)),
        'f' => lit(c, i, "false").then_some(J::Bool(false)),
        'N' => lit(c, i, "NaN").then_some(J::Num),
        'I' => lit(c, i, "Infinity").then_some(J::Num),
        '-' | '0'..='9' => {
            if lit(c, i, "-Infinity") {
                return Some(J::Num);
            }
            number(c, i).then_some(J::Num)
        }
        _ => None,
    }
}

fn digits(c: &[char], i: &mut usize) -> bool {
    let s = *i;
    while c.get(*i).is_some_and(char::is_ascii_digit) {
        *i += 1;
    }
    *i > s
}

fn number(c: &[char], i: &mut usize) -> bool {
    if c.get(*i) == Some(&'-') {
        *i += 1;
    }
    match c.get(*i) {
        Some('0') => *i += 1,
        Some('1'..='9') => {
            digits(c, i);
        }
        _ => return false,
    }
    // A bad fraction or exponent ends the number there; the caller then fails on the leftover.
    if c.get(*i) == Some(&'.') && c.get(*i + 1).is_some_and(char::is_ascii_digit) {
        *i += 1;
        digits(c, i);
    }
    if matches!(c.get(*i), Some('e' | 'E')) {
        let mut j = *i + 1;
        if matches!(c.get(j), Some('+' | '-')) {
            j += 1;
        }
        if digits(c, &mut j) {
            *i = j;
        }
    }
    true
}

fn hex4(c: &[char], i: usize) -> Option<u32> {
    let h: String = c.get(i..i + 4)?.iter().collect();
    if h.chars().all(|d| d.is_ascii_hexdigit()) {
        u32::from_str_radix(&h, 16).ok()
    } else {
        None
    }
}

fn string(c: &[char], i: &mut usize) -> Option<String> {
    *i += 1;
    let mut o = String::new();
    loop {
        let ch = *c.get(*i)?;
        *i += 1;
        match ch {
            '"' => return Some(o),
            '\\' => {
                let e = *c.get(*i)?;
                *i += 1;
                match e {
                    '"' | '\\' | '/' => o.push(e),
                    'b' => o.push('\x08'),
                    'f' => o.push('\x0c'),
                    'n' => o.push('\n'),
                    'r' => o.push('\r'),
                    't' => o.push('\t'),
                    'u' => {
                        let mut u = hex4(c, *i)?;
                        *i += 4;
                        if (0xd800..0xdc00).contains(&u)
                            && c.get(*i) == Some(&'\\')
                            && c.get(*i + 1) == Some(&'u')
                        {
                            if let Some(lo) =
                                hex4(c, *i + 2).filter(|l| (0xdc00..0xe000).contains(l))
                            {
                                u = 0x10000 + ((u - 0xd800) << 10) + (lo - 0xdc00);
                                *i += 6;
                            }
                        }
                        // shortcut: a lone surrogate (Python keeps it) becomes U+FFFD.
                        o.push(char::from_u32(u).unwrap_or('\u{fffd}'));
                    }
                    _ => return None,
                }
            }
            c if (c as u32) < 0x20 => return None,
            c => o.push(c),
        }
    }
}

/// `json.dumps(s, ensure_ascii=False)` for a string.
pub fn quote(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            '\x08' => o.push_str("\\b"),
            '\x0c' => o.push_str("\\f"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// `json.dumps(list_of_str)`, the default separators.
pub fn list(v: &[String]) -> String {
    format!(
        "[{}]",
        v.iter().map(|s| quote(s)).collect::<Vec<_>>().join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_prompt_with_escapes() {
        let j = parse(r#"{"a": [1, 2.5e3, null], "prompt": "ptbr \"x\"\nç 😀"}"#).unwrap();
        match j.get("prompt") {
            Some(J::Str(s)) => assert_eq!(s, "ptbr \"x\"\nç 😀"),
            _ => panic!(),
        }
    }

    #[test]
    fn rejects_what_python_rejects() {
        for bad in [
            "",
            "{",
            "{\"a\":1,}",
            "[1,]",
            "01",
            "{\"a\":\"\n\"}",
            "'x'",
            "{} x",
            "\u{feff}{}",
            "-",
            "1.",
        ] {
            assert!(parse(bad).is_none(), "{bad:?}");
        }
        for ok in ["NaN", "-Infinity", "1e5", " [] ", "{\"a\":{}}", "-0"] {
            assert!(parse(ok).is_some(), "{ok:?}");
        }
    }

    #[test]
    fn quote_matches_dumps() {
        assert_eq!(
            quote("a\"b\\c\n\u{1}é\u{7f}"),
            "\"a\\\"b\\\\c\\n\\u0001é\u{7f}\""
        );
    }
}
