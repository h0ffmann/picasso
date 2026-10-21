{
  description = "picasso's site: Gleam on the JavaScript target with Lustre, bundled by esbuild";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { nixpkgs, ... }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAll = f: lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});

      # Hex packages are pinned by checksum in manifest.toml; everything else comes from here.
      toolsFor = pkgs: with pkgs; [
        gleam
        nodejs # gleam test and the prerender run on node
        esbuild
        just
        miniserve # just serve
        curl # site-releases asks Zenodo for the version DOIs
      ];
    in
    {
      devShells = forAll (pkgs: {
        default = pkgs.mkShell { packages = toolsFor pkgs; };
      });

      # The build itself needs the Hex registry, so it runs in the dev shell, not the sandbox;
      # this only proves the pinned toolchain still builds and runs.
      checks = forAll (pkgs: {
        tools = pkgs.runCommand "site-tools" { nativeBuildInputs = toolsFor pkgs; } ''
          {
            gleam --version
            echo "node $(node --version)"
            echo "esbuild $(esbuild --version)"
            just --version
          } | tee "$out"
        '';

        # history.nix on a four-commit log, one per era: the parsing `just history` relies on.
        history =
          let
            h = import ./history.nix {
              labs = { lint = "a list"; };
              log = "@@@1111111aaaa\t2024-01-01\tme\tsong (#1)\n\n1\t2\ta\n-\t-\tb.png\n"
                + "@@@2222222bbbb\t2024-06-01\tme\tMerge pull request #10 from x\n"
                + "@@@3333333cccc\t2025-01-01\tme\tlab (#40)\n\n4\t0\tc\n"
                + "@@@4444444dddd\t2026-01-02\tbot[bot]\tfeat(site/x): y (#70)\n\n3\t0\tc\n";
            };
            eras = map (e: map (c: c.sha) e.commits) h.data.eras;
            last = builtins.elemAt (builtins.elemAt h.data.eras 3).commits 0;
            first = builtins.head (builtins.head h.data.eras).commits;
          in
          assert eras == [ [ "1111111" ] [ "2222222" ] [ "3333333" ] [ "4444444" ] ] || throw "eras: ${builtins.toJSON eras}";
          assert first == { sha = "1111111"; date = "2024-01-01"; subject = "song (#1)"; pr = 1; scope = null; bot = false; files = 2; added = 1; deleted = 2; };
          assert last.pr == 70 && last.scope == "site/x" && last.bot;
          assert builtins.fromJSON h.json == builtins.fromJSON (builtins.toJSON h.data);
          pkgs.writeText "site-history.json" h.json;
      });

      formatter = forAll (pkgs: pkgs.nixpkgs-fmt);
    };
}
