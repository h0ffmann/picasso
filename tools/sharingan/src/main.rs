mod bench;
mod licence;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(String::as_str) {
        Some("licence-scan") => licence::main(&args[1..]),
        Some("bench") => bench::main(&args[1..]),
        _ => {
            eprintln!(
                "usage: sharingan licence-scan [--into SPDX-ID] DIR... | --matrix\n       sharingan bench trigger|output|all [options]"
            );
            2
        }
    };
    std::process::exit(code);
}
