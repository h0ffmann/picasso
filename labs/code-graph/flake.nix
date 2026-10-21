{
  description = "code-graph — the code graph of a repository as graphify's graph.json and an interactive graph.html: one Rust binary, no Python";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable"; # locked to the same rev as labs/viz
  };

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAll = f: lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});

      # The crate is in tools/; the viewer script (assets/) is compiled into it, the example is the
      # tests' and the smoke's input.
      src = lib.fileset.toSource {
        root = ./.;
        fileset = lib.fileset.unions [
          ./tools/Cargo.toml
          ./tools/Cargo.lock
          ./tools/src
          ./tools/tests
          ./assets
          ./example
        ];
      };

      # code-graph: tree-sitter grammars compiled in, so no Python, no pip and no network at run time.
      # The tests run in the build (checkPhase).
      codeGraphFor = pkgs: pkgs.rustPlatform.buildRustPackage {
        pname = "code-graph";
        version = "0.1.0";
        inherit src;
        sourceRoot = "${src.name}/tools";
        cargoLock.lockFile = ./tools/Cargo.lock;
        stripAllList = [ "bin" ];
        meta = { mainProgram = "code-graph"; license = lib.licenses.agpl3Only; };
      };

      # bytes: the measured size of packages.default plus 10% (README, Size).
      sizeCeiling = 18300000;

      # Same sources, `cargo clippy -D warnings`.
      clippyFor = pkgs: (codeGraphFor pkgs).overrideAttrs (old: {
        pname = "code-graph-clippy";
        nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
        buildPhase = "cargo clippy --release --all-targets --offline -- -D warnings";
        doCheck = false;
        installPhase = "touch $out";
      });

      # `nix flake check` builds this: the example is graphed in the sandbox and the edges that
      # labs/viz asserts for graphify must be there, with the same ids.
      smokeFor = pkgs: pkgs.runCommand "code-graph-smoke" { nativeBuildInputs = [ (codeGraphFor pkgs) pkgs.jq ]; } ''
        export HOME=$TMPDIR
        cp -r ${./example} example && chmod -R u+w example
        code-graph extract example --out "$TMPDIR/out"
        jq -e '.links[] | select(.relation == "calls" and .source == "greet_greet" and .target == "shout_shout")' "$TMPDIR/out/graph.json" > /dev/null
        jq -e '.links[] | select(.relation == "imports_from" and .source == "greet" and .target == "shout")' "$TMPDIR/out/graph.json" > /dev/null
        test -s "$TMPDIR/out/graph.html" && ! grep -q 'https\?://' "$TMPDIR/out/graph.html"
        code-graph query greet --graph "$TMPDIR/out/graph.json" | tee query.txt
        grep -q greet query.txt
        mkdir -p "$out" && cp "$TMPDIR"/out/* query.txt "$out"/
      '';

      # lab.json, the card the profile README renders (copied in every lab: a lab references
      # nothing outside itself).
      labCardFor = pkgs:
        let
          card = lib.importJSON ./lab.json;
          version = attr: (lib.getAttrFromPath (lib.splitString "." attr) pkgs).version;
        in
        assert lib.assertMsg (lib.isString card.summary && card.summary != "" && lib.stringLength card.summary <= 100)
          "lab.json: summary must be one non-empty line of at most 100 characters";
        assert lib.assertMsg (lib.isList card.headline && card.headline != [ ] && lib.all lib.isString card.headline)
          "lab.json: headline must be a non-empty list of nixpkgs attribute paths";
        pkgs.writeText "lab-json" (lib.concatMapStrings (a: "${a} ${version a}\n") card.headline);
    in
    {
      lib = forAll (pkgs: {
        tools = [ (codeGraphFor pkgs) ];
      });

      packages = forAll (pkgs: {
        default = codeGraphFor pkgs;
        code-graph = codeGraphFor pkgs;
        smoke = smokeFor pkgs;
      } // lib.optionalAttrs (pkgs.stdenv.hostPlatform.system == "x86_64-linux") {
        # a static musl binary, for release assets
        static = codeGraphFor pkgs.pkgsStatic;
      });

      apps = forAll (pkgs: {
        default = { type = "app"; program = "${codeGraphFor pkgs}/bin/code-graph"; };
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          name = "code-graph";
          packages = [ pkgs.rustc pkgs.cargo pkgs.clippy pkgs.rustfmt pkgs.rust-analyzer ] ++ lib.optional pkgs.stdenv.hostPlatform.isLinux pkgs.gcc;
        };
      });

      checks = forAll (pkgs: {
        lab-json = labCardFor pkgs;
        code-graph = codeGraphFor pkgs; # builds and runs cargo test
        clippy = clippyFor pkgs;
        fmt = pkgs.runCommand "code-graph-fmt" { nativeBuildInputs = [ pkgs.cargo pkgs.rustfmt ]; } ''
          cargo fmt --check --manifest-path ${src}/tools/Cargo.toml && touch $out
        '';
        # the binary must stay light: fails if it grows past the ceiling (measured size + 10%)
        size = pkgs.runCommand "code-graph-size" { } ''
          size=$(stat -L -c %s ${self.packages.${pkgs.stdenv.hostPlatform.system}.default}/bin/code-graph)
          echo "code-graph: $size bytes, ceiling ${toString sizeCeiling}"
          [ "$size" -le ${toString sizeCeiling} ] || { echo "code-graph grew past its ceiling" >&2; exit 1; }
          touch $out
        '';
        smoke = self.packages.${pkgs.stdenv.hostPlatform.system}.smoke;
      });
    };
}
