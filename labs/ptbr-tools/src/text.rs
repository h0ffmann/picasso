//! Helpers that mimic the parts of Python's `str` and `re` that the scripts rely on.

/// Python's `\s` and `str.isspace`: Unicode White_Space plus U+001C..U+001F.
pub fn py_space(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

/// Python's `\w`.
pub fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Lower case, then strip the accents of Portuguese (and a few neighbours).
/// shortcut: a table of precomposed letters plus a drop of U+0300..U+036F, not a full NFD;
/// accents of other scripts stay (Python's NFD would strip them), upgrade if a level or a route
/// word is ever not Latin-1.
pub fn fold(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| !('\u{300}'..='\u{36f}').contains(c))
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            'ý' | 'ÿ' => 'y',
            c => c,
        })
        .collect()
}

/// `re.search(rf"(?<!\w){re.escape(needle)}(?!\w)", hay)`.
pub fn has_word(hay: &str, needle: &str) -> bool {
    let h: Vec<char> = hay.chars().collect();
    let n: Vec<char> = needle.chars().collect();
    if n.is_empty() || n.len() > h.len() {
        return false;
    }
    (0..=h.len() - n.len()).any(|i| {
        h[i..i + n.len()] == n[..]
            && (i == 0 || !is_word(h[i - 1]))
            && h.get(i + n.len()).is_none_or(|&c| !is_word(c))
    })
}

/// Python's `repr(str)`.
pub fn py_repr(s: &str) -> String {
    let q = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut o = String::from(q);
    for c in s.chars() {
        match c {
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if c == q => {
                o.push('\\');
                o.push(c);
            }
            c if (c as u32) < 0x20 || (0x7f..=0xa0).contains(&(c as u32)) => {
                o.push_str(&format!("\\x{:02x}", c as u32))
            }
            c => o.push(c),
        }
    }
    o.push(q);
    o
}

/// Python's `os.path.join` for two parts.
pub fn join(a: &str, b: &str) -> String {
    if b.starts_with('/') {
        b.to_string()
    } else if a.is_empty() || a.ends_with('/') {
        format!("{a}{b}")
    } else {
        format!("{a}/{b}")
    }
}

/// Python's `str.splitlines`.
pub fn splitlines(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut it = s.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        if matches!(
            c,
            '\n' | '\r'
                | '\x0b'
                | '\x0c'
                | '\x1c'
                | '\x1d'
                | '\x1e'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        ) {
            out.push(&s[start..i]);
            let mut end = i + c.len_utf8();
            if c == '\r' && it.peek().is_some_and(|&(_, n)| n == '\n') {
                it.next();
                end += 1;
            }
            start = end;
        }
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

/// Python's `open(path, encoding="utf-8").read()`: universal newlines.
pub fn read_text(path: &str) -> std::io::Result<String> {
    let s = std::fs::read_to_string(path)?;
    Ok(universal_newlines(&s))
}

pub fn universal_newlines(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// `OSError` as Python prints it: `[Errno 2] No such file or directory: 'x'`.
pub fn os_error(e: &std::io::Error, path: &str) -> String {
    let msg = e.to_string();
    let msg = msg.split(" (os error").next().unwrap_or(&msg);
    match e.raw_os_error() {
        Some(n) => format!("[Errno {n}] {msg}: {}", py_repr(path)),
        None => msg.to_string(),
    }
}
