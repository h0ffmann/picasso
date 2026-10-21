{
  description = "ptbr-pdf — Markdown to PDF for pt-BR text: a Rust binary with Typst as a library, tailored for ptbr-writter";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable"; # locked to the same rev as labs/publisher
  };

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAll = f: lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});

      src = lib.fileset.toSource {
        root = ./.;
        fileset = lib.fileset.unions [ ./Cargo.toml ./Cargo.lock ./src ./example ];
      };

      # to-pdf FILE.md [OUT.pdf]: pulldown-cmark -> Typst markup -> typst crate -> PDF. Fonts are
      # compiled in (typst-assets), so the binary needs no fontconfig, no TeX and no network.
      # Tests run in the build (checkPhase), the end-to-end one compiles example/smoke.md.
      toPdfFor = pkgs: pkgs.rustPlatform.buildRustPackage {
        pname = "to-pdf";
        version = "0.1.0";
        inherit src;
        cargoLock.lockFile = ./Cargo.lock;
        meta.mainProgram = "to-pdf";
      };

      # Same sources, `cargo clippy -D warnings` as the build.
      clippyFor = pkgs: (toPdfFor pkgs).overrideAttrs (old: {
        pname = "to-pdf-clippy";
        nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
        buildPhase = "cargo clippy --release --all-targets --offline -- -D warnings";
        doCheck = false;
        installPhase = "touch $out";
      });

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
      packages = forAll (pkgs: {
        default = toPdfFor pkgs;
        to-pdf = toPdfFor pkgs;
        smoke = pkgs.runCommand "ptbr-pdf-smoke" { nativeBuildInputs = [ (toPdfFor pkgs) pkgs.poppler-utils ]; } ''
          export HOME=$TMPDIR OUT_DIR=$TMPDIR/out
          bash ${./example}/build.sh
          pdftotext "$OUT_DIR/smoke.pdf" - | grep -q 'ação, coração, pão' || { echo "smoke: accents lost" >&2; exit 1; }
          pdftotext "$OUT_DIR/smoke.pdf" - | grep -q '⚠' || { echo "smoke: ⚠ lost" >&2; exit 1; }
          mkdir -p "$out" && cp "$OUT_DIR/smoke.pdf" "$out"/
        '';
      } // lib.optionalAttrs (pkgs.stdenv.hostPlatform.system == "x86_64-linux") {
        # a static musl binary, for release assets
        static = toPdfFor pkgs.pkgsStatic;
      });

      apps = forAll (pkgs: {
        default = { type = "app"; program = "${toPdfFor pkgs}/bin/to-pdf"; };
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          name = "ptbr-pdf";
          packages = [ pkgs.rustc pkgs.cargo pkgs.clippy pkgs.rustfmt pkgs.rust-analyzer ] ++ lib.optional pkgs.stdenv.hostPlatform.isLinux pkgs.gcc;
        };
      });

      checks = forAll (pkgs: {
        lab-json = labCardFor pkgs;
        to-pdf = toPdfFor pkgs; # builds and runs cargo test
        clippy = clippyFor pkgs;
        fmt = pkgs.runCommand "to-pdf-fmt" { nativeBuildInputs = [ pkgs.rustfmt ]; } ''
          rustfmt --check --edition 2021 ${src}/src/main.rs && touch $out
        '';
        smoke = self.packages.${pkgs.stdenv.hostPlatform.system}.smoke;
      });
    };
}
