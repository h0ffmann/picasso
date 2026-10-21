//! Per-language knowledge: which tree-sitter node kinds are symbols, calls and imports.
use tree_sitter::{Language, Node};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Rust,
    Python,
    Js,
    Ts,
    Tsx,
    Scala,
    Fortran,
    Gleam,
    Nix,
}

pub enum Role {
    Class(String),
    Func(String),
    Impl { ty: String, tr: Option<String> },
    None,
}

#[derive(Default, Debug)]
pub struct Imp {
    pub segs: Vec<String>,
    pub names: Vec<String>,
    pub path: Option<String>,
    pub dots: usize,
    pub weak: bool,
    pub line: u32,
}

pub struct Call {
    pub name: String,
    pub qual: Option<String>,
    /// written `receiver.name(...)`
    pub recv: bool,
}

impl Lang {
    pub fn of_path(p: &str) -> Option<Lang> {
        let ext = p.rsplit('.').next()?;
        Some(match ext {
            "rs" => Lang::Rust,
            "py" => Lang::Python,
            "js" | "jsx" | "mjs" | "cjs" => Lang::Js,
            "ts" | "mts" | "cts" => Lang::Ts,
            "tsx" => Lang::Tsx,
            "scala" | "sc" => Lang::Scala,
            "f" | "F" | "f90" | "F90" | "f95" | "F95" | "f03" | "F03" | "f08" | "F08" => {
                Lang::Fortran
            }
            "gleam" => Lang::Gleam,
            "nix" => Lang::Nix,
            _ => return None,
        })
    }

    pub fn grammar(self) -> Language {
        match self {
            Lang::Rust => tree_sitter_rust::LANGUAGE.into(),
            Lang::Python => tree_sitter_python::LANGUAGE.into(),
            Lang::Js => tree_sitter_javascript::LANGUAGE.into(),
            Lang::Ts => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Lang::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Lang::Scala => tree_sitter_scala::LANGUAGE.into(),
            Lang::Fortran => tree_sitter_fortran::LANGUAGE.into(),
            Lang::Gleam => tree_sitter_gleam::LANGUAGE.into(),
            Lang::Nix => tree_sitter_nix::LANGUAGE.into(),
        }
    }

    /// Family of files that can satisfy an import.
    pub fn family(self) -> u8 {
        match self {
            Lang::Js | Lang::Ts | Lang::Tsx => 1,
            l => l as u8 + 10,
        }
    }
}

fn t<'a>(n: Node, s: &'a [u8]) -> &'a str {
    n.utf8_text(s).unwrap_or("")
}
fn f<'a>(n: Node<'a>, k: &str) -> Option<Node<'a>> {
    n.child_by_field_name(k)
}
fn ft(n: Node, k: &str, s: &[u8]) -> Option<String> {
    f(n, k)
        .map(|x| t(x, s).to_string())
        .filter(|x| !x.is_empty())
}
fn kids(n: Node) -> Vec<Node> {
    let mut c = n.walk();
    n.children(&mut c).collect()
}
fn named_with_field<'a>(n: Node<'a>, k: &str) -> Vec<Node<'a>> {
    let mut c = n.walk();
    n.children_by_field_name(k, &mut c).collect()
}

fn rust_type_name(n: Node, s: &[u8]) -> String {
    match n.kind() {
        "generic_type" | "reference_type" => f(n, "type")
            .map(|x| rust_type_name(x, s))
            .unwrap_or_default(),
        "scoped_type_identifier" => ft(n, "name", s).unwrap_or_default(),
        _ => t(n, s).to_string(),
    }
}

/// Fortran keeps a unit's name in its opening statement.
fn fortran_name(n: Node, s: &[u8]) -> Option<String> {
    let st = kids(n)
        .into_iter()
        .find(|c| c.kind().ends_with("_statement"))?;
    let name = f(st, "name").or_else(|| kids(st).into_iter().find(|c| c.kind() == "name"))?;
    Some(t(name, s).to_string())
}

fn last_attr(n: Node, s: &[u8]) -> Option<String> {
    let mut c = n.walk();
    let last = n.named_children(&mut c).last()?;
    Some(t(last, s).to_string())
}

pub fn role(l: Lang, n: Node, s: &[u8]) -> Role {
    let k = n.kind();
    let name = || ft(n, "name", s);
    let some = |c: Option<String>, mk: fn(String) -> Role| c.map(mk).unwrap_or(Role::None);
    match (l, k) {
        (
            Lang::Rust,
            "struct_item" | "enum_item" | "union_item" | "trait_item" | "const_item"
            | "static_item",
        ) => some(name(), Role::Class),
        (Lang::Rust, "function_item" | "function_signature_item") => some(name(), Role::Func),
        (Lang::Rust, "impl_item") => match f(n, "type") {
            Some(ty) => Role::Impl {
                ty: rust_type_name(ty, s),
                tr: f(n, "trait").map(|x| rust_type_name(x, s)),
            },
            None => Role::None,
        },
        (Lang::Python, "class_definition") => some(name(), Role::Class),
        (Lang::Python, "function_definition") => some(name(), Role::Func),
        (
            Lang::Js | Lang::Ts | Lang::Tsx,
            "class_declaration"
            | "abstract_class_declaration"
            | "class"
            | "interface_declaration"
            | "enum_declaration",
        ) => some(name(), Role::Class),
        (
            Lang::Js | Lang::Ts | Lang::Tsx,
            "function_declaration" | "generator_function_declaration" | "method_definition",
        ) => some(name(), Role::Func),
        (Lang::Js | Lang::Ts | Lang::Tsx, "variable_declarator") => match f(n, "value") {
            Some(v)
                if matches!(
                    v.kind(),
                    "arrow_function" | "function_expression" | "function" | "generator_function"
                ) =>
            {
                some(name(), Role::Func)
            }
            _ => Role::None,
        },
        (
            Lang::Scala,
            "class_definition" | "object_definition" | "trait_definition" | "enum_definition",
        ) => some(name(), Role::Class),
        (Lang::Scala, "function_definition" | "function_declaration") => some(name(), Role::Func),
        (Lang::Fortran, "module" | "submodule" | "program") => {
            some(fortran_name(n, s), Role::Class)
        }
        (Lang::Fortran, "subroutine" | "function") => some(fortran_name(n, s), Role::Func),
        (Lang::Gleam, "function") => some(name(), Role::Func),
        (Lang::Gleam, "type_definition" | "type_alias") => some(
            kids(n)
                .into_iter()
                .find(|c| c.kind() == "type_name")
                .and_then(|x| ft(x, "name", s))
                .or_else(name),
            Role::Class,
        ),
        (Lang::Nix, "binding") => match (f(n, "expression"), f(n, "attrpath")) {
            (Some(e), Some(a)) if e.kind() == "function_expression" => {
                some(last_attr(a, s), Role::Func)
            }
            _ => Role::None,
        },
        _ => Role::None,
    }
}

fn reduce(n: Node, s: &[u8]) -> Option<Call> {
    let plain = |x: String| {
        Some(Call {
            name: x,
            qual: None,
            recv: false,
        })
    };
    let qual_of = |x: Option<Node>| {
        x.filter(|q| {
            matches!(
                q.kind(),
                "identifier"
                    | "self"
                    | "this"
                    | "type_identifier"
                    | "variable_expression"
                    | "scoped_identifier"
            )
        })
        .map(|q| t(q, s).rsplit("::").next().unwrap_or("").to_string())
    };
    match n.kind() {
        "identifier"
        | "type_identifier"
        | "property_identifier"
        | "field_identifier"
        | "variable_expression" => plain(t(n, s).to_string()),
        "scoped_identifier" => Some(Call {
            name: ft(n, "name", s)?,
            qual: qual_of(f(n, "path")),
            recv: false,
        }),
        "field_expression" => {
            let name = ft(n, "field", s)?;
            Some(Call {
                name,
                qual: qual_of(f(n, "value")),
                recv: true,
            })
        }
        "attribute" => Some(Call {
            name: ft(n, "attribute", s)?,
            qual: qual_of(f(n, "object")),
            recv: true,
        }),
        "member_expression" => {
            let q = f(n, "object").map(|o| {
                if o.kind() == "new_expression" {
                    f(o, "constructor")
                } else {
                    Some(o)
                }
            });
            Some(Call {
                name: ft(n, "property", s)?,
                qual: qual_of(q.flatten()),
                recv: true,
            })
        }
        "field_access" => Some(Call {
            name: ft(n, "field", s)?,
            qual: qual_of(f(n, "record")),
            recv: true,
        }),
        "select_expression" => Some(Call {
            name: last_attr(f(n, "attrpath")?, s)?,
            qual: qual_of(f(n, "expression")),
            recv: true,
        }),
        "generic_function" | "apply_expression" => reduce(f(n, "function")?, s),
        _ => None,
    }
}

pub fn call(l: Lang, n: Node, s: &[u8]) -> Option<Call> {
    let target = match (l, n.kind()) {
        (Lang::Rust, "call_expression") => f(n, "function")?,
        (Lang::Python, "call") => f(n, "function")?,
        (Lang::Js | Lang::Ts | Lang::Tsx, "call_expression") => f(n, "function")?,
        (Lang::Js | Lang::Ts | Lang::Tsx, "new_expression") => f(n, "constructor")?,
        (Lang::Scala, "call_expression") => f(n, "function")?,
        (Lang::Fortran, "subroutine_call") => f(n, "subroutine")?,
        (Lang::Fortran, "call_expression") => n.named_child(0)?,
        (Lang::Gleam, "function_call") => f(n, "function")?,
        (Lang::Nix, "apply_expression") => n,
        _ => return None,
    };
    reduce(target, s)
}

/// (relation, name) of the parents a class names: extends, implements.
pub fn bases(l: Lang, n: Node, s: &[u8]) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    match l {
        Lang::Python if n.kind() == "class_definition" => {
            if let Some(a) = f(n, "superclasses") {
                let mut c = a.walk();
                for x in a.named_children(&mut c) {
                    if let Some(c) = reduce(x, s) {
                        out.push(("inherits", c.name));
                    }
                }
            }
        }
        Lang::Js | Lang::Ts | Lang::Tsx => {
            fn go(n: Node, s: &[u8], rel: &'static str, out: &mut Vec<(&'static str, String)>) {
                for c in kids(n) {
                    match c.kind() {
                        "extends_clause" | "extends_type_clause" => go(c, s, "inherits", out),
                        "implements_clause" => go(c, s, "implements", out),
                        "class_heritage" => go(c, s, rel, out),
                        "identifier" | "type_identifier" => out.push((rel, t(c, s).to_string())),
                        "member_expression" | "generic_type" if !rel.is_empty() => {
                            if let Some(x) =
                                reduce(c, s).map(|x| x.name).or_else(|| ft(c, "name", s))
                            {
                                out.push((rel, x));
                            }
                        }
                        _ => {}
                    }
                }
            }
            for c in kids(n) {
                if matches!(c.kind(), "class_heritage" | "extends_type_clause") {
                    go(n, s, "", &mut out);
                    break;
                }
            }
        }
        Lang::Scala => {
            for c in kids(n) {
                if c.kind() == "extends_clause" {
                    for x in kids(c) {
                        let nm = match x.kind() {
                            "type_identifier" => Some(t(x, s).to_string()),
                            "generic_type" => f(x, "type").map(|y| t(y, s).to_string()),
                            _ => None,
                        };
                        if let Some(nm) = nm {
                            out.push(("inherits", nm));
                        }
                    }
                }
            }
        }
        _ => {}
    }
    out
}

fn split_names(inner: &str) -> Vec<String> {
    let mut v = Vec::new();
    let (mut depth, mut cur) = (0, String::new());
    for ch in inner.chars().chain(std::iter::once(',')) {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                let x = cur
                    .split(" as ")
                    .next()
                    .unwrap_or("")
                    .trim()
                    .rsplit("::")
                    .next()
                    .unwrap_or("")
                    .to_string();
                if !x.is_empty() && x != "self" && x != "*" && !x.contains('{') {
                    v.push(x);
                }
                cur.clear();
                continue;
            }
            _ => {}
        }
        cur.push(ch);
    }
    v
}

fn segs(p: &str, sep: &str) -> Vec<String> {
    p.split(sep)
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(String::from)
        .collect()
}

pub fn imports(l: Lang, n: Node, s: &[u8]) -> Vec<Imp> {
    let line = n.start_position().row as u32 + 1;
    let k = n.kind();
    match (l, k) {
        (Lang::Rust, "use_declaration") => {
            let arg = f(n, "argument");
            let arg = arg.map(|x| {
                if x.kind() == "use_as_clause" {
                    f(x, "path").unwrap_or(x)
                } else {
                    x
                }
            });
            let a: String = arg
                .map(|x| t(x, s))
                .unwrap_or("")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let (prefix, names) = match a.find('{') {
                Some(i) => (
                    a[..i].trim_end_matches("::").to_string(),
                    split_names(&a[i + 1..a.rfind('}').unwrap_or(a.len())]),
                ),
                None => {
                    let mut sg = segs(&a, "::");
                    let last = sg.pop().unwrap_or_default();
                    (sg.join("::"), if last == "*" { vec![] } else { vec![last] })
                }
            };
            let mut sg = segs(&prefix, "::");
            if sg.is_empty() {
                sg = names.clone();
            }
            vec![Imp {
                segs: sg,
                names,
                line,
                ..Default::default()
            }]
        }
        (Lang::Rust, "mod_item") if f(n, "body").is_none() => match ft(n, "name", s) {
            Some(nm) => vec![Imp {
                segs: vec![nm],
                weak: true,
                line,
                ..Default::default()
            }],
            None => vec![],
        },
        (Lang::Python, "import_statement") => named_with_field(n, "name")
            .into_iter()
            .map(|x| {
                let d = if x.kind() == "aliased_import" {
                    f(x, "name").unwrap_or(x)
                } else {
                    x
                };
                Imp {
                    segs: segs(t(d, s), "."),
                    line,
                    ..Default::default()
                }
            })
            .collect(),
        (Lang::Python, "import_from_statement") => {
            let m = ft(n, "module_name", s).unwrap_or_default();
            let dots = m.chars().take_while(|c| *c == '.').count();
            let names = named_with_field(n, "name")
                .into_iter()
                .map(|x| {
                    if x.kind() == "aliased_import" {
                        f(x, "name").unwrap_or(x)
                    } else {
                        x
                    }
                })
                .map(|x| t(x, s).to_string())
                .collect();
            vec![Imp {
                segs: segs(&m[dots..], "."),
                names,
                dots,
                line,
                ..Default::default()
            }]
        }
        (Lang::Js | Lang::Ts | Lang::Tsx, "import_statement") => {
            let p = ft(n, "source", s)
                .unwrap_or_default()
                .trim_matches(|c| c == '"' || c == '\'' || c == '`')
                .to_string();
            let mut names = Vec::new();
            fn specs(n: Node, s: &[u8], out: &mut Vec<String>) {
                for c in kids(n) {
                    if c.kind() == "import_specifier" {
                        if let Some(x) = ft(c, "name", s) {
                            out.push(x);
                        }
                    } else {
                        specs(c, s, out);
                    }
                }
            }
            specs(n, s, &mut names);
            vec![Imp {
                path: Some(p),
                names,
                line,
                ..Default::default()
            }]
        }
        (Lang::Scala, "import_declaration") => {
            let sg: Vec<String> = named_with_field(n, "path")
                .into_iter()
                .map(|x| t(x, s).to_string())
                .collect();
            vec![Imp {
                segs: sg,
                line,
                ..Default::default()
            }]
        }
        (Lang::Fortran, "use_statement") => {
            match kids(n).into_iter().find(|c| c.kind() == "module_name") {
                Some(m) => vec![Imp {
                    segs: vec![t(m, s).to_string()],
                    line,
                    ..Default::default()
                }],
                None => vec![],
            }
        }
        (Lang::Gleam, "import") => {
            let m = ft(n, "module", s).unwrap_or_default();
            let mut names = Vec::new();
            if let Some(u) = f(n, "imports") {
                for c in kids(u) {
                    if let Some(x) = ft(c, "name", s) {
                        names.push(x);
                    }
                }
            }
            vec![Imp {
                segs: segs(&m, "/"),
                names,
                line,
                ..Default::default()
            }]
        }
        (Lang::Nix, "path_expression") => {
            vec![Imp {
                path: Some(t(n, s).to_string()),
                line,
                ..Default::default()
            }]
        }
        _ => vec![],
    }
}
