# Evals for ouroboric-sharingan

Two sets, in skill-creator's formats, run by `sharingan bench` (`tools/sharingan`) with the `claude` CLI.

- `trigger_evals.json`: queries that should and should not load the skill. `sharingan bench trigger`
  installs the skill in a throwaway project, runs `claude -p` on each query (3 runs by default) and
  records whether the first tool call is the skill. A query passes when its trigger rate is on the
  right side of 0.5.
- `evals.json`: four output cases. `sharingan bench output` builds a throwaway workspace: the five
  fixture repos in `files/acme/` (each made a git repo with a fixed commit, so the agent has SHAs to
  cite) next to a generated destination that looks like picasso (AGPL-3.0-only code, CC BY 4.0
  docs, a `REUSE.toml`, an `AGENTS.md` that reserves the licence files and workflows for a human, a
  vendored-skills table). It runs each case with the skill installed and without it, then grades
  the answer and the git diff: `expectations` by a second `claude -p` call with no tools, `checks`
  by code (`diff_empty`, `exists`, `unchanged`, `not_identical`).

| Fixture repo | Licence | What it tests |
|---|---|---|
| `acme-skills` | MIT | a skill with a script: reuse, with its notice and provenance |
| `acme-ci` | GPL-2.0 (no "or later") | a workflow and a justfile: incompatible with AGPL-3.0, so borrow or cite |
| `acme-prompts` | none | all rights reserved: cite only |
| `acme-docs` | CC BY-SA 4.0, and CC BY-NC 4.0 in `docs/diagrams/` | ShareAlike stays ShareAlike; NC is cite only; a subdirectory licence beats the root |
| `acme-lint` | Apache-2.0 with NOTICE; `package.json` says MIT; `vendor/gplfmt/` is GPL-3.0 | manifest mismatch, NOTICE, a vendored dir under another licence |

The licence files in the fixtures are headers only, enough for a reader and for
`sharingan licence-scan` to identify them; the fixtures are invented and belong to no real project.

```bash
cargo test --manifest-path tools/sharingan/Cargo.toml      # no model calls
nix run ./tools#sharingan -- bench all             # trigger x3, output x2
nix run ./tools#sharingan -- bench output --ids 3 --runs 3
```

A full run makes 78 trigger calls, 16 agent runs and 16 grader calls (about 3 USD at the CLI's default model in October 2026). Record the summary
it prints in `BENCHMARK.md`, with the date and the CLI version.
