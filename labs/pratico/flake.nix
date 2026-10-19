{
  description = "prático — WW3-grade Fortran/C++/MPI/NetCDF toolchain for ww-lab, plus a local zsh pilot (zsh-ai → llm → Ollama) and ai-jail for sandboxed agents";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    zsh-ai = {
      url = "github:thiswillbeyourgithub/zsh-ai"; # default branch is `fork`; carries zsh-ai.zsh
      flake = false; # plain source; hash pinned in flake.lock
    };
    # ai-jail — sandboxes AI coding agents (Claude Code, OpenCode, ...) behind
    # bubblewrap/Landlock/seccomp on Linux. See `just jail-*` and `.ai-jail`.
    ai-jail = {
      url = "github:akitaonrails/ai-jail";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, zsh-ai, ai-jail, ... }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" "aarch64-linux" "aarch64-darwin" ];
      # ParMETIS carries a non-commercial licence, so nixpkgs marks it unfree; WW3's PDLIB
      # (unstructured grids, domain decomposition) with the METIS switch needs it. The SCOTCH
      # switch uses PT-Scotch instead (ptscotchStatic below). Allow exactly that one package.
      pkgsFor = system: import nixpkgs {
        inherit system;
        # The CUDA halves are unfree too; only the `cuda` shell below pulls them in.
        config.allowUnfreePredicate = pkg: builtins.elem (lib.getName pkg) [
          "parmetis"
          "cuda_nvcc"
          "cuda_cudart"
          "cuda_cccl"
          "cuda_nvrtc"
          "libcublas"
          "cudatoolkit"
          "cuda-merged"
        ];
        config.cudaSupport = false; # only the #cuda shell pulls CUDA packages in
      };
      forAll = f: lib.genAttrs systems (s: f (pkgsFor s));
      model = "qwen2.5-coder:7b"; # ollama pull qwen2.5-coder:7b  (once)

      # ------------------------------------------------------------------
      # WAVEWATCH III toolchain. Everything WW3's CMake build (v6.07+) and the
      # classic w3_make path look for, from one locked nixpkgs so a build on
      # any machine — laptop, CI, cluster login node — links the same
      # gfortran/OpenMPI/NetCDF bit for bit. Not included: NCEPLIBS-g2/w3emc
      # (not packaged; only needed for GRIB2 output through WW3 itself —
      # ecCodes covers GRIB on the pre/post-processing side).
      # ------------------------------------------------------------------
      ww3Toolchain = pkgs: with pkgs; [
        gfortran # gfortran + the C/C++ front ends it wraps
        cmake
        ninja
        gnumake
        pkg-config
        perl # WW3's switch/comp scripts
        openmpi # mpifort/mpicc/mpicxx live in its dev output
        netcdf # nc-config
        netcdffortran # nf-config, netcdf.mod
        hdf5 # h5dump & co. (NetCDF-4's storage layer); `nf-config --flibs` links it
        zlib
        curl # both also on nf-config's link line
        metis
        parmetis # PDLIB domain decomposition (unfree, see pkgsFor)
        scotch # gpart/gord for grid experiments; WW3 links ptscotchStatic, through SCOTCH_PATH
        eccodes # GRIB read/write for forcing & output post-processing
        openblas
        nco # ncks/ncdiff/ncra on the results
        cdo
      ];

      # ------------------------------------------------------------------
      # Kokkos. nixpkgs' `kokkos` is Serial-only with its (slow) upstream test
      # suite on. ww3-gpu's kernels run Serial + OpenMP on any host and CUDA on
      # the owner's RTX 4090 / an H100, so pin the same source twice with the
      # backends turned on and the tests off. `kokkosTooling` is what goes with
      # them in every shell: GoogleTest for the kernels' unit tests, gdb and
      # valgrind for the debugging exercises.
      # ------------------------------------------------------------------
      kokkosHost = pkgs: pkgs.kokkos.overrideAttrs (_: {
        pname = "kokkos-openmp";
        cmakeFlags = [
          "-DKokkos_ENABLE_SERIAL=ON"
          "-DKokkos_ENABLE_OPENMP=ON"
          "-DKokkos_ENABLE_TESTS=OFF"
          "-DKokkos_ENABLE_DEPRECATED_CODE_4=OFF"
          "-DCMAKE_CXX_STANDARD=20"
        ];
        doCheck = false;
      });
      kokkosCuda = pkgs: (pkgs.kokkos.override { stdenv = pkgs.cudaPackages.backendStdenv; }).overrideAttrs (o: {
        pname = "kokkos-cuda";
        nativeBuildInputs = o.nativeBuildInputs ++ [ pkgs.cudaPackages.cuda_nvcc ];
        buildInputs = (o.buildInputs or [ ]) ++ [ pkgs.cudaPackages.cuda_cudart pkgs.cudaPackages.cccl ]; # cccl: cuda_cccl's current name
        cmakeFlags = [
          "-DKokkos_ENABLE_SERIAL=ON"
          "-DKokkos_ENABLE_OPENMP=ON"
          "-DKokkos_ENABLE_CUDA=ON"
          # Exactly one GPU architecture per build — cmake/kokkos_arch.cmake's CHECK_CUDA_ARCH
          # hard-errors on a second one. ADA89 is the RTX 4090 this lab runs on; for an H100
          # rebuild with HOPPER90 in its place.
          "-DKokkos_ARCH_ADA89=ON"
          "-DKokkos_ENABLE_TESTS=OFF"
          "-DCMAKE_CXX_STANDARD=20"
        ];
        # With CUDA on, Kokkos rejects a plain GNU compiler: the CXX compiler has to be its
        # own nvcc_wrapper, which splits the command line between nvcc and the host g++. It
        # lives in the source tree, so the flag can only be formed after unpackPhase.
        preConfigure = ''
          export NVCC_WRAPPER_DEFAULT_COMPILER="$CXX"
          cmakeFlagsArray+=("-DCMAKE_CXX_COMPILER=$PWD/bin/nvcc_wrapper")
        '';
        doCheck = false;
      });
      kokkosTooling = pkgs: with pkgs; [ gtest gdb valgrind ];

      # ------------------------------------------------------------------
      # What WW3's PDLIB builds and its regtests read, as derivations of this
      # lab's nixpkgs, so a consumer only runs the regtests (ww3-gpu's
      # `just hoff-matrix`) and every machine builds and reads the same bytes.
      # ------------------------------------------------------------------
      # WW3's cmake/FindSCOTCH.cmake wants five static archives under SCOTCH_PATH, the
      # PT-Scotch ones among them, and links nothing else: so MPI on, shared libraries and
      # compression off. SCOTCH_METIS_PREFIX stays on, as in nixpkgs: for SCOTCH > 7.0.6
      # model/src/PDLIB/yowpdlibmain.F90 calls the SCOTCHF-prefixed ParMETIS names.
      ptscotchStatic = pkgs: (pkgs.scotch.override { withPtScotch = true; }).overrideAttrs (o: {
        pname = "ptscotch-static";
        outputs = [ "out" ];
        buildInputs = [ ];
        cmakeFlags = o.cmakeFlags ++ [
          "-DBUILD_SHARED_LIBS=OFF"
          "-DUSE_ZLIB=OFF"
          "-DUSE_BZ2=OFF"
          "-DUSE_LZMA=OFF"
        ];
        doCheck = false;
        postFixup = "";
      });
      # WW3's FindMETIS, FindParMETIS and FindGKLIB all search METIS_PATH alone, so METIS and
      # ParMETIS have to share one prefix.
      ww3Metis = pkgs: pkgs.symlinkJoin { name = "ww3-metis"; paths = [ pkgs.metis pkgs.parmetis ]; };
      # The regtest inputs WW3 keeps off git, from the tarball model/bin/ww3_from_ftp.sh names
      # (ww3ver=v7.14.2), unpacked: data_regtests/ at the top, as that script's cp lines read it.
      ww3RegtestData = pkgs: pkgs.stdenvNoCC.mkDerivation {
        pname = "ww3-regtest-data";
        version = "7.14.2";
        src = pkgs.fetchurl {
          url = "https://ftp.emc.ncep.noaa.gov/static_files/public/WW3/ww3_from_ftp.v7.14.2.tar.gz";
          hash = "sha256-VZgtipk2njlShvqJ9IY+/PisH7LOTIeyBTH9UHmvhGU=";
        };
        dontConfigure = true;
        dontBuild = true;
        installPhase = ''
          mkdir -p "$out/data_regtests"
          cp -r . "$out/data_regtests/"
        '';
      };

      sciPython = ps: with ps; [ numpy scipy xarray netcdf4 matplotlib ];
      aiPython = ps: with ps; [ llm llm-ollama ];

      # Hints WW3's FindNetCDF/FindMETIS and the classic build read. Set once here so
      # `nix develop .#ww3` and the interactive shell agree.
      ww3Env = pkgs: {
        FC = "gfortran";
        F77 = "gfortran";
        F90 = "gfortran";
        CC = "gcc";
        CXX = "g++";
        NETCDF = "${pkgs.netcdf}";
        NETCDF_FORTRAN = "${pkgs.netcdffortran}";
        NETCDF_CONFIG = "${pkgs.netcdf}/bin/nc-config"; # classic w3_make
        WWATCH3_NETCDF = "NC4";
        METIS_PATH = "${ww3Metis pkgs}";
        PARMETIS_PATH = "${pkgs.parmetis}";
        SCOTCH_PATH = "${ptscotchStatic pkgs}"; # WW3's PDLIB with the SCOTCH switch
        # Single-node MPI without a scheduler: let a laptop oversubscribe cores.
        OMPI_MCA_rmaps_base_oversubscribe = "true";
      };

      # `mkShell` re-exports these as plain env vars; keep the attribute-set shape so the
      # two shells below can share it.
      ww3Banner = ''
        echo "ww3 toolchain: $(gfortran --version | head -1) | $(mpirun --version | head -1) | netcdf-c $(nc-config --version | cut -d' ' -f2) / netcdf-fortran $(nf-config --version | cut -d' ' -f2)"
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
      devShells = forAll (pkgs:
        let
          inherit (pkgs.stdenv.hostPlatform) system;
          isLinux = pkgs.stdenv.hostPlatform.isLinux;
          # ai-jail's test suite needs a working sandbox at build time, which the
          # Nix build sandbox does not provide — same override marola uses.
          aiJail = ai-jail.packages.${system}.default.overrideAttrs (_: { doCheck = false; });
        in
        rec {
          # The reproducible toolchain alone: no zsh re-exec, no AI tooling. This is what
          # ww-lab's build scripts and CI should use:  nix develop .#ww3 --command cmake ...
          ww3 = pkgs.mkShell ({
            name = "ww3";
            packages = ww3Toolchain pkgs ++ kokkosTooling pkgs ++ [
              (pkgs.python3.withPackages sciPython)
              (kokkosHost pkgs)
            ];
            shellHook = ww3Banner;
          } // ww3Env pkgs);

          # The interactive shell: the same toolchain, plus zsh-ai → llm → Ollama and ai-jail.
          pratico = pkgs.mkShell ({
            name = "pratico";

            packages = ww3Toolchain pkgs ++ kokkosTooling pkgs ++ [
              (pkgs.python3.withPackages (ps: sciPython ps ++ aiPython ps))
              (kokkosHost pkgs)
              pkgs.zsh
              pkgs.curl
              pkgs.fzf # zsh-ai picks a suggestion through fzf
              pkgs.just # `just jail-claude`, `just pull`, ...
              pkgs.vale # ww3-gpu's `just vale` gate
              pkgs.shellcheck # ww3-gpu's shell gate
              pkgs.lychee # ww3-gpu's Markdown link gate (CI)
            ] ++ lib.optionals isLinux [
              aiJail # bubblewrap/Landlock/seccomp — Linux only
              pkgs.bubblewrap
            ];

            LLM_MODEL = model; # default model for `llm`
            ZSH_AI_LLM_NAME = model; # zsh-ai's own model variable (defaults to o4-mini otherwise)
            OLLAMA_HOST = "http://127.0.0.1:11434"; # host's ollama.service

            shellHook = ww3Banner + ''
              if ! curl -sf "$OLLAMA_HOST/api/tags" >/dev/null 2>&1; then
                echo "prático: Ollama not reachable at $OLLAMA_HOST — run 'just ollama-serve' or check systemd" >&2
              elif ! curl -sf "$OLLAMA_HOST/api/tags" | grep -q '"${model}"'; then
                echo "prático: model ${model} not found — run: just pull" >&2
              fi

              # Your real ~/.zshrc loads the plugin when this var is set:
              #   [[ -n "$ZSH_AI_PLUGIN" ]] && source "$ZSH_AI_PLUGIN"
              export ZSH_AI_PLUGIN=${zsh-ai}/zsh-ai.zsh

              # Re-exec into zsh once (nix develop starts in bash).
              if [ -z "$IN_PRATICO" ]; then
                export IN_PRATICO=1
                exec zsh
              fi
            '';
          } // ww3Env pkgs // lib.optionalAttrs isLinux {
            # ai-jail's own flake sets this in its devShell; it does not propagate when
            # consumed as a package input, so point it at bwrap explicitly.
            BWRAP_BIN = "${pkgs.bubblewrap}/bin/bwrap";
          });

          default = pratico;
        }
        # The CUDA half of the Kokkos pin: only x86_64-linux has cudaPackages, so the shell
        # stays out of every other system's evaluation. It is deliberately not a check —
        # nothing in CI has a GPU to run what an nvcc build of Kokkos would produce.
        // lib.optionalAttrs (system == "x86_64-linux") {
          cuda = pkgs.mkShell ({
            name = "ww3-cuda";
            packages = ww3Toolchain pkgs ++ kokkosTooling pkgs ++ [
              (pkgs.python3.withPackages sciPython)
              (kokkosCuda pkgs)
              pkgs.cudaPackages.cuda_nvcc
              pkgs.cudaPackages.cuda_cudart
            ];
            CUDACXX = "${pkgs.cudaPackages.cuda_nvcc}/bin/nvcc";
            shellHook = ww3Banner + ''
              echo "kokkos: CUDA backend (Ada 8.9, i.e. the RTX 4090) — binary cache: see labs/cuda setup-cuda-cache"

              # libcuda.so.1 comes from the installed NVIDIA driver and from nowhere else: what
              # cuda_cudart ships is a stub that links happily and then fails at run time with
              # cudaErrorStubLibrary. A nix binary does not search /usr/lib, so the driver has to
              # go on LD_LIBRARY_PATH — but never a whole distro lib directory, whose glibc would
              # shadow the nix one every binary in this shell was linked against. So link the
              # NVIDIA libraries alone into a directory of our own and put that on the path.
              # Same trick, same reason, as labs/cuda's setup-ml-venv; copied rather than shared
              # because a lab references nothing outside itself.
              __pratico_drv=""
              for __d in /run/opengl-driver/lib /usr/lib/x86_64-linux-gnu /usr/lib64; do
                if [ -e "$__d/libcuda.so.1" ]; then __pratico_drv="$__d"; break; fi
              done
              if [ -n "$__pratico_drv" ]; then
                __pratico_libs="''${XDG_CACHE_HOME:-''${HOME:-/tmp}/.cache}/pratico/nvidia-libs"
                mkdir -p "$__pratico_libs"
                find "$__pratico_libs" -maxdepth 1 -xtype l -delete 2>/dev/null || true
                for __f in "$__pratico_drv"/libcuda.so* "$__pratico_drv"/libnvidia-*.so*; do
                  [ -e "$__f" ] && ln -sfn "$__f" "$__pratico_libs/$(basename "$__f")"
                done
                export LD_LIBRARY_PATH="$__pratico_libs''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
                echo "cuda: NVIDIA driver from $__pratico_drv on LD_LIBRARY_PATH (links in $__pratico_libs)"
              else
                echo "cuda: no libcuda.so.1 in /run/opengl-driver/lib, /usr/lib/x86_64-linux-gnu or /usr/lib64 — CUDA code will compile here but not run" >&2
              fi
              unset __pratico_drv __pratico_libs __d __f
            '';
          } // ww3Env pkgs);
        });

      # Built on demand, never by CI: the regtest data is a 327 MB download, and ww3-metis
      # compiles ParMETIS (AGENTS.md, Safety).
      packages = forAll (pkgs: {
        ptscotch-static = ptscotchStatic pkgs;
        ww3-metis = ww3Metis pkgs;
        ww3-regtest-data = ww3RegtestData pkgs;
      });

      # `nix flake check` / CI: prove the toolchain actually links and runs a WW3-shaped
      # program — Fortran 2008, MPI (singleton, no launcher), NetCDF-4 write, then ncdump
      # reads it back. Runs inside the Nix sandbox, so it is exactly as reproducible as the
      # shell that ww-lab will build WW3 in.
      checks = forAll (pkgs: {
        lab-json = labCardFor pkgs;
        # The archives and headers WW3's cmake/FindSCOTCH.cmake looks for, by its own names.
        ptscotch = pkgs.runCommand "pratico-ptscotch-layout" { } ''
          for f in lib/libscotch.a lib/libptscotch.a lib/libscotcherr.a lib/libptscotcherr.a \
                   lib/libptscotchparmetisv3.a include/scotch.h include/ptscotch.h include/parmetis.h; do
            test -f "${ptscotchStatic pkgs}/$f" || { echo "ptscotch-static: no $f" >&2; exit 1; }
          done
          grep -q "^#define SCOTCH_VERSION 7" "${ptscotchStatic pkgs}/include/scotch.h"
          ls "${ptscotchStatic pkgs}/lib" > "$out"
        '';
        toolchain = pkgs.stdenv.mkDerivation {
          name = "ww3-toolchain-smoke";
          dontUnpack = true;
          nativeBuildInputs = with pkgs; [ gfortran openmpi netcdf netcdffortran ];
          dontConfigure = true;
          buildInputs = with pkgs; [ netcdffortran netcdf hdf5 zlib curl ]; # what `nf-config --flibs` links
          # OpenMPI singleton inside the sandbox: no network, no launcher, shared memory only.
          OMPI_MCA_btl = "self,vader";
          OMPI_MCA_plm = "isolated";
          OMPI_MCA_pml = "ob1"; # keep UCX out of a sandbox with no /sys
          OMPI_MCA_btl_vader_single_copy_mechanism = "none";
          buildPhase = ''
            export HOME="$TMPDIR"
            cat > smoke.f90 <<'F90'
            program smoke
              use mpi
              use netcdf
              implicit none
              integer :: ierr, rank, nproc, ncid, dimid, varid, i
              real(kind=8) :: hs(8)
              call MPI_Init(ierr)
              call MPI_Comm_rank(MPI_COMM_WORLD, rank, ierr)
              call MPI_Comm_size(MPI_COMM_WORLD, nproc, ierr)
              do i = 1, 8
                hs(i) = 0.5d0 * i + rank
              end do
              if (rank == 0) then
                call check(nf90_create("hs.nc", ior(NF90_NETCDF4, NF90_CLOBBER), ncid))
                call check(nf90_def_dim(ncid, "node", 8, dimid))
                call check(nf90_def_var(ncid, "hs", NF90_DOUBLE, dimid, varid))
                call check(nf90_put_att(ncid, varid, "units", "m"))
                call check(nf90_enddef(ncid))
                call check(nf90_put_var(ncid, varid, hs))
                call check(nf90_close(ncid))
                print '(a,i0,a)', "smoke: wrote hs.nc from ", nproc, " rank(s)"
              end if
              call MPI_Finalize(ierr)
            contains
              subroutine check(status)
                integer, intent(in) :: status
                if (status /= nf90_noerr) then
                  print *, trim(nf90_strerror(status)); stop 1
                end if
              end subroutine check
            end program smoke
            F90
            sed -i 's/^            //' smoke.f90
            mpifort -std=f2008 -Wall -o smoke smoke.f90 $(nf-config --fflags) $(nf-config --flibs)
            ./smoke
            ncdump hs.nc
          '';
          installPhase = ''
            mkdir -p "$out"
            cp hs.nc "$out/"
            ncdump hs.nc > "$out/hs.cdl"
            { echo "gfortran: $(gfortran --version | head -1)"; echo "openmpi: $(mpirun --version | head -1)"; echo "netcdf-c: $(nc-config --version)"; echo "netcdf-fortran: $(nf-config --version)"; } > "$out/versions.txt"
          '';
        };

        # The C++ half: configure the smoke project the way ww3-gpu configures its
        # `kokkos/` tree (find_package for both), run its GoogleTest suite through
        # ctest, then run the plain binary and keep what it printed.
        kokkos-smoke = pkgs.stdenv.mkDerivation {
          name = "pratico-kokkos-smoke";
          src = ./smoke/kokkos;
          nativeBuildInputs = [ pkgs.cmake pkgs.ninja ];
          buildInputs = [ (kokkosHost pkgs) pkgs.gtest ];
          cmakeFlags = [ "-GNinja" ];
          doCheck = true;
          # Not `| tee`: a pipeline would hide a non-zero exit from ./smoke.
          checkPhase = "ctest --output-on-failure && ./smoke > smoke.txt && cat smoke.txt";
          installPhase = ''mkdir -p "$out"; cp smoke.txt "$out/"'';
        };
      });
    };
}
