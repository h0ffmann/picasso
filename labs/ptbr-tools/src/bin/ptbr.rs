//! One multi-call binary: it runs as the command named by argv[0] (`ptbr-ask-levels`,
//! `ptbr-reading-time`, `ptbr-route`, the names of symlinks to it), or by its first argument
//! (`ptbr route --check`). One std copy instead of three.
use ptbr_tools::{ask_levels, reading_time, route};

fn main() {
    let mut a: Vec<String> = std::env::args().collect();
    let mut name = a
        .first()
        .and_then(|p| p.rsplit('/').next())
        .unwrap_or("")
        .to_string();
    if name == "ptbr" && a.len() > 1 {
        name = format!("ptbr-{}", a.remove(1));
    }
    std::process::exit(match name.as_str() {
        "ptbr-ask-levels" => ask_levels::main(a),
        "ptbr-reading-time" => reading_time::main(a),
        "ptbr-route" => route::main(a.into_iter().skip(1).collect()),
        _ => {
            eprintln!("usage: ptbr ask-levels|reading-time|route ...  (or run it as ptbr-ask-levels, ptbr-reading-time, ptbr-route)");
            2
        }
    });
}
