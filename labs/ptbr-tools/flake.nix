{
  description = "ptbr-tools — the runtime scripts of the ptbr-writer plugin as small static Rust binaries";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable"; # locked to the same rev as labs/ptbr-pdf
  };

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAll = f: lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});

      src = lib.fileset.toSource {
        root = ./.;
        fileset = lib.fileset.unions [ ./Cargo.toml ./Cargo.lock ./src ./tests ];
      };

      # One multi-call binary, `ptbr`, with ptbr-ask-levels, ptbr-reading-time and ptbr-route as
      # symlinks to it: std only, no crates, one copy of std (0.54 MiB against 1.37 MiB for
      # three binaries). Tests run in the build (checkPhase).
      toolsFor = pkgs: pkgs.rustPlatform.buildRustPackage {
        pname = "ptbr-tools";
        version = "0.1.0";
        inherit src;
        cargoLock.lockFile = ./Cargo.lock;
        stripAllList = [ "bin" ]; # the symbol table too, not only debug info
        postInstall = ''
          for c in ask-levels reading-time route; do ln -s ptbr "$out/bin/ptbr-$c"; done
        '';
        meta.mainProgram = "ptbr";
      };

      # bytes: the measured size of the static binary plus 10% (README, Sizes).
      sizeCeiling = 627123;

      # Same sources, `cargo clippy -D warnings` as the build.
      clippyFor = pkgs: (toolsFor pkgs).overrideAttrs (old: {
        pname = "ptbr-tools-clippy";
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
        default = toolsFor pkgs;
      } // lib.optionalAttrs (pkgs.stdenv.hostPlatform.system == "x86_64-linux") {
        # static musl binaries, for the plugin to ship
        static = toolsFor pkgs.pkgsStatic;
        static-aarch64 = toolsFor pkgs.pkgsCross.aarch64-multiplatform.pkgsStatic;
      });

      apps = forAll (pkgs: {
        default = { type = "app"; program = "${toolsFor pkgs}/bin/ptbr"; };
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          name = "ptbr-tools";
          packages = [ pkgs.rustc pkgs.cargo pkgs.clippy pkgs.rustfmt pkgs.rust-analyzer ] ++ lib.optional pkgs.stdenv.hostPlatform.isLinux pkgs.gcc;
        };
      });

      checks = forAll (pkgs:
        let system = pkgs.stdenv.hostPlatform.system; in {
          lab-json = labCardFor pkgs;
          ptbr-tools = toolsFor pkgs; # builds and runs cargo test
          clippy = clippyFor pkgs;
          fmt = pkgs.runCommand "ptbr-tools-fmt" { nativeBuildInputs = [ pkgs.cargo pkgs.rustfmt ]; } ''
            cargo fmt --check --manifest-path ${src}/Cargo.toml && touch $out
          '';
        } // lib.optionalAttrs (system == "x86_64-linux") {
          # the static binary must stay light: fails if it grows past the ceiling (measured size + 10%)
          static-aarch64 = self.packages.${system}.static-aarch64; # builds; not run here, there is no emulator
          size = pkgs.runCommand "ptbr-tools-size" { } ''
            size=$(stat -L -c %s ${self.packages.${system}.static}/bin/ptbr)
            echo "ptbr: $size bytes, ceiling ${toString sizeCeiling}"
            [ "$size" -le ${toString sizeCeiling} ] || { echo "ptbr grew past its ceiling" >&2; exit 1; }
            touch $out
          '';
        });
    };
}
