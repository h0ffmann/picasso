# gleam-website

The toolchain of picasso's own [site](../../site): **Gleam** on the JavaScript target (for
[Lustre](https://lustre.build) applications), **node**, **esbuild**, **just** and **python 3**. A
site keeps its Hex packages pinned by checksum in its own `manifest.toml`; everything else comes
from this lab. First consumer: [h0ffmann/closed-source](https://github.com/h0ffmann/closed-source)
(the Paper Fabric site).

```
just shell       # the tools on PATH, in your current directory
just versions    # every tool's version, built in the Nix sandbox (what CI checks)
just smoke       # example/: gleam -> JavaScript -> node -> esbuild, built in the sandbox (CI)
```

A site with Hex dependencies (Lustre, gleam_stdlib) needs the Hex registry when it builds, so it
builds in the dev shell, not in the Nix sandbox. The smoke project has no dependencies, so
`checks.smoke` proves the toolchain in the sandbox: it compiles a Gleam module to JavaScript,
runs it on node, bundles it with esbuild, and checks that both print the same page.

## Reusing it from another repository

```nix
{
  inputs.gleam-website.url = "github:h0ffmann/picasso?dir=labs/gleam-website";
  outputs = { nixpkgs, gleam-website, ... }: {
    devShells.x86_64-linux.default = gleam-website.devShells.x86_64-linux.default;
    # or, with the project's own tools:
    # devShells.x86_64-linux.default = nixpkgs.legacyPackages.x86_64-linux.mkShell {
    #   packages = [ /* the project's own tools */ ] ++ gleam-website.lib.x86_64-linux.tools;
    # };
  };
}
```

Without a flake: `nix develop 'github:h0ffmann/picasso?dir=labs/gleam-website'`.

The layout that picasso's `site/` uses, and that a consumer can copy: `src/<app>.gleam` starts a
Lustre application, `src/<app>/prerender.gleam` prints the first render as `dist/index.html`,
`entry.mjs` is esbuild's entry point, and `static/` (with `style.css`) is copied into `dist/`.

## What it pins

| Type       | Program |
| :--------- | :-----: |
| Language   | [Gleam 1.19](https://gleam.run/) on the JavaScript target; sites use [Lustre](https://lustre.build) from Hex |
| Runtime    | [Node.js 24.22.1](https://nodejs.org/en/blog/vulnerability/october-2026-security-releases), the official nodejs.org build pinned by sha256 in `nodejs-bin.nix` until nixpkgs ships it (`gleam test`, `gleam run` for the prerender); `checks.node-version` fails below 24.22.1 |
| Bundler    | [esbuild](https://esbuild.github.io/) |
| Scripting  | [just](https://github.com/casey/just), python 3 |
| Reuse      | `lib.<system>.tools`, `packages.<system>.{gleam, esbuild}`, `checks.<system>.{smoke, versions}` |
