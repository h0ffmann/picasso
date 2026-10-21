# Benchmark

`sharingan bench all`, run in a cloud session on 2026-10-12 at the `claude` CLI's default model
(no `--model`, no effort pin). Each output case runs in a fresh throwaway workspace (see
`README.md`); "without_skill" is the same agent with no `ouroboric-sharingan` installed.

## History

| Round | Change before it | With skill | Without skill | Trigger |
|---|---|---|---|---|
| 1 | first version (1 run per case) | 0.79 | 0.51 | 20/20 (1 run, first 20 queries) |
| 2 | vendoring licence text and holders are required; reserved paths taken literally; GPL-2.0 text without "or later" is -only; no "copy it anyway" | 0.79 ± 0.27 | 0.59 ± 0.33 | 25/26 (3 runs) |
| 3 | description also covers "copy, paste or port a file from another repo", which is when round 2's GPL-2.0 case did not load the skill | **0.94 ± 0.06** | 0.46 ± 0.32 | **26/26** (3 runs) |

Known gaps in round 3: the org report twice missed that the NC diagram is cite-only, and the
grader wants the REUSE.toml proposal to name AGENTS.md as the reason. In 2 of the 8 runs with
the skill installed, the agent did not call the Skill tool (the docs question) and still passed.

## Round 3 (current)

**Trigger:** 26/26 queries right (3 run(s) each; 0 missed, 0 false triggers).

| Query | Should | Rate |
|---|---|---|
| REA the github.com/charmbracelet org for anything we could reuse in our CLI tooling | yes | 1.00 |
| sharingan https://github.com/anthropics/skills — what can we take from it? | yes | 1.00 |
| go through every public repo in the nix-community org and tell me which CI workflows and justfiles we could borrow, respecting their licences | yes | 1.00 |
| can I copy the release workflow from github.com/foo/bar into this repo? it's GPL-2.0 | yes | 1.00 |
| harvest the useful claude code skills and hooks from wshobson/agents into .claude here, with proper credit | yes | 1.00 |
| I found a nice pre-commit setup in astral-sh/ruff, study it and port what makes sense here without breaking their license | yes | 1.00 |
| vendor the humanizer skill from blader/humanizer into this repo and record where it came from | yes | 1.00 |
| mine the gvolpe/nix-config repo for patterns we could adopt, and tell me how to cite it | yes | 1.00 |
| which of the scripts in github.com/acme/tools have no license? I want to know what's safe to reuse | yes | 1.00 |
| look at the ouroboric-dev org and list the tooling we could borrow for picasso, with licences | yes | 1.00 |
| is it fine to paste the parser from github.com/acme/yaml-lite (Apache-2.0) into our AGPL repo? how do I credit it? | yes | 1.00 |
| port marola-devkit's ponytail skill into this repo with its credit | yes | 1.00 |
| add an MIT LICENSE file to this repository | no | 0.00 |
| bump nixpkgs in labs/lint and commit the lock | no | 0.00 |
| explain the difference between AGPL and GPL in two sentences | no | 0.00 |
| regenerate CITATION.cff from .zenodo.json | no | 0.00 |
| run reuse lint and fix whatever it complains about | no | 0.00 |
| review my PR #112 for bugs | no | 0.00 |
| write a new skill that formats nix files with nixpkgs-fmt | no | 0.00 |
| why does the ci job 'justfiles parse with Ubuntu's just' fail? | no | 0.00 |
| clone github.com/foo/bar and run its test suite | no | 0.00 |
| search the web for the best open-source wave forecasting models | no | 0.00 |
| fork github.com/foo/bar to my account and add it as a remote | no | 0.00 |
| summarise the README of github.com/foo/bar for me | no | 0.00 |
| rename the labs/present lab to labs/stage and fix every reference | no | 0.00 |
| star every repo in the ouroboric-dev org | no | 0.00 |

**Output** (2 run(s) per case and configuration; models: claude-haiku-5-5, claude-sonnet-5-5):

| Configuration | Pass rate | Skill loaded | Seconds | Cost (USD) | Output tokens |
|---|---|---|---|---|---|
| with_skill | 0.94 ± 0.06 | 6/8 | 38.17 ± 22.59 | 0.16 ± 0.08 | 4347.00 ± 3255.31 |
| without_skill | 0.46 ± 0.32 | 0/8 | 29.54 ± 17.88 | 0.11 ± 0.05 | 3085.12 ± 2422.09 |

| Case | With skill | Without skill |
|---|---|---|
| org-report | 8/9, 8/9 | 5/9, 3/9 |
| bring-in-mit-skill | 8/9, 8/9 | 8/9, 6/9 |
| refuse-gpl2-copy | 5/5, 5/5 | 0/5, 0/5 |
| docs-sharealike-nc | 5/5, 5/5 | 3/5, 3/5 |

Failed with the skill:
- org-report: The diagram in acme-docs/docs/diagrams (CC BY-NC) is marked cite only, because NonCommercial is not compatible
- org-report: The diagram in acme-docs/docs/diagrams (CC BY-NC) is marked cite only, because NonCommercial is not compatible
- bring-in-mit-skill: REUSE.toml was left unchanged, and the response proposes the REUSE.toml annotation for a human to approve, citing AGENTS.md
- bring-in-mit-skill: REUSE.toml was left unchanged, and the response proposes the REUSE.toml annotation for a human to approve, citing AGENTS.md
