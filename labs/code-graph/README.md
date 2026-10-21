# code-graph

`code-graph` is one Rust binary that makes the code graph of a repository. It writes
`graph.json` in the schema of [graphify](https://github.com/Graphify-Labs/graphify), a
`GRAPH_REPORT.md`, and `graph.html`: one self-contained, interactive page. It has no Python, no
pip, no model and no network at run time. The tree-sitter grammars are compiled in.

It does the same job as [`labs/viz`](../viz) does with graphify's code-only path. `labs/viz` is not
changed by this lab. A later change can switch `viz` to this binary.

```console
just                         # the recipes
just graph                   # graph the current directory into ~/.cache/code-graph/<name>/
just graph query "ask levels" --budget 400
just graph path "main()" "write()"
just graph explain "route()"
just test                    # cargo test in the dev shell
just smoke                   # graph example/ in the Nix sandbox (what CI builds)
just size                    # binary, static binary and closure
```

Commands of the binary: `extract DIR [--out OUT] [--no-viz] [--exclude PATH]... [--title TEXT]`, `update DIR` (the same, nothing is
cached), `cluster-only DIR`, `html [--graph F] [--out F]`, `query "Q" [--budget N] [--depth N]`,
`path A B`, `explain X`. `--code-only` and `--no-label` are accepted and ignored, so a graphify
command line still works. The name is `code-graph` and not `graphify`: the binary is compatible with
graphify's `graph.json`, and it is not graphify.

## Research (pinned, read-only)

| Question | Finding |
|---|---|
| Licence of graphify | Apache-2.0 (`LICENSE`), with the earlier MIT parts kept in `LICENSE-MIT` and a `NOTICE` ("Copyright 2026 Safi Shamsi and the Graphify contributors"). Both licences allow a re-implementation. No source is copied: the behaviour and the output format were learnt from the real tool's output and from reading it, and the code here is written new. Apache-2.0 section 6 gives no right to the name, so this tool has a neutral name. |
| What was read | graphify `v0.9.74` (`e10df08877f8819a625a1afa38c3297a31fda296`, the version in nixpkgs `6510408d`); `graphify/ids.py`, `cluster.py`, `extract.py`, `extractors/rust.py`, `extractors/markdown.py`, `serve.py` (budget), and the output of `extract`, `cluster-only`, `query`, `path`, `explain` on this repository. |
| Clustering in graphify | Leiden (`graspologic`), with Louvain from networkx as the fallback. |
| Languages in graphify | About 40 file types (tree-sitter and its own extractors). It has no grammar for Gleam or Nix: it skips those files. |
| `graph.json` | `{directed:false, multigraph:false, graph:{}, nodes, links, hyperedges:[]}`. Node: `id`, `label`, `norm_label`, `source_file`, `source_location` (`"L5"`), `community`, `file_type:"code"`, `_origin:"ast"`; an unresolved import is a node with `external:true`. Edge: `source`, `target`, `relation`, `confidence` (`EXTRACTED` or `INFERRED`), `confidence_score`, `context` (`call` or `import`), `source_file`, `source_location`, `weight`. Relations seen: `contains`, `method`, `calls`, `imports_from`, `imports`, `inherits`, `implements`, `references`, `defines` (Fortran). |
| Ids | The path without extension, then the symbol: `src/lib.rs` is `src_lib`, its function `top` is `src_lib_top`, its method `Greeter::new` is `src_lib_greeter_new`. Lowercase; each run of non-word characters is one `_`. A method label is `.new()`, a function label is `top()`, a file label is `lib.rs`. |
| `GRAPH_REPORT.md` | Summary, community hubs, god nodes, surprising connections, import cycles, communities, knowledge gaps, suggested questions. |
| `query` output | A header line, then `NODE label [src= loc= community=]` lines, then `EDGE a --rel [CONF context=..]--> b at=file:Lnn` lines. The budget is about 3 characters per token. |
| Reader in marola-devkit | `skills/routing/scripts/route.py` (`marola-dev/marola-devkit` at `2d49d22`) reads `nodes` (`id`, `label`, `source_file`, `source_location`, `community`, `community_name`, `entry`) and `links` or `edges` (`source`, `target`, `relation`, `source_location`). It joins the edges by node pair and ignores the other fields. This output has every one of those fields. |

## Languages

| Language | Grammar crate | Files |
|---|---|---|
| Rust | `tree-sitter-rust` | `.rs` |
| Python | `tree-sitter-python` | `.py` |
| JavaScript | `tree-sitter-javascript` | `.js .jsx .mjs .cjs` |
| TypeScript, TSX | `tree-sitter-typescript` | `.ts .mts .cts .tsx` |
| Scala | `tree-sitter-scala` | `.scala .sc` |
| Fortran | `tree-sitter-fortran` | `.f .f90 .f95 .f03 .f08` and upper-case forms |
| Gleam | `tree-sitter-gleam` | `.gleam` (graphify has no Gleam grammar: an improvement) |
| Nix | `tree-sitter-nix` | `.nix` (graphify has no Nix grammar: an improvement) |

Dependencies: `tree-sitter`, the eight grammar crates above and `serde_json`. Nothing else.

## What it does

- **Extract.** Per file: a file node; a node for each struct, enum, trait, const, class, module,
  type and function; `contains` and `method` (Fortran: `defines`) edges; `calls`, `inherits` and
  `implements` edges; `imports_from` edges to a file in the tree, or to an `external` node; and
  `imports` edges to the symbols a `from x import a` or `import { a }` names. A name is
  resolved in the same class, then in the same file, then in a file that the caller imports
  (`EXTRACTED`, score 1.0), then to the one symbol of that name in the tree (`INFERRED`, 0.8). A call
  written `receiver.name()` is never guessed across files: `x.join()` is mostly a std method.
  A name with two possible targets gets no edge. The tree walk skips hidden directories, `target`,
  `node_modules`, `graphify-out`, `result*`, `dist`, `build`, `_build`, `vendor` and
  `__pycache__`, and files over 2 MiB. It does not read `.gitignore`. It also skips any directory below
  DIR that has its own `.git` (file or directory): a submodule or a nested repository is another
  repository. `--exclude PATH` (repeatable, relative to DIR) skips one more path, for example
  `--exclude docs/generated`. `--title TEXT` names the page (default: the directory name).
- **Cluster.** Louvain modularity, one deterministic pass order (nodes sorted by id, a tie goes to the
  lowest community). Community 0 is the largest. A community's name is its best-connected node.
  ⚠ This is not graphify's Leiden, and no check shows that the partitions are equal. They are not
  equal on the examples below.
- **Query.** Each label is scored against the question (whole label, then prefix or token, then
  substring; IDF weight; source path as a hint). The best 3 nodes are the seeds, a BFS of depth 2
  walks from them (it does not expand a hub), and the output is cut at `--budget` tokens.
  `path` is a shortest path on the undirected graph. `explain` lists a node and its edges.
- **graph.html.** One file, offline, no network request. The layout is computed in Rust
  (Fruchterman-Reingold, 120 steps, a fixed start, community by community), and the SVG is complete
  with JavaScript off: nodes by community, edges by relation, labels for the best-connected nodes.
  A 6 KB viewer script (`assets/viewer.js`, hand-written, no dependency, compiled in with
  `include_str!`) adds pan, zoom, a search box, a click on a node to see its neighbours, and a click on
  a community in the legend. Colours follow `prefers-color-scheme`.
- **Deterministic.** The same tree gives byte-identical `graph.json`, `GRAPH_REPORT.md` and
  `graph.html` (a test runs it twice). No date is written. Coordinates are rounded to integers.

## Differential check against graphify 0.9.74

On this repository (without `forks/`, without this lab), at the commit this lab branched from:

| | graphify | code-graph |
|---|---|---|
| Time | 1.3 s | 0.2 s |
| Nodes, edges, communities | 878, 1685, 49 | 1074, 2206, 45 |
| Rust files: nodes (same id in both) | 738 | 725 (707) |
| Rust: `calls` edges (same pair in both) | 553 | 698 (532) |
| Rust: `contains` | 625 | 634 (625) |
| Rust: `method` | 45 | 45 (38) |
| Rust: `imports_from` | 14 | 31 (12) |
| Gleam and Nix | skipped | 125 and 154 nodes |
| `references` (type use), `case_of` | 286 and 38 | not written |

On `labs/viz/example`: graphify writes 5 nodes and 5 edges in 2 communities. `code-graph` writes the
same 5 nodes and the same 5 edges, plus one `imports` edge (`greet` to `shout()`), in 1 community.

What the numbers say, without rounding in our favour:

- Of graphify's Rust `calls` edges, 532 of 553 (96%) are also here. Here there are 166 more. Some are
  true edges (top-level calls and calls between a file and a function), and some can be wrong:
  `INFERRED` edges are a guess. No person checked the 166.
- 7 of graphify's 45 `method` ids differ from ours, because graphify puts the type text of a generic
  impl in the id (`q_a_new` for `impl Q<'a>`), and this tool puts the type name (`q_new`).
- Not done here: `references` edges, Rust `enum` variants and fields, shell, Lua, TOML and the other
  graphify extractors, graphify's cross-file symbol resolvers for each language, and its
  community splitting. Clustering differs: 45 communities against 49.
- A fixture with Python, TypeScript, Scala, Fortran, Gleam and Nix is in `tools/tests/cli.rs`. For
  Python, TypeScript and Scala the edges of graphify's own output on the same files were compared by
  hand: the same `calls`, `inherits` and `method` edges are there.

## Size

Measured on x86_64-linux with `nix path-info -Sh` and `stat`, nixpkgs `6510408d`:

| | Size |
|---|---|
| `packages.default` binary (glibc, stripped) | 15.8 MiB (16,591,008 bytes) |
| `packages.default` closure | 62.8 MiB |
| `packages.static` (musl) binary and closure | 15.9 MiB (static, no other file) |
| `packages.static` with `xz -9e` | 1.1 MiB (a measurement only: see the xz warning in [`ptbr-pdf`](../ptbr-pdf/README.md#benchmark-from-pandoc-to-rust)) |
| nixpkgs `graphify` 0.9.74, closure | 579.7 MiB |

The 18.3 MB ceiling in `checks.size` is the measured binary plus 10%. Most of the size is the Scala,
TypeScript and Fortran parser tables.

## Not implemented ⚠

- Documents, papers, images and video, and the LLM passes: community labels from a model
  (`label`), `--backend`, `add`, `install`, `hook`, the MCP server (`serve`).
- Markdown pages, headings and links (graphify's `update` reads them). Only code files are read.
- `references` (type use) edges, call-flow and tree HTML, `affected`, `god-nodes`, `merge-graphs`,
  `watch`, `--dfs`, `--context`, the extraction cache and the manifest.
- The 30 other languages of graphify (Go, Java, C, C++, Ruby, PHP, Kotlin, Swift and others).
- graphify's NFKC step in ids (only a lowercase and non-word rule here).
- `.gitignore` is not read.

## Linking graphs across repositories

`extract --link NAME=PATH/graph.json[@URL]` (repeatable) draws the boundary between two repositories
without merging their graphs. NAME is the other repository's short name, PATH its graph.json, URL
where its graph.html is published (relative or `http(s)`; default `NAME.html`).

The text files of DIR (the files the walk visits, plus `.nix`, `.sh`, `.md`, `.toml`, `.lock`, `.yml`
and `justfile`) are read for references to the repository. A reference is, on one line:

1. `github:OWNER/NAME...?dir=PATH` (or `&dir=PATH`), a flake URL, up to the next quote;
2. `github.com/OWNER/NAME/tree/REV/PATH` or `.../blob/REV/PATH`, with REV one path segment;
3. `lab=PATH`, at the start of a word (the key in a lock file);
4. a word `labs/X`, in a file whose text contains NAME (any case).

A PATH resolves to one node of the linked graph, below it: `PATH/flake.nix` if there is one, else the
node with most edges, then the lowest id. A PATH with no node is dropped. Each node found is added as
an external node `NAME:<id>` (label `NAME: <label>`, `"external": "NAME"`, `source_file` `NAME:<file>`,
`href` `URL#node=<id>`), in a community of its own named NAME, with a `depends_on` edge from the file
that holds the reference (a file the walk does not parse gets a node for this). Only fields and
relations are added; without `--link` the graph.json is the same bytes as before. ⚠ Rule 3 trusts the
key: it needs no mention of NAME, only a PATH that exists in the linked graph. Rule 2 does not read
a REV that contains a `/`.

In graph.html an external node is a dashed ring, with the legend entry `external: NAME`, and a click on
it opens its `href`; the side panel shows the same link. A page opened with `#node=<id>` selects that
node and centres the view on it. The page loads nothing: the link is a relative `<a>` that the viewer
script sets, only on external nodes.

ptbr-writer publishes two pages, `grafo/picasso.html` and `grafo/graph.html`:

```sh
code-graph extract forks/picasso --out build/grafo-picasso --title picasso
code-graph extract . --exclude forks --link picasso=build/grafo-picasso/graph.json@picasso.html --title ptbr-writer
```

The second run already skips `forks/picasso` (a submodule has its own `.git`), so `--exclude forks` is
a second guard. On ptbr-writer at `adfa9a5`, with picasso `4bc03e5`: 4 external nodes (the `flake.nix`
of `labs/gleam-website`, `labs/code-graph`, `labs/ptbr-pdf` and `labs/ptbr-tools`) and 15 edges from
10 files. Measured with this binary on x86_64-linux; the pages were not published.

## Reusing it from another repository

```nix
{
  inputs.code-graph.url = "github:h0ffmann/picasso?dir=labs/code-graph";
  # outputs: code-graph.packages.${system}.default, .static (x86_64-linux), lib.${system}.tools
}
```

The graph of picasso itself is made by `just graph` at the repository root of a checkout:
`~/.cache/code-graph/<directory name>/graph.html` (370 KB for this repository, so it is not committed).

## Flake outputs

`packages.<system>.{default, code-graph, smoke}` and `static` on x86_64-linux; `apps.<system>.default`;
`devShells.<system>.default`; `lib.<system>.tools`; `checks.<system>.{lab-json, code-graph (build and
`cargo test`), clippy (`-D warnings`), fmt, size, smoke}`.
