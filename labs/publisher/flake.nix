{
  description = "publisher — markdown → LaTeX → PDF toolchain (pandoc, TeX Live, python+openai) and a mkPdf helper for sandboxed document builds";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable"; # locked to the same rev as labs/pratico
  };

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      forAll = f: lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});

      # TeX Live: scheme-medium plus what the ww-lab documents need (found with \listfiles).
      # xelatex (book: DejaVu by filename, no fontconfig) and pdflatex (UFRJ proposal) both come
      # from this one environment.
      texFor = pkgs: pkgs.texliveMedium.withPackages (ps: with ps; [
        babel-portuges
        hyphen-portuguese
        dejavu
        fontspec
        unicode-math
        xetex
        booktabs
        caption
        enumitem
        float
        multirow
        tools
        framed
        fvextra
        lineno
        microtype
        titlesec
        upquote
        xcolor
        csquotes
        luatexja # Japanese typesetting under lualatex (CJK line breaking, jfonts)
        # luatexja's default jfont and the one to use: static OTFs (nixpkgs' Noto Sans CJK is a
        # variable-font collection luatexja cannot instantiate for bold or vertical faces)
        haranoaji
      ]);
      pythonFor = pkgs: pkgs.python3.withPackages (ps: [ ps.openai ]);
      # librsvg: pandoc's LaTeX writer converts SVG images with rsvg-convert.
      toolsFor = pkgs: [ pkgs.pandoc (texFor pkgs) (pythonFor pkgs) pkgs.just pkgs.poppler-utils pkgs.librsvg ];

      # What a consumer's pandoc/lualatex needs to find this lab's filter, style and emoji font.
      # Exported by mkPdf and the devShell; also exposed as lib.<system>.env.
      envFor = pkgs: {
        PUBLISHER_FILTERS = "${self}/filters";
        TEXINPUTS = "${self}/tex//:";
        # luaotfload: "Noto Color Emoji" (emoji fallback). kpathsea list: keep the `//` (search
        # subdirectories) suffix on every entry — with more than one plain entry luaotfload's
        # database finds nothing in the sandbox. Japanese uses Harano Aji from the TeX tree.
        OSFONTDIR = "${pkgs.noto-fonts-color-emoji}/share/fonts//";
      };

      # mkDocument: run `command` inside `src` with this toolchain in the Nix sandbox and collect
      # the documents it leaves in $OUT_DIR. The consumer owns the sources and the script.
      # `formats` lists the extensions to collect; the build fails when it produced none of them.
      #
      #   publisher.lib.${system}.mkPdf  { name = "my-report"; src = ./.; command = "..."; }
      #   publisher.lib.${system}.mkDocx { name = "my-report-docx"; src = ./.; command = "..."; }
      #   publisher.lib.${system}.mkDocument { ...; formats = [ "pdf" "docx" ]; }
      mkDocumentFor = pkgs: { name, src, command, formats ? [ "pdf" ] }: pkgs.stdenv.mkDerivation ({
        inherit name src;
        nativeBuildInputs = toolsFor pkgs;
        dontConfigure = true;
        buildPhase = ''
          export HOME=$TMPDIR TEXMFVAR=$TMPDIR/texmf-var OUT_DIR=$TMPDIR/out
          mkdir -p "$OUT_DIR"
          ${command}
        '';
        installPhase = ''
          mkdir -p "$out"
          shopt -s nullglob
          collected=0
          for ext in ${lib.concatStringsSep " " formats}; do
            for doc in "$TMPDIR"/out/*."$ext"; do
              cp "$doc" "$out"/
              collected=$((collected + 1))
            done
          done
          if [ "$collected" -eq 0 ]; then
            echo "publisher: ${name} produced no ${lib.concatStringsSep "/" formats} file in \$OUT_DIR" >&2
            exit 1
          fi
        '';
      } // envFor pkgs);

      # What checks.filter expects in example/badges.md's LaTeX: one exact macro per badge shape.
      filterExpects = [
        ''\badge{}{Scala}{DC322F}''
        ''\badge{AWS}{ML Specialty}{FF9900}''
        ''\href{https://github.com/topics/civic-tech}{\badge{}{civic-tech}{0077BE}}''
        ''\badge{}{Hugging Face}{FFD21E}''
        ''\badge{build}{passing}{44CC11}''
        ''\href{https://marola.dev}{\badge{}{🌊 marola.dev}{0077BE}}''
        ''\badge{PADI}{Rescue Diver}{0B5FA5}''
        ''Texto normal com \textbf{negrito} continua igual.''
        ''{\raggedright \badge{}{Scala}{DC322F}''
      ];

      mkPdfFor = pkgs: args: mkDocumentFor pkgs ({ formats = [ "pdf" ]; } // args);
      mkDocxFor = pkgs: args: mkDocumentFor pkgs ({ formats = [ "docx" ]; } // args);

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
        python = pythonFor pkgs;
        tools = toolsFor pkgs;
        mkPdf = mkPdfFor pkgs;
        mkDocx = mkDocxFor pkgs;
        mkDocument = mkDocumentFor pkgs;
        env = envFor pkgs;
      });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          name = "publisher";
          packages = toolsFor pkgs;
          shellHook = ''
            ${lib.concatStringsSep "\n" (lib.mapAttrsToList (k: v: "export ${k}=${lib.escapeShellArg v}") (envFor pkgs))}
            echo "publisher: pandoc $(pandoc --version | head -1 | cut -d' ' -f2) | $(xelatex --version | head -1) | $(lualatex --version | head -1)"
          '';
        };
      });

      packages = forAll (pkgs: {
        tex = texFor pkgs;
        # The smoke document, so `nix build .#smoke` shows a real PDF from this toolchain.
        smoke = mkPdfFor pkgs {
          name = "publisher-smoke";
          src = ./example;
          # the same sample through both engines this lab supports, then the lualatex badges
          command = ''
            here=$PWD build=$OUT_DIR
            common=(--citeproc --bibliography "$here/refs.bib" --fail-if-warnings)
            # xelatex: unicode symbols via DejaVu Sans loaded by filename (no fontconfig in the sandbox)
            pandoc "$here/smoke.md" "''${common[@]}" --pdf-engine=xelatex \
              -V mainfont=DejaVuSans -V mainfontoptions="Extension=.ttf" -V monofont=DejaVuSansMono -V monofontoptions="Extension=.ttf" \
              -o "$build/smoke-xelatex.pdf"
            # pdflatex: T1/utf8 path (the UFRJ proposal template); strip the symbols it cannot encode
            sed 's/⚠ ✔ → ≥/(simbolos omitidos em pdflatex)/' "$here/smoke.md" > "$build/smoke-pdflatex.md"
            pandoc "$build/smoke-pdflatex.md" "''${common[@]}" --pdf-engine=pdflatex -o "$build/smoke-pdflatex.pdf"
            rm -f "$build/smoke-pdflatex.md"
            # lualatex: shields badges → TikZ pills through the filter, color emoji through the fallback font,
            # Japanese through luatexja. Two explicit steps (pandoc → .tex, lualatex → .pdf) so a TeX failure
            # shows its own log here instead of pandoc's one-line summary.
            pandoc "$here/badges.md" --from markdown+raw_html --to latex --standalone --fail-if-warnings \
              --lua-filter "$PUBLISHER_FILTERS/shields-badges.lua" -H "$here/badges-preamble.tex" \
              -o "$build/badges-lualatex.tex"
            if ! (cd "$build" && lualatex -interaction=nonstopmode -halt-on-error badges-lualatex.tex > badges-lualatex.stdout 2>&1); then
              echo "badges-lualatex.tex: lualatex failed; log tail:" >&2
              tail -n 60 "$build/badges-lualatex.log" >&2
              exit 1
            fi
            if grep -q 'Missing character' "$build/badges-lualatex.log"; then
              echo "badges-lualatex.pdf: glyphs missing (a font lacks a character that should have a fallback):" >&2
              grep 'Missing character' "$build/badges-lualatex.log" | sort -u | head >&2
              exit 1
            fi
            rm -f "$build"/badges-lualatex.{aux,out,stdout}
            # luaotfload embeds color emoji as bitmap images (one image + soft mask each); badges.md has four
            # SMP emoji. ☁ ❄ are BMP symbols DejaVu Sans itself has, so they stay monochrome (no fallback).
            emoji="$(pdfimages -list "$build/badges-lualatex.pdf" | awk 'NR>2 && $3=="image"' | wc -l)"
            [ "$emoji" -ge 4 ] || { echo "badges-lualatex.pdf: expected 4 color emoji bitmaps, found $emoji" >&2; exit 1; }
            # Japanese text must survive as text (luatexja + Harano Aji Gothic), not tofu
            pdftotext "$build/badges-lualatex.pdf" - | grep -q '決済処理と不正対策' || { echo "badges-lualatex.pdf: Japanese text missing" >&2; exit 1; }
            pdffonts "$build/badges-lualatex.pdf" | grep -q 'HaranoAjiGothic' || { echo "badges-lualatex.pdf: Harano Aji Gothic not embedded" >&2; exit 1; }
            echo "publisher smoke: $build/smoke-xelatex.pdf $build/smoke-pdflatex.pdf $build/badges-lualatex.pdf"
          '';
        };
        # The same sample as .docx: pandoc's docx writer, no TeX in the path.
        smoke-docx = mkDocxFor pkgs {
          name = "publisher-smoke-docx";
          src = ./example;
          # no TeX in the path: docx is what a consumer asks for to edit or comment in Word or Google Docs
          command = ''
            pandoc smoke.md --citeproc --bibliography refs.bib --fail-if-warnings -o "$OUT_DIR/smoke.docx"
            # A .docx is a zip of XML parts. Prove the writer produced a real document part and that the
            # sample's text survived, so a broken pandoc build fails here and not in a consumer's release.
            unzip=${pkgs.unzip}/bin/unzip
            parts="$("$unzip" -Z1 "$OUT_DIR/smoke.docx")"
            grep -qx word/document.xml <<<"$parts" || { echo "word/document.xml missing: $parts" >&2; exit 1; }
            xml="$("$unzip" -p "$OUT_DIR/smoke.docx" word/document.xml)"
            for expected in hifenização WAVEWATCH; do
              grep -qF "$expected" <<<"$xml" || { echo "$expected missing from word/document.xml" >&2; exit 1; }
            done
            echo "docx ok: $(wc -l <<<"$parts") parts"
            echo "publisher docx smoke: $OUT_DIR/smoke.docx"
          '';
        };
      });

      # CI (nix flake check) builds this: pandoc → xelatex and pandoc → pdflatex on a sample with
      # math, a table, code, Portuguese hyphenation and a citation — everything the documents use.
      checks = forAll (pkgs: {
        lab-json = labCardFor pkgs;
        smoke = self.packages.${pkgs.system}.smoke;
        # docx is optional for consumers but not untested here: the writer must produce a real
        # document part with the sample's text and citation in it (the smoke-docx command).
        docx = self.packages.${pkgs.system}.smoke-docx;
        # The shields filter alone, to LaTeX source: exact \badge macros for every badge shape.
        filter = pkgs.runCommand "publisher-filter-check" ({ nativeBuildInputs = [ pkgs.pandoc ]; } // envFor pkgs) ''
          pandoc ${./example}/badges.md --from markdown+raw_html --to latex \
            --lua-filter "$PUBLISHER_FILTERS/shields-badges.lua" -o badges.tex
          ${lib.concatMapStrings (e: ''
            grep -qF -- ${lib.escapeShellArg e} badges.tex || { echo "filter: missing: "${lib.escapeShellArg e} >&2; cat badges.tex >&2; exit 1; }
          '') filterExpects}
          if grep -q 'not-a-badge' badges.tex; then echo "filter: non-shields image leaked into LaTeX" >&2; exit 1; fi
          mkdir -p "$out" && cp badges.tex "$out"/
        '';
      });
    };
}
