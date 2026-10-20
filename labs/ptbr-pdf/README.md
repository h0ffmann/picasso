# ptbr-pdf

Markdown → PDF for Portuguese text, and nothing else: **pandoc 3.7** and a minimal **XeLaTeX**
(TeX Live 2025 `scheme-basic` plus the packages pandoc's template loads), DejaVu by file name, so
no fontconfig. Tailored for [ptbr-writter](https://github.com/h0ffmann/ptbr-writter)'s `.pdf`
output. One command, `to-pdf`.

```console
nix run github:h0ffmann/picasso?dir=labs/ptbr-pdf -- texto.md          # texto.pdf
nix run github:h0ffmann/picasso/<rev>?dir=labs/ptbr-pdf -- texto.md   # pinned
just to-pdf texto.md saida.pdf   # from this lab
just smoke                       # the sample in the sandbox; accents and ⚠ checked (CI)
just size                        # the closure size
```

| | ptbr-pdf | [publisher](../publisher) |
| :- | :- | :- |
| on disk (closure, x86_64-linux, 2026-10-20) | **729 MiB**, 284 store paths | 3.2 GiB, 1815 store paths |
| TeX | `texliveBasic` + 21 packages, xelatex | `texliveMedium` + extras, xelatex, pdflatex, lualatex |
| extras | none | python + openai, librsvg, emoji, Japanese, badges filter, docx, `mkPdf` |

Most of the 729 MiB is pandoc itself (~300 MiB, a Haskell binary) and the TeX tree (~540 MiB with
its scripts). The first `nix run` downloads it once; later runs reuse the store. Use
[publisher](../publisher) for books, citations, other engines or docx.
