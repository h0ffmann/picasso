<div align="center">

<img alt="A cubist guitar, one line: an original drawing after Picasso" src="site/static/drawings/guitar.svg" width="300"/>

# picasso

<sub>since 2024 · formerly <code>nix-config</code> · <a href="https://h0ffmann.github.io/picasso/">the site</a> · <a href="#labs">labs</a> · <a href="#papers">papers</a> · <a href="#releases">releases</a> · <a href="#how-to-cite">cite</a> · <a href="WHITEPAPER.md">whitepaper</a></sub>

[![release](https://img.shields.io/github/v/tag/h0ffmann/picasso?sort=semver&filter=v*&label=release&style=flat-square&color=FF4F00)](#releases)
[![ci](https://github.com/h0ffmann/picasso/actions/workflows/ci.yml/badge.svg)](https://github.com/h0ffmann/picasso/actions/workflows/ci.yml)
[![DOI](https://img.shields.io/badge/DOI-10.5281%2Fzenodo.23311208-1682D4?style=flat-square)](https://doi.org/10.5281/zenodo.23311208)
[![license: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-1682D4?style=flat-square&logo=gnu&logoColor=white)](#license)
[![docs: CC BY 4.0](https://img.shields.io/badge/docs-CC%20BY%204.0-1682D4?style=flat-square&logo=creativecommons&logoColor=white)](#license)

</div>

## In one minute

- picasso is a set of **labs**: self-contained [Nix](https://nixos.org/) flakes that pin scientific,
  publishing and agentic toolchains for other repositories, with deterministic builds, on an
  Ubuntu workstation (i9 + RTX 4090) with [Determinate Nix](https://determinate.systems/nix/).
- A lab is one directory with its own `flake.nix`, `flake.lock`, `justfile`, `lab.json` and README.
  It references nothing outside itself, so other repositories can borrow it: as a flake input, as a
  sparse git submodule, or as a GitHub Action.
- The code is Nix and Rust only (the Gleam sites and one C++ smoke test are the exceptions that
  `AGENTS.md` lists). The repository-wide tools are Rust crates in one workspace, `tools/`.
- The layout of this README comes from [Gabriel Volpe's `nix-config`](https://github.com/gvolpe/nix-config).
  [His repository is better anyway](https://github.com/gvolpe/nix-config): read it first.
- The root files (`flake.nix`, `configuration.nix`, ...) are the NixOS system that this machine used
  to run. They are legacy: see [`notes/legacy-nixos-root.md`](notes/legacy-nixos-root.md). To try
  the system in a VM, or to set up Ubuntu, read [`notes/vm-and-ubuntu.md`](notes/vm-and-ubuntu.md).

## Who uses picasso

Checked on 2026-10-21 against each repository's default branch (`flake.nix`, `flake.lock`,
`.gitmodules`, workflows). A flake input follows `main` until its `flake.lock` moves; the
revision is what the lock (or the submodule) holds now. `nix-config` is picasso's old name: GitHub
redirects it, and the repositories marked so still write it.

| Repository | Lab | How | Where | Pinned at |
|---|---|---|---|---|
| [h0ffmann/ww3-gpu](https://github.com/h0ffmann/ww3-gpu) (was `ww-lab`) | `publisher` | flake input `github:h0ffmann/picasso?dir=labs/publisher` | [`flake.nix`](https://github.com/h0ffmann/ww3-gpu/blob/main/flake.nix) | `52c5772` |
| | `publisher` | Action `uses: h0ffmann/picasso/labs/publisher@main` | [`.github/workflows/pubs.yml`](https://github.com/h0ffmann/ww3-gpu/blob/main/.github/workflows/pubs.yml) | `main` |
| | `gleam-website` | flake input `github:h0ffmann/picasso?dir=labs/gleam-website` | `flake.nix` | `52c5772` |
| | `pratico` | git submodule `picasso/`, sparse (`just submodule-init`) | [`.gitmodules`](https://github.com/h0ffmann/ww3-gpu/blob/main/.gitmodules), [`justfile`](https://github.com/h0ffmann/ww3-gpu/blob/main/justfile) | `4bd3b22` |
| [h0ffmann/h0ffmann](https://github.com/h0ffmann/h0ffmann) (the profile; CV PDF) | `publisher` | flake input `github:h0ffmann/nix-config?dir=labs/publisher` | [`flake.nix`](https://github.com/h0ffmann/h0ffmann/blob/main/flake.nix) | `0d55b07` |
| | `publisher` | Action `uses: h0ffmann/nix-config/labs/publisher@main` | [`.github/workflows/cv.yml`](https://github.com/h0ffmann/h0ffmann/blob/main/.github/workflows/cv.yml) | `main` |
| | `gleam-website` | flake input `github:h0ffmann/picasso?dir=labs/gleam-website` | `flake.nix` | `52c5772` |
| [h0ffmann/closed-source](https://github.com/h0ffmann/closed-source) (Paper Fabric) | `paper` | flake input `github:h0ffmann/picasso?dir=labs/paper` | [`flake.nix`](https://github.com/h0ffmann/closed-source/blob/main/flake.nix) | `2d6f8d8` |
| | `gleam-website` | flake input `github:h0ffmann/picasso?dir=labs/gleam-website` | `flake.nix` | `68723ab` |
| | `viz` | git submodule `forks/picasso` (`just viz` runs `forks/picasso/labs/viz`); ⚠ sparse not checked | [`.gitmodules`](https://github.com/h0ffmann/closed-source/blob/main/.gitmodules), [`justfile`](https://github.com/h0ffmann/closed-source/blob/main/justfile) | `3d60c00` |
| [h0ffmann/nashkell](https://github.com/h0ffmann/nashkell) | `gleam-website`, `paper` | flake inputs `github:h0ffmann/picasso?dir=labs/gleam-website` and `...?dir=labs/paper` | [`site/flake.nix`](https://github.com/h0ffmann/nashkell/blob/main/site/flake.nix) | `52c5772` (both) |
| [marola-dev/marola](https://github.com/marola-dev/marola) | `lint` | flake input `github:h0ffmann/nix-config?dir=labs/lint`, `nixpkgs.follows` | [`flake.nix`](https://github.com/marola-dev/marola/blob/main/flake.nix) | `6421951` |
| | `agentic` | flake input `github:h0ffmann/nix-config/labs/agentic?dir=labs/agentic` (a branch ref) | `flake.nix` | `d42ff99` |
| [marola-dev/marola-research](https://github.com/marola-dev/marola-research) | `paper`, `gleam-website` | flake inputs `github:h0ffmann/picasso?dir=labs/paper` and `...?dir=labs/gleam-website` | [`flake.nix`](https://github.com/marola-dev/marola-research/blob/main/flake.nix) | `52c5772` (both) |
| [h0ffmann/gcp-agentic-architect](https://github.com/h0ffmann/gcp-agentic-architect) | `publisher`, `lint`, `agentic` | flake inputs `github:h0ffmann/nix-config?dir=labs/<lab>` | [`flake.nix`](https://github.com/h0ffmann/gcp-agentic-architect/blob/main/flake.nix) | `461413a` (all three) |
| | `publisher` | Action `uses: h0ffmann/nix-config/labs/publisher@main` | [`.github/workflows/pubs.yml`](https://github.com/h0ffmann/gcp-agentic-architect/blob/main/.github/workflows/pubs.yml) | `main` |
| [h0ffmann/ptbr-writer](https://github.com/h0ffmann/ptbr-writer) | `gleam-website` | flake input `github:h0ffmann/picasso?dir=labs/gleam-website` | [`flake.nix`](https://github.com/h0ffmann/ptbr-writer/blob/main/flake.nix) | `4bd3b22` |
| | `gleam-website` (the site's sources) | git submodule `forks/picasso`; `just pins` fails when it differs from the lock; ⚠ sparse not checked | [`.gitmodules`](https://github.com/h0ffmann/ptbr-writer/blob/main/.gitmodules) | `4bd3b22` |
| | `ptbr-pdf` | `nix run "github:h0ffmann/picasso/$PICASSO_REV?dir=labs/ptbr-pdf"` for the plugin's `.pdf` output; ⚠ only on the unmerged branch `feat/ask-levels` | `plugins/ptbr-writer/scripts/to-pdf.sh` | `dcacdde` |

Labs with no consumer found: `present` (the workstation only). `pratico`, `cuda` and `agentic` are
also used on this workstation. marola does not take `cuda` as an input today. Private repositories
that I could not read are not listed. The same list, repository by repository, comes from

## Labs

Each lab README has the recipes, what the lab pins (`What it pins`) and how to reuse it.

| Lab | What it gives | How to use it |
|---|---|---|
| [`pratico`](labs/pratico) | WAVEWATCH III toolchain (gfortran, OpenMPI, NetCDF), Kokkos, zsh-ai pilot, ai-jail recipes | `just ww3`, `just smoke` |
| [`publisher`](labs/publisher) | pandoc + TeX Live; `mkPdf` and `mkDocx`; an Action that builds and commits PDFs | `just shell`, `lib.<system>.mkPdf`, Action |
| [`ptbr-pdf`](labs/ptbr-pdf) | Markdown to PDF for Portuguese: one Rust binary, `to-pdf`, Typst as a library, six fonts inside | `nix run github:h0ffmann/picasso?dir=labs/ptbr-pdf -- texto.md` |
| [`lint`](labs/lint) | hadolint, actionlint, shellcheck, ruff, pyflakes, cloc, coverage.py, pdoc as one list | `lib.<system>.tools` in your `mkShell` |
| [`paper`](labs/paper) | Markdown papers to PDF and arXiv source: pandoc-crossref, XeLaTeX, pdfLaTeX | `just shell`, `just smoke` |
| [`gleam-website`](labs/gleam-website) | Gleam (JavaScript target), node, esbuild, just: the toolchain of this repo's site | `just shell`, `just smoke` |
| [`agentic`](labs/agentic) | ai-jail, OpenCode, Open Code Review, gh; `jail-run`, `gh-token`, `clip`, `clip-relay` | `just jco`, `just test` |
| [`cuda`](labs/cuda) | nixos-cuda binary cache setup, torch venv with the driver libraries (x86_64-linux only) | `just cache-setup`, `just venv <requirements>` |
| [`present`](labs/present) | Screen annotation, capture and recording, webcam `cam` profiles, pdfpc, presenterm | `just annotate`, `just cam save <name>` |
| [`viz`](labs/viz) | The code graph of a repository with graphify: code-only, keyless, offline | `just graph build` |

`ptbr-pdf` is small on purpose. For the same job, pandoc + XeLaTeX needs 729 MiB on disk (206 MiB
to download); Rust + Typst, static, needs 25.2 MiB (8.1 MiB with `xz -9`). The method and the
platforms are in [`labs/ptbr-pdf`](labs/ptbr-pdf#benchmark-from-pandoc-to-rust).

## tools/

One Cargo workspace. Run a crate with `nix run ./tools#<crate>`.

| Crate | What it does |
|---|---|
| `citation` | Makes `CITATION.cff` and the citation block below from `.zenodo.json`; `-- --check` fails on drift (`just citation`) |
| `zenodo` | Deposits the whitepaper on Zenodo as a record of its own |
| `social` | Checks the social posts in `docs/social/` and sends the posted ones to Threads (`just social`) |
| `render` | Prints the papers to PDF (`just render paper`, `just render ste`, `just render study`) |
| `site-check` | CI guard for the site's hard rules: both logos, Gabriel Volpe's credit, the hosting plan |
| `site-releases` | Writes the site's changelog from the `v*` tags at build time |
| `sharingan` | Licence scan and benchmark for the ouroboric-sharingan skill (`just sharingan-scan`) |

## Using a lab from another repository

[Who uses picasso](#who-uses-picasso) lists the repositories that use each way.

```nix
# 1. a flake input (publisher exports lib.<system>.mkPdf; see labs/publisher/README.md)
inputs.publisher.url = "github:h0ffmann/picasso?dir=labs/publisher";
```

```console
# 2. a sparse git submodule: only the lab is checked out (`nix run ./labs/pratico#ww-lab-submodule`)
nix develop ./picasso/labs/pratico#ww3
```

```yaml
# 3. a GitHub Action: build, upload and commit PDFs in one step
- uses: h0ffmann/picasso/labs/publisher@main
```

Do not break a lab's exported shape (`lib.<system>.tools`, `mkPdf`'s arguments, a script's flags)
without a major bump (a minor bump before 1.0.0): the consumers break on their next `nix flake update`.

## CI

GitHub-hosted runners only. [`ci.yml`](.github/workflows/ci.yml) checks that the legacy root
evaluates; builds every lab (`nix flake check` with the lock required current, every `checks.*`
output built) and lints it (`nixpkgs-fmt`, `statix`, `deadnix`, `shellcheck`, justfile parse,
self-tests); runs the site rules (`just ci` in `site/`: format, tests, build and `site-check`);
parses every justfile with Ubuntu 24.04's `just` 1.21; runs `reuse lint`, `actionlint` and the
`citation` check. [`release.yml`](.github/workflows/release.yml) and
[`pages.yml`](.github/workflows/pages.yml) make the release and deploy the site. After a merged PR,
[`profile-ping.yml`](.github/workflows/profile-ping.yml) tells [h0ffmann/h0ffmann](https://github.com/h0ffmann/h0ffmann)
to rebuild its activity list. Dependabot bumps the actions monthly.

## Papers

Every PDF in this repository. Each source is next to its PDF, or the table says that there is no source.

| Document | PDF | Source | What it is |
|---|---|---|---|
| Same inputs, same factory (2026) | [PICASSOPAPER.pdf](PICASSOPAPER.pdf) | [docs/whitepapers/determinism-2026/](docs/whitepapers/determinism-2026/) (`whitepaper.html`, `just render paper`) | The determinism whitepaper, coloured original: why reproducibility matters in 2026 and what the labs can do for ouroboric-dev. |
| Same inputs, same factory, ASD-STE100 text | [WHITEPAPER.pdf](WHITEPAPER.pdf) | [WHITEPAPER.md](WHITEPAPER.md) (`just render ste`) | The same paper in Simplified Technical English, plain look. |
| picasso and Determinate Nix (2026) | [study.pdf](docs/studies/determinate-2026/study.pdf) | [docs/studies/determinate-2026/](docs/studies/determinate-2026/) (`study.html`, `just render study`) | A side-by-side study of Determinate Systems and picasso, a 6-month plan, and distribution through marola and ouroboric-dev ([#96](https://github.com/h0ffmann/picasso/issues/96)). |
| Site hosting plan | [hosting-plan.pdf](site/static/plan/hosting-plan.pdf) | none in the repository | The plan for hosting the site ([#85](https://github.com/h0ffmann/picasso/issues/85)); the site links it from its header. |

When you add a PDF, add its row here in the same PR.

## Releases

A release is a git tag, `vMAJOR.MINOR.PATCH`; the tags are the only record of the version. The
commit subjects set the bump: a break in a lab's exported shape is major (minor before 1.0.0), a
`feat` is minor, a fix, a pin bump or documentation is patch. Only the human runs `just release`
(`scripts/release.sh`), on a clean `main` level with `origin/main`: each release makes a permanent
Zenodo DOI.

```console
just release                  # show the unreleased commits and the bump that they need
just release minor --dry-run  # do all the checks, make no tag
just release minor            # tag vX.Y.Z and push the tag (or: patch, major)
```

The tag starts [`release.yml`](.github/workflows/release.yml): the GitHub release (with
`WHITEPAPER.pdf` and `PICASSOPAPER.pdf`), the Zenodo archive (only if the repository is switched on
at zenodo.org/account/settings/github before the tag; v0.1.1 is the first release with a DOI) and
[`pages.yml`](.github/workflows/pages.yml), which shows the new version and changelog on the site.

## How to cite

Each release from v0.1.1 is archived on [Zenodo](https://zenodo.org/), which gives it a DOI. Cite
the concept DOI, [10.5281/zenodo.23311208](https://doi.org/10.5281/zenodo.23311208), which always resolves to the latest
version. GitHub's **Cite this repository** button exports APA and BibTeX from
[`CITATION.cff`](CITATION.cff). `just citation` makes that file and the block below from
[`.zenodo.json`](.zenodo.json).

<!-- citation:start -->

BibTeX:

```bibtex
@software{hoffmann_2026_picasso,
  author    = {Hoffmann, Matheus},
  title     = {{picasso: self-contained Nix flakes that pin scientific, publishing and agentic toolchains}},
  year      = {2026},
  publisher = {Zenodo},
  doi       = {10.5281/zenodo.23311208},
  url       = {https://github.com/h0ffmann/picasso}
}
```

APA:

> Hoffmann, M. (2026). *picasso: self-contained Nix flakes that pin scientific, publishing and agentic toolchains* [Computer software]. Zenodo. https://doi.org/10.5281/zenodo.23311208

ABNT (NBR 6023):

> HOFFMANN, Matheus. **picasso**: self-contained Nix flakes that pin scientific, publishing and agentic toolchains. [S. l.]: Zenodo, 2026. DOI 10.5281/zenodo.23311208.

<!-- citation:end -->


**History.** Started on 2024-12-30 as `nix-config`, renamed picasso on 2026-10-12. Every change, era by era, is
on [the site](https://h0ffmann.github.io/picasso/) (`just history` in `site/` writes
`site/static/history.json`).

## Related work

The works that picasso builds on or that study the same problem: functional package management,
deterministic builds and the software supply chain. `.zenodo.json` lists them, so Zenodo indexes them.

- Dolstra, E., Löh, A. (2008). [NixOS: A purely functional Linux distribution](https://doi.org/10.1145/1411204.1411255). ICFP 2008. Journal version with Pierron, A. (2010): [doi:10.1017/S0956796810000195](https://doi.org/10.1017/S0956796810000195).
- Dolstra, E. (2006). [The Purely Functional Software Deployment Model](https://edolstra.github.io/pubs/phd-thesis.pdf). PhD thesis, Utrecht University.
- Dolstra, E., de Jonge, M., Visser, E. (2004). [Nix: A Safe and Policy-Free System for Software Deployment](https://www.usenix.org/legacy/event/lisa04/tech/dolstra.html). LISA 2004.
- Courtès, L. (2013). [Functional Package Management with Guix](https://doi.org/10.48550/arXiv.1305.4584). arXiv:1305.4584.
- Lamb, C., Zacchiroli, S. (2022). [Reproducible Builds: Increasing the Integrity of Software Supply Chains](https://doi.org/10.1109/MS.2021.3073045). IEEE Software.
- Malka, J., Zacchiroli, S., Zimmermann, T. (2024). [Reproducibility of Build Environments through Space and Time](https://doi.org/10.1145/3639476.3639767). ICSE-NIER 2024.
- Malka, J., Zacchiroli, S., Zimmermann, T. (2025). [Does Functional Package Management Enable Reproducible Builds at Scale? Yes](https://doi.org/10.48550/arXiv.2501.15919). arXiv:2501.15919.

## Credits

The layout of this README, the idea of a catalogue of environments, and more than one
habit in these flakes come straight from [Gabriel Volpe's `nix-config`](https://github.com/gvolpe/nix-config).
Thank you, Gabriel. [His repository is better anyway](https://github.com/gvolpe/nix-config): go read it.

**Luck.** People say of an artist that he got lucky. The answer that is credited to Pablo Picasso
is: "Inspiration does exist, but it must find you working." It is the habit these labs are made
for: the work is done every day, and the tools are ready when the idea comes. The saying is
credited to Picasso in print only since 2007, and there is no documentary evidence that he said
it. In 1977 the Spanish poet Vicente Aleixandre, asked for advice to young poets, said
"Inspiration does exist. But you have to find it." Source: Barry Popik,
["Inspiration does exist, but it must find you working"](https://barrypopik.com/blog/inspiration_does_exist_but_it_must_find_you_working),
*The Big Apple*, 27 July 2013, read on 13 October 2026.

Matheus Hoffmann ([@h0ffmann](https://github.com/h0ffmann)), who keeps picasso, is a member of
[ouroboric-dev](https://github.com/ouroboric-dev). picasso follows ouroboric-dev's licence scheme,
and its labs, whitepaper and site are written with ouroboric-dev as a reader.


## License

picasso uses the same licences as the [ouroboric-dev](https://github.com/ouroboric-dev) repositories:

- **Code** (the flakes, the labs and their scripts, `scripts/`, the site's source):
  [AGPL-3.0-only](LICENSE).
- **Documentation and drawings** (the READMEs and other Markdown, `docs/`, `notes/`, the papers, the
  drawings in `site/static/drawings/`): [CC BY 4.0](LICENSES/CC-BY-4.0.txt).

[`REUSE.toml`](REUSE.toml) says which licence and which copyright holder apply to each file, so no
file carries a header; [`LICENSES/`](LICENSES) has every licence text, and CI runs `reuse lint`.

Releases up to and including v0.1.1 ([10.5281/zenodo.23311208](https://doi.org/10.5281/zenodo.23311208))
were published under MIT, and those copies stay under MIT. The AGPL-3.0 and CC BY 4.0 terms apply
from the next release. What is not mine, and under which terms:

| What | Whose | Terms |
|---|---|---|
| The software each lab pins | nixpkgs and upstream projects | their own licences; nothing here vendors their source |
| [`.claude/skills/`](.claude/skills/README.md) | ten community skills, each credited there with its upstream commit | MIT, one Apache-2.0; licence texts in [`.claude/skills/LICENSES/`](.claude/skills/LICENSES) |
| The README layout | [Gabriel Volpe](https://github.com/gvolpe/nix-config) | Apache-2.0 repository; the layout is borrowed, no code was copied |
| The ouroboric-dev readiness items the site quotes | [ouroboric-dev/ouroboric](https://github.com/ouroboric-dev/ouroboric), © 2026 Bruno Guilhermo de Barros Valério | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/), fetched live and shown unchanged; the Ouroboric names and logos are trademarks and are not used as this site's brand |
| The readiness item ids and levels, the assessment layout and the front matter in [`docs/readiness/`](docs/readiness/) and [`docs/rea/`](docs/rea/) | [ouroboric-dev/asgard@1acab20](https://github.com/ouroboric-dev/asgard/blob/1acab205610d3c9d816076bf83baa0b333c4dd08/templates/readiness/checklist.md) (MIT) and [ouroboric-dev/ouroboric@b355b1d](https://github.com/ouroboric-dev/ouroboric/tree/b355b1d52d6906f80447fe84283f705403533d0b/docs) (CC BY 4.0), © 2026 Bruno Guilhermo de Barros Valério | the approach is borrowed and no text was copied; the short quotations are marked and linked |
| The Marola and Ouroboric logos on the site | [marola-dev](https://github.com/marola-dev/marola) and [ouroboric-dev](https://github.com/ouroboric-dev/ouroboric) | trademarks of their projects, loaded from their own repositories to identify them, never copied here |
| The quote on slide 2 of [docs/social/instagram_001/](docs/social/instagram_001/) | Fabio Akita, [akitaonrails.com](https://www.akitaonrails.com/en/2026/09/19/new-ai-language-just-released-bend-2/) | one sentence, quoted with credit and a link; not under this repo's licence |
| Fonts on the site (Fjalla One, Inter, JetBrains Mono) | their designers, served by Google Fonts | SIL Open Font License |
| Fonts inside the `to-pdf` binary ([`labs/ptbr-pdf/fonts/`](labs/ptbr-pdf/fonts)): Libertinus Serif, DejaVu Sans Mono | The Libertinus Project Authors; Bitstream, Inc. and the DejaVu contributors, copied from the typst-assets crate | SIL OFL 1.1 and the Bitstream Vera / DejaVu licence; texts in [`LICENSES/`](LICENSES) |
| The site's bundled JavaScript dependencies ([`site/manifest.toml`](site/manifest.toml)) | [Lustre](https://github.com/lustre-labs/lustre); the Gleam standard library, gleam_json, gleam_regexp and their dependencies | Lustre MIT, the rest Apache-2.0; compiled into the published `app.js`, not vendored here |

The drawings in [`site/static/drawings/`](site/static/drawings) are original works made for this repository,
in homage to Pablo Picasso's one-line drawings. They are not copies of his works, which stay under
copyright until the end of 2043 in most countries (he died in 1973), so none is reproduced here.
"Picasso" names this repository only; it implies no affiliation with or endorsement by the
Picasso estate, and the name is not used for any product or service.
