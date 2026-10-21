# The VM and Ubuntu

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

## Ubuntu users

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

