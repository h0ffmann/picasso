# ptbr-tools

The runtime scripts of the [ptbr-writer](https://github.com/h0ffmann/ptbr-writer) plugin, ported
from Python to Rust. They are std only: the crate has no dependencies, and `Cargo.lock` lists one
package. The plugin ships the static binary, so a person who installs it does not need Python.

There is one multi-call binary, `ptbr`. It runs as the command that its file name gives, so the
plugin installs three symlinks to it:

| command | what it does | replaces |
| :- | :- | :- |
| `ptbr-ask-levels SKILL.md TO_PDF DEFAULTS [READING_TIME] < hook.json` | The `UserPromptSubmit` hook. It reads the hook JSON on stdin and, for a request that starts with `ptbr`, prints the hook JSON with one instruction for Claude. The config layers are the plugin's `ptbr-writer.default.toml`, `~/.config/ptbr-writer/ptbr-writer.toml`, `./ptbr-writer.toml` and the request. `--self-test` runs the built-in checks. | `hooks/ask_levels.py` |
| `ptbr-reading-time [--lang pt-BR\|en] [--json] [FILE]` | Prints the reading-time line for a file or for stdin. `--self-test` runs the built-in checks. | `scripts/reading_time.py` |
| `ptbr-route "<request>"` / `--check` / `--self-test` | Prints `<skill> · <level>` for a request, from `routes.tsv`. `--check` compares `routes.tsv` with `SKILL.md`. | `skills/ptbr-routing/scripts/route.py` |

`ptbr ask-levels`, `ptbr reading-time` and `ptbr route` work too, with the same arguments.

```console
nix run github:h0ffmann/picasso?dir=labs/ptbr-tools#static -- reading-time README.md
nix build github:h0ffmann/picasso?dir=labs/ptbr-tools#static    # result/bin/ptbr and three symlinks
just test                                                        # cargo test in the dev shell
just build                                                       # the static binary in ./result
just size                                                        # bytes, xz bytes, closure
```

`ptbr-route` finds `routes.tsv` and `SKILL.md` from the path that it is started with, as
`route.py` did from `__file__`: the binary (or the symlink to it) goes in `<skill>/scripts/`,
`routes.tsv` is in `<skill>/`, and `SKILL.md` is two directories above `<skill>/`.

## Same output as the Python

The output is the same byte for byte, and the exit codes are the same, for every input that was
tested (see the pull request that added this lab). These differences are on purpose:

- **Usage text.** It names the Rust command, not `python3 -I reading_time.py`.
- **`READING_TIME` that does not end in `.py`.** The instruction that `ptbr-ask-levels` writes for
  Claude says `python3 -I <path>` when the path ends in `.py`, and `<path>` alone when it does not.
  A path to `ptbr-reading-time` gives a command that Claude can run.
- **Where Python crashes with a traceback** (a TOML file that is not UTF-8, a list inside a list
  in a TOML array of levels), `ptbr-ask-levels` reads the file as "cannot read" or ignores the
  value, and goes on. A prompt that is not a JSON string still gives exit code 1.
- **Order of the duplicate errors of `ptbr-route --check`.** Python walks a set, so its order
  changes from run to run. The Rust order is the order of the file.

Shortcuts, each with a comment in the code:

- **Accent folding.** The level and route words are matched after `fold`: lower case, then a
  table of the precomposed Portuguese letters (and a few neighbours: `à â ã ä å é è ê ë í ì î ï ó ò ô õ ö ú ù û ü ç ñ ý ÿ`)
  to plain ASCII, and the combining marks U+0300..U+036F dropped. Python used a full NFD. An
  accent of another script stays in the Rust text.
- **TOML.** The reader takes `key = value` lines (string, integer, boolean, array of these;
  floats and inline tables are skipped), comments and `[table]` headers. Dates, multi-line
  strings, dotted keys and `[[arrays]]` make the file "cannot read" (the hook then tells Claude to
  read it itself), where Python's `tomllib` would read them.
- **JSON.** The parser is strict as Python's `json.load` is. A lone `\ud800` escape in the prompt
  becomes U+FFFD.
- The regular expressions of `reading_time.py` are written by hand, with the order of the
  engine's backtracking. `\w` is `char::is_alphanumeric` or `_`.

## Sizes

Measured on x86_64-linux, 2026-10-21, `nix build`, stripped, `opt-level = "z"`, `lto`,
`codegen-units = 1`, `panic = "abort"`.

| build | file | closure |
| :- | -: | -: |
| **`packages.static`**, one multi-call binary (musl, static-pie) | **570 112 bytes (0.54 MiB)**; 241 720 bytes with `xz -9` | 557.8 KiB |
| `packages.static`, three separate binaries (the first design) | 492 288 + 480 000 + 459 488 = 1 431 776 bytes (1.37 MiB) | 1.37 MiB |
| `packages.static-aarch64` (musl, static) | 528 696 bytes | 517.4 KiB |
| `packages.default` (glibc) | 480 176 bytes | 47.4 MiB (glibc and libgcc_s from the store) |

The three separate binaries each carry their own copy of `std`. One binary that dispatches on its
name is 2.5 times smaller in total, so the lab ships that one. `checks.size` fails if the static
binary grows more than 10% over 570 112 bytes (a ceiling of 627 123 bytes).

## Platforms

| package | runs on | needs |
| :- | :- | :- |
| `packages.static` | any x86_64 Linux | nothing. Ran with an empty environment (`env -i`) |
| `packages.static-aarch64` | aarch64 Linux | nothing. Built; **not run** ⚠ (no emulator here) |
| `packages.default` | x86_64 Linux with Nix | glibc from the store |
| aarch64-darwin | declared for `packages.default` | not built ⚠ |
| x86_64-darwin, Windows | not declared | not built ⚠ |

## Checks

`nix flake check --no-update-lock-file` builds and runs `cargo test` (the self-tests of all three
scripts, ported one for one, and tests of the TOML and JSON readers), `clippy -D warnings`,
`cargo fmt --check`, `lab.json`, the size ceiling and the aarch64 build.

## Reusing it from another repository

```nix
inputs.ptbr-tools.url = "github:h0ffmann/picasso?dir=labs/ptbr-tools";
```

Then use `ptbr-tools.packages.x86_64-linux.static` for the plugin's files, or `.default` in a
Nix shell. A plugin that ships the binary copies `bin/ptbr` and makes the three symlinks.
