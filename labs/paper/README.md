# paper

The toolchain for papers written in Markdown, after Sandy Maguire's
[isovector](https://github.com/isovector) book layout: **pandoc** with **pandoc-crossref** and
**citeproc**, **XeLaTeX** for the PDF and **pdfLaTeX** for the arXiv source check (arXiv
compiles with pdfLaTeX), plus **python 3**, **just**, **git**, **gh**, **poppler-utils** and **librsvg** (`rsvg-convert`, for SVG images, which XeLaTeX cannot include). No
scripts; the paper's repository has its own build. First consumer:
[h0ffmann/closed-source](https://github.com/h0ffmann/closed-source) (Paper Fabric).

```
just shell       # the tools on PATH, in your current directory
just versions    # every tool's version, built in the Nix sandbox (what CI checks)
just smoke       # example/paper.md through XeLaTeX and pdfLaTeX, in the sandbox (CI)
```

pandoc-crossref is `haskellPackages.pandoc-crossref`, built against the same pandoc as
`pkgs.pandoc`. A release binary of pandoc-crossref is built for one pandoc version, and with a
different pandoc it warns or fails. Here the two always agree.

The smoke check builds a paper with a section reference, a table reference and a citation, with
`--fail-if-warnings`, and then reads the text of each PDF for the resolved references.

## Reusing it from another repository

```nix
{
  inputs.paper.url = "github:h0ffmann/picasso?dir=labs/paper";
  outputs = { nixpkgs, paper, ... }: {
    devShells.x86_64-linux.default = paper.devShells.x86_64-linux.default;
    # or, with the project's own tools:
    # devShells.x86_64-linux.default = nixpkgs.legacyPackages.x86_64-linux.mkShell {
    #   packages = [ /* the project's own tools */ ] ++ paper.lib.x86_64-linux.tools;
    # };
  };
}
```

Without a flake: `nix develop 'github:h0ffmann/picasso?dir=labs/paper'`.

`lib.<system>.tex` is the TeX Live environment alone. `packages.<system>.pandoc-crossref`
re-exports the filter for `nix run`.

## Pandoc 3.1.8 and later

pandoc 3.1.8 and later write citations as `\citeproc{...}` and the reference list with
`\bibitem`. A custom LaTeX template must define these, as pandoc's own `common.latex` does
(`pandoc --print-default-data-file templates/common.latex`, the `csl-refs` block).

## What it pins

| Type        | Program |
| :---------- | :-----: |
| Converter   | [pandoc 3.7](https://pandoc.org/) (citeproc) + [pandoc-crossref 0.3.21](https://github.com/lierdakil/pandoc-crossref) (section, table and figure numbers) |
| TeX         | [TeX Live 2025](https://tug.org/texlive/) `texliveMedium` + fontspec, unicode-math, Latin Modern, csquotes, fancyhdr, titling, xurl, … |
| Engines     | `xelatex` (the PDF) and `pdflatex` (the arXiv source check) |
| Scripting   | python 3, [just](https://github.com/casey/just), git, [gh](https://cli.github.com/) (releases) |
| PDF tools   | [poppler-utils](https://poppler.freedesktop.org/) (`pdfinfo`, `pdftotext`), [librsvg](https://gitlab.gnome.org/GNOME/librsvg) (`rsvg-convert`, SVG images in PDFs) |
| Reuse       | `lib.<system>.{tools, tex}`, `packages.<system>.{pandoc-crossref, tex}`, `checks.<system>.{smoke, versions}` |
