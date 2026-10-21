---
status: POSTED
instagram: https://www.instagram.com/p/DeZal3lkfRh/
caption: pt-BR
---

# instagram_001: the first picasso post

A carousel of five slides, 1080x1350 (4:5), for Hoffmann's personal Instagram page. The story is
the v0.1.1 release, the Zenodo concept DOI, the site and the determinism whitepaper. Each number
on a slide shows its source under it. Slides 2 and 4 take their numbers from
[WHITEPAPER.md](../../../WHITEPAPER.md); slide 3 counts them from the advisory data.

| Slide | Shows | Sources |
|---|---|---|
| [slide-1.png](slide-1.png) | the logo, "a toolchain is a dependency, so pin it like one", the DOI | [10.5281/zenodo.23311208](https://doi.org/10.5281/zenodo.23311208) |
| [slide-2.png](slide-2.png) | Python and JavaScript, two cans of worms; the owl | Pimentel et al., MSR 2019 ([doi:10.1109/MSR.2019.00077](https://doi.org/10.1109/MSR.2019.00077)); Ohm et al., DIMVA 2020 ([doi:10.1007/978-3-030-52683-2_2](https://doi.org/10.1007/978-3-030-52683-2_2)); Fabio Akita, [New AI-Focused Language Just Released: Bend 2](https://www.akitaonrails.com/en/2026/09/19/new-ai-language-just-released-bend-2/), 19 September 2026 |
| [slide-3.png](slide-3.png) | CVEs in npm and PyPI over time, cumulative, drawn like a GitHub star-history chart | [GitHub Advisory Database](https://github.com/advisories), reviewed advisories with a CVE id, counted from the [OSV](https://osv.dev) dumps by `social cves` ([tools/social](../../../tools/social)) into [cves.json](cves.json) on 2026-10-12 |
| [slide-4.png](slide-4.png) | if the generator is stochastic, the rest must be deterministic; the guitar | He / Thinking Machines Lab, [Defeating Nondeterminism in LLM Inference](https://thinkingmachines.ai/blog/defeating-nondeterminism-in-llm-inference), 2025; Malka, Zacchiroli, Zimmermann, [arXiv:2501.15919](https://arxiv.org/abs/2501.15919) |
| [slide-5.png](slide-5.png) | the whitepaper, the DOI, the site and the source; the dove | the whitepaper ([WHITEPAPER.md](../../../WHITEPAPER.md)) |

Akita's sentence is quoted word for word from the English version of his post. He names only
JavaScript as a can of worms; "two cans of worms" for Python and JavaScript is our heading, not his.

The logo is [docs/media/logo.svg](../../media/logo.svg) and the drawings are the repo's own one-line
homages in [site/static/drawings/](../../../site/static/drawings/). No slide shows a Picasso work.

## Caption (pt-BR)

```text
Lancei o picasso v0.1.1, a primeira versão com DOI no Zenodo (10.5281/zenodo.23311208). São labs Nix que fixam a toolchain como qualquer dependência. Python e JavaScript são duas latas de minhocas: só em 2026 o GitHub publicou 5.078 CVEs em pacotes npm e PyPI. As fontes estão nos slides. Citação do Fabio Akita no slide 2. Site, paper e código: link na bio.

#nixos #nix #reproducibility #opensource #determinism
```

## Caption (English)

```text
picasso v0.1.1 is out, the first release with a Zenodo DOI (10.5281/zenodo.23311208). Nix labs that pin a toolchain like any other dependency. Python and JavaScript are two cans of worms: in 2026 alone GitHub published 5,078 CVEs in npm and PyPI packages. Sources are on the slides. Quote on slide 2 by Fabio Akita. Site, paper and code: link in bio.

#nixos #nix #reproducibility #opensource #determinism
```

Link in the bio: https://h0ffmann.github.io/picasso

## Alt text

1. The picasso logo, a one-line face in profile with a lambda for an ear, over a blue circle and a rose square. Text: picasso, a toolchain is a dependency, so pin it like one.
2. Python and JavaScript, two cans of worms. 4.03% of 1.4 million Jupyter notebooks gave their own results again. 174 malicious packages from real attacks, shipped through registries like npm and PyPI. A quote by Fabio Akita about JavaScript. A one-line owl.
3. A line chart of cumulative CVE advisories since 2018: npm 6,390 and PyPI 6,118 by 12 October 2026. 5,078 of them were published in 2026, more than in 2023, 2024 and 2025 together.
4. If the generator is stochastic, the rest must be deterministic. 80 different answers from 1,000 identical LLM requests at temperature 0. More than 99% of 709,816 nixpkgs packages rebuilt from their recorded inputs. A one-line cubist guitar.
5. Same inputs, same factory: the whitepaper, the DOI, the site and the source link. A one-line dove.

## Render again

```console
nix run ./tools#social -- render   # from the repo root; headless Chromium on PATH, CHROMIUM=<path> to pick a binary

# refresh the CVE counts (downloads about 250 MB)
mkdir -p /tmp/osv
curl -o /tmp/osv/npm.zip https://osv-vulnerabilities.storage.googleapis.com/npm/all.zip
curl -o /tmp/osv/PyPI.zip https://osv-vulnerabilities.storage.googleapis.com/PyPI/all.zip
nix run ./tools#social -- cves /tmp/osv > docs/social/instagram_001/cves.json
```
