{
  description = "picasso tools: the Rust crates that CI, the justfiles and the site call (citation, zenodo, social, render, site-check, site-releases, sharingan)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/6510408d147d8c5e6d76043e899e9251756f6a75"; # same rev as labs/ptbr-pdf
  };

  outputs = { nixpkgs, ... }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAll = f: lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});

      crates = [ "citation" "zenodo" "social" "render" "site-check" "site-releases" "sharingan" ];

      # Some tests read repo files by a path relative to their crate (the sharingan skill's
      # reference docs, the whitepaper's zenodo.json), so the source is the repo root, trimmed.
      src = lib.fileset.toSource {
        root = ../.;
        fileset = lib.fileset.unions [
          ./.
          ../.claude/skills/ouroboric-sharingan
          ../docs/whitepapers/determinism-2026
          ../docs/social
        ];
      };

      # What a crate runs as a subprocess and the caller's PATH may not have. Chromium is not
      # wrapped (it is a big closure): render and social find it by CHROMIUM=<path> or on PATH.
      runtime = pkgs: {
        render = [ pkgs.pandoc ]; # `render ste`
        site-releases = [ pkgs.curl pkgs.git ];
        sharingan = [ pkgs.git ]; # `bench` needs the `claude` CLI from the caller's PATH too
        social = [ pkgs.git ];
      };

      # One package per crate; its tests run in the build (checkPhase).
      crateFor = pkgs: crate: pkgs.rustPlatform.buildRustPackage {
        pname = crate;
        version = "0.1.0";
        inherit src;
        sourceRoot = "source/tools";
        cargoLock.lockFile = ./Cargo.lock;
        cargoBuildFlags = [ "-p" crate ];
        cargoTestFlags = [ "-p" crate ];
        nativeBuildInputs = [ pkgs.makeWrapper ];
        nativeCheckInputs = [ pkgs.git ];
        # social finds the repo root with `git rev-parse --show-toplevel`
        preCheck = "chmod u+w .. && git -C .. init -q";
        postInstall = lib.optionalString ((runtime pkgs) ? ${crate}) ''
          wrapProgram $out/bin/${crate} --prefix PATH : ${lib.makeBinPath (runtime pkgs).${crate}}
        '';
        meta.mainProgram = crate;
      };

      # Same sources, the whole workspace under clippy -D warnings.
      clippyFor = pkgs: (crateFor pkgs "citation").overrideAttrs (old: {
        pname = "tools-clippy";
        cargoBuildFlags = [ ];
        nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
        buildPhase = "cargo clippy --workspace --all-targets --offline -- -D warnings";
        doCheck = false;
        postInstall = "";
        installPhase = "touch $out";
      });
    in
    {
      packages = forAll (pkgs: lib.genAttrs crates (crateFor pkgs));

      apps = forAll (pkgs: lib.genAttrs crates (c: {
        type = "app";
        program = "${crateFor pkgs c}/bin/${c}";
      }));

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          name = "picasso-tools";
          packages = [ pkgs.cargo pkgs.rustc pkgs.clippy pkgs.rustfmt pkgs.rust-analyzer pkgs.cargo-insta ]
            ++ lib.optional pkgs.stdenv.hostPlatform.isLinux pkgs.gcc;
        };
      });

      checks = forAll (pkgs: {
        # the workspace builds and `cargo test --workspace` passes
        tests = (crateFor pkgs "citation").overrideAttrs {
          pname = "tools-tests";
          cargoBuildFlags = [ ];
          cargoTestFlags = [ "--workspace" ];
          nativeCheckInputs = [ pkgs.git pkgs.curl pkgs.pandoc ];
          postInstall = "";
        };
        clippy = clippyFor pkgs;
        fmt = pkgs.runCommand "tools-fmt" { nativeBuildInputs = [ pkgs.cargo pkgs.rustfmt ]; } ''
          cd ${src}/tools && cargo fmt --check --all && touch $out
        '';
      });
    };
}
