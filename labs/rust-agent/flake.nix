{
  description = "rust-agent — the Rust toolchain for a CLI that reads Brazilian weather alerts and calls an LLM over HTTPS: rustc, cargo, clippy, rustfmt, rust-analyzer, nextest, deny, audit, machete";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAll = f: lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});

      # The one place the tools are named. A consumer appends this to its own mkShell packages.
      toolsFor = pkgs: with pkgs; [
        rustc
        cargo
        clippy
        rustfmt
        rust-analyzer
        cargo-nextest
        cargo-deny
        cargo-audit
        cargo-machete # reads Cargo.toml and the sources; cargo-udeps needs nightly
        pkg-config
        openssl # only for crates with native TLS; prefer rustls (see README)
        jq
        xh
      ];

      # `nix flake check` builds this: every tool answers --version in the sandbox, so a nixpkgs
      # bump that drops or breaks one fails here, not in a consumer's CI.
      versionsFor = pkgs: pkgs.runCommand "rust-agent-versions" { nativeBuildInputs = toolsFor pkgs; } ''
        {
          echo "nixpkgs       ${nixpkgs.rev or "dirty"}"
          echo "rustc         $(rustc --version)"
          echo "cargo         $(cargo --version)"
          echo "clippy        $(cargo-clippy --version)"
          echo "rustfmt       $(rustfmt --version)"
          echo "rust-analyzer $(rust-analyzer --version)"
          echo "nextest       $(cargo-nextest nextest --version | head -1)"
          echo "deny          $(cargo-deny --version)"
          echo "audit         $(cargo-audit --version)"
          echo "machete       $(cargo-machete --version)"
          echo "pkg-config    $(pkg-config --version)"
          echo "openssl       $(openssl version)"
          echo "jq            $(jq --version)"
          echo "xh            $(xh --version | head -1)"
        } | tee versions.txt
        mkdir -p "$out" && cp versions.txt "$out"/
      '';

      # No dependencies, so no vendoring: the lock file has only the crate itself. Tests run in
      # the build (checkPhase).
      smokeFor = pkgs: pkgs.rustPlatform.buildRustPackage {
        pname = "rust-agent-smoke";
        version = "0.1.0";
        src = lib.fileset.toSource {
          root = ./smoke;
          fileset = lib.fileset.unions [ ./smoke/Cargo.toml ./smoke/Cargo.lock ./smoke/src ];
        };
        cargoLock.lockFile = ./smoke/Cargo.lock;
        meta.mainProgram = "rust-agent-smoke";
      };

      # Same sources, `cargo clippy -D warnings` as the build.
      clippyFor = pkgs: (smokeFor pkgs).overrideAttrs (old: {
        pname = "rust-agent-smoke-clippy";
        nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
        buildPhase = "cargo clippy --release --all-targets --offline -- -D warnings";
        doCheck = false;
        installPhase = "touch $out";
      });

      fmtFor = pkgs: pkgs.runCommand "rust-agent-smoke-fmt" { nativeBuildInputs = [ pkgs.rustfmt ]; } ''
        rustfmt --check --edition 2021 ${./smoke/src/main.rs}
        touch $out
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

      packages = forAll (pkgs: {
        versions = versionsFor pkgs;
        inherit (pkgs) rustc cargo clippy rustfmt rust-analyzer cargo-nextest cargo-deny cargo-audit cargo-machete pkg-config openssl jq xh;
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          name = "rust-agent";
          packages = toolsFor pkgs;
          RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
          shellHook = ''echo "rust-agent: $(rustc --version) | $(cargo --version)"'';
        };
      });

      checks = forAll (pkgs: {
        lab-json = labCardFor pkgs;
        versions = self.packages.${pkgs.stdenv.hostPlatform.system}.versions;
        smoke = pkgs.linkFarm "rust-agent-smoke-checks" [
          { name = "build"; path = smokeFor pkgs; }
          { name = "clippy"; path = clippyFor pkgs; }
          { name = "fmt"; path = fmtFor pkgs; }
        ];
      });
    };
}
