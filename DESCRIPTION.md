# picasso (formerly nix-config), for the LinkedIn Projects section

Self-contained Nix flakes ("labs") that pin scientific, publishing and agentic toolchains for other repositories, with deterministic builds. Ten labs: WAVEWATCH III (gfortran, OpenMPI, NetCDF), pandoc + TeX Live publishing with its own GitHub Action, Markdown to PDF for Portuguese in one Rust + Typst binary (25.2 MiB static, against 729 MiB for pandoc + XeLaTeX), Markdown papers for arXiv, a Gleam toolchain for static sites, a lint set, a sandbox for coding agents, CUDA and PyTorch host setup, a presenter's desk, and a code graph of any repository. The code is only Nix and Rust. Any repo can borrow a lab as a flake input, a submodule or an Action, and CI builds each one on every push. It is the toolchain layer under WW3 GPU Lab, marola and ptbr-writer. Inspired by gvolpe/nix-config.

## One line (the GitHub "About" box)

Self-contained Nix flakes that pin scientific, publishing and agentic toolchains: deterministic builds, only Nix and Rust.

Apply it:

```console
gh repo edit h0ffmann/picasso \
  --description "Self-contained Nix flakes that pin scientific, publishing and agentic toolchains: deterministic builds, only Nix and Rust." \
  --homepage "https://github.com/h0ffmann/picasso"
```

The citation title and abstract live in `.zenodo.json` (`CITATION.cff` is generated from it);
change them together with this page when what the repo is changes.
