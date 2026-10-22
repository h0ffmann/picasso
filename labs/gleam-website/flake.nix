{
  description = "gleam-website — static sites in Gleam on the JavaScript target (Lustre), node, esbuild, just, python";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable"; # locked to the same rev as labs/paper
  };

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAll = f: lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});

      # The toolchain of picasso's own site/ (Gleam + Lustre, bundled by esbuild). A site pins its
      # Hex packages by checksum in its manifest.toml; everything else comes from here.
      nodeFor = pkgs: pkgs.callPackage ./nodejs-bin.nix { };

      # The October 2026 security release (nodejs.org/en/blog/vulnerability/october-2026-security-releases):
      # the scanners' databases lag the advisory, so the minimum is stated here and checked in CI.
      nodeVersionFor = pkgs:
        let node = nodeFor pkgs; in
        assert lib.assertMsg (lib.versionAtLeast node.version "24.22.1")
          "gleam-website: node ${node.version} is below 24.22.1, the October 2026 security release";
        pkgs.runCommand "gleam-website-node-version" { nativeBuildInputs = [ node ]; } ''
          node --version | tee "$out"
        '';

      toolsFor = pkgs: with pkgs; [
        gleam
        # gleam test, gleam run (the prerender) and the bundle check run on node: the official
        # 24.22.1 build (nodejs-bin.nix), the October 2026 security release, until nixpkgs ships it.
        # Only this list gets it: an overlay of pkgs.nodejs would rebuild erlang and webkitgtk.
        (nodeFor pkgs)
        esbuild
        just
        python3 # the small generators and checks a site keeps next to its Gleam code
      ];

      # `nix flake check` builds this: every tool answers --version in the sandbox.
      versionsFor = pkgs: pkgs.runCommand "gleam-website-versions" { nativeBuildInputs = toolsFor pkgs; } ''
        export HOME=$TMPDIR
        {
          echo "nixpkgs  ${nixpkgs.rev or "dirty"}"
          echo "gleam    $(gleam --version)"
          echo "node     $(node --version)"
          echo "esbuild  $(esbuild --version)"
          echo "just     $(just --version)"
          echo "python3  $(python3 --version)"
        } | tee versions.txt
        mkdir -p "$out" && cp versions.txt "$out"/
      '';

      # A site with Hex dependencies needs the registry, so it builds in the dev shell, not the
      # sandbox. The smoke project has none: it proves gleam → JavaScript → node → esbuild here.
      smokeFor = pkgs: pkgs.runCommand "gleam-website-smoke" { nativeBuildInputs = toolsFor pkgs; } ''
        export HOME=$TMPDIR
        cp -r ${./example} example && chmod -R u+w example
        build=$TMPDIR/out
        mkdir -p "$build"
        cd example
        # compile with gleam, run the result on node, bundle it with esbuild, and check that both print the page
        gleam build --no-print-progress
        gleam run --no-print-progress > "$build/index.html"
        esbuild entry.mjs --bundle --minify --format=esm --outfile="$build/app.js" --log-level=warning
        node "$build/app.js" | cmp - "$build/index.html"
        grep -qF '<h1>gleam-website smoke</h1>' "$build/index.html" || { echo "smoke: page missing" >&2; exit 1; }
        echo "gleam-website smoke: $build/index.html $build/app.js"
        cd ..
        mkdir -p "$out" && cp "$build"/* "$out"/
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
          name = "gleam-website";
          packages = toolsFor pkgs;
          shellHook = ''echo "gleam-website: $(gleam --version) | node $(node --version) | esbuild $(esbuild --version)"'';
        };
      });

      packages = forAll (pkgs: {
        versions = versionsFor pkgs;
        smoke = smokeFor pkgs;
        inherit (pkgs) gleam esbuild;
      });

      checks = forAll (pkgs: {
        lab-json = labCardFor pkgs;
        node-version = nodeVersionFor pkgs;
        versions = self.packages.${pkgs.stdenv.hostPlatform.system}.versions;
        smoke = self.packages.${pkgs.stdenv.hostPlatform.system}.smoke;
      });
    };
}
