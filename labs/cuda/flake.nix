{
  description = "cuda — host setup for a CUDA box: the nixos-cuda binary cache in nix.custom.conf, and a torch venv from PyTorch's wheel index with libstdc++ and the driver libraries on its path";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { nixpkgs, ... }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" ]; # CUDA wheels and the driver-library layout are x86_64 Linux only
      forAll = f: lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});

      # One crate (tools/), one binary per tool: the tests run in each build (checkPhase), and the
      # wrapper puts the few host commands the tools call on PATH. nix, systemctl and getent are the
      # host's own and stay off the list.
      src = lib.fileset.toSource {
        root = ./tools;
        fileset = lib.fileset.unions [ ./tools/Cargo.toml ./tools/Cargo.lock ./tools/src ];
      };
      toolFor = pkgs: { name, runtimeInputs }: pkgs.rustPlatform.buildRustPackage {
        pname = name;
        version = "0.1.0";
        inherit src;
        # baked into setup-ml-venv at compile time, so it never shells out to `nix build` at run time;
        # set for both builds because each runs the whole crate's tests
        env.LIBSTDCXX_DIR = "${pkgs.stdenv.cc.cc.lib}/lib";
        cargoLock.lockFile = ./tools/Cargo.lock;
        cargoBuildFlags = [ "--bin" name ];
        nativeBuildInputs = [ pkgs.makeWrapper ];
        postInstall = "wrapProgram $out/bin/${name} --prefix PATH : ${lib.makeBinPath runtimeInputs}";
        meta = { mainProgram = name; license = lib.licenses.agpl3Only; };
      };
      setupCudaCacheFor = pkgs: toolFor pkgs { name = "setup-cuda-cache"; runtimeInputs = [ pkgs.coreutils ]; };
      setupMlVenvFor = pkgs: toolFor pkgs { name = "setup-ml-venv"; runtimeInputs = [ pkgs.python3 ]; };
      toolsFor = pkgs: [ (setupCudaCacheFor pkgs) (setupMlVenvFor pkgs) ];

      # Same sources, `cargo clippy -D warnings`.
      clippyFor = pkgs: (setupCudaCacheFor pkgs).overrideAttrs (old: {
        pname = "cuda-tools-clippy";
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
      lib = forAll (pkgs: { tools = toolsFor pkgs; });

      packages = forAll (pkgs: {
        setup-cuda-cache = setupCudaCacheFor pkgs;
        setup-ml-venv = setupMlVenvFor pkgs;
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          name = "cuda";
          packages = toolsFor pkgs ++ [ pkgs.cargo pkgs.rustc pkgs.clippy ];
          shellHook = ''echo "cuda: setup-cuda-cache --dry-run | REQUIREMENTS=<file> setup-ml-venv"'';
        };
      });

      # The generic halves only: no GPU, no root, no network in the sandbox. Building a tool runs its
      # `cargo test` (the old --self-tests), so the package is the check.
      checks = forAll (pkgs: {
        lab-json = labCardFor pkgs;
        setup-cuda-cache = setupCudaCacheFor pkgs;
        setup-ml-venv = setupMlVenvFor pkgs;
        clippy = clippyFor pkgs;
      });
    };
}
