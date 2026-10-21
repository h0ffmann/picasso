# lint

The tools a repository runs before a PR, from one locked nixpkgs: **hadolint** (Dockerfiles),
**actionlint** + **shellcheck** (workflows and the `run:` blocks inside them), **ruff** and
**pyflakes** (Python), **cloc** (line counts), **coverage.py**, **pdoc**. No scripts; the lab
is a list.

```
just shell       # the tools on PATH, in your current directory
just versions    # every tool's --version, built in the Nix sandbox (what CI checks)
```

## Reusing it from another repository

```nix
{
  inputs.lint = { url = "github:h0ffmann/picasso?dir=labs/lint"; inputs.nixpkgs.follows = "nixpkgs"; };
  outputs = { nixpkgs, lint, ... }: {
    devShells.x86_64-linux.default = nixpkgs.legacyPackages.x86_64-linux.mkShell {
      packages = [ /* the project's own tools */ ] ++ lint.lib.x86_64-linux.tools;
    };
  };
}
```

`follows` keeps one nixpkgs closure in the consumer's shell; the price is that this lab's CI
evaluates on its own lock and the consumer on its own, so a tool can differ by a patch version
between the two. `nix flake update lint` in the consumer picks up a new revision of this lab.

`packages.<system>.<tool>` re-exports each tool for `nix run`, and `checks.<system>.versions`
fails the flake check if a nixpkgs bump breaks any of them.

First consumer: [marola](https://github.com/h0ffmann/marola) (`flake.nix` and `ci.yml` there).

## What it pins

| Type       | Program |
| :--------- | :-----: |
| Dockerfile | [hadolint](https://github.com/hadolint/hadolint) |
| Workflows  | [actionlint](https://github.com/rhysd/actionlint) + [shellcheck](https://www.shellcheck.net/) for the `run:` blocks |
| Python     | [ruff](https://docs.astral.sh/ruff/), [pyflakes](https://github.com/PyCQA/pyflakes), [coverage.py](https://coverage.readthedocs.io/), [pdoc](https://pdoc.dev/) |
| Counting   | [cloc](https://github.com/AlDanial/cloc) |
| Reuse      | `lib.<system>.tools` (the list), `packages.<system>.<tool>`, `checks.<system>.versions` |
