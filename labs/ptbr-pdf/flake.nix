{
  description = "ptbr-pdf — Markdown to PDF for pt-BR text: pandoc and a minimal XeLaTeX, tailored for ptbr-writter";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable"; # locked to the same rev as labs/publisher
  };

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAll = f: lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});

      # scheme-basic plus the packages pandoc's LaTeX template loads under xelatex for Portuguese
      # text with tables, code and links (example/ builds with them). No TeX Live medium, no
      # lualatex fonts, no CJK, no python: that is publisher's job. 729 MiB closure against
      # publisher's 3.2 GiB (nix path-info -Sh, x86_64-linux, 2026-10-20).
      texFor = pkgs: pkgs.texliveBasic.withPackages (ps: with ps; [
        xetex
        fontspec
        unicode-math
        lm-math
        babel-portuges
        hyphen-portuguese
        dejavu
        geometry
        booktabs
        etoolbox
        upquote
        xcolor
        bookmark
        fancyvrb
        parskip
        tools
        selnolig
        xurl
        footnotehyper
        iftex
        lualatex-math
      ]);

      # to-pdf FILE.md [OUT.pdf]: the one command. DejaVu by file name, so no fontconfig is needed.
      toPdfFor = pkgs: pkgs.writeShellApplication {
        name = "to-pdf";
        runtimeInputs = [ pkgs.pandoc (texFor pkgs) pkgs.coreutils ];
        text = ''
          in=''${1:?usage: to-pdf texto.md [saida.pdf]}
          out=''${2:-''${in%.*}.pdf}
          export HOME=''${HOME:-$(mktemp -d)} TEXMFVAR=''${TEXMFVAR:-$(mktemp -d)}
          pandoc "$in" --pdf-engine=xelatex -V lang=pt-BR -V geometry:margin=2.5cm \
            -V mainfont=DejaVuSans -V mainfontoptions=Extension=.ttf \
            -V monofont=DejaVuSansMono -V monofontoptions=Extension=.ttf -o "$out"
          echo "$out"
        '';
      };

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
        tex = texFor pkgs;
        smoke = pkgs.runCommand "ptbr-pdf-smoke" { nativeBuildInputs = [ (toPdfFor pkgs) pkgs.poppler-utils ]; } ''
          export HOME=$TMPDIR OUT_DIR=$TMPDIR/out
          bash ${./example}/build.sh
          pdftotext "$OUT_DIR/smoke.pdf" - | grep -q 'ação, coração, pão' || { echo "smoke: accents lost" >&2; exit 1; }
          pdftotext "$OUT_DIR/smoke.pdf" - | grep -q '⚠' || { echo "smoke: ⚠ lost" >&2; exit 1; }
          mkdir -p "$out" && cp "$OUT_DIR/smoke.pdf" "$out"/
        '';
      });

      apps = forAll (pkgs: {
        default = { type = "app"; program = "${toPdfFor pkgs}/bin/to-pdf"; };
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          name = "ptbr-pdf";
          packages = [ (toPdfFor pkgs) pkgs.pandoc (texFor pkgs) ];
        };
      });

      checks = forAll (pkgs: {
        lab-json = labCardFor pkgs;
        smoke = self.packages.${pkgs.system}.smoke;
      });
    };
}
