# ouroboric-sharingan: road to production and to a DOI

## Publication

This skill will be published with a DOI, as the generic skill for learning patterns from open
source, citing them properly, and reusing them only as their licences allow. The DOI is for a
white paper, working title:

> **Sharingan: Read, Evaluate, Adopt. A generic agent skill for learning from open source with
> proper citation and licence-respecting reuse**

The paper goes in `docs/whitepapers/sharingan-2026/` and follows the existing determinism paper.
It covers the REA method, the licence matrix and its sources, and the benchmark (`evals/`,
`evals/BENCHMARK.md`). The DOI comes from a Zenodo release of this repository. Only the human runs
`just release` (AGENTS.md, Releases), so an agent never mints it.

## Production-ready checklist

Tick a box only with evidence: a commit, a benchmark row or a review link.

- [x] `SKILL.md` covers the destination, Read, Evaluate, Adopt, Report and Never, and pins no
      model or effort
- [x] Licence matrix in `references/licences.md`, with each row's reason checked against the FSF
      licence list and Creative Commons' compatibility page (2026-10-12)
- [x] `sharingan licence-scan` (`tools/sharingan`): reads files only, `cargo test` passes, and the
      test fails if the matrix in the doc and the code differ
- [x] Evals: 26 trigger queries and 4 output cases on 5 fixture repos, with code checks on the diff
- [x] `sharingan bench` runs both sets with and without the skill, and its tests make no
      model calls
- [x] First benchmark recorded in `evals/BENCHMARK.md`
- [x] CI runs both tools' tests (`skills self-tests` job in `.github/workflows/ci.yml`, added with
      the human's approval)
- [ ] Benchmark at 3 runs per output case: with the skill ≥ 0.90 pass rate, and ahead of no skill
      in every case (round 3, at 2 runs: 0.94 against 0.46, ahead or level in every case)
- [x] Trigger set ≥ 95 % over 3 runs, with no false triggers (round 3: 26/26)
- [ ] One real REA, for example ouroboric-dev into picasso, done with the skill and its report
      reviewed by the human (the report is ready for that review:
      `docs/rea/2026-10-13-ouroboric-dev.md`)
- [ ] Licence matrix reviewed by a person who knows licensing (it is not legal advice)
- [ ] White paper written in `docs/whitepapers/sharingan-2026/`, with its row in the README's
      Papers table
- [ ] DOI minted by a release (human only), and the DOI badge added here and in the README
- [ ] **ouroboric-sharingan is de facto production-ready**
