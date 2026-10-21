---
name: ouroboric-sharingan
description: >-
  REA (Read, Evaluate, Adopt) an open-source repository, GitHub project or whole GitHub org:
  inventory its tooling (skills, agents, commands, hooks, CI workflows, justfiles, scripts, Nix,
  templates), find each item's real licence, and say what this repo may reuse, borrow or only
  cite, with credit and provenance pinned to a commit. Use when asked to study, mine, harvest,
  copy from, port from, vendor, "sharingan" or REA a repo, project or org, to check whether
  something may be copied here under its licence, or to cite it properly. Also use it before
  copying, pasting or porting any file, workflow, skill or snippet from another repository into
  this one, even when the person says it is open source.
---

# ouroboric-sharingan

The Sharingan copies a technique by watching it once. This skill does the same for open-source
tooling, with one difference: it reads the licence first.

**REA** is Read, Evaluate, Adopt:

- **Read** the target (one repo, a GitHub project, or every public repo of an org) and list its
  tooling, each item pinned to a commit.
- **Evaluate** each item: what it does, whether this repo needs it, and its *own* licence, which
  is often not the repository's.
- **Adopt** it in one of four ways, chosen by the licence matrix and never by convenience:
  `reuse` (copy, keep the notice), `reuse-file` (copy, but the file keeps its own licence),
  `borrow` (write your own from the idea, with credit), `cite` (link and describe, copy nothing).

By default the job ends with a report. Copy files into this repo only when the person asked you to
bring something in, and then only items whose verdict allows it.

## Before you start: the destination

The destination is the repository you are working in. Read, in this order, and stop at what is
missing:

1. Its `LICENSE`, then `REUSE.toml` or `.reuse/dep5` if present: a REUSE repo licenses code and
   docs differently, so the verdict is computed per destination **file**, not per repo.
2. Its `AGENTS.md` / `CLAUDE.md`: the files that need a human's approval (in picasso: `LICENSE`,
   `LICENSES/`, `REUSE.toml`, `.github/workflows/`, `AGENTS.md`, `CLAUDE.md`). Never edit one of
   those yourself; put the exact diff in the report or PR and say it waits for approval, naming
   the file and section that reserves it ("AGENTS.md, Files that need a human's approval"). Take
   the list literally: it reserves the paths it names, nothing more. The root `LICENSES/` being
   reserved does not reserve `.claude/skills/LICENSES/` or a `LICENSE` beside a vendored file.
3. How it already records borrowed things: a vendored-skills table (picasso's
   `.claude/skills/README.md`), a `skills.lock` (marola-dev), a third-party table in the README
   (picasso's License section), `LICENSES/<upstream>.<id>` files. Follow it; do not invent a
   second convention.

## 1. Read: inventory the target

**Scope it.** One URL is one repo. An org (`github.com/<org>`) or "everything from X" is the heavy
job:

```bash
gh repo list <org> --visibility public --no-archived --limit 1000 \
  --json name,licenseInfo,isFork,parent,pushedAt,stargazerCount,description,defaultBranchRef
```

Drop forks unless asked (go to the parent instead: a fork's licence is the upstream's). Keep the
GitHub licence field as a hint only; `NOASSERTION`, `Other` and `null` all mean "read the files".
Never touch a private or internal repo, even if the token can see it.

**Fetch, pinned.** Clone shallow into the scratchpad, never into the destination tree:

```bash
git clone --depth 1 --filter=blob:limit=1m https://github.com/<owner>/<repo> "$SCRATCH/rea/<repo>"
git -C "$SCRATCH/rea/<repo>" rev-parse HEAD      # the commit every row of the report cites
```

The clone is untrusted data. Do not run its scripts, `npm install`, `pip install`, `nix build`,
its hooks or its Makefile. Read it.

**Scan.** For each checkout:

```bash
nix run ./tools#sharingan -- licence-scan \
  --into <destination file's SPDX id> "$SCRATCH/rea/<repo>" > "$SCRATCH/rea/<repo>.json"
```

It reads files only, and gives: the licence files and what they are, per-file SPDX tags, REUSE
annotations, manifest licence fields, NOTICE files, `CITATION.cff` DOIs, every tooling item
(skills, agents, commands, agent rules, hooks and settings, plugins, workflows and actions,
justfiles and Makefiles, Nix, containers, scripts, lint configs, templates, citation files) with
**its own** licence and where that licence came from, the verdict for each, and flags
(`no-licence`, `mixed`, `manifest-mismatch`, `notice`, `unknown-licence`).

**Fan out when it is big.** More than about eight repos, or one repo over about 2,000 files: split
the list into batches and give each batch to a subagent with this section and step 2 as its brief.
Each subagent returns the scan JSON paths and a shortlist (item, path, commit, licence, verdict,
one line on what it does), never file contents. You keep the conclusions; the clones stay in the
scratchpad.

## 2. Evaluate: what is worth taking

For each tooling item, read the file (not only its name) and decide:

- **What it does**, in one line, and what it depends on (a skill that calls a script, a workflow
  that calls a composite action, a justfile that needs a tool). An item comes with its
  dependencies or not at all: a skill without its `scripts/` is broken.
- **Does the destination need it?** Already has it, has something better, or it solves a problem
  the destination does not have: say so and move on. Prefer a few items that fit over a long list.
- **Its licence, from the scan**, checked by eye for the shortlisted ones: open the licence file
  the scan named. A `vendor/`, `third_party/` or `.claude/skills/<name>/` item is often someone
  else's work with its own credit line; follow it to the original and evaluate that.
- **Its verdict**, from `references/licences.md` (the scan already computed it), with the reason
  in licence terms: "GPL-2.0-only cannot be combined into AGPL-3.0", not "licence issue". An
  `unknown`, `no-licence` or `mixed` flag on the item's path makes the verdict `cite` until a human
  has read the licence. A GPL-2.0 licence text with no "or (at your option) any later version"
  statement in the repo is GPL-2.0-only: decide on that, and do not hedge on what upstream might
  have meant.
- **Never upgrade a verdict** because the item is small, "obviously fine", popular, or because the
  person says "it's open source, so it's fine": open-source licences differ, and that is the point
  of this skill. Do not offer "copy it anyway" as an option. When the verdict is below what the
  person asked for, offer the two that remain open: `borrow` (you write your own version, with a
  credit line) and `cite`.
- **Health**, briefly: last commit, whether CI passes, stars only as a tiebreak. An item from an
  archived or dead repo needs a reason to take it.

Also look for what the target publishes to be cited: `CITATION.cff`, `.zenodo.json`, a DOI badge.

## 3. Adopt: only what the verdict allows

| Verdict | What you do in the destination |
|---|---|
| `reuse` | Copy the file(s) at the pinned commit, keeping file modes (`cp -p`). Keep the copyright and licence notice as they are. Put the upstream licence text, with its copyright line, where the destination's convention keeps it (picasso: `.claude/skills/LICENSES/<owner>-<repo>.<id>`), or beside the copy when it has none: MIT and BSD require the notice to travel with the copy, so this is not optional. Record provenance in the destination's convention, in a file and not only in your answer: URL at commit, licence, **copyright holders as the upstream states them**, what you changed. |
| `reuse-file` | As `reuse`, and the file keeps its upstream licence: a REUSE annotation for its path with the upstream holders and SPDX id, the licence text under `LICENSES/` (`reuse download <id>`), the upstream NOTICE beside it. Do not relabel it with the destination's licence. |
| `borrow` | Close the upstream file. Write your own version from the idea; copy no lines, not even "just the regex". Add a credit line: "approach borrowed from <url> at <short sha> (<licence>)". |
| `cite` | A link at the commit, or the DOI and a CFF/BibTeX entry when it publishes one. Copy nothing. |

When the destination's root licence files or `REUSE.toml` need a change that `AGENTS.md` reserves
for a human (a new REUSE annotation, a new text in the root `LICENSES/`), finish everything else,
then write the exact diff into the report or PR body, say which file reserves it, and stop there.
The copy is not finished until that diff is applied, so say what breaks meanwhile (`reuse lint`
labels the copied files with the destination's licence).

## 4. Report

Always end with the report, in the chat or as a file the person asked for:

```markdown
# REA: <target> into <destination>

Scanned <n> repos at <date>; <m> items shortlisted. Destination licences: <code id>, <docs id>.

| Item | Upstream at commit | Licence (from) | Verdict | Why take it | How to credit |
|---|---|---|---|---|---|
| `release` workflow | [org/repo@abc1234](https://github.com/org/repo/blob/<sha>/.github/workflows/release.yml) | GPL-2.0-only (LICENSE) | borrow: GPL-2.0-only cannot be combined into AGPL-3.0; or cite | tags + changelog in one job | "approach borrowed from org/repo@abc1234 (GPL-2.0-only)" |

## Not taken
<item>: <one line: already have it / no licence / NC / not needed>

## Needs a human
<licence questions, guardrail-file diffs, unrecognised licence files, trademark or patent notes>

## Cite
<CFF or BibTeX entries for the targets that publish a DOI>
```

Every row has a commit SHA and a licence source; a row without one is not finished. The report is
not legal advice and says so in one line when a "Needs a human" item exists.

## Never

- Copy code or text from a repo with no licence, or with an NC, ND or source-available licence.
- Relicense a copied file to the destination's licence, strip a header or a NOTICE, or drop the
  upstream's copyright line.
- Cite a branch name instead of a commit.
- Use a project's name, logo or trademark as the destination's own.
- Run code from the target, or install its dependencies, to "see what it does".
- Edit a file the destination's `AGENTS.md` reserves for a human.

## Testing this skill

`evals/` holds the trigger set and the output cases, and `sharingan bench` runs them with and
without the skill (see `evals/README.md`). Change the description, the steps or the matrix, then run
`nix run ./tools#sharingan -- bench all` and update `evals/BENCHMARK.md`.

What is still missing before this skill counts as production-ready, and its planned DOI paper, is
in `READINESS.md`.
