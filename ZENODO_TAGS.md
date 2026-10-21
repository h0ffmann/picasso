# Zenodo keywords

The keywords in `.zenodo.json`, which Zenodo shows on the record and indexes for search, and which
`CITATION.cff` copies. GitHub topics ([TAGS.md](TAGS.md)) are slugs for GitHub search; these are
the words a researcher types into Zenodo, OpenAIRE or Google Scholar, so they are spelled out and
capitalised the way the field writes them. Keep each one tied to something the repo ships.

| Keyword | Why |
|---|---|
| Nix | every lab is a Nix flake |
| Nix flakes | labs are consumed as flake inputs (`?dir=labs/<lab>`) |
| nixpkgs | labs pin nixos-unstable, the root nixos-26.05 |
| reproducible builds | committed locks; CI builds every lab with `--no-update-lock-file` |
| reproducible research | the point of pinning a model's toolchain |
| development environments | what a lab is, for readers who don't know Nix |
| research software engineering | the discipline this repo is a tool for |
| continuous integration | CI builds every lab's checks in the Nix sandbox |
| GitHub Actions | labs/publisher ships a composite Action |
| WAVEWATCH III | labs/pratico: the toolchain WW3 GPU Lab builds on |
| wave modelling | what WAVEWATCH III does |
| Fortran | labs/pratico: gfortran |
| MPI | labs/pratico: OpenMPI |
| NetCDF | labs/pratico: NetCDF-4 |
| CUDA | labs/cuda and the NVIDIA system config |
| PyTorch | labs/cuda: the torch venv |
| pandoc | labs/publisher |
| LaTeX | labs/publisher: TeX Live, lualatex |
| AI coding agents | labs/agentic: OpenCode, Open Code Review, and the rules in AGENTS.md |
| sandboxing | labs/agentic: `jail-run` runs agents in ai-jail |
| Rust | the repository tools and every lab's tools are Rust crates |
| Typst | labs/ptbr-pdf: Typst as a library inside `to-pdf` |
| deterministic builds | the same inputs give the same toolchain; locks are committed |
| software packaging | a lab packages a toolchain as a flake |
| software distribution | other repositories borrow a lab as a flake input, a submodule or an Action |
| functional package management | Nix, the model under every lab |
| supply chain | pinned inputs by hash, no unpinned download in a lab |
| research software | Zenodo's community term for archived code |

Change them in `.zenodo.json`, then run `just citation` so `CITATION.cff` follows; CI's
`citation` job fails when the two drift.
