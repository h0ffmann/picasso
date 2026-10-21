{
  description = "agentic — ai-jail, OpenCode, Open Code Review, gh, and the host-side tools (gh-token, clip, clip-relay, jail-run), written in Rust for running coding agents sandboxed";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    ai-jail = {
      url = "github:akitaonrails/ai-jail";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, ai-jail, ... }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAll = f: lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});

      # The tools are one Rust crate (tools/), one binary each. `cargo test` runs in the build, so a
      # package that builds has passed its tests; the tests fake `gh` and `ai-jail` on PATH.
      toolsSrc = lib.fileset.toSource {
        root = ./tools;
        fileset = lib.fileset.unions [ ./tools/Cargo.toml ./tools/Cargo.lock ./tools/src ./tools/tests ];
      };
      toolsFor = pkgs: pkgs.rustPlatform.buildRustPackage {
        pname = "agentic-tools";
        version = "0.1.0";
        src = toolsSrc;
        cargoLock.lockFile = ./tools/Cargo.lock;
        meta.license = lib.licenses.asl20;
      };

      # One package per binary, named as before. `runtimeInputs` never lists a tool the tests fake
      # on PATH: gh-token finds `gh` on the caller's PATH, jail-run finds `ai-jail` there.
      wrap = pkgs: name: runtimeInputs: pkgs.runCommand name
        {
          nativeBuildInputs = [ pkgs.makeWrapper ];
          meta = { mainProgram = name; license = lib.licenses.asl20; };
        }
        ''
          makeWrapper ${toolsFor pkgs}/bin/${name} $out/bin/${name} ${lib.optionalString (runtimeInputs != [ ]) "--prefix PATH : ${lib.makeBinPath runtimeInputs}"}
        '';
      scriptsFor = pkgs: rec {
        gh-token = wrap pkgs "gh-token" [ ];
        clip = wrap pkgs "clip" [ pkgs.coreutils ];
        clip-relay = wrap pkgs "clip-relay" [ pkgs.coreutils ];
        jail-run = wrap pkgs "jail-run" [ gh-token clip-relay pkgs.coreutils ];
      };

      # ai-jail's test suite needs a working sandbox at build time, which the Nix build sandbox
      # does not provide — same override marola and labs/pratico use.
      aiJailFor = pkgs: ai-jail.packages.${pkgs.stdenv.hostPlatform.system}.default.overrideAttrs (_: { doCheck = false; });

      # alibaba/open-code-review's `ocr`, from source rather than the release binary: the module
      # wants `go 1.25.5` and this lab's nixpkgs carries Go 1.26, so buildGoModule works and the
      # build stays a source build on every system instead of an x86_64 blob. `rev` is the commit
      # the *annotated* tag v1.12.7 dereferences to. The upstream binary is named `ocr` (the Go
      # package is `cmd/opencodereview`), so the install is renamed to match.
      ocrVersion = "1.12.7";
      ocrFor = pkgs: pkgs.buildGoModule {
        pname = "ocr";
        version = ocrVersion;
        src = pkgs.fetchFromGitHub {
          owner = "alibaba";
          repo = "open-code-review";
          rev = "85cecfe5f935da2b2aae8f91ce4fee8ed343a681";
          hash = "sha256-L1xiiwbgRRZFeVYKwK3Huod97i1lJD12c8EVJGoUW1s=";
        };
        vendorHash = "sha256-f5Ty22wicf1J8+RKnHYcEO7flWn9gkWQODlfunn23EA=";
        subPackages = [ "cmd/opencodereview" ];
        ldflags = [ "-s" "-w" "-X" "main.Version=v${ocrVersion}" "-X" "main.GitCommit=85cecfe" ];
        # Upstream's suite includes e2e tests that talk to a live LLM endpoint and to github.com;
        # nothing in the Nix sandbox can answer them.
        doCheck = false;
        postInstall = "mv $out/bin/opencodereview $out/bin/ocr";
        meta = {
          description = "Open Code Review — an LLM code reviewer that runs against any OpenAI- or Anthropic-compatible endpoint";
          homepage = "https://github.com/alibaba/open-code-review";
          license = lib.licenses.asl20;
          mainProgram = "ocr";
        };
      };

      toolListFor = pkgs:
        let s = scriptsFor pkgs; in
        [ pkgs.gh pkgs.opencode (ocrFor pkgs) s.gh-token s.clip ]
        ++ lib.optionals pkgs.stdenv.hostPlatform.isLinux [
          (aiJailFor pkgs)
          pkgs.bubblewrap
          pkgs.wl-clipboard # clip / clip-relay sinks
          pkgs.xclip
          s.clip-relay
          s.jail-run
        ];

      envFor = pkgs: lib.optionalAttrs pkgs.stdenv.hostPlatform.isLinux {
        # ai-jail's own devShell sets this; it does not propagate when consumed as a package.
        BWRAP_BIN = "${pkgs.bubblewrap}/bin/bwrap";
      };

      # Same sources, `cargo clippy -D warnings` as the build.
      clippyFor = pkgs: (toolsFor pkgs).overrideAttrs (old: {
        pname = "agentic-tools-clippy";
        nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
        buildPhase = "cargo clippy --release --all-targets --offline -- -D warnings";
        doCheck = false;
        installPhase = "touch $out";
      });

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
        tools = toolListFor pkgs;
        env = envFor pkgs;
        scripts = scriptsFor pkgs;
      });

      packages = forAll (pkgs: scriptsFor pkgs // { ocr = ocrFor pkgs; });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell ({
          name = "agentic";
          packages = toolListFor pkgs;
          shellHook = ''echo "agentic: gh $(gh --version | head -1 | cut -d' ' -f3) | ocr ${ocrVersion} | $(command -v ai-jail >/dev/null && echo ai-jail || echo 'ai-jail: Linux only') | jail-run claude|opencode|ocr from a directory with .ai-jail"'';
        } // envFor pkgs);
      });

      # Built by `nix flake check`: the crate's tests run in the sandbox against fakes on PATH (no
      # gh login, no ai-jail, no display needed), then clippy and rustfmt over the same sources.
      checks = forAll (pkgs: {
        tools = toolsFor pkgs; # builds and runs cargo test
        clippy = clippyFor pkgs;
        fmt = pkgs.runCommand "agentic-tools-fmt" { nativeBuildInputs = [ pkgs.rustfmt ]; } ''
          rustfmt --check --edition 2021 ${toolsSrc}/src/bin/*.rs ${toolsSrc}/tests/*.rs && touch $out
        '';
        lab-json = labCardFor pkgs;
      });
    };
}
