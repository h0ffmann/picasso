# prático

The lab that ww-lab consumes: a **reproducible WAVEWATCH III toolchain** (Fortran / C++ /
MPI / NetCDF, one locked nixpkgs), an interactive shell on top of it with a local zsh pilot
(**zsh-ai → llm → Ollama**, Qwen coder), and the **ai-jail** recipes for running Claude Code /
OpenCode sandboxed.

```
just ww3              # toolchain-only shell — what ww-lab builds WW3 in
just ww3-run <cmd>    # one command inside that shell, e.g. just ww3-run cmake -S model -B build ...
just toolchain        # exact versions of everything (nixpkgs rev, gfortran, OpenMPI, NetCDF, ...)
just smoke            # Fortran 2008 + MPI + NetCDF-4 smoke test in the Nix sandbox (what CI runs)
just kokkos-smoke     # Kokkos + GoogleTest smoke test in the Nix sandbox (what CI runs)
just cuda             # toolchain shell with Kokkos built for CUDA (x86_64-linux)

just dev              # interactive shell: same toolchain + zsh-ai + ai-jail, re-execs into zsh
just pull             # ollama pull qwen2.5-coder:7b (once)
just jco / jcf / jcs  # Claude Code (opus / fable / sonnet) inside ai-jail
just jo               # OpenCode inside ai-jail
```

## The WW3 toolchain (`nix develop .#ww3`)

Everything WW3's CMake build (v6.07+) and the classic `w3_make` path look for, from the one
nixpkgs revision pinned in `flake.lock`, so a build on a laptop, in CI, or on a cluster login
node links the same gfortran / OpenMPI / NetCDF bit for bit:

| Piece | Package | Notes |
|---|---|---|
| Compilers | `gfortran` (GCC 15) | `FC`/`F77`/`F90`/`CC`/`CXX` exported |
| MPI | `openmpi` 5 | `mpifort`, `mpicc`, `mpirun`; oversubscription on for single-node runs |
| NetCDF | `netcdf`, `netcdffortran`, `hdf5`, `zlib`, `curl` | `NETCDF`, `NETCDF_FORTRAN`, `NETCDF_CONFIG`, `WWATCH3_NETCDF=NC4` exported; `nc-config`/`nf-config` on PATH |
| Domain decomposition | `metis`, `parmetis`, `ptscotch-static` | `METIS_PATH` is one prefix with METIS and ParMETIS, because WW3's `FindMETIS`, `FindParMETIS` and `FindGKLIB` read only that variable. `SCOTCH_PATH` is PT-Scotch as static archives with MPI, the five files WW3's `FindSCOTCH` reads. ParMETIS is unfree in nixpkgs; the flake allows exactly that one package |
| GRIB | `eccodes` | forcing and post-processing; NCEPLIBS-g2/w3emc are not packaged, so WW3's own GRIB2 output stays off |
| C++ portability | `kokkos` (Serial + OpenMP), `gtest`, `gdb`, `valgrind` | see [Kokkos, GoogleTest, CMake](#kokkos-googletest-cmake) |
| Build | `cmake`, `ninja`, `gnumake`, `pkg-config`, `perl` | |
| Analysis | `nco`, `cdo`, python with numpy / scipy / xarray / netCDF4 / matplotlib | |

`checks.<system>.toolchain` compiles and runs a WW3-shaped program in the sandbox (MPI
singleton, NetCDF-4 write, `ncdump` read-back) and keeps `versions.txt` next to the output.
`nix flake check` builds it; so does CI on every push. `checks.<system>.ptscotch` builds
PT-Scotch and makes sure that it has each file that WW3's `FindSCOTCH` reads.

### Packages for WW3's regression tests

The lab also gives the inputs of WW3's regression tests as packages. They come from the same
nixpkgs as the shell, so each machine builds WW3 against the same libraries and reads the same
data. A consumer runs the tests; it does not build these inputs itself. CI does not build these
packages: the data is a 327 MB download and `ww3-metis` compiles ParMETIS.

| Package | Contents | Use |
|---|---|---|
| `ptscotch-static` | Scotch 7 with PT-Scotch, static archives, MPI, no compression | `SCOTCH_PATH` in the `ww3` shell; WW3's PDLIB with the SCOTCH switch |
| `ww3-metis` | METIS and ParMETIS in one prefix | `METIS_PATH` in the `ww3` shell; WW3's PDLIB with the METIS switch |
| `ww3-regtest-data` | The data that WW3 keeps out of git (`ww3_from_ftp.v7.14.2.tar.gz` from NOAA, SHA-256 pinned), unpacked as `data_regtests/` | the input files that `model/bin/ww3_from_ftp.sh` copies into `regtests/` |

```
nix build --no-link --print-out-paths ./nix-config/labs/pratico#ww3-regtest-data
```

When WW3 changes `ww3ver` in `model/bin/ww3_from_ftp.sh`, change `version`, `url` and `hash`
of `ww3RegtestData` in `flake.nix` in the same PR as the consumer's WW3 bump.

### Using it from ww-lab

Git cannot submodule a subdirectory, so `ww-lab-submodule` (a Rust binary, `packages.${system}.ww-lab-submodule`) adds nix-config as a
shallow submodule and sparse-checks-out only `labs/pratico`. It is idempotent: one tool for
the first add, for every fresh clone (sparse-checkout is local state), and for bumps.

```
# first time, from anywhere (run it out of a nix-config checkout)
nix run ./labs/pratico#ww-lab-submodule -- ~/code/ww-lab --commit

# fresh clone of ww-lab: materialise the sparse submodule
git clone --recurse-submodules git@github.com:h0ffmann/ww-lab.git && cd ww-lab
nix run ./nix-config/labs/pratico#ww-lab-submodule -- .      # or: git -C nix-config sparse-checkout set --no-cone /labs/pratico/

# move the pin to the latest origin/main and commit it
nix run ./nix-config/labs/pratico#ww-lab-submodule -- . --bump --commit
```

`--branch <name>` tracks a branch instead of `main` (useful before a PR merges); `--url` and
`--path` override the remote and the submodule directory. Then:

```
nix develop ./nix-config/labs/pratico#ww3 --command cmake -S model -B build -DSWITCH=... -DCMAKE_INSTALL_PREFIX=install
nix develop ./nix-config/labs/pratico#ww3 --command cmake --build build -j
just -f nix-config/labs/pratico/justfile toolchain      # record the versions with the results
```

**WW3 itself** goes into ww-lab the same way, through your fork of NOAA-EMC/WW3, with
`ww3-submodule`. The fork is what ww-lab pins; upstream is added as a second
remote inside the submodule so the fork can follow it and pick up upstream pull requests:

```
nix run ./nix-config/labs/pratico#ww3-submodule -- . --commit                  # add h0ffmann/WW3 as ./WW3 (branch develop)
nix run ./nix-config/labs/pratico#ww3-submodule -- . --sync --push --bump --commit   # fork <- upstream/develop, push, re-pin
nix run ./nix-config/labs/pratico#ww3-submodule -- . --pr 1234 --commit        # pin ww-lab to upstream PR #1234 before it merges
```

`--sync` fast-forwards only; if the fork carries its own commits, add `--merge`. `--fork`,
`--upstream`, `--path`, `--branch` override the defaults.

Nix reads the flake from the submodule's git objects, so the sparse checkout does not affect
it. Fetching the flake through a git URL rather than a path needs `?submodules=1`. If ww-lab
only needs the shell and not the justfile on disk, skip the submodule entirely and pin
`github:h0ffmann/picasso?dir=labs/pratico` as an input of ww-lab's own flake.

## Kokkos, GoogleTest, CMake

The C++ side, for kernels written against [Kokkos](https://kokkos.org/) rather than raw CUDA.
nixpkgs' `kokkos` is Serial-only with its (slow) upstream test suite on, so the flake pins the
same source with the backends turned on and the tests off:

| Piece | Pin | Where |
|---|---|---|
| Kokkos | 5.2.0, `Serial` + `OpenMP`, C++20 | `#ww3`, `#pratico` (as `kokkos-openmp`) |
| Kokkos | 5.2.0, `Serial` + `OpenMP` + `CUDA`, built through Kokkos' `nvcc_wrapper`, `Kokkos_ARCH_ADA89` — Kokkos takes one GPU arch per build, so an H100 needs `Kokkos_ARCH_HOPPER90` in its place | `#cuda` only, x86_64-linux (as `kokkos-cuda`; `nvcc` on PATH, `CUDACXX` exported) |
| Unit tests | `gtest` 1.18.0 | every shell |
| Debugging | `gdb` 17.2, `valgrind` 3.27.1 | every shell |
| Build | `cmake` 4.4.2, `ninja` | every shell (already in the WW3 toolchain) |

Both are found the ordinary way — `find_package(Kokkos REQUIRED)` and `find_package(GTest
REQUIRED)` from a CMake project with `LANGUAGES CXX`, with no `Kokkos_DIR` / `GTest_DIR` to
set: the shells put both prefixes on the search path nixpkgs' CMake reads
(`NIXPKGS_CMAKE_PREFIX_PATH`). Two things to know:

- Kokkos' config does `find_dependency(OpenMP)`, so it resolves from a real project only, not
  from `cmake -P` script mode, which has no enabled language.
- In `#cuda`, Kokkos refuses a plain `g++`; configure with
  `-DCMAKE_CXX_COMPILER=$(command -v nvcc_wrapper)` (the wrapper is installed by `kokkos-cuda`
  and is on `PATH` in that shell).
- `#cuda` needs an **installed NVIDIA driver** to *run* what it builds. `libcuda.so.1` only ever
  comes from the driver; `cuda_cudart` ships a stub that links and then fails with
  `cudaErrorStubLibrary`. The shell hook probes `/run/opengl-driver/lib`,
  `/usr/lib/x86_64-linux-gnu` and `/usr/lib64`, symlinks the NVIDIA libraries alone into
  `${XDG_CACHE_HOME:-~/.cache}/pratico/nvidia-libs` and puts that on `LD_LIBRARY_PATH` — the
  whole distro lib directory must never go there, because its glibc would shadow the nix one
  the shell's binaries were linked against (the same reasoning as `labs/cuda`'s
  `setup-ml-venv`). Without a driver the shell still works, says so, and only compiles.

```
just kokkos-smoke     # configure smoke/kokkos, run its GoogleTest suite through ctest, run the binary
just cuda             # the same toolchain with the CUDA build of Kokkos (needs an NVIDIA driver to run)
```

The same smoke project builds and runs on the GPU from `#cuda`:

```console
$ nix develop .#cuda
$ cmake -S smoke/kokkos -B /tmp/b -DCMAKE_CXX_COMPILER="$(command -v nvcc_wrapper)" && cmake --build /tmp/b
$ /tmp/b/smoke
kokkos 5.2.0 backend=Cuda sum=14.440160
```

`checks.<system>.kokkos-smoke` is that smoke project — `smoke/kokkos/{CMakeLists.txt,smoke.cpp,
smoke_test.cpp}`: a `parallel_reduce` on the default host backend plus a GoogleTest case with
Kokkos initialised from a `::testing::Environment`. It keeps what the binary printed in
`smoke.txt`. The `#cuda` shell is deliberately not a check: CI has no GPU, and nothing there
would exercise a `nvcc` build of Kokkos.

Consumer: [ww3-gpu](https://github.com/h0ffmann/ww3-gpu) builds its `kokkos/` tree in `#ww3`
and `#cuda`.

## The interactive shell (`nix develop`, `just dev`)

The toolchain plus zsh, fzf, just, `llm` + `llm-ollama`, and ai-jail. Your real `~/.zshrc`
loads the plugin when the shell exports the path:

```zsh
[[ -n "$ZSH_AI_PLUGIN" ]] && source "$ZSH_AI_PLUGIN"
```

Inside the shell, press `^o` on a natural-language prompt line and zsh-ai asks the local
model for command suggestions (fzf picks one). `LLM_MODEL` / `ZSH_AI_LLM_NAME` are set to
the same model so `llm` and the plugin agree.

## ai-jail

`.ai-jail` is the committed project policy (it can only tighten). Per-machine trust goes in
`~/.ai-jail`. Recipes forward the host's GitHub token (`gh-token`) because
`~/.config/gh` is deliberately not mapped into the jail. `PRATICO_JAIL_CLIPBOARD=1` starts a
write-only clipboard relay so `just clip` works from inside.

## Layout

Self-contained on purpose: no reference to the parent flake, every recipe anchored on
`justfile_directory()`, so it works unchanged with nix-config as a git submodule.

The host-side tools are one Rust crate, `tools/`, one binary each: `gh-token`, `clip`,
`clip-relay`, `ww-lab-submodule` and `ww3-submodule` (`packages.${system}.<tool>`, all five on one
`PATH` as `packages.${system}.tools`, which `nix develop` puts in front). `checks.${system}.{tools,
tools-clippy, tools-fmt}` run the tests (against a fake `gh` and throwaway git repositories),
clippy and rustfmt. `clip` finds the relay's FIFO under `$PRATICO_ROOT/.tmp` (the `clip` recipe
sets it to this directory), else the current directory.
