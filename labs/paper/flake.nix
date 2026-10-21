{
  description = "paper — Markdown papers to PDF and arXiv source: pandoc + pandoc-crossref + citeproc, XeLaTeX and pdfLaTeX, python, just, gh, rsvg-convert";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable"; # locked to the same rev as labs/publisher
  };

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAll = f: lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});

      # TeX Live: scheme-medium plus what h0ffmann/closed-source's template loads. XeLaTeX builds
      # the PDF; pdfLaTeX checks the arXiv source, which arXiv compiles with pdfLaTeX.
      texFor = pkgs: pkgs.texliveMedium.withPackages (ps: with ps; [
        xetex
        fontspec
        unicode-math
        lm
        lm-math
        babel-english
        csquotes
        enumitem
        etoolbox
        fancyhdr
        footnotehyper
        titling
        xurl
        booktabs
        microtype
        xcolor
        upquote
        bookmark
        parskip
      ]);

      # pandoc-crossref from haskellPackages is built against the same pandoc as pkgs.pandoc, so
      # the filter and the converter agree (a mismatch is what the apt + release-binary route got
      # wrong: crossref 0.3.16 for pandoc 3.1.2 under pandoc 3.1.3).
      toolsFor = pkgs: [
        pkgs.pandoc
        pkgs.haskellPackages.pandoc-crossref
        (texFor pkgs)
        pkgs.python3
        pkgs.just
        pkgs.git
        pkgs.gh
        pkgs.poppler-utils
        pkgs.librsvg # rsvg-convert: SVG logos and figures to PDF, which XeLaTeX cannot include
      ];

      # `nix flake check` builds this: every tool answers --version in the sandbox, so a nixpkgs
      # bump that drops or breaks one fails here, not in a paper's CI.
      versionsFor = pkgs: pkgs.runCommand "paper-versions" { nativeBuildInputs = toolsFor pkgs; } ''
        export HOME=$TMPDIR
        {
          echo "nixpkgs          ${nixpkgs.rev or "dirty"}"
          echo "pandoc           $(pandoc --version | head -1)"
          echo "pandoc-crossref  $(pandoc-crossref --version | head -1)"
          echo "xelatex          $(xelatex --version | head -1)"
          echo "pdflatex         $(pdflatex --version | head -1)"
          echo "python3          $(python3 --version)"
          echo "just             $(just --version)"
          echo "git              $(git --version)"
          echo "gh               $(gh --version | head -1)"
          echo "pdfinfo          $(pdfinfo -v 2>&1 | head -1)"
          echo "rsvg-convert     $(rsvg-convert --version)"
        } | tee versions.txt
        mkdir -p "$out" && cp versions.txt "$out"/
      '';

      # The sample paper through both engines, in the sandbox: crossref and citeproc must resolve
      # every reference (--fail-if-warnings), and the PDFs must carry the resolved text.
      smokeFor = pkgs: pkgs.runCommand "paper-smoke" { nativeBuildInputs = toolsFor pkgs; } ''
        export HOME=$TMPDIR TEXMFVAR=$TMPDIR/texmf-var
        cp -r ${./example} example && chmod -R u+w example
        here=$PWD/example build=$TMPDIR/out
        mkdir -p "$build"
        # one paper with cross-references and a citation, through XeLaTeX (the PDF) and pdfLaTeX
        # (what arXiv runs); warnings fail the build, so an unresolved reference fails it
        common=(-F pandoc-crossref --citeproc --bibliography "$here/refs.bib" --number-sections --fail-if-warnings)
        for engine in xelatex pdflatex; do
          pandoc "$here/paper.md" "''${common[@]}" --pdf-engine="$engine" -o "$build/paper-$engine.pdf"
          text="$(pdftotext "$build/paper-$engine.pdf" -)"
          for want in "Section 2" "Table 1" "Raymond" "10.5210/fm.v3i2.578"; do
            grep -qF "$want" <<<"$text" || { echo "paper-$engine.pdf: missing '$want'" >&2; exit 1; }
          done
        done
        echo "paper smoke: $build/paper-xelatex.pdf $build/paper-pdflatex.pdf"
        mkdir -p "$out" && cp "$build"/*.pdf "$out"/
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
        tex = texFor pkgs;
        tools = toolsFor pkgs;
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          name = "paper";
          packages = toolsFor pkgs;
          shellHook = ''echo "paper: $(pandoc --version | head -1) | $(pandoc-crossref --version | head -1) | $(xelatex --version | head -1)"'';
        };
      });

      packages = forAll (pkgs: {
        tex = texFor pkgs;
        versions = versionsFor pkgs;
        smoke = smokeFor pkgs;
        pandoc-crossref = pkgs.haskellPackages.pandoc-crossref;
      });

      checks = forAll (pkgs: {
        lab-json = labCardFor pkgs;
        versions = self.packages.${pkgs.stdenv.hostPlatform.system}.versions;
        smoke = self.packages.${pkgs.stdenv.hostPlatform.system}.smoke;
      });
    };
}
