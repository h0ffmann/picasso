//! A minimal TOML reader for the one flat shape of ptbr-writer.toml: `key = value` lines
//! (string, integer, boolean, array of those), comments, and `[table]` headers whose contents
//! the plugin ignores. Anything else is an error, which the caller reports as "cannot read".
//! shortcut: floats and inline tables are skipped (kept as text); no dates, multi-line strings,
//! dotted keys or `[[arrays]]`;
//! Python's tomllib reads those, so such a file is "unreadable" here. Upgrade if the shape grows.

/// The file is not in the supported shape.
#[derive(Debug)]
pub struct Error;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Bool(bool),
    Arr(Vec<Value>),
    Table,
    /// A float or an inline table: read, kept as written, never used.
    Other(String),
}

#[derive(Default)]
pub struct Doc {
    /// Top-level keys and table names, in file order.
    pub top: Vec<(String, Value)>,
}

impl Doc {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.top.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

pub fn parse(src: &str) -> Result<Doc, Error> {
    let c: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut doc = Doc::default();
    let mut in_table = false;
    loop {
        blanks(&c, &mut i);
        match c.get(i) {
            None => return Ok(doc),
            Some('\n') => i += 1,
            Some('\r') if c.get(i + 1) == Some(&'\n') => i += 2,
            Some('#') => comment(&c, &mut i),
            Some('[') => {
                i += 1;
                if c.get(i) == Some(&'[') {
                    return Err(Error);
                }
                blanks(&c, &mut i);
                let name = key(&c, &mut i)?;
                blanks(&c, &mut i);
                if c.get(i) != Some(&']') {
                    return Err(Error);
                }
                i += 1;
                eol(&c, &mut i)?;
                if doc.get(&name).is_some() {
                    return Err(Error);
                }
                doc.top.push((name, Value::Table));
                in_table = true;
            }
            Some(_) => {
                let k = key(&c, &mut i)?;
                blanks(&c, &mut i);
                if c.get(i) != Some(&'=') {
                    return Err(Error);
                }
                i += 1;
                blanks(&c, &mut i);
                let v = value(&c, &mut i)?;
                eol(&c, &mut i)?;
                if !in_table {
                    if doc.get(&k).is_some() {
                        return Err(Error);
                    }
                    doc.top.push((k, v));
                }
            }
        }
    }
}

fn blanks(c: &[char], i: &mut usize) {
    while matches!(c.get(*i), Some(' ' | '\t')) {
        *i += 1;
    }
}

fn comment(c: &[char], i: &mut usize) {
    while c.get(*i).is_some_and(|&x| x != '\n') {
        *i += 1;
    }
}

/// After a value or a header: spaces, an optional comment, then a newline or the end.
fn eol(c: &[char], i: &mut usize) -> Result<(), Error> {
    blanks(c, i);
    if c.get(*i) == Some(&'#') {
        comment(c, i);
    }
    match c.get(*i) {
        None => Ok(()),
        Some('\n') => {
            *i += 1;
            Ok(())
        }
        Some('\r') if c.get(*i + 1) == Some(&'\n') => {
            *i += 2;
            Ok(())
        }
        _ => Err(Error),
    }
}

fn key(c: &[char], i: &mut usize) -> Result<String, Error> {
    match c.get(*i) {
        Some('"') => basic(c, i),
        Some('\'') => literal(c, i),
        _ => {
            let s = *i;
            while c
                .get(*i)
                .is_some_and(|x| x.is_ascii_alphanumeric() || *x == '_' || *x == '-')
            {
                *i += 1;
            }
            if *i == s {
                Err(Error)
            } else {
                Ok(c[s..*i].iter().collect())
            }
        }
    }
}

fn basic(c: &[char], i: &mut usize) -> Result<String, Error> {
    if c[*i..].starts_with(&['"', '"', '"']) {
        return Err(Error);
    }
    *i += 1;
    let mut o = String::new();
    loop {
        let ch = *c.get(*i).ok_or(Error)?;
        *i += 1;
        match ch {
            '"' => return Ok(o),
            '\\' => {
                let e = *c.get(*i).ok_or(Error)?;
                *i += 1;
                match e {
                    'b' => o.push('\x08'),
                    't' => o.push('\t'),
                    'n' => o.push('\n'),
                    'f' => o.push('\x0c'),
                    'r' => o.push('\r'),
                    '"' | '\\' => o.push(e),
                    'u' | 'U' => {
                        let n = if e == 'u' { 4 } else { 8 };
                        let h: String = c.get(*i..*i + n).ok_or(Error)?.iter().collect();
                        if !h.chars().all(|d| d.is_ascii_hexdigit()) {
                            return Err(Error);
                        }
                        o.push(
                            char::from_u32(u32::from_str_radix(&h, 16).map_err(|_| Error)?)
                                .ok_or(Error)?,
                        );
                        *i += n;
                    }
                    _ => return Err(Error),
                }
            }
            c if (c as u32) < 0x20 && c != '\t' || c == '\x7f' => return Err(Error),
            c => o.push(c),
        }
    }
}

fn literal(c: &[char], i: &mut usize) -> Result<String, Error> {
    if c[*i..].starts_with(&['\'', '\'', '\'']) {
        return Err(Error);
    }
    *i += 1;
    let s = *i;
    loop {
        let ch = *c.get(*i).ok_or(Error)?;
        if ch == '\'' {
            let o = c[s..*i].iter().collect();
            *i += 1;
            return Ok(o);
        }
        if (ch as u32) < 0x20 && ch != '\t' || ch == '\x7f' {
            return Err(Error);
        }
        *i += 1;
    }
}

fn value(c: &[char], i: &mut usize) -> Result<Value, Error> {
    match c.get(*i).ok_or(Error)? {
        '"' => basic(c, i).map(Value::Str),
        '\'' => literal(c, i).map(Value::Str),
        '[' => {
            *i += 1;
            let mut v = Vec::new();
            loop {
                gap(c, i);
                if c.get(*i) == Some(&']') {
                    *i += 1;
                    return Ok(Value::Arr(v));
                }
                v.push(value(c, i)?);
                gap(c, i);
                match c.get(*i) {
                    Some(',') => *i += 1,
                    Some(']') => {}
                    _ => return Err(Error),
                }
            }
        }
        't' if c[*i..].starts_with(&['t', 'r', 'u', 'e']) => {
            *i += 4;
            Ok(Value::Bool(true))
        }
        'f' if c[*i..].starts_with(&['f', 'a', 'l', 's', 'e']) => {
            *i += 5;
            Ok(Value::Bool(false))
        }
        '+' | '-' | '0'..='9' => {
            let s = *i;
            if matches!(c[*i], '+' | '-') {
                *i += 1;
            }
            while c.get(*i).is_some_and(|x| x.is_ascii_digit() || *x == '_') {
                *i += 1;
            }
            let t: String = c[s..*i].iter().collect();
            let d = t.trim_start_matches(['+', '-']);
            let ok = !d.is_empty()
                && !d.starts_with('_')
                && !d.ends_with('_')
                && !d.contains("__")
                && (d == "0" || !d.starts_with('0'));
            if ok && matches!(c.get(*i), Some('.' | 'e' | 'E')) {
                while c.get(*i).is_some_and(|x| {
                    x.is_ascii_digit() || matches!(x, '_' | '.' | 'e' | 'E' | '+' | '-')
                }) {
                    *i += 1;
                }
                return Ok(Value::Other(c[s..*i].iter().collect()));
            }
            if !ok || matches!(c.get(*i), Some('x' | 'o' | 'b' | ':' | '-')) {
                return Err(Error);
            }
            t.replace('_', "")
                .parse()
                .map(Value::Int)
                .map_err(|_| Error)
        }
        '{' => {
            let (s, mut depth) = (*i, 0);
            loop {
                match *c.get(*i).ok_or(Error)? {
                    '"' => drop(basic(c, i)?),
                    '\'' => drop(literal(c, i)?),
                    ch => {
                        depth += (ch == '{') as i32 - (ch == '}') as i32;
                        *i += 1;
                        if depth == 0 {
                            return Ok(Value::Other(c[s..*i].iter().collect()));
                        }
                    }
                }
            }
        }
        _ => Err(Error),
    }
}

/// Spaces, newlines and comments inside an array.
fn gap(c: &[char], i: &mut usize) {
    loop {
        match c.get(*i) {
            Some(' ' | '\t' | '\n' | '\r') => *i += 1,
            Some('#') => comment(c, i),
            _ => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_default_file() {
        let d = parse(include_str!("../tests/fixtures/ptbr-writer.default.toml")).unwrap();
        assert_eq!(d.get("folder"), Some(&Value::Str(".".into())));
        assert_eq!(
            d.get("languages"),
            Some(&Value::Arr(vec![Value::Str("pt-BR".into())]))
        );
        assert_eq!(d.get("levels"), Some(&Value::Arr(vec![])));
    }

    #[test]
    fn arrays_span_lines_and_comments() {
        let d = parse("a = [\n  \"x\", # c\n  'y',\n]\n[t]\nb = 1\n").unwrap();
        assert_eq!(
            d.get("a"),
            Some(&Value::Arr(vec![
                Value::Str("x".into()),
                Value::Str("y".into())
            ]))
        );
        assert_eq!(d.get("t"), Some(&Value::Table));
        assert!(d.get("b").is_none());
    }

    #[test]
    fn rejects_broken_files() {
        for bad in [
            "levels = [\n",
            "a = 1\na = 2\n",
            "a = \"x\" y\n",
            "a = 01\n",
            "a = \"\n\"\n",
            "a\n",
            "[[t]]\n",
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
    }
}
