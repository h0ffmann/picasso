---
title: Graduation as an RSI app, by ouroboric-dev — assessment 2026-10-21
status: draft
date: 2026-10-21
related: [2026-10-13-adoption-by-ouroboric-dev.md, ../rea/2026-10-13-ouroboric-dev.md, "https://github.com/ouroboric-dev/asgard/blob/3e614b4e7ad986b9764d1e6bf5a0cd10b769f00c/templates/readiness/checklist.md", "https://github.com/ouroboric-dev/ouroboric/blob/e403734df962c73d1debdf00d1773c807f0b2ba6/docs/1-company/charter.md", "https://github.com/ouroboric-dev/yggdrasil/blob/dcaad27d4a856cbb56f51abbca17c5e2e714da0b/docs/1-product/roadmap.md"]
style: direction
---

# Graduation as an RSI app, by ouroboric-dev — assessment 2026-10-21

**Bottom line: picasso does not graduate yet: it passes no level of the checklist and it is a partial, human-in-the-loop self-improving system, not an RSI app.** Of 35 items, 5 are met, 23 are partial, 6 are missing and 1 is not applicable. The harness, the checks and the release guard work; the loop that turns a measurement into a change is run by hand, has no held-out measure, and has no guard against a weaker checker.

An agent wrote this assessment. The human did not review it yet. A line marked ⚠ is not verified.

## What "graduation" means here

The term is not in the ouroboric-dev repositories. A search (`git grep -i graduat`) in the 13 public repositories, at the commits below, found no match. The 2026-10-13 assessment found the same in 11 repositories. The organisation has no stage called graduation. So this document defines it, and the definition is the owner's framing and not theirs:

> **Graduation** = picasso, as a self-improving app with an agent harness, meets the readiness checklist and the three conditions that the Ouroboric charter gives to an "ouroboric" product.

The charter says what an "ouroboric" product is, in three conditions ([lines 26-32](https://github.com/ouroboric-dev/ouroboric/blob/e403734df962c73d1debdf00d1773c807f0b2ba6/docs/1-company/charter.md#L26-L32)): it improves itself through its own loop, it documents itself, and it never weakens its own guardrails while it improves. The SI items (self-improvement; two letters, because the site reads only ids like `OP-3`) use those three conditions, plus the harness and the loop around it.

The parts of picasso that this assessment treats as the app:

- the **harness**: [`AGENTS.md`](../../AGENTS.md), [`CLAUDE.md`](../../CLAUDE.md), [`.claude/settings.json`](../../.claude/settings.json), [`.claude/commands/`](../../.claude/commands/ste100.md), [`.claude/skills/`](../../.claude/skills/README.md), [`labs/agentic`](../../labs/agentic);
- its **checks**: [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml) and `citation.yml`, `nix flake check`, the [`tools/`](../../tools/Cargo.toml) tests, `site-check`;
- its **feedback loops**: [`notes/`](../../notes/), [`docs/adr/`](../adr/README.md), [`site/static/history.json`](../../site/static/history.json), the release with Zenodo ([`scripts/release.sh`](../../scripts/release.sh)), the A2A issues;
- its **self-change**: the agents' own change to the rules, for example the move to Nix and Rust only in [53372f0](https://github.com/h0ffmann/picasso/commit/53372f0e4fe8fbb8d3f17e7ec455e3849e034cf9) (#129).

## What the organisation defines now

All links are pinned to the default-branch commit of 2026-10-21. I read them again at that commit. This document links to them and does not copy them.

| Repository | Commit |
|---|---|
| ouroboric | `e403734` |
| asgard | `3e614b4` |
| yggdrasil | `dcaad27` |
| gleipnir | `18756c2` |

- Levels: unchanged. Level 1 is [lines 21-45 of the checklist](https://github.com/ouroboric-dev/asgard/blob/3e614b4e7ad986b9764d1e6bf5a0cd10b769f00c/templates/readiness/checklist.md#L21-L45), level 2 is [lines 47-58](https://github.com/ouroboric-dev/asgard/blob/3e614b4e7ad986b9764d1e6bf5a0cd10b769f00c/templates/readiness/checklist.md#L47-L58), level 3 is [lines 60-67](https://github.com/ouroboric-dev/asgard/blob/3e614b4e7ad986b9764d1e6bf5a0cd10b769f00c/templates/readiness/checklist.md#L60-L67). Since `1acab20` the file changed only in the table separators. The 21 items RB-01 to RB-21 are the same.
- Evidence rule: "Give evidence (a link) for each item, or mark the item missing." ([line 14](https://github.com/ouroboric-dev/asgard/blob/3e614b4e7ad986b9764d1e6bf5a0cd10b769f00c/templates/readiness/checklist.md#L14)).
- New since 2026-10-13: a *realm* readiness checklist, RR-01 to RR-22, for a leaf repository with a `pack.yaml` ([realm-readiness.md](https://github.com/ouroboric-dev/yggdrasil/blob/dcaad27d4a856cbb56f51abbca17c5e2e714da0b/docs/2-engineering/realm-readiness.md)). picasso is not a realm, so it is not assessed against it. ⚠ Whether ouroboric-dev would treat a lab as a realm is not stated anywhere I found.
- "Self-improving": "Build open, self-improving factories" is the mission ([charter, line 21](https://github.com/ouroboric-dev/ouroboric/blob/e403734df962c73d1debdf00d1773c807f0b2ba6/docs/1-company/charter.md#L21)). The self-hosting stages are in [spec-driven-development.md, lines 243-263](https://github.com/ouroboric-dev/ouroboric/blob/e403734df962c73d1debdf00d1773c807f0b2ba6/docs/4-ways-of-working/spec-driven-development.md#L243-L263).
- "Harness": [gleipnir](https://github.com/ouroboric-dev/gleipnir/blob/18756c25e920e4f348fa958762df8ff4e2469ee3/README.md#L3) is the name of their harness (agent adapters, sandbox driver, policies, check runner). No file defines "RSI" (`git grep -iE 'recursive self|\bRSI\b'` found nothing).
- Skills and evals: `havamal` holds skills "with their evals" ([README line 3](https://github.com/ouroboric-dev/havamal/blob/33c0d249c254d2bc8d45d02eb3d17f9ef3887422/README.md#L3)); `mimir` holds the benchmark tasks.

## Summary

| Status | Count | Items |
|---|---|---|
| met | 5 | RB-08, RB-16, SI-8, SI-11, SI-12 |
| partial | 23 | RB-02, RB-04, RB-05, RB-06, RB-07, RB-09, RB-11, RB-12, RB-13, RB-14, RB-15, RB-19, RB-20, RB-21, SI-1, SI-2, SI-3, SI-4, SI-6, SI-7, SI-9, SI-10, SI-14 |
| missing | 6 | RB-03, RB-10, RB-17, RB-18, SI-5, SI-13 |
| n/a | 1 | RB-01 |

Level 1 is not passed (as on 2026-10-13: 2 met, 14 partial, 4 missing, 1 n/a). Levels 2 and 3 are not assessed, because each level gates the next one. For comparison, the ouroboric repository's own level-1 assessment says all 21 items are met ([bottom line, line 11](https://github.com/ouroboric-dev/ouroboric/blob/e403734df962c73d1debdf00d1773c807f0b2ba6/docs/1-company/readiness/2026-10-11-ready-to-build.md#L11)). ⚠ I did not recount its table.

## Items

Questions are in picasso's own words. Evidence is a path at the picasso commit `53372f0`; every path was checked to exist. The RB statuses come from [2026-10-13](2026-10-13-adoption-by-ouroboric-dev.md), unless the row says it moved. ⚠ I did not re-run the GitHub checks (settings, domains, credits) for the RB rows that did not move.

| ID | Question | Status | Evidence | To close |
|---|---|---|---|---|
| RB-01 | Company charter accepted? | n/a | picasso is one person's repository; the Ouroboric charter does not name it. | — |
| RB-02 | Product charter accepted and conforming? | partial | [`PHILOSOPHY.md`](../../PHILOSOPHY.md), [`DESCRIPTION.md`](../../DESCRIPTION.md); no charter with status and measures. The Focus boundary of their charter now reads differently ([line 94](https://github.com/ouroboric-dev/ouroboric/blob/e403734df962c73d1debdf00d1773c807f0b2ba6/docs/1-company/charter.md#L94)); ⚠ conformance is not checked. | the human writes and accepts a charter |
| RB-03 | Design document of milestone 1 accepted? | missing | none; [`docs/adr/`](../adr/README.md) has one record. | the human decides |
| RB-04 | Roadmap written? | partial | goals are issues with the label `type/goal` (#66, #82, #95, #100, #104); no roadmap file. | a roadmap file |
| RB-05 | Principles as `AGENTS.md` invariants? | partial | moved: [`AGENTS.md`](../../AGENTS.md) now has 10 lines with "hard rule" (5 on 2026-10-13), among them "Only Nix and Rust", "English", the justfile rule and the approval table. The heading `## Invariants` is still absent (`grep -c` gives 0). | the human: `AGENTS.md` is a guardrail file |
| RB-06 | Name checked? | partial | unchanged; #79 is open. | the human |
| RB-07 | Brand seed written? | partial | unchanged; logo in [`docs/media/`](../media/). | a story and naming document |
| RB-08 | Licence chosen for all public files? | met | [`LICENSE`](../../LICENSE), [`REUSE.toml`](../../REUSE.toml); CI job `licences (REUSE)` in [`ci.yml`](../../.github/workflows/ci.yml). ⚠ `reuse lint` not re-run today. | — |
| RB-09 | Contribution terms in place? | partial | unchanged (guide inherited from a fork of their `.github`). | own text, the setting, an agreement |
| RB-10 | Ownership clear? | missing | no statement from the owner. An agent cannot judge this. | the human confirms |
| RB-11 | Shape decided? | partial | unchanged; own domain open (#85). | the human decides |
| RB-12 | Repository exists and follows the docs standard? | partial | ruleset `main-protect` is active (squash only, code-owner review required). `AGENTS.md` has 340 lines (327 before); `docs/` is not the numbered tree. | the human decides which differences stay |
| RB-13 | Budgets and caps set? | partial | unchanged; the skill benchmark costs about 3 USD per run (#116), no cap recorded. | record a cap |
| RB-14 | Dedicated credentials, no secrets, secret scanning? | partial | unchanged; no gitleaks hook, no scan of history recorded. | a hook and one recorded scan |
| RB-15 | Standards accepted? | partial | `AGENTS.md` is the standard (lab contract, commit format, ASD-STE100). Ouroboric's standards are not accepted. | the human decides |
| RB-16 | Research recorded? | met | [`WHITEPAPER.md`](../../WHITEPAPER.md), [`docs/studies/`](../studies/determinate-2026/README.md), [`notes/`](../../notes/). | — |
| RB-17 | Journal started? | missing | none. [`site/static/history.json`](../../site/static/history.json) is commits, not challenges and results. Their repository has 34 journal files at the pinned commit. | the human decides; then one entry for each challenge |
| RB-18 | Implementation plan accepted? | missing | none. | a plan, after the two sides agree |
| RB-19 | Technical risks identified, tested first? | partial | unchanged; [`WHITEPAPER.md`](../../WHITEPAPER.md) §7 and §8. | the first measurement of §7 |
| RB-20 | Measure of success fixed in advance? | partial | moved: the CI gate grew. `ci.yml` now has the jobs `skills self-tests`, `tools (Rust)` and `site rules`, and `citation.yml` has `validate`. Still no frozen brief or hidden tests. | fix the measure (see SI-2) |
| RB-21 | Minimal visual identity defined? | partial | unchanged; [`docs/media/`](../media/). | a short identity document |
| SI-1 | Does it improve itself through its own loop, measured run against run? Condition 1 of the charter. | partial | the loop runs by hand: [`BENCHMARK.md`](../../.claude/skills/ouroboric-sharingan/evals/BENCHMARK.md) has three rounds, each a change by a person then a measure; the runner is [`tools/sharingan/src/bench.rs`](../../tools/sharingan/src/bench.rs). The automatic loop is only a plan: issue #116. | build #116 (analyse, propose, prove), and a person merges |
| SI-2 | Is there at least one change, made after a run, that improved a metric? Their v0 design asks for the same ([lines 73-75](https://github.com/ouroboric-dev/yggdrasil/blob/dcaad27d4a856cbb56f51abbca17c5e2e714da0b/docs/1-product/design/2026-10-11-yggdrasil-v0-design.md#L73-L75)). | partial | `BENCHMARK.md` round 2 to round 3: with-skill score from 0.79 to 0.94, trigger from 25/26 to 26/26. One skill, 1 to 3 runs per case, default model. ⚠ Not rerun by me. | repeat on a second skill or lab, with the CLI version recorded |
| SI-3 | Is the measure fixed in advance, with a held-out part the improver cannot see? | partial | [`evals.json`](../../.claude/skills/ouroboric-sharingan/evals/evals.json) and [`trigger_evals.json`](../../.claude/skills/ouroboric-sharingan/evals/trigger_evals.json) are public to the agent that edits the skill. #116 plans a tuning and held-out split; not built. Their model uses hidden suites in a private repository. | the split in #116 |
| SI-4 | Is a change to the guardrails a hard stop for a human? Condition 3. Their rule: "`guardrail-change` is a hard approval point." ([line 259](https://github.com/ouroboric-dev/ouroboric/blob/e403734df962c73d1debdf00d1773c807f0b2ba6/docs/4-ways-of-working/spec-driven-development.md#L259)) | partial | [`AGENTS.md`](../../AGENTS.md) "Files that need a human's approval" names the files; [`.github/CODEOWNERS`](../../.github/CODEOWNERS) is `* @h0ffmann` and the ruleset requires code-owner review. The table is prose: no check marks a pull request that touches a listed file. A person must see the diff. | a CI job that labels such a pull request (a change to `.github/workflows/`, so the human asks for it) |
| SI-5 | Does the test suite refuse to shrink silently? Their rule: "The security suite can't shrink silently." ([line 261](https://github.com/ouroboric-dev/ouroboric/blob/e403734df962c73d1debdf00d1773c807f0b2ba6/docs/4-ways-of-working/spec-driven-development.md#L261)) | missing | no count of tests or `site-check` rules is kept. AGENTS.md says "Never make a check pass by removing it", as a rule only. | a check that fails when the number of tests or rules goes down |
| SI-6 | Is the checker different from the code that it checks? Their rule: "Never build in place." ([line 257](https://github.com/ouroboric-dev/ouroboric/blob/e403734df962c73d1debdf00d1773c807f0b2ba6/docs/4-ways-of-working/spec-driven-development.md#L257)) | partial | CI runs the tools from the same tree as the change (`nix run ./tools#social`, `nix build ./tools#checks…` in [`ci.yml`](../../.github/workflows/ci.yml)). Only `tools/site-check/` is in the approval table; `tools/sharingan` (the licence scanner) and `tools/citation` are not. | the human decides which crates join the table, or CI runs the checker from `main` |
| SI-7 | Is there a harness: instructions, permissions, sandbox, hooks, commands, skills? Their harness is gleipnir ([README line 3](https://github.com/ouroboric-dev/gleipnir/blob/18756c25e920e4f348fa958762df8ff4e2469ee3/README.md#L3)). | partial | instructions: [`AGENTS.md`](../../AGENTS.md), [`CLAUDE.md`](../../CLAUDE.md); command: [`.claude/commands/ste100.md`](../../.claude/commands/ste100.md); 11 skills in [`.claude/skills/`](../../.claude/skills/README.md); sandbox: [`labs/agentic`](../../labs/agentic) (`jail-run`). [`.claude/settings.json`](../../.claude/settings.json) only enables the `nixos` MCP server: no hooks, no permission list. The sandbox is a rule in AGENTS.md, not forced on each session. | hooks and permissions, proposed by the agent and approved by the human (settings are a guardrail file) |
| SI-8 | Do checks run by machine on every change? Their principle: "Checks, not promises." ([line 38](https://github.com/ouroboric-dev/ouroboric/blob/e403734df962c73d1debdf00d1773c807f0b2ba6/docs/1-company/charter.md#L38)) | met | [`ci.yml`](../../.github/workflows/ci.yml): `nixos-eval`, `labs/*`, `ubuntu-just`, `reuse`, `social`, `skills`, `tools`, `actionlint`, `site`; [`citation.yml`](../../.github/workflows/citation.yml). On `main` (`53372f0`): 20 check runs `success`, 1 not finished (`labs/ptbr-pdf`, in progress when I looked). | — (finish `labs/ptbr-pdf`) |
| SI-9 | Does it document itself like what it makes? Condition 2. | partial | README per lab, [`notes/`](../../notes/), [`docs/adr/`](../adr/README.md) (one record), [`docs/studies/`](../studies/determinate-2026/README.md), REA reports in [`docs/rea/`](../rea/2026-10-13-ouroboric-dev.md). No journal (RB-17); one ADR only. | the journal, and an ADR for each hard rule |
| SI-10 | Do lessons from a failure return to the rules? | partial | `AGENTS.md` ends with the rule to update the file when something is wrong; the rules grew (approval table 2026-10-12 in 61ce6eb; Nix-and-Rust and English rules in 53372f0). ⚠ Not shown that the human asked for each of these edits by name, as AGENTS.md requires. No place records the failure that caused a rule. | the journal entry for each rule, with the failure and the commit |
| SI-11 | Do the humans steer: agents cannot merge, release or spend? Their principle: "Humans steer, agents build." ([line 36](https://github.com/ouroboric-dev/ouroboric/blob/e403734df962c73d1debdf00d1773c807f0b2ba6/docs/1-company/charter.md#L36)) | met | [`AGENTS.md`](../../AGENTS.md): "An agent never makes or pushes a release tag"; Safety rules; the A2A "Trust" rule; the ruleset needs a code-owner review; issues carry `status/needs-human`. | — |
| SI-12 | Is a release guarded and traceable? | met | [`scripts/release.sh`](../../scripts/release.sh) (clean `main`, version must rise, self-test), [`release.yml`](../../.github/workflows/release.yml) (tag on `main`, `citation --check`), [`tools/zenodo`](../../tools/zenodo/Cargo.toml), [`.zenodo.json`](../../.zenodo.json). Last tag is `v0.1.1`; ⚠ the release that Zenodo makes was not run. | — |
| SI-13 | Does a release stop if it regresses on the measures? Their rule: "No release that regresses on benchmarks." ([line 263](https://github.com/ouroboric-dev/ouroboric/blob/e403734df962c73d1debdf00d1773c807f0b2ba6/docs/4-ways-of-working/spec-driven-development.md#L263)) | missing | no gate in `release.sh` or `release.yml` reads a benchmark. The skill benchmark runs by hand and outside CI. | a gate that reads `BENCHMARK.md` or a result file, after SI-3 |
| SI-14 | Is the channel to ouroboric-dev working (A2A issues)? | partial | the rule is in [`AGENTS.md`](../../AGENTS.md) and [`LABEL.md`](../../LABEL.md). `gh label list` in picasso shows no label with "ouroboric", and no issue has "A2A" in the title. `ouroboric-dev/contact` gave HTTP 404 (⚠ private, renamed or absent). | the labels, and a first `[A2A]` issue |

## What moved since 2026-10-13

- **Counts of the 21 RB items did not move**: 2 met, 14 partial, 4 missing, 1 n/a. The repository work since then was in labs and tooling, not in records.
- **Hard rules**: "Only Nix and Rust" and "English" in [53372f0](https://github.com/h0ffmann/picasso/commit/53372f0e4fe8fbb8d3f17e7ec455e3849e034cf9); the approval table dates from 2026-10-12 (before the first assessment). RB-05 stays partial.
- **Tools workspace**: [`tools/`](../../tools/Cargo.toml) is one Rust workspace (citation, render, sharingan, site-check, site-releases, social, zenodo); CI builds it with tests, `clippy -D warnings` and `cargo fmt --check`. The `tools (Rust)` job is in `ci.yml`.
- **Checks**: the main branch had 16 check runs on 2026-10-13 and has 21 now.
- **7 commits** reached `main` after `9c7f99f`: the REA/readiness docs (#123), a note on a saying credited to Picasso (#124), Zenodo identifiers (#125), PT-Scotch and METIS (#126), vale, shellcheck, lychee (#127), ptbr-pdf (#128) and the Rust move (#129).
- **Their side**: Go code and CI workflows now exist in 8 of 13 repositories (A2 of the old document no longer holds), and the repositories still contain no Nix file. ⚠ Counted with `git ls-files` on a shallow clone. Their charter's Focus line changed to "Yggdrasil, with its Asgard realm" ([line 94](https://github.com/ouroboric-dev/ouroboric/blob/e403734df962c73d1debdf00d1773c807f0b2ba6/docs/1-company/charter.md#L94)); the quote in the old RB-02 row still matches its pinned commit.

## References from 2026-10-13 that I rechecked

Every pinned commit in both older documents resolves. The short quotes that I could match to a line match at the pinned commit: the evidence rule, the levels sentence, invariant 1, the Focus line, the "read-only for now" line, and the "not ready to build yet" bottom line. `ouroboric-dev/realm-web` now redirects to `midgard-react`. The old document was not edited.

## Proposed order

1. The human reviews this assessment.
2. The human decides SI-4 and SI-6 (which files are guardrails); the agent proposes the diff.
3. Build #116 with a held-out split (SI-1, SI-3), then a release gate (SI-13).
4. Records: the journal (RB-17, SI-9, SI-10), then a charter and a plan.
5. Make the channel work (SI-14) and ask ouroboric-dev what they want graduation to mean.

## Credit

The item ids RB-01 to RB-21 and the layout come from ouroboric-dev/asgard (MIT) and ouroboric-dev/ouroboric (CC-BY-4.0), © 2026 Bruno Guilhermo de Barros Valério. No text was copied beyond the short quotes, each marked and linked to its pinned line. The SI ids are picasso's own.
