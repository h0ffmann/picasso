<div align="center">

<img alt="A cubist guitar, one line: an original drawing after Picasso" src="site/static/drawings/guitar.svg" width="300"/>

# picasso

<sub>since 2024 · formerly <code>nix-config</code> · <a href="https://h0ffmann.github.io/picasso/">the site</a> · <a href="#history">history</a> · <a href="#releases">releases</a> · <a href="#how-to-cite">cite</a> · <a href="WHITEPAPER.md">whitepaper</a></sub>

[![since 2024](https://img.shields.io/badge/since-2024-FF4F00?style=flat-square)](#history)
[![Nix VM: live](https://img.shields.io/badge/Nix%20VM-live-3FB950?style=flat-square&logo=nixos&logoColor=white)](#vm)
[![NixOS development: soon](https://img.shields.io/badge/NixOS%20development-soon-D29922?style=flat-square&logo=nixos&logoColor=white)](#vm)
[![release](https://img.shields.io/github/v/tag/h0ffmann/picasso?sort=semver&filter=v*&label=release&style=flat-square&color=FF4F00)](#releases)
[![ci](https://github.com/h0ffmann/picasso/actions/workflows/ci.yml/badge.svg)](https://github.com/h0ffmann/picasso/actions/workflows/ci.yml)
[![DOI](https://img.shields.io/badge/DOI-10.5281%2Fzenodo.23311208-1682D4?style=flat-square)](https://doi.org/10.5281/zenodo.23311208)
[![license: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-1682D4?style=flat-square&logo=gnu&logoColor=white)](#license)
[![docs: CC BY 4.0](https://img.shields.io/badge/docs-CC%20BY%204.0-1682D4?style=flat-square&logo=creativecommons&logoColor=white)](#license)

My [Nix](https://nixos.org/) labs: self-contained flakes for the tools I actually work in, on an
Ubuntu workstation (i9 + RTX 4090) running [Determinate Nix](https://determinate.systems/nix/).
Each lab is one directory with its own `flake.nix`, `flake.lock`, `justfile` and README, references
nothing outside itself, and works unchanged as a git submodule of the repositories that consume it.

Built on [Gabriel Volpe's `nix-config`](https://github.com/gvolpe/nix-config), and
[his repository is better anyway](https://github.com/gvolpe/nix-config): read it first.
</div>

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
`gh search code 'h0ffmann/picasso' --owner <owner>`.

## VM

The workstation's NixOS system (`nixosConfigurations.nixos`, nixos-26.05 + Home Manager) in a QEMU
VM, from any Linux with Nix. Log in as `h0ffmann` / `nixos`. `just vm` first prints the pins from
`flake.lock` and the headline versions (linux, nix, claude-code, gh, graphify); the VM's login
message repeats them.

```console
just vm           # build and boot it; Claude Code and gh already logged in, ~/repos cloned
just vm-reset     # delete the VM's disk (nixos.qcow2) and start fresh next time
just vm-uninstall # undo `just vm`: the disk, `result` and every store path only the VM uses
just freeze vm    # every package the VM installs, name==version (pip freeze); `just freeze` for the host
just skills       # the vendored agent skills, and which to ask for to improve this repo
just graphify     # code graph of this repo (build | query "<q>" | path | explain), keyless
```

Without a checkout:

```console
nix run nixpkgs#nixos-rebuild -- build-vm --flake github:h0ffmann/picasso#nixos
./result/bin/run-nixos-vm
```

What `just vm` shares from the host:

| In the VM | From the host | |
|---|---|---|
| `~/nix-config` | this checkout | edit on either side |
| `~/.claude` | `~/.claude` | Claude Code logged in |
| `GH_TOKEN` | `gh auth token` | through qemu's fw_cfg, never a file |
| `~/repos` | `~/repos` (or `$VM_REPOS`) | on boot, clones what is missing: [marola](https://github.com/marola-dev/marola) and its submodules, marola-devkit, [ww3-gpu](https://github.com/h0ffmann/ww3-gpu) and its submodules, the private `h0ffmann/nagori`, and Karpathy's main repos ([nanochat](https://github.com/karpathy/nanochat), [nanoGPT](https://github.com/karpathy/nanoGPT), [llm.c](https://github.com/karpathy/llm.c), [llama2.c](https://github.com/karpathy/llama2.c), [minbpe](https://github.com/karpathy/minbpe), [micrograd](https://github.com/karpathy/micrograd)); never re-clones, survives `just vm-reset` |

The VM gets modesetting instead of the NVIDIA driver, no DaVinci, 8 GB / 4 cores / 20 GB; change
that in `virtualisation.vmVariant` in `configuration.nix`. The first `just vm` downloads about
4.6 GiB and builds the unfree packages the binary cache does not carry (vault and pritunl-client
from Go source; steam, zoom, vscode and obsidian repackaged); later runs reuse the store. `.mcp.json` gives Claude Code the
[mcp-nixos](https://github.com/utensils/mcp-nixos) server for package and option search.

<details>
<summary>Ubuntu users</summary>

Everything here runs on Ubuntu with Nix installed; the VM is optional, and every lab works on the
host directly. Written for Ubuntu 24.04 with Determinate Nix, the setup this repo is used on; the
AppArmor profile in step 4 is not exercised by CI.

1. **Nix, with flakes on.** Determinate Nix turns flakes on and keeps its own settings out of
   `/etc/nix/nix.conf` (custom ones go in `/etc/nix/nix.custom.conf`):

   ```console
   curl -fsSL https://install.determinate.systems/nix | sh -s -- install
   ```

   If any `nix` command then warns `unable to download 'https://cuda-maintainers.cachix.org/...':
   HTTP error 401`, an old `cachix use cuda-maintainers` left that dead cache in your config. This
   lists each file that names it and the command that fixes that file:

   ```console
   nix run 'github:h0ffmann/picasso?dir=labs/cuda#setup-cuda-cache' -- --doctor
   ```

2. **`just` and `gh`.** `sudo apt install just` is enough: every justfile here runs with the
   just 1.21 of Ubuntu 24.04, and CI checks it. Nix gives you newer versions:

   ```console
   nix profile add nixpkgs#just nixpkgs#gh   # `nix profile install` on an older Nix
   gh auth login                             # once; `just vm` hands this token to the VM
   ```

3. **KVM for `just vm`.** Without write access to `/dev/kvm` QEMU falls back to software
   emulation and the VM is unusably slow. `just vm` stops and says so; the fix is:

   ```console
   sudo usermod -aG kvm "$USER"   # then log out and back in
   ```

4. **ai-jail and AppArmor.** Ubuntu 23.10 and later block unprivileged user namespaces, so
   bubblewrap from the Nix store fails with `bwrap: setting up uid map: Permission denied` (the
   `labs/agentic` sandbox, `just jco` in `labs/pratico`, marola's `just jail-claude`). Ubuntu's
   own `bwrap-userns-restrict` profile covers only `/usr/bin/bwrap`; this one grants `userns` to
   the Nix store's bwrap and nothing else:

   ```console
   sudo tee /etc/apparmor.d/nix-bwrap >/dev/null <<'EOF'
   abi <abi/4.0>,
   include <tunables/global>
   profile nix-bwrap /nix/store/*/bin/bwrap flags=(unconfined) {
     userns,
   }
   EOF
   sudo apparmor_parser -r /etc/apparmor.d/nix-bwrap
   ```

5. **marola on the host.** `git clone --recurse-submodules https://github.com/marola-dev/marola`,
   then `nix develop` in it; Docker comes from Ubuntu (`sudo apt install docker.io` and
   `sudo usermod -aG docker "$USER"`), not from Nix. In the VM, `~/repos/marola` is already cloned
   and Docker is already on.

</details>

## prático

The WAVEWATCH III toolchain (used by [ww3-gpu](#who-uses-picasso)), plus a local
zsh pilot and a sandbox for coding agents. See [`labs/pratico`](labs/pratico).

```console
cd labs/pratico
just ww3              # toolchain-only shell — gfortran, OpenMPI, NetCDF, METIS, ecCodes, python
just toolchain        # exact pinned versions, for a paper's methods section
just smoke            # Fortran 2008 + MPI + NetCDF-4 compiled and run in the sandbox (CI)
just kokkos-smoke     # Kokkos + GoogleTest compiled, tested and run in the sandbox (CI)
just cuda             # toolchain shell with Kokkos built for CUDA — x86_64-linux, needs a GPU
just dev              # interactive shell: same toolchain + zsh-ai (Ctrl+O) → llm → Ollama
just jco              # Claude Code inside ai-jail (jcf / jcs for other models, jo for OpenCode)
```

<details>
<summary>Environment details</summary>

| Type              | Program |
| :---------------- | :-----: |
| Fortran / C / C++ | [GCC 15](https://gcc.gnu.org/) (`gfortran`) |
| MPI               | [OpenMPI 5](https://www.open-mpi.org/) |
| NetCDF            | [netcdf-c 4.10](https://www.unidata.ucar.edu/software/netcdf/) + netcdf-fortran, HDF5 |
| Decomposition     | [METIS](https://github.com/KarypisLab/METIS) / ParMETIS, [PT-Scotch](https://gitlab.inria.fr/scotch/scotch) (static, for WW3's PDLIB) |
| WW3 regtest data  | NOAA's `ww3_from_ftp` tarball, SHA-256 pinned (`ww3-regtest-data`) |
| GRIB              | [ecCodes](https://confluence.ecmwf.int/display/ECC) |
| C++ portability   | [Kokkos](https://kokkos.org/) (Serial + OpenMP, CUDA in `just cuda`) + [GoogleTest](https://google.github.io/googletest/), gdb, valgrind |
| Build             | [CMake](https://cmake.org/) / Ninja |
| Analysis          | [nco](https://nco.sourceforge.net/), [cdo](https://code.mpimet.mpg.de/projects/cdo), python (numpy, scipy, xarray, netCDF4, matplotlib) |
| Shell pilot       | [zsh-ai](https://github.com/thiswillbeyourgithub/zsh-ai) → [llm](https://llm.datasette.io/) → [Ollama](https://ollama.com/) (`qwen2.5-coder:7b`) |
| Agent sandbox     | [ai-jail](https://github.com/akitaonrails/ai-jail) (bubblewrap / Landlock / seccomp) |
| Task runner       | [just](https://github.com/casey/just) |

</details>

## publisher

Markdown → LaTeX → PDF, reproducibly, and `mkPdf` / `mkDocx` helpers so other repositories build
their documents in the Nix sandbox with this toolchain. docx is optional and takes the pandoc
writer, no TeX. See [`labs/publisher`](labs/publisher).

```console
cd labs/publisher
just shell            # pandoc, xelatex / pdflatex, python + openai on PATH
just versions         # what is pinned
just smoke            # sample document through both engines, built in the sandbox (CI)
just smoke-docx       # the same sample as .docx, the optional non-TeX output (CI)
```

<details>
<summary>Environment details</summary>

| Type            | Program |
| :-------------- | :-----: |
| Converter       | [pandoc 3.7](https://pandoc.org/) (citeproc, Lua filters) |
| TeX             | [TeX Live 2025](https://tug.org/texlive/) `texliveMedium` + babel-portuges, fontspec, DejaVu, fvextra, titlesec, … |
| Engines         | `xelatex` (books, unicode fonts by filename) and `pdflatex` (T1 templates) |
| Scripting       | python 3 + [openai](https://github.com/openai/openai-python) (translation against any OpenAI-compatible endpoint) |
| PDF tools       | [poppler-utils](https://poppler.freedesktop.org/) (`pdfinfo`, `pdftotext`, `pdftoppm`), [librsvg](https://gitlab.gnome.org/GNOME/librsvg) (`rsvg-convert`, SVG images in PDFs) |
| Badges & emoji  | `filters/shields-badges.lua` → `publisher-badges.sty` (shields.io badges as TikZ pills), [Noto Color Emoji](https://github.com/googlefonts/noto-emoji) via luaotfload fallback (`lualatex`) |
| Japanese        | [luatexja](https://ctan.org/pkg/luatexja) + [Harano Aji](https://github.com/trueroad/HaranoAjiFonts) Gothic/Mincho (`lualatex`) |
| Reuse           | `lib.<system>.{mkPdf, mkDocx, mkDocument}` and the composite GitHub Action `h0ffmann/picasso/labs/publisher@main` |
| docx (optional) | pandoc's docx writer — no TeX, no LaTeX template, styling from a consumer's `--reference-doc` |

</details>

## ptbr-pdf

Markdown → PDF for Portuguese text and nothing else: one Rust binary, `to-pdf`, with Typst as a
library and six fonts inside it. First consumer: the `.pdf` output of
[ptbr-writer](https://github.com/h0ffmann/ptbr-writer). See [`labs/ptbr-pdf`](labs/ptbr-pdf).

| | on disk | download |
| :- | -: | -: |
| before: pandoc + XeLaTeX | 729 MiB | 206 MiB |
| **now: Rust + Typst, static** | **25.2 MiB** | **8.1 MiB** (`xz -9`) |

29× less on disk, 25× less to download; one page converts in 0.2 s. The full benchmark and the
platforms are in [`labs/ptbr-pdf`](labs/ptbr-pdf#benchmark-from-pandoc-to-rust).

```console
nix run github:h0ffmann/picasso?dir=labs/ptbr-pdf -- texto.md
```

## lint

The gate a repository runs before a PR — hadolint, actionlint + shellcheck, ruff, pyflakes,
cloc, coverage.py, pdoc — exported as one list for a consumer's `mkShell`. See
[`labs/lint`](labs/lint).

```console
cd labs/lint
just shell            # the tools on PATH
just versions         # what is pinned, built in the sandbox (CI)
```

<details>
<summary>Environment details</summary>

| Type       | Program |
| :--------- | :-----: |
| Dockerfile | [hadolint](https://github.com/hadolint/hadolint) |
| Workflows  | [actionlint](https://github.com/rhysd/actionlint) + [shellcheck](https://www.shellcheck.net/) for the `run:` blocks |
| Python     | [ruff](https://docs.astral.sh/ruff/), [pyflakes](https://github.com/PyCQA/pyflakes), [coverage.py](https://coverage.readthedocs.io/), [pdoc](https://pdoc.dev/) |
| Counting   | [cloc](https://github.com/AlDanial/cloc) |
| Reuse      | `lib.<system>.tools` (the list), `packages.<system>.<tool>`, `checks.<system>.versions` |

</details>

## paper

Markdown papers to PDF and to arXiv source: pandoc with pandoc-crossref and citeproc, XeLaTeX for
the PDF and pdfLaTeX for the arXiv check, with python, just, git and gh. pandoc-crossref is built
against the same pandoc, so the two always agree. See
[`labs/paper`](labs/paper).

```console
cd labs/paper
just shell            # the tools on PATH
just versions         # what is pinned, built in the sandbox (CI)
just smoke            # a sample paper through XeLaTeX and pdfLaTeX, built in the sandbox (CI)
```

<details>
<summary>Environment details</summary>

| Type        | Program |
| :---------- | :-----: |
| Converter   | [pandoc 3.7](https://pandoc.org/) (citeproc) + [pandoc-crossref 0.3.21](https://github.com/lierdakil/pandoc-crossref) (section, table and figure numbers) |
| TeX         | [TeX Live 2025](https://tug.org/texlive/) `texliveMedium` + fontspec, unicode-math, Latin Modern, csquotes, fancyhdr, titling, xurl, … |
| Engines     | `xelatex` (the PDF) and `pdflatex` (the arXiv source check) |
| Scripting   | python 3, [just](https://github.com/casey/just), git, [gh](https://cli.github.com/) (releases) |
| PDF tools   | [poppler-utils](https://poppler.freedesktop.org/) (`pdfinfo`, `pdftotext`), [librsvg](https://gitlab.gnome.org/GNOME/librsvg) (`rsvg-convert`, SVG images in PDFs) |
| Reuse       | `lib.<system>.{tools, tex}`, `packages.<system>.{pandoc-crossref, tex}`, `checks.<system>.{smoke, versions}` |

</details>

## gleam-website

Static sites in Gleam: the toolchain of picasso's own site (Gleam on the JavaScript target for
Lustre, node, esbuild, just, python), as a lab other repositories borrow. See
[`labs/gleam-website`](labs/gleam-website).

```console
cd labs/gleam-website
just shell            # the tools on PATH
just versions         # what is pinned, built in the sandbox (CI)
just smoke            # a Gleam module to JavaScript, run on node and bundled, in the sandbox (CI)
```

<details>
<summary>Environment details</summary>

| Type       | Program |
| :--------- | :-----: |
| Language   | [Gleam 1.19](https://gleam.run/) on the JavaScript target; sites use [Lustre](https://lustre.build) from Hex |
| Runtime    | [Node.js 24](https://nodejs.org/) (`gleam test`, `gleam run` for the prerender) |
| Bundler    | [esbuild](https://esbuild.github.io/) |
| Scripting  | [just](https://github.com/casey/just), python 3 |
| Reuse      | `lib.<system>.tools`, `packages.<system>.{gleam, esbuild}`, `checks.<system>.{smoke, versions}` |

</details>

## agentic

The sandbox for coding agents and the host-side tools around it: **ai-jail** (bubblewrap /
Landlock / seccomp), **OpenCode**, **Open Code Review** (`ocr`, built from source), **gh**, and
`jail-run` / `gh-token` / `clip` / `clip-relay` — one Rust binary each, with tests that
`nix flake check` runs in the sandbox. See [`labs/agentic`](labs/agentic).

```console
cd labs/agentic
just test             # cargo test, clippy and rustfmt (nix flake check)
just jco              # Claude Code inside ai-jail, in YOUR directory (jcf / jcs for other models, jo for OpenCode)
just jocr             # Open Code Review inside ai-jail: project read-only, no GitHub token
just jail-dry-run ls  # what ai-jail would run
```

<details>
<summary>Environment details</summary>

| Type          | Program |
| :------------ | :-----: |
| Agent sandbox | [ai-jail](https://github.com/akitaonrails/ai-jail) (bubblewrap / Landlock / seccomp, Linux) |
| Agents        | Claude Code (the consumer's own), [OpenCode](https://opencode.ai/) |
| Reviewer      | [Open Code Review](https://github.com/alibaba/open-code-review) v1.12.7, `buildGoModule` from the tag — `jail-run ocr` gives it no token and a read-only project |
| GitHub        | [gh](https://cli.github.com/); `gh-token` resolves the token on the host, `jail-run` forwards it as `GH_TOKEN` |
| Clipboard     | `clip` (write-only, from inside the jail) → `clip-relay` → wl-copy / xclip |
| Reuse         | `lib.<system>.{tools, env, scripts}`, `packages.<system>.{<tool>, ocr}`, `checks.<system>.{tools, clippy, fmt}` |

</details>

## cuda

Host setup for a CUDA box, x86_64-linux only: `setup-cuda-cache` puts the nixos-cuda binary
cache into Determinate Nix's `nix.custom.conf` (replacing the dead Cachix block that 401s), and
`REQUIREMENTS=<file> setup-ml-venv` builds a torch venv from PyTorch's wheel index with the nix
libstdc++ and the NVIDIA driver libraries on its path. See [`labs/cuda`](labs/cuda).

```console
cd labs/cuda
just self-test                       # both --self-tests, from source
just cache-setup                     # the nix.custom.conf block, dry-run; `sudo just cache-setup apply` writes it
just venv path/to/requirements.txt   # the venv; call bin/python-cuda afterwards
```

<details>
<summary>Environment details</summary>

| Type         | Program |
| :----------- | :-----: |
| Binary cache | [cache.nixos-cuda.org](https://cache.nixos-cuda.org) via `extra-substituters` + `trusted-users` |
| torch        | PyTorch's CUDA wheel index (`CUDA_INDEX`, default cu129), never nixpkgs' `torchWithCuda` |
| Wrapper      | `$VENV_ROOT/bin/python-cuda` — nix libstdc++ baked in at build, driver libs linked at run |
| Reuse        | `lib.x86_64-linux.tools`, `packages.x86_64-linux.{setup-cuda-cache,setup-ml-venv}`, `checks.x86_64-linux.*` |

</details>

## present

The desk during a talk or a demo: draw over the screen, screenshot or record it, keep the
webcam looking the same as last time, present from a PDF or a markdown file. Recipes pick
Wayland or X11 by `$WAYLAND_DISPLAY`; `cam` saves and restores V4L2 controls, the one thing
nothing packaged does. macOS gets the terminal half only. See [`labs/present`](labs/present).

```console
cd labs/present
just annotate                        # gromit-mpx: F9 draw, Shift+F9 hide, F10 clear
just shot / just rec [region]        # grim+slurp+satty or flameshot; wf-recorder or ffmpeg x11grab
just cam save meeting                # webcam controls → ~/.config/present/cam/meeting.conf; `cam load` after a replug
just pdf deck.pdf / just md deck.md  # pdfpc with notes and timer / presenterm in the terminal
```

<details>
<summary>Environment details</summary>

| Type       | Program |
| :--------- | :-----: |
| Annotate   | [gromit-mpx](https://github.com/bk138/gromit-mpx), [find-cursor](https://github.com/arp242/find-cursor), [screenkey](https://gitlab.com/screenkey/screenkey) / [wshowkeys](https://git.sr.ht/~sircmpwn/wshowkeys) |
| Capture    | [flameshot](https://flameshot.org/), [grim](https://sr.ht/~emersion/grim/) + [slurp](https://github.com/emersion/slurp) + [satty](https://github.com/gabm/Satty) |
| Record     | [wf-recorder](https://github.com/ammen99/wf-recorder), [kooha](https://github.com/SeaDve/Kooha), [OBS Studio](https://obsproject.com/), [ffmpeg](https://ffmpeg.org/) + [gifski](https://gif.ski/) |
| Webcam     | [v4l-utils](https://linuxtv.org/wiki/index.php/V4l-utils) + `cam` (save / load / reset profiles), [cameractrls](https://github.com/soyersoyer/cameractrls), [guvcview](https://guvcview.sourceforge.net/) |
| Present    | [pdfpc](https://pdfpc.github.io/), [presenterm](https://github.com/mfontanini/presenterm) |
| Reuse      | `lib.<system>.{tools, scripts}`, `packages.<system>.{cam, <tool>}`, `checks.<system>.{cam, versions}` |

</details>

## viz

The code graph of a repository with **graphify** and nothing else: code-only, keyless and offline,
the way marola-devkit's `just graph` runs it. `viz build` writes `graph.json` and `graph.html`, an
interactive picture of the code, under `~/.cache/marola-graph/<repo>`. See [`labs/viz`](labs/viz).

```console
cd labs/viz
just graph build        # graph the directory you are in; prints the path of graph.html
just graph update       # the same graph, with the Markdown pages, headings and links
just graph query "<q>"  # answer from the graph, capped at --budget 400
just smoke              # example/ graphed and queried in the sandbox (CI)
```

<details>
<summary>Environment details</summary>

| Type       | Program |
| :--------- | :-----: |
| Code graph | [graphify 0.9](https://github.com/Graphify-Labs/graphify) (tree-sitter extraction, clustering, `graph.html`), with `PYTHONHASHSEED` pinned |
| Script     | `viz` (build / query / where / any graphify command), with a `--self-test` against a fake graphify |
| Reuse      | `lib.<system>.tools`, `packages.<system>.{viz, graphify, smoke}`, `checks.<system>.{viz, smoke}` |

</details>

## Structure

```
.
├── labs/
│   ├── pratico/        WW3 toolchain, zsh-ai pilot, ai-jail       (flake, lock, justfile, README, tools/)
│   ├── publisher/      pandoc + TeX Live, mkPdf, action.yml       (flake, lock, justfile, README, example/)
│   ├── ptbr-pdf/       Rust + Typst, one to-pdf binary  (flake, lock, justfile, README, example/)
│   ├── lint/           lint toolchain as one list, checks.versions   (flake, lock, justfile, README)
│   ├── paper/          pandoc + crossref, XeLaTeX / pdfLaTeX for papers  (flake, lock, justfile, README, example/)
│   ├── gleam-website/  Gleam (JavaScript) + node + esbuild for sites  (flake, lock, justfile, README, example/)
│   ├── agentic/        ai-jail, OpenCode, ocr, gh, jail-run/gh-token/clip  (flake, lock, justfile, README, tools/)
│   ├── cuda/           CUDA binary cache + torch venv, x86_64 only   (flake, lock, justfile, README, scripts/)
│   ├── present/        annotate, capture, record, webcam `cam`, presenters  (flake, lock, justfile, README, scripts/)
│   └── viz/            graphify: the code graph of a repo, offline  (flake, lock, justfile, README, scripts/, example/)
├── site/               the site, Gleam + Lustre: own flake, `just ci`, deployed to GitHub Pages
├── scripts/            release.sh (`just release`)
├── tools/              Rust crates: citation (`just citation`), zenodo, social (Threads posts,
│                       `just social`; docs/adr/0001), render, site-check, site-releases, sharingan
├── WHITEPAPER.md       the determinism whitepaper in ASD-STE100 (and WHITEPAPER.pdf);
│                       PICASSOPAPER.pdf is the original, coloured paper
├── docs/               media/ (logo), whitepapers/ and studies/ (the sources of the PDFs under Papers),
│                       social/ (social media posts, one directory each, with sources),
│                       adr/ (architecture decision records),
│                       readiness/ (dated assessments against the ouroboric-dev checklist),
│                       rea/ (dated REA reports of the ouroboric-sharingan skill)
├── HUMAN_KNOWLEDGE.md  what the human plans and tracks (SOCIAL PLANNING: networks, cadence)
├── notes/              things worth writing down once (legacy NixOS root, …)
├── .github/workflows/  ci.yml: root evaluates, every lab is built and linted
└── flake.nix, configuration.nix, home.nix, …   legacy NixOS system configuration (see notes/)
```

Every lab also carries a `lab.json` — a one-line summary and the nixpkgs attributes that headline
it — which the profile README at [github.com/h0ffmann](https://github.com/h0ffmann) renders daily
next to the lab's lock date. Each lab's `checks.lab-json` asserts the card's shape when it
evaluates and resolves every listed attribute in the lab's pinned nixpkgs.

## Flake outputs

<details>
<summary>Expand to see available outputs</summary>

```console
$ nix flake show github:h0ffmann/picasso?dir=labs/pratico
├───checks
│   └───x86_64-linux
│       ├───kokkos-smoke: CI test       # Kokkos + GoogleTest smoke test
│       ├───lab-json: CI test           # lab.json's shape and headline versions, pure Nix
│       ├───ptscotch: CI test           # PT-Scotch has every file WW3's FindSCOTCH reads
│       └───toolchain: CI test          # Fortran + MPI + NetCDF-4 smoke test
├───devShells
│   └───x86_64-linux
│       ├───cuda: development environment      # toolchain + Kokkos for CUDA
│       ├───default: development environment   # = pratico
│       ├───pratico: development environment   # toolchain + zsh-ai + ai-jail
│       └───ww3: development environment       # toolchain only
└───packages
    └───x86_64-linux
        ├───ptscotch-static: package    # SCOTCH_PATH: PT-Scotch, static, MPI
        ├───ww3-metis: package          # METIS_PATH: METIS + ParMETIS
        └───ww3-regtest-data: package   # WW3's regtest data, SHA-256 pinned

$ nix flake show github:h0ffmann/picasso?dir=labs/publisher
├───checks
│   └───x86_64-linux
│       ├───docx: CI test                # the sample as .docx, asserted to be a real Word document
│       ├───filter: CI test              # the shields filter's exact \badge macros, listed in flake.nix
│       ├───lab-json: CI test
│       └───smoke: CI test
├───devShells
│   └───x86_64-linux
│       └───default: development environment
├───lib                                  # tex, python, tools, mkPdf, mkDocx, mkDocument, env per system
└───packages
    └───x86_64-linux
        ├───smoke: package
        ├───smoke-docx: package
        └───tex: package                 # the TeX Live environment

$ nix flake show github:h0ffmann/picasso?dir=labs/lint
├───checks
│   └───x86_64-linux
│       ├───lab-json: CI test
│       └───versions: CI test          # every tool's --version, built in the sandbox
├───devShells
│   └───x86_64-linux
│       └───default: development environment
├───lib                                  # tools (the list) per system
└───packages
    └───x86_64-linux
        ├───versions: package
        └───hadolint, actionlint, shellcheck, ruff, pyflakes, cloc, coverage, pdoc: package

$ nix flake show github:h0ffmann/picasso?dir=labs/paper
├───checks
│   └───x86_64-linux
│       ├───lab-json: CI test
│       ├───smoke: CI test             # example/paper.md through both engines, references resolved
│       └───versions: CI test          # every tool's version, built in the sandbox
├───devShells
│   └───x86_64-linux
│       └───default: development environment
├───lib                                  # tools (the list) and tex per system
└───packages
    └───x86_64-linux
        ├───pandoc-crossref: package
        ├───smoke: package
        ├───tex: package                 # the TeX Live environment
        └───versions: package

$ nix flake show github:h0ffmann/picasso?dir=labs/gleam-website
├───checks
│   └───x86_64-linux
│       ├───lab-json: CI test
│       ├───smoke: CI test             # example/: gleam -> JavaScript -> node -> esbuild
│       └───versions: CI test          # every tool's version, built in the sandbox
├───devShells
│   └───x86_64-linux
│       └───default: development environment
├───lib                                  # tools (the list) per system
└───packages
    └───x86_64-linux
        ├───esbuild: package
        ├───gleam: package
        ├───smoke: package
        └───versions: package

$ nix flake show github:h0ffmann/picasso?dir=labs/agentic
├───checks
│   └───x86_64-linux
│       ├───clippy: CI test
│       ├───fmt: CI test
│       ├───tools: CI test              # cargo test of jail-run, gh-token, clip, in the sandbox
│       └───lab-json: CI test
├───devShells
│   └───x86_64-linux
│       └───default: development environment
├───lib                                  # tools, env (BWRAP_BIN), scripts per system
└───packages
    └───x86_64-linux
        ├───ocr: package                 # Open Code Review, from the tagged Go source
        └───gh-token, clip, clip-relay, jail-run: package

$ nix flake show github:h0ffmann/picasso?dir=labs/cuda
├───checks
│   └───x86_64-linux
│       ├───lab-json: CI test
│       ├───setup-cuda-cache: CI test   # the scripts' --self-test, in the sandbox
│       └───setup-ml-venv: CI test
├───devShells
│   └───x86_64-linux
│       └───default: development environment
├───lib                                  # tools per system (x86_64-linux only)
└───packages
    └───x86_64-linux
        └───setup-cuda-cache, setup-ml-venv: package

$ nix flake show github:h0ffmann/picasso?dir=labs/present
├───checks
│   └───x86_64-linux
│       ├───cam: CI test                 # the script's --self-test against a fake v4l2-ctl
│       ├───lab-json: CI test
│       └───versions: CI test            # CLI tools answer --version; no GUI launched
├───devShells
│   └───x86_64-linux
│       └───default: development environment
├───lib                                  # tools (desktop half Linux only), scripts.cam per system
└───packages
    └───x86_64-linux
        ├───cam, versions: package
        └───gromit-mpx, flameshot, grim, slurp, satty, wf-recorder, kooha, obs-studio, v4l-utils, cameractrls, guvcview, pdfpc, presenterm, ffmpeg, gifski, …: package

$ nix flake show github:h0ffmann/picasso?dir=labs/viz
├───checks
│   └───x86_64-linux
│       ├───lab-json: CI test
│       ├───smoke: CI test             # example/ graphed and queried in the sandbox
│       └───viz: CI test               # the script's --self-test against a fake graphify
├───devShells
│   └───x86_64-linux
│       └───default: development environment
├───lib                                  # tools (the list) per system
└───packages
    └───x86_64-linux
        ├───default, viz: package
        ├───graphify: package
        └───smoke: package

$ nix flake show github:h0ffmann/picasso        # legacy root
├───checks.x86_64-linux.build            # NixOS toplevel evaluates
├───devShells.x86_64-linux.default
├───formatter.x86_64-linux
└───nixosConfigurations.nixos
```

`aarch64-linux` and `aarch64-darwin` are declared for the labs too (`--all-systems`).

</details>

<details>
<summary>Using a lab from another repository</summary>

Three ways; [Who uses picasso](#who-uses-picasso) lists the repositories that use each:

```console
# 1. as a flake input (publisher exports lib.<system>.mkPdf; see labs/publisher/README.md)
inputs.publisher.url = "github:h0ffmann/picasso?dir=labs/publisher";

# 2. as a sparse git submodule, only the lab checked out (`nix run ./labs/pratico#ww-lab-submodule`)
nix develop ./picasso/labs/pratico#ww3

# 3. in GitHub Actions (build, upload and commit PDFs in one step)
- uses: h0ffmann/picasso/labs/publisher@main
```

</details>

## Papers

Every PDF in this repository. Each source is next to its PDF, or the table says that there is no source.

| Document | PDF | Source | What it is |
|---|---|---|---|
| Same inputs, same factory (2026) | [PICASSOPAPER.pdf](PICASSOPAPER.pdf) | [docs/whitepapers/determinism-2026/](docs/whitepapers/determinism-2026/) (`whitepaper.html`, `just render paper`) | The determinism whitepaper, coloured original: why reproducibility matters in 2026 and what the labs can do for ouroboric-dev. |
| Same inputs, same factory, ASD-STE100 text | [WHITEPAPER.pdf](WHITEPAPER.pdf) | [WHITEPAPER.md](WHITEPAPER.md) (`just render ste`) | The same paper in Simplified Technical English, plain look. |
| picasso and Determinate Nix (2026) | [study.pdf](docs/studies/determinate-2026/study.pdf) | [docs/studies/determinate-2026/](docs/studies/determinate-2026/) (`study.html`, `just render study`) | A side-by-side study of Determinate Systems and picasso, a 6-month plan, and distribution through marola and ouroboric-dev ([#96](https://github.com/h0ffmann/picasso/issues/96)). |
| Site hosting plan | [hosting-plan.pdf](site/static/plan/hosting-plan.pdf) | none in the repository | The plan for hosting the site ([#85](https://github.com/h0ffmann/picasso/issues/85)); the site links it from its header. |

When you add a PDF, add its row here in the same PR.

## History

Started on 2024-12-30 as `nix-config`, renamed picasso on 2026-10-12. Every change, era by era,
is on [the site](https://h0ffmann.github.io/picasso/) ([source](site/)) (generated by `site/history.nix`, `just history` in `site/`, into `site/static/history.json`).

| Era | When | PRs | What happened |
|---|---|---|---|
| sic mundus | 2024-12 → 2025-02 | [#1](https://github.com/h0ffmann/picasso/pull/1)–[#7](https://github.com/h0ffmann/picasso/pull/7) | A NixOS desktop from scratch: Home Manager, DaVinci Resolve, Obsidian, a Python/uv dev shell; the PRs are named after songs. |
| the workstation | 2025-04 → 2025-06 | [#8](https://github.com/h0ffmann/picasso/pull/8)–[#29](https://github.com/h0ffmann/picasso/pull/29) | Cachix, flake dev shells, a compilation profile, repomix for code quality, aider, Ollama and Bazel, Steam, Haskell, Helix, Python 3.13. |
| the labs | 2026-09 | [#30](https://github.com/h0ffmann/picasso/pull/30)–[#67](https://github.com/h0ffmann/picasso/pull/67) | The system becomes self-contained labs other repos borrow: pratico, publisher and its Action, lint, agentic, cuda, present; CI per lab, `lab.json`, AGENTS.md, the MIT licence. |
| back to NixOS | 2026-10 | [#68](https://github.com/h0ffmann/picasso/pull/68)–[#78](https://github.com/h0ffmann/picasso/pull/78) | nixos-26.05 and Home Manager 26.05, a lighter system, `just vm`, vendored Nix skills, mcp-nixos, graphify, the cuda doctor; then the rename to picasso. |

64 commits on `main`, 62 merged pull requests, +21,276 / −4,385 lines.

## Releases

picasso uses semantic versions, `vMAJOR.MINOR.PATCH`. The git tags are the only record of the
version: no file in the repository holds it. The commit subjects on `main` set the bump.

| Change since the last tag | Bump | Bump before 1.0.0 |
|---|---|---|
| A break in a lab's exported shape (`lib.<system>.tools`, `mkPdf`'s arguments, a script's flags), a `!` after the type | major | minor |
| A new lab, recipe, script, page or workflow (`feat`) | minor | minor |
| A fix, a pin bump or documentation only | patch | patch |

To make a release, do these steps on a clean `main` that is level with `origin/main`. The `just` of
Ubuntu 24.04 (apt) is sufficient. Without `just`, run `scripts/release.sh` with the same arguments.

```console
git switch main && git pull
just release                  # show the unreleased commits and the bump that they need
just release minor --dry-run  # do all the checks, make no tag
just release minor            # tag vX.Y.Z and push the tag (or: patch, major, 0.1.0)
just releases                 # list the release tags
```

The tag starts [`release.yml`](.github/workflows/release.yml), which does these steps:

1. It makes the GitHub release. The notes list the merged PRs by their `type/*` label, and
   `WHITEPAPER.pdf` and `PICASSOPAPER.pdf` are attached.
2. Zenodo archives the release with `.zenodo.json` and gives it a DOI. This occurs only when the
   repository is switched on at zenodo.org/account/settings/github before the tag is pushed.
   Zenodo does not archive an earlier release. v0.1.0 was released before the switch, so it has
   no DOI; v0.1.1 is the first release on Zenodo.
3. It starts [`pages.yml`](.github/workflows/pages.yml) on `main`. The site then shows the new
   version and its changelog, which `tools/site-releases/` makes from the tags.

A Zenodo DOI is permanent. For this reason, `scripts/release.sh` stops if `main` is not clean,
if `main` has no new commits, or if the new version is not more than the last tag.

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
  title     = {{picasso: self-contained Nix flakes for scientific and agentic toolchains, and the NixOS workstation they run on}},
  year      = {2026},
  publisher = {Zenodo},
  doi       = {10.5281/zenodo.23311208},
  url       = {https://github.com/h0ffmann/picasso}
}
```

APA:

> Hoffmann, M. (2026). *picasso: self-contained Nix flakes for scientific and agentic toolchains, and the NixOS workstation they run on* [Computer software]. Zenodo. https://doi.org/10.5281/zenodo.23311208

ABNT (NBR 6023):

> HOFFMANN, Matheus. **picasso**: self-contained Nix flakes for scientific and agentic toolchains, and the NixOS workstation they run on. [S. l.]: Zenodo, 2026. DOI 10.5281/zenodo.23311208.

<!-- citation:end -->

## CI

GitHub-hosted runners only. [`ci.yml`](.github/workflows/ci.yml):

* **NixOS flake evaluates** — `nix flake check --no-build` on the root; nothing is built.
* **labs/<lab>** (matrix) — `nix flake check` with the lock required current: every devShell
  evaluates and every `checks.*` output is **built**; then `nixpkgs-fmt --check`, `statix`,
  `deadnix`, `shellcheck` on every script, justfile parse, and the lab's self-tests when it has
  them. Lint tools come from the lab's own locked nixpkgs.
* **site rules** — in `site/`'s own flake: `nix flake check`, `nixpkgs-fmt`, then `just ci`
  (`gleam format --check`, the unit tests, the build, and `site-check` on the built page: both
  logos hotlinked, Gabriel Volpe credited, the hosting plan linked).
  [`pages.yml`](.github/workflows/pages.yml) runs the same `just ci` and publishes `site/dist`.
* **release** — [`release.yml`](.github/workflows/release.yml), on a pushed `v*` tag: the
  GitHub release, then a new deploy of the site (see [Releases](#releases)).
* **justfiles parse with Ubuntu's just** — on `ubuntu-24.04`, `apt install just` (1.21.0) and
  `just --summary` on every justfile, so `just` works on stock Ubuntu (AGENTS.md hard rule).
* **licences (REUSE)** — `reuse lint`: every file has a licence and a copyright holder, from
  [`REUSE.toml`](REUSE.toml) and [`LICENSES/`](LICENSES).
* **workflows lint** — `actionlint`.
* **profile ping** — [`profile-ping.yml`](.github/workflows/profile-ping.yml), a reusable
  workflow: after a merged PR it sends one `repository_dispatch` (`activity`) to
  [h0ffmann/h0ffmann](https://github.com/h0ffmann/h0ffmann), which rebuilds the profile's
  recent-activity list right away instead of at its daily cron. Callers are one job
  (`profile-activity.yml`) in picasso, marola, ww3-gpu and gcp-agentic-architect; each needs a
  `PROFILE_DISPATCH_TOKEN` secret (fine-grained PAT, Contents: read & write on the profile repo only).

Dependabot bumps the actions monthly.

## Credits

The layout of this README, the idea of one `<details>` table per environment, and more than one
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
