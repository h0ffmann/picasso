# ptbr-pdf

Markdown to PDF for Portuguese text, and nothing else: one Rust binary, `to-pdf`. It parses the
Markdown with `pulldown-cmark`, writes Typst markup and compiles it with Typst as a library
(`typst` 0.15.1, `typst-pdf` 0.15.1, `typst-as-lib` 0.17.0). The fonts are inside the binary
(`typst-assets`), so it needs no TeX, no fontconfig and no network. Made for
[ptbr-writter](https://github.com/h0ffmann/ptbr-writter)'s `.pdf` output.

```console
nix run github:h0ffmann/picasso?dir=labs/ptbr-pdf -- texto.md          # texto.pdf
nix run github:h0ffmann/picasso/<rev>?dir=labs/ptbr-pdf -- texto.md   # pinned
just to-pdf texto.md saida.pdf   # from this lab
just smoke                       # the sample in the sandbox; accents and ⚠ checked (CI)
just test                        # cargo test in the dev shell
just size                        # the closure size
```

The page is A4 with 2.5 cm margins, language `pt-BR`. Supported Markdown: headings, emphasis,
strong, strikethrough, inline code, code blocks, nested lists, tables, links, block quotes and
rules. Images keep only their alt text. Typst special characters in the text are escaped.

## Size

Measured on x86_64-linux, 2026-10-20, with `nix path-info -Sh` and `ls -l`.

| | closure | store paths | binary |
| :- | :- | :- | :- |
| `packages.to-pdf` | **93.6 MiB** | 7 | 46.6 MiB (48 883 456 bytes) |
| `packages.static` (musl, static-pie) | **46.7 MiB** | 1 | 46.7 MiB (48 997 424 bytes) |
| before: pandoc + XeLaTeX | 729 MiB | 284 | n/a |
| [publisher](../publisher) | 3.2 GiB | 1815 | n/a |

The build is `cargo build --release` with LTO. The binary is large because it holds Typst, its
syntaxes and the fonts. Use [publisher](../publisher) for books, citations, other engines or docx.

## Fonts and characters

Accented Portuguese text renders. `⚠` (U+26A0) renders too: `just smoke` extracts the text of the
PDF and fails if the accents line or `⚠` is missing. It comes from the embedded DejaVu Sans Mono
(checked as text in the PDF with `pdftotext`; the glyph was not inspected by eye ⚠).

## Code blocks

A fence with a language is highlighted by Typst's bundled syntaxes, in DejaVu Sans Mono on a
light background. Verified by `cargo test` (the PDF of the block differs from the same code under an
unknown language): **json, python, sql, rust, scala**. Best effort, not verified: aliases map to
Typst names (`sh`, `shell`, `console`, `zsh` to bash; `yml`; `js`; `ts`; `py`; `rs`; `golang`;
`c++`, `cxx`; `md`) and bash, yaml, javascript, typescript, go, cpp, c, java, kotlin, toml, html,
css, dockerfile, markdown are passed to Typst as they are ⚠. An unknown language renders as plain
monospace and does not fail.

## Reusing it from another repository

```nix
inputs.ptbr-pdf.url = "github:h0ffmann/picasso?dir=labs/ptbr-pdf";
# ptbr-pdf.packages.${system}.to-pdf, or .static on x86_64-linux
```

`flake.nix` checks: `lab-json`, `to-pdf` (build and `cargo test`), `clippy` (`-D warnings`), `fmt`
and `smoke`.
