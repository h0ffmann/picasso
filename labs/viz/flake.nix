{
  description = "viz — graphify, the code graph of a repository: code-only, keyless, offline";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable"; # locked to the same rev as labs/gleam-website
  };

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAll = f: lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});

      # extract / cluster-only / update re-exec themselves to pin PYTHONHASHSEED, and the re-exec
      # loses the nixpkgs wrapper ("No module named graphify"), so the seed is set before they start.
      graphifyFor = pkgs: pkgs.symlinkJoin {
        name = "graphify-${pkgs.graphify.version}";
        paths = [ pkgs.graphify ];
        nativeBuildInputs = [ pkgs.makeWrapper ];
        postBuild = "wrapProgram $out/bin/graphify --set-default PYTHONHASHSEED 0";
      };

      src = lib.fileset.toSource {
        root = ./tools;
        fileset = lib.fileset.unions [ ./tools/Cargo.toml ./tools/Cargo.lock ./tools/src ];
      };

      # viz (tools/): graphify is baked in at compile time (GRAPHIFY) rather than looked up on PATH;
      # git is the one host command it calls. The tests run in the build (checkPhase).
      vizFor = pkgs: pkgs.rustPlatform.buildRustPackage {
        pname = "viz";
        version = "0.1.0";
        inherit src;
        cargoLock.lockFile = ./tools/Cargo.lock;
        env.GRAPHIFY = "${graphifyFor pkgs}/bin/graphify";
        nativeBuildInputs = [ pkgs.makeWrapper ];
        postInstall = "wrapProgram $out/bin/viz --prefix PATH : ${lib.makeBinPath [ pkgs.git ]}";
        meta = { mainProgram = "viz"; license = lib.licenses.agpl3Only; };
      };

      # Same sources, `cargo clippy -D warnings`.
      clippyFor = pkgs: (vizFor pkgs).overrideAttrs (old: {
        pname = "viz-clippy";
        nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
        buildPhase = "cargo clippy --release --all-targets --offline -- -D warnings";
        doCheck = false;
        installPhase = "touch $out";
      });

      toolsFor = pkgs: [ (graphifyFor pkgs) (vizFor pkgs) ];

      # `nix flake check` builds this: graphify graphs example/ in the sandbox, with no network.
      smokeFor = pkgs: pkgs.runCommand "viz-smoke" { nativeBuildInputs = toolsFor pkgs ++ [ pkgs.jq ]; } ''
        export HOME=$TMPDIR VIZ_OUT=$TMPDIR/graph
        cp -r ${./example} example && chmod -R u+w example
        viz build example
        test -s "$VIZ_OUT/graph.json" && test -s "$VIZ_OUT/graph.html"
        viz query "greet" | tee query.txt
        grep -q greet query.txt
        # example/greet.rs calls shout() from shout.rs: the graph must hold that call and the import
        jq -e '(.links // .edges)[] | select(.relation == "calls" and .source == "greet_greet" and .target == "shout_shout")' "$VIZ_OUT/graph.json" > /dev/null
        jq -e '(.links // .edges)[] | select(.relation == "imports_from" and .source == "greet" and .target == "shout")' "$VIZ_OUT/graph.json" > /dev/null
        mkdir -p "$out" && cp "$VIZ_OUT/graph.json" "$VIZ_OUT/graph.html" query.txt "$out"/
      '';

      # lab.json, the card the profile README renders: its shape is asserted at evaluation, and the
      # build writes each headline attribute's version from this lab's own nixpkgs. Copied in every
      # lab, since a lab references nothing outside itself.
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
        tools = toolsFor pkgs;
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          name = "viz";
          packages = toolsFor pkgs ++ [ pkgs.cargo pkgs.rustc pkgs.clippy ];
          shellHook = ''echo "viz: graphify ${pkgs.graphify.version} | viz build, viz query \"<q>\""'';
        };
      });

      packages = forAll (pkgs: {
        default = vizFor pkgs;
        viz = vizFor pkgs;
        graphify = graphifyFor pkgs;
        smoke = smokeFor pkgs;
      });

      checks = forAll (pkgs: {
        lab-json = labCardFor pkgs;
        viz = vizFor pkgs; # building it runs its `cargo test`
        clippy = clippyFor pkgs;
        smoke = self.packages.${pkgs.stdenv.hostPlatform.system}.smoke;
      });
    };
}
