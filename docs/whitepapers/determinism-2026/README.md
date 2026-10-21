# Same inputs, same factory

A whitepaper on why determinism and reproducibility matter in 2026, and what picasso's labs can do
for [ouroboric-dev](https://github.com/ouroboric-dev). The repo root has it in two forms:

- [`PICASSOPAPER.pdf`](../../../PICASSOPAPER.pdf): the original paper, ten coloured A4 pages.
- [`WHITEPAPER.md`](../../../WHITEPAPER.md) and [`WHITEPAPER.pdf`](../../../WHITEPAPER.pdf): the
  same paper in ASD-STE100 Simplified Technical English, after two `/ste100` passes
  ([`STE100.md`](STE100.md) logs them).

| File | Is |
|---|---|
| `whitepaper.html` | the source; every reference in it was checked against Crossref, arXiv or the primary page |
| `render.mjs` | prints it to `PICASSOPAPER.pdf` with headless Chromium (`node render.mjs`, Playwright on the module path) |
| `render-ste.mjs` | prints `WHITEPAPER.md` to `WHITEPAPER.pdf` the same way (needs pandoc) |
| `STE100.md` | what each ASD-STE100 pass changed, and the sentences kept outside the rules |
| `image.nix` | experiment B of §5.3: a factory-layer OCI image that `nix-build --check` rebuilds bit-for-bit |
| `zenodo.json`, `tools/zenodo` | the Zenodo record for the paper alone (a report, CC BY 4.0), and the script that deposits it |

The ouroboric-dev material it quotes is CC BY 4.0 (© its authors), cited by repository and commit.
The Ouroboric name is a trademark of its project and is used only to identify it.

## Publishing on Zenodo

The paper gets its own Zenodo record and DOI, separate from picasso's software record in the root
`.zenodo.json`. Nothing here publishes: a published DOI can't be withdrawn, so the last step is a
person pressing Publish on zenodo.org.

```console
export ZENODO_TOKEN=...              # zenodo.org → Applications → Personal access token, scope deposit:write
nix run ../../../tools#zenodo -- --self-test        # metadata and files, no network
nix run ../../../tools#zenodo -- draft              # draft from zenodo.json, reserves the DOI into doi.txt
node render.mjs                      # the cover's "Cite as" line now shows the DOI
nix run ../../../tools#zenodo -- upload             # attaches PICASSOPAPER.pdf to the draft
```

Then review the draft on zenodo.org, publish it, and commit `doi.txt` with the re-rendered PDF.
`--sandbox` on `draft` rehearses the whole flow on sandbox.zenodo.org with a sandbox token.
