# cuda

Host setup for a CUDA box, as two Rust programs in one dependency-free crate (`tools/`), built by
the lab's flake with `rustPlatform.buildRustPackage`. x86_64-linux only.

| Command | Does |
|---|---|
| `setup-cuda-cache` | appends the [nixos-cuda](https://cache.nixos-cuda.org) binary cache (substituter, public key, `trusted-users`) to `/etc/nix/nix.custom.conf` — the file Determinate Nix `!include`s, since it marks `nix.conf` itself "do not modify" — replacing the dead `cuda-maintainers.cachix.org` block if one is there (it answers 401, which Determinate Nix treats as fatal for every `nix develop`). `--dry-run` prints, `sudo` applies and restarts the daemon, `--remove` takes it out, `--verify` checks that CUDA torch resolves as a fetch, not a build; `--doctor` (`just doctor`) lists every Nix config file that still names the dead cache (system `nix.conf`, `nix.custom.conf`, the user's `~/.config/nix/nix.conf`, `NIX_CONFIG`) with the fix for each, and `sudo … --fix-stale` strips it from `nix.custom.conf` alone, keeping the rest of each line |
| `REQUIREMENTS=<file> setup-ml-venv` | a Python venv at `$VENV_ROOT` (default `~/.ml-venv`) with torch from PyTorch's CUDA wheel index (`CUDA_INDEX`, default cu129) and the rest of the file from PyPI, plus a wrapper `bin/python-cuda` that puts the nix libstdc++ and the NVIDIA driver libraries — only those, never a distro lib dir whose glibc would shadow nix's — on the search path. Call the wrapper, never `bin/python` |

```
just self-test                                   # both tools' cargo tests (CI builds them as checks)
just cache-setup                                 # --dry-run: the block it would write
sudo just cache-setup apply                      # write it, restart nix-daemon
just venv path/to/requirements.txt               # build the venv (VENV_ROOT to move it)
```

## Reusing it from another repository

```nix
{
  inputs.cuda = { url = "github:h0ffmann/picasso?dir=labs/cuda"; inputs.nixpkgs.follows = "nixpkgs"; };
  outputs = { nixpkgs, cuda, ... }: {
    devShells.x86_64-linux.default = nixpkgs.legacyPackages.x86_64-linux.mkShell {
      packages = [ /* the project's own tools */ ] ++ cuda.lib.x86_64-linux.tools;
    };
  };
}
```
On a flake that declares other systems, guard it: `++ lib.optionals (system == "x86_64-linux") cuda.lib.${system}.tools`.

`checks.x86_64-linux.{setup-cuda-cache,setup-ml-venv}` build each tool, which runs its `cargo test` in the
sandbox (`checks.x86_64-linux.clippy` adds `-D warnings`) — the generic halves: no GPU, no root, no network there. The real run (`cuda_available=True`)
is a manual step on the workstation.

First consumer: [marola](https://github.com/h0ffmann/marola) — `just ml-venv` and
`just gpu-cache-setup` there, and its `marola-sea-publish` workflow.
