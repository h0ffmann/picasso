# agentic

The sandbox for coding agents, and the host-side tools around it: **ai-jail**
(bubblewrap / Landlock / seccomp, Linux), **OpenCode**, **`ocr`** ([Open Code
Review](https://github.com/alibaba/open-code-review) v1.12.7, Apache-2.0, built here with
`buildGoModule` from the tag), **gh**, and four tools, one Rust binary each (the crate is
`tools/`, built by `buildRustPackage` and wrapped with the runtime tools on `PATH` for consumers):

| Tool | Does |
|---|---|
| `jail-run claude [args]` / `jail-run opencode [args]` | the agent inside ai-jail, from the current directory, with the host's GitHub token forwarded as `GH_TOKEN`; `jail-run --dry-run -- <cmd>` prints the sandbox line and runs nothing |
| `jail-run ocr [args]` | Open Code Review in the same jail on *reviewer* terms: no GitHub token, no agent state, the project read-only, one writable directory (`$JAIL_OCR_OUT`, which is also the jail's `HOME`). `JAIL_DRY_RUN=1` prints the sandbox line for any mode |
| `gh-token` / `gh-token --source` | the token a jail should get: `GH_TOKEN`, then `GITHUB_TOKEN`, then the host's `gh auth token`. `~/.config/gh` is never mapped into a jail, so a login inside it dies with the sandbox — authenticate once, on the host |
| `clip` | write-only clipboard push from inside the jail (or outside); no paste counterpart, by design |
| `clip-relay <fifo>` | the host half of that bridge; `jail-run` starts it when `JAIL_CLIPBOARD=1` |

```
just shell            # the tools on PATH
just test             # cargo test (against fakes on PATH), clippy and rustfmt: nix flake check
just jco / jcf / jcs  # Claude Code (opus / fable / sonnet) inside ai-jail — in YOUR directory ([no-cd])
just jo               # OpenCode inside ai-jail
just jocr             # Open Code Review inside ai-jail
just jail-dry-run ls  # what ai-jail would run for `ls`
```

Two opt-ins, both off by default: `JAIL_CLIPBOARD=1` starts a write-only clipboard relay the
jail reaches through `<project>/.tmp/clip.fifo` (the project directory is what ai-jail maps in,
so the fifo has to live there); `JAIL_CLIPBOARD_PASTE=1` passes the real X11/Wayland display
through, which lets Claude Code's own image paste work but also lets the jail *read* the
clipboard (and, on X11, other windows). `--exec` — direct execution, no PTY proxy — is what keeps
Ctrl+C and bracketed paste working; the proxy owned the raw terminal and relayed neither.

## Reviewing code with `ocr`

Open Code Review reads a diff and asks an LLM about it. It authors nothing, so it runs with less
than an agent does. From the repository being reviewed, with Ollama on the host:

```console
export OCR_LLM_URL=http://127.0.0.1:11434/v1/chat/completions
export OCR_LLM_MODEL=qwen2.5-coder:7b
export JAIL_OCR_OUT="$(mktemp -d)"
jail-run ocr review --from main --to HEAD --format json --output "$JAIL_OCR_OUT/ocr.json"
```

`JAIL_DRY_RUN=1` in front of that prints the sandbox line and starts nothing. What the mode does
differently from `claude` / `opencode`, in ai-jail's own flags:

| Flag | Why |
|---|---|
| `env -u GH_TOKEN -u GITHUB_TOKEN ai-jail …` | the token is removed from the child's environment, not merely left off the flags — a reviewer that cannot push cannot be talked into pushing |
| `--clean --private-home --no-agent-state` | no `~/.claude`, no host home dotdirs, and the **reviewed repo's own `.ai-jail` is ignored**: otherwise the code under review would be choosing the reviewer's mounts (marola's asks for `~/.claude` rw) |
| `--mask .env --mask '.env.*' --mask '*.pem' --mask '*.key'` | `--clean` drops the project's `mask` list too, so the secret patterns are restated — a masked file cannot end up in a prompt |
| `--map "$PWD"` | the project **read-only**. ai-jail always binds it rw first; a `--map` of the same path lands after it, and its `--landlock-ro-path` follows, so bwrap and Landlock agree |
| `--rw-map "$JAIL_OCR_OUT" --env HOME=$JAIL_OCR_OUT` | the one writable directory. OCR keeps all of its state under `$HOME/.opencodereview` (`config.json`, `sessions/`, `raw/`, `rule.json` — `os.UserHomeDir()`, no XDG variable), so pointing `HOME` there is what keeps it out of the real home |
| `--env OCR_LLM_URL …` | `OCR_LLM_URL`, `OCR_LLM_MODEL`, `OCR_LLM_TOKEN`, `OCR_USE_ANTHROPIC`, and only the ones actually set on the host |
| `--network` | it has to reach the model |

**The network is all or nothing.** `--network` is unrestricted: this jail can reach whatever the
host can, not only `127.0.0.1:11434`. ai-jail (rev `126a67e`) has no loopback-only mode —
`--allow-tcp-port` is refused outright ("disabled because UDP cannot be isolated"), and
`--lockdown`, which does deny TCP, replaces the mount set with `/proc`, `/dev`, `/tmp` and drops
`--rw-map`, so it cannot be combined with an output directory. Keeping the source on the machine
is a property of the *endpoint* you configure, not of the sandbox.

`ocr` is built, not fetched: `buildGoModule` from tag `v1.12.7` (annotated, dereferenced to
`85cecfe`) with a real `vendorHash`, because this lab's nixpkgs has Go 1.26 and the module only
asks for 1.25.5. Its own test suite is skipped — it reaches a live LLM endpoint and github.com.
Like ai-jail, it is evaluated but not built by `nix flake check`, so no Go compile lands on a CI
runner; `nix develop` or `nix build .#ocr` builds it.

## The project's `.ai-jail`

Committed per project, an untrusted layer that can only tighten; per-machine trust goes in
`~/.ai-jail`. The policy every consumer of this lab uses today:

```toml
command = ["claude"]
rw_maps = ["~/.claude", "~/.claude.json"]
mask = [".env", ".env.*", "*.pem", "*.key"]
network = true
terminal_passthrough = true
```

## Reusing it from another repository

```nix
{
  inputs.agentic = { url = "github:h0ffmann/picasso?dir=labs/agentic"; inputs.nixpkgs.follows = "nixpkgs"; };
  outputs = { nixpkgs, agentic, ... }:
    let system = "x86_64-linux"; in {
      devShells.${system}.default = nixpkgs.legacyPackages.${system}.mkShell ({
        packages = [ /* the project's own tools */ ] ++ agentic.lib.${system}.tools;
      } // agentic.lib.${system}.env);   # BWRAP_BIN on Linux
    };
}
```

and the consumer's recipes are one line each:

```just
jail-claude *args:
    jail-run claude {{args}}
jail-ocr *args:
    jail-run ocr {{args}}
jail-dry-run *cmd:
    jail-run --dry-run -- {{cmd}}
```

`lib.${system}` exposes `tools` (the list, `ocr` included), `env` (`BWRAP_BIN`, Linux) and
`scripts` (the four packages); `packages.${system}.<tool>` and `packages.${system}.ocr` for
`nix run`; `checks.${system}.{tools,clippy,fmt}` run the crate's tests in the sandbox against fakes
of `gh` and `ai-jail` on `PATH`, then clippy and rustfmt.

First consumer: [marola](https://github.com/h0ffmann/marola). `labs/pratico` still carries its
own copies of these tools and recipes; making it consume this lab is a follow-up.
