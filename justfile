# load environment variables from .env file
set dotenv-load
# no recipe attributes or syntax newer than just 1.21, the version Ubuntu 24.04 ships (AGENTS.md)
set positional-arguments

# recipes
default:
    @just --list


# rebuild
rb:
    @echo "Rebuilding NixOS configuration..."
    sudo nixos-rebuild switch --flake "$(pwd)"


history:
    nix profile history --profile /nix/var/nix/profiles/system


# format the nix files in this repo
fmt:
    nix fmt


# format the nix files in this repo
gc:
    sudo nix-collect-garbage -d


# boot the workstation in a QEMU VM; shares this checkout, ~/.claude, ~/repos and gh's token
vm:
    #!/usr/bin/env bash
    set -euo pipefail
    # without KVM, QEMU falls back to TCG and the desktop is unusable; on Ubuntu the fix is the kvm group
    if [ ! -w /dev/kvm ] && [ -z "${VM_NO_KVM:-}" ]; then
      echo "just vm: /dev/kvm is not writable; run: sudo usermod -aG kvm \"\$USER\", log in again (VM_NO_KVM=1 boots anyway, slowly)" >&2
      exit 1
    fi
    flake="$(just _flake)"
    nix eval --raw "$flake#lib.vmBanner"
    nix build "$flake#nixosConfigurations.nixos.config.system.build.vm"
    mkdir -p "$HOME/.claude"
    export VM_REPOS="${VM_REPOS:-$HOME/repos}"
    mkdir -p "$VM_REPOS" && touch "$VM_REPOS/.vm-repos"
    args=()
    if command -v gh >/dev/null && gh auth token >/dev/null 2>&1; then
      exec 3< <(gh auth token)
      args=(-fw_cfg name=opt/gh-token,file=/dev/fd/3)
    fi
    # Ubuntu's GTK_MODULES names canberra-gtk-module, which QEMU's nixpkgs GTK cannot load
    env -u GTK_MODULES VM_NIX_CONFIG="$PWD" VM_CLAUDE_DIR="$HOME/.claude" ./result/bin/run-nixos-vm "${args[@]}"


# code graph of this repo, the marola-devkit way: build | query "<q>" | path <a> <b> | explain <x>
graphify cmd="build" *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    shift
    out="${XDG_CACHE_HOME:-$HOME/.cache}/marola-graph/nix-config"
    bin="$(nix build --no-link --print-out-paths "$(just _flake)#graphify")/bin/graphify"
    # code-only, no API key, no model: the LLM pass over docs is what costs tokens
    g() { env -i PATH="$PATH" HOME="$out" GRAPHIFY_OUT="$out" GRAPHIFY_NO_AUTO_REFRESH=1 \
      OLLAMA_BASE_URL=http://127.0.0.1:9 "$bin" "$@"; }
    case "{{cmd}}" in
      build) mkdir -p "$out"
             g extract . --code-only --no-label
             g cluster-only . --no-label --graph "$out/graph.json" ;;
      query) g query "$@" --graph "$out/graph.json" --budget 400 ;;
      *) g "{{cmd}}" "$@" --graph "$out/graph.json" ;;
    esac


# the flake ref for this checkout: a shallow clone (ww3-gpu's submodule) has no revCount,
# and nix's git fetcher refuses it unless the ref says ?shallow=1
_flake:
    @if [ "$(git rev-parse --is-shallow-repository)" = true ]; then echo "git+file://$PWD?shallow=1"; else echo .; fi


# pip freeze for the system (or `just freeze vm`): name==version of every package, nothing downloaded
freeze target="system":
    #!/usr/bin/env bash
    set -euo pipefail
    cfg="$(just _flake)#nixosConfigurations.nixos.config"
    [ "{{target}}" = vm ] && cfg="$cfg.virtualisation.vmVariant"
    # parseDrvName splits "graphify-0.9.74"; glue derivations (dummy-*, *.sh) have no version
    nix eval --raw "$cfg" --apply 'c: let
      ps = c.environment.systemPackages ++ c.home-manager.users.h0ffmann.home.packages;
      ds = builtins.filter (d: d.version != "") (map (p: builtins.parseDrvName (p.name or "")) ps);
    in builtins.concatStringsSep "\n" (map (d: "${d.name}==${d.version}") ds) + "\n"' | sort -u


# delete the VM's disk (nixos.qcow2); the next `just vm` starts from a fresh one
vm-reset:
    rm -f nixos.qcow2


# undo `just vm`: its disk, its out-link and every store path only the VM uses (~/repos stays)
vm-uninstall:
    #!/usr/bin/env bash
    set -euo pipefail
    # the VM `result` points at is the one the last `just vm` built, whatever the checkout says now
    if [ -L result ] && [[ "$(readlink result)" == *-nixos-vm ]]; then vm="$(readlink result)"
    else vm="$(nix eval --raw "$(just _flake)#nixosConfigurations.nixos.config.system.build.vm.outPath")"; fi
    rm -f nixos.qcow2
    [ "$(readlink result 2>/dev/null)" = "$vm" ] && rm result
    if ! nix path-info "$vm" >/dev/null 2>&1; then echo "vm-uninstall: no VM in the store"; exit 0; fi
    # only paths nothing else keeps alive: the host's own profiles and builds are never touched
    mapfile -t paths < <(comm -12 <(nix-store -qR "$vm" | sort) <(nix-store --gc --print-dead 2>/dev/null | sort))
    [ "${#paths[@]}" -gt 0 ] || { echo "vm-uninstall: every path of the VM is still in use elsewhere"; exit 0; }
    size="$(nix path-info --size "${paths[@]}" | awk '{s+=$2} END {printf "%.1f GiB", s/2^30}')"
    read -rp "delete ${#paths[@]} store paths ($size)? [y/N] " ok
    [ "$ok" = y ] || exit 0
    # a path the daemon finds alive after all (a running process holds it) fails the call part-way:
    # keep the paths still valid, drop the live one with its closure, try again
    while ! out="$(nix store delete "${paths[@]}" 2>&1)"; do
      n="${#paths[@]}"
      live="$(grep -o "/nix/store/[^' ]*" <<<"$out" | head -1)"
      mapfile -t paths < <(comm -23 <(printf '%s\n' "${paths[@]}") \
        <( (nix-store --check-validity --print-invalid "${paths[@]}"; nix-store -qR "$live" 2>/dev/null) | sort -u))
      [ "${#paths[@]}" -gt 0 ] || { echo "vm-uninstall: done"; exit 0; }
      [ "${#paths[@]}" -lt "$n" ] || { echo "$out" >&2; exit 1; }
    done
    tail -1 <<<"$out"


# list the vendored agent skills and the ones to ask for to improve this repo
skills:
    @echo "Skills (.claude/skills), vendored and own:"
    @awk -F'|' '/^\| `/ { gsub(/[ `]/, "", $2); gsub(/^ +| +$/, "", $6); printf "  %-22s %s\n", $2, $6 }' .claude/skills/README.md
    @echo
    @echo "Ask for these by name to improve this repo:"
    @sed -n '/^## On demand/,/^Licence/p' .claude/skills/README.md | grep '^- ' | sed 's/^- /  /; s/`//g'


# ouroboric-sharingan's licence and tooling scan of local checkouts: `just sharingan-scan --into AGPL-3.0-only <dir>...`
sharingan-scan *args:
    python3 -I .claude/skills/ouroboric-sharingan/scripts/licence_scan.py {{ args }}


# benchmark ouroboric-sharingan with the claude CLI: `just sharingan-bench all`, `--self-test` makes no model calls
sharingan-bench *args:
    python3 .claude/skills/ouroboric-sharingan/scripts/bench.py {{ args }}


# regenerate CITATION.cff from .zenodo.json (`just citation --check` to only verify)
citation *args:
    python3 scripts/citation.py {{ args }}


# Threads posts from docs/social (docs/adr/0001): `just social check`, `just social post <dir> --sha <sha> --dry-run`
social *args:
    nix run ./tools#social -- "$@"


# renew the 60-day Threads token straight into the repo secret; the token is never printed
threads-refresh:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${THREADS_ACCESS_TOKEN:?set THREADS_ACCESS_TOKEN to the current token (e.g. in .env)}"
    nix run ./tools#social -- refresh | gh secret set THREADS_ACCESS_TOKEN --repo h0ffmann/picasso


# tag main and publish: `just release` lists what is unreleased, then `just release minor|patch|major|X.Y.Z [--dry-run]`
release *args:
    scripts/release.sh "$@"


# the release tags, newest first
releases:
    git fetch -q --tags origin && git tag -l 'v*' --sort=-v:refname --format='%(refname:short)  %(creatordate:short)  %(subject)'


# restore audio after sleep
audio:
    systemctl --user restart pipewire pipewire-pulse wireplumber


# m down
ydla id:
    yt-dlp https://www.youtube.com/watch?v={{id}} --extract-audio


# dump main files, usefull for LLM debugging
mix:
    repomix
    @cat nixos.md | xclip -selection clipboard
    @echo "Contents of nixos.md copied to clipboard"

# Code quality analysis with custom instruction file
mixcq:
    @cp repomix.config.json code-quality.config.json
    @sed -i 's/"instructionFilePath": "repomix-instruction.md"/"instructionFilePath": "code-quality-instruction.md"/g' code-quality.config.json
    @repomix --config code-quality.config.json
    @cat nixos.md | xclip -selection clipboard
    @echo "CODE QUALITY: Contents of nixos.md copied to clipboard"
