# ptbr-pdf

Markdown to PDF for Portuguese text, and nothing else: one Rust binary, `to-pdf`. It parses the
Markdown with `pulldown-cmark`, writes Typst markup and compiles it with Typst as a library
(`typst` 0.15.1, `typst-pdf` 0.15.1, `typst-as-lib` 0.17.0). Six fonts are inside the binary
(`fonts/`: Libertinus Serif and DejaVu Sans Mono), so it needs no TeX, no fontconfig and no
network. Made for [ptbr-writer](https://github.com/h0ffmann/ptbr-writer)'s `.pdf` output.

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

## Benchmark: from pandoc to Rust

The first `ptbr-pdf` was pandoc + XeLaTeX. It worked, but a person who only wanted a PDF
downloaded a Haskell binary and a TeX tree. The lab is now one Rust binary with Typst inside.
Measured on x86_64-linux, 2026-10-20 and 2026-10-21, with `nix path-info -Sh`, `ls -l`, `xz -9`
and ptbr-writer's `scripts/pdf-bench.sh`.

| | what it is | on disk | download | first run, fresh runner |
| :- | :- | -: | -: | -: |
| 1. pandoc + XeLaTeX ([#128](https://github.com/h0ffmann/picasso/pull/128)) | pandoc 3.7 (Haskell, ~300 MiB) + `texliveBasic` and 21 packages | 729 MiB, 284 paths | 206 MiB from cache.nixos.org | 27 s |
| 2. Rust + Typst, first build | one binary, all of `typst-assets`' fonts | 46.7 MiB static | 13.7 MiB (`xz -9`) | 17 min through Nix ⚠ |
| 3. **Rust + Typst, slim** | six fonts, `opt-level = "z"`, `lto = "fat"`, `panic = "abort"`, stripped | **25.2 MiB static** | **8.1 MiB** (`xz -9`), 10.8 MiB (gzip) | **0.2 s** to convert a page |

> **⚠ xz.** The xz sizes are a measurement only: nothing here ships an `.xz` file. If a release
> asset is ever compressed with xz, use xz 5.8.4 or newer from the lab's pinned nixpkgs (the lock
> has 5.8.4), never the host's. XZ Utils had a backdoor in 5.6.0 and 5.6.1
> ([CVE-2024-3094](https://nvd.nist.gov/vuln/detail/CVE-2024-3094)) and memory-safety fixes in 5.8.1
> ([CVE-2025-31115](https://github.com/tukaani-project/xz/security/advisories/GHSA-6cc8-p5mm-29w2)),
> 5.8.3 ([CVE-2026-34743](https://github.com/tukaani-project/xz/security/advisories/GHSA-x872-m794-cxhv))
> and 5.8.4 ([GHSA-5qpq-xqfv-j9pg](https://github.com/tukaani-project/xz/security/advisories/GHSA-5qpq-xqfv-j9pg)).
> Prefer zstd or gzip for assets.

- Step 1 to step 3: **29× less on disk and 25× less to download.**
- ⚠ Through Nix, row 2 took 17 minutes on a fresh 4-core runner: the binary is not in
  cache.nixos.org, so Nix compiles Typst and 326 crates. Download the prebuilt static binary
  instead; that path is the 8.1 MiB above.
- `checks.size` fails if the binary grows more than 10% over 26 406 480 bytes.

## Platforms

| package | runs on | needs |
| :- | :- | :- |
| `packages.static` (musl, static-pie) | any x86_64 Linux | nothing: one file, 25.2 MiB. Ran with an empty environment (`env -i`) |
| `packages.to-pdf` (glibc) | x86_64 Linux with Nix | Nix: glibc and libgcc_s from the store, 72.2 MiB closure |
| aarch64-linux, aarch64-darwin | declared in the flake | not built or run ⚠ |
| x86_64-darwin, Windows | not declared | Typst supports them; a builder is missing ⚠ |

`Cargo.lock` pins 326 crates so the build is reproducible. It is build-time only: nothing of it
ships except the one binary.

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

`flake.nix` checks: `lab-json`, `to-pdf` (build, `cargo test` with the `insta` snapshots of the
Markdown to Typst writer), `clippy` (`-D warnings`), `fmt`, `size` and `smoke`.
