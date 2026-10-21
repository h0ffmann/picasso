# rust-agent

The tools to build a Rust CLI that reads official Brazilian weather alerts (INMET, CEMADEN,
Defesa Civil: JSON, XML, CAP and RSS feeds), calls an LLM over Anthropic's HTTP API, and writes
static reports. They come from one locked nixpkgs. Anthropic has no official Rust SDK, so the
agent uses plain HTTPS and JSON. This lab ships the toolchain only. The agent code is in the
consumer.

```
just shell       # the tools on PATH, in your current directory (RUST_SRC_PATH is set)
just versions    # every tool's --version, built in the Nix sandbox (what CI checks)
just smoke       # build, test, clippy and rustfmt the smoke crate in the sandbox (what CI builds)
just test        # cargo test in the smoke crate, in the dev shell
```

## Reusing it from another repository

```nix
{
  inputs.rust-agent = { url = "github:h0ffmann/picasso?dir=labs/rust-agent"; inputs.nixpkgs.follows = "nixpkgs"; };
  outputs = { nixpkgs, rust-agent, ... }: {
    devShells.x86_64-linux.default = nixpkgs.legacyPackages.x86_64-linux.mkShell {
      packages = [ /* the project's own tools */ ] ++ rust-agent.lib.x86_64-linux.tools;
      RUST_SRC_PATH = "${nixpkgs.legacyPackages.x86_64-linux.rustPlatform.rustLibSrc}";
    };
  };
}
```

`follows` keeps one nixpkgs closure in the consumer's shell. The price is that this lab's CI
uses its own lock and the consumer uses its own, so a tool can differ by a patch version.
`nix flake update rust-agent` in the consumer picks up a new revision of this lab.

`packages.<system>.<tool>` re-exports each tool for `nix run`. `checks.<system>.versions` fails
the flake check if a nixpkgs bump breaks a tool. `checks.<system>.smoke` builds and tests a
small std-only crate (`smoke/`) with the pinned toolchain, and runs `clippy -D warnings` and
`rustfmt --check` on it.

No consumer yet. The first one will be [marola-site](https://github.com/marola-dev/marola-site)
(issue 172).

## TLS

Prefer `rustls` in the agent's crates. A static musl build then needs neither `openssl` nor
`pkg-config`. The lab pins both only for crates that need native TLS.

## What it pins

| Type | Program |
| :--- | :-----: |
| Compiler | [rustc and cargo](https://www.rust-lang.org/) |
| Lint and format | [clippy](https://github.com/rust-lang/rust-clippy), [rustfmt](https://github.com/rust-lang/rustfmt) |
| Editor | [rust-analyzer](https://rust-analyzer.github.io/) |
| Tests | [cargo-nextest](https://nexte.st/) |
| Supply chain | [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) (licences, bans), [cargo-audit](https://github.com/rustsec/rustsec) (advisories) |
| Unused crates | [cargo-machete](https://github.com/bnjbvr/cargo-machete) (works on stable; cargo-udeps needs nightly) |
| Native TLS | [pkg-config](https://www.freedesktop.org/wiki/Software/pkg-config/), [openssl](https://www.openssl.org/) |
| Feeds and API | [jq](https://jqlang.github.io/jq/), [xh](https://github.com/ducaale/xh) |
| Reuse | `lib.<system>.tools` (the list), `packages.<system>.<tool>`, `checks.<system>.{lab-json,versions,smoke}` |
