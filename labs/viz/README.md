# viz

The code graph of a repository with **[graphify](https://github.com/Graphify-Labs/graphify)**, and
nothing else. graphify parses the code with tree-sitter into `graph.json`, clusters it into
communities, and draws `graph.html`, an interactive picture of the graph that you open in a browser.
`viz` runs it the way marola-devkit's `just graph` and picasso's root `just graphify` do:
code-only, with no API key and no model, and the output goes to
`~/.cache/marola-graph/<repo>`, never into the checkout.

```
just shell                  # graphify and viz on PATH, in your current directory
just graph build            # graph the directory you are in; prints the path of graph.html
just graph update           # the same graph, with the Markdown pages, headings and links
just graph query "<q>"      # answer from the graph, capped at --budget 400
just graph path <a> <b>     # also explain <x>, or any other graphify command, on the same graph
just self-test              # viz's cargo tests (CI builds them as the viz check)
just smoke                  # example/ (two Rust files, one calling the other) graphed and queried in the sandbox (CI)
```

| Command | Does |
|---|---|
| `viz build [dir]` | `graphify extract <dir> --code-only --no-label`, then `cluster-only`, which writes `graph.json`, `GRAPH_REPORT.md` and `graph.html` |
| `viz update [dir]` | `graphify update <dir>`: the code graph, and one node for each Markdown page and heading, with an edge for each relative link. It reads no prose and calls no model |
| `viz query "<q>"` | `graphify query` on that graph, `--budget 400` |
| `viz where` | the output directory; `VIZ_OUT` sets another one |
| `viz <cmd> …` | any other graphify command, with `--graph` set to the same `graph.json` |

graphify runs under `env -i` with `OLLAMA_BASE_URL` on the discard port, so a provider key in your
environment never reaches it and no step can call a model. Do not use `graphify install` or the
`/graphify` skill: their LLM pass over the docs costs about 0.6M tokens. `PYTHONHASHSEED` is set
in a wrapper, because `extract` and `cluster-only` start themselves again to pin it and the
restart loses the nixpkgs wrapper ("No module named graphify").

## Reusing it from another repository

```nix
{
  inputs.viz.url = "github:h0ffmann/picasso?dir=labs/viz";
  outputs = { nixpkgs, viz, ... }: {
    devShells.x86_64-linux.default = viz.devShells.x86_64-linux.default;
    # or, with the project's own tools:
    # devShells.x86_64-linux.default = nixpkgs.legacyPackages.x86_64-linux.mkShell {
    #   packages = [ /* the project's own tools */ ] ++ viz.lib.x86_64-linux.tools;
    # };
  };
}
```

Without a flake: `nix run 'github:h0ffmann/picasso?dir=labs/viz' -- build`. From a checkout
that has picasso as a submodule: `nix run path:<submodule>/labs/viz -- build`. First consumer:
[h0ffmann/closed-source](https://github.com/h0ffmann/closed-source) (`just viz`).
