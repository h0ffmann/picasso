# LABEL.md

The label set for picasso's issues and pull requests. Every item carries one `type/*` and at least
one `area/*`. Open items also carry one `status/*` and one `priority/*`; a closed item drops
neither, but they stop meaning anything once it is closed.

The shape comes from marola-dev/marola's convention (MIP-0063 §5.2: `kind/*`, `area/*`,
`priority/*`, one manifest with colour and description per label). What differs here: `type/*`
follows the commit subjects this repo's log already uses (`feat`, `fix`, `docs`, `chore`), `area/*`
is one label per lab plus the parts of the repo outside `labs/`, and there is no `size/*`,
`agent-ready` or `phase/*`, since picasso has no issue-readiness gate or phase list.

## type: what kind of change

| Label | Colour | Means |
|---|---|---|
| `type/feat` | `#A2EEEF` | A new capability: a lab, a recipe, a script, a page, a workflow |
| `type/fix` | `#D73A4A` | Something that was broken now works |
| `type/docs` | `#0075CA` | Documentation, metadata or prose only |
| `type/chore` | `#C5DEF5` | Pins, bumps and housekeeping, no change in behaviour |
| `type/refactor` | `#D4C5F9` | Same behaviour, different shape |
| `type/goal` | `#5319E7` | A tracking issue: an outcome with a "Done when" list, closed by several PRs |

## area: where it lands

| Label | Colour | Means |
|---|---|---|
| `area/pratico` | `#C2E0C6` | `labs/pratico`: WAVEWATCH III toolchain, zsh-ai, ai-jail recipes |
| `area/publisher` | `#C2E0C6` | `labs/publisher`: pandoc + TeX Live, `mkPdf`, the PDF Action |
| `area/lint` | `#C2E0C6` | `labs/lint`: the lint toolchain |
| `area/agentic` | `#C2E0C6` | `labs/agentic`: ai-jail, OpenCode, `ocr`, `jail-run` and friends |
| `area/cuda` | `#C2E0C6` | `labs/cuda`: CUDA cache setup and the torch venv |
| `area/present` | `#C2E0C6` | `labs/present`: annotation, capture, webcam, presenters |
| `area/ptbr-pdf` | `#C2E0C6` | `labs/ptbr-pdf`: `to-pdf`, Markdown to PDF with Typst |
| `area/ptbr-tools` | `#C2E0C6` | `labs/ptbr-tools`: the `ptbr` binary that ptbr-writer ships |
| `area/viz` | `#C2E0C6` | `labs/viz`: the visualisation toolchain |
| `area/paper` | `#C2E0C6` | `labs/paper`: the paper and report toolchain |
| `area/labs` | `#C2E0C6` | Every lab at once: `lab.json`, a nixpkgs bump across labs |
| `area/tools` | `#BFD4F2` | `tools/`: the repo-wide Rust crates (`citation`, `render`, `site-check`, …) |
| `area/system` | `#F9D0C4` | The root NixOS system, `home.nix`, `just vm`, the move back to NixOS |
| `area/site` | `#BFDADC` | `site/`: the Gleam page, drawings, history, GitHub Pages |
| `area/ci` | `#006B75` | `.github/workflows/`, the CI gate, Dependabot, reusable workflows |
| `area/repo` | `#FEF2C0` | README, AGENTS.md, LICENSE, citation, logo, vendored skills, the repo's name |

## status: where an open item stands

| Label | Colour | Means |
|---|---|---|
| `status/triage` | `#EDEDED` | Not looked at yet |
| `status/ready` | `#0E8A16` | Scoped; anyone can start it |
| `status/in-progress` | `#FBCA04` | Merged PRs did part of it, or a PR for it is open |
| `status/needs-human` | `#E99695` | Waits on a decision or an action only the owner can take: an account, DNS, the hardware, a review |
| `status/blocked` | `#5A5A5A` | Waits on another issue or PR, named in its body |

## priority

| Label | Colour | Means |
|---|---|---|
| `priority/high` | `#B60205` | Do before other open work |
| `priority/medium` | `#FF9F1C` | Next, once nothing high is open |
| `priority/low` | `#FFF5B1` | When there is room |

## A2A: messages between Claude projects

AGENTS.md, "Messages between Claude projects", has the protocol. It is marola's A2A pattern. The
two labels are the same in `h0ffmann/picasso` and `ouroboric-dev/contact`. An issue has one of
them, never the two.

| Label | Colour | Means |
|---|---|---|
| `para:ouroboric` | `#FF4F00` | An `[A2A]` message to the ouroboric-dev project. The ouroboric-dev project answers next |
| `de:ouroboric` | `#5277C3` | An `[A2A]` message from the ouroboric-dev project. This project ("Nix OS") answers next |

## Labels kept from before

Run `just labels` to create or update every label in the tables above (`gh label create --force`, needs `gh` logged in).

`dependencies` and `github_actions` stay: Dependabot adds them to its own PRs. GitHub's defaults
(`bug`, `enhancement`, `documentation`, …) and the older `bark` and `refactor` are no longer applied;
they map to `type/fix`, `type/feat`, `type/docs` and `type/refactor`. They are left in place rather
than deleted, so nothing that still filters on them breaks.

## Adding a label

Add its row here and create it with the same name, colour and description:

```console
gh label create "area/<name>" --color C2E0C6 --description "<the Means column>"
```

A new lab gets its `area/<lab>` row in the same PR that adds the lab.
