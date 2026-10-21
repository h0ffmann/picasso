//! Create (or update) a Python venv with CUDA torch from PyTorch's own wheel index, not nixpkgs
//! (nixpkgs' torchWithCuda has no binary cache for most pins and compiles torch, magma and triton).
//!   REQUIREMENTS=finetune/requirements.txt setup-ml-venv          # create/update at ~/.ml-venv
//!   REQUIREMENTS=... VENV_ROOT=/path setup-ml-venv
//!   setup-ml-venv --path                                          # print the wrapper's path
//! Call $VENV_ROOT/bin/python-cuda afterwards, never bin/python: the wrapper puts libstdc++ and the
//! NVIDIA driver libraries on the search path.
use std::env;
use std::fs;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::Path;
use std::process::{exit, Command};

const WRAPPER: &str = "python-cuda";
// Baked in at build time by the lab's flake; from `cargo test` it is unset and the lookup below runs.
const LIBSTDCXX_BAKED: Option<&str> = option_env!("LIBSTDCXX_DIR");

const USAGE: &str = "\
setup-ml-venv — a Python venv with CUDA torch from PyTorch's wheel index
  REQUIREMENTS=<file> setup-ml-venv                 create/update at $VENV_ROOT (default ~/.ml-venv)
  REQUIREMENTS=<file> VENV_ROOT=/path setup-ml-venv
  setup-ml-venv --path                              print the wrapper's path
Call $VENV_ROOT/bin/python-cuda afterwards, never bin/python.
";

const CHECK: &str = r#"import torch; print(f"torch {torch.__version__}  cuda_available={torch.cuda.is_available()}"); print("device:", torch.cuda.get_device_name(0)) if torch.cuda.is_available() else exit("torch cannot see a CUDA device — check nvidia-smi on this host")"#;

fn var(k: &str, default: &str) -> String {
    env::var(k).ok().filter(|v| !v.is_empty()).unwrap_or_else(|| default.to_string())
}

/// Everything except torch, which is installed first from the CUDA index; one package per argument.
fn pkgs_from_requirements(text: &str) -> Vec<String> {
    let torch = |l: &str| l.strip_prefix("torch").is_some_and(|r| !r.starts_with(|c: char| c.is_alphanumeric() || c == '_'));
    text.lines()
        .filter(|l| !(l.trim_start().is_empty() || l.trim_start().starts_with('#') || torch(l)))
        .flat_map(|l| l.split(';').next().unwrap_or("").split_whitespace().map(str::to_string).collect::<Vec<_>>())
        .collect()
}

/// Where the NVIDIA *driver* libraries live. libcuda.so.1 is installed by the driver, never shipped
/// by pip, and a nix linker does not search /usr/lib, so torch imports fine and reports
/// cuda_available=False, which looks like a driver problem and is not one.
fn libcuda_dir() -> Option<String> {
    ["/run/opengl-driver/lib", "/usr/lib/x86_64-linux-gnu", "/usr/lib64"]
        .iter()
        .find(|d| Path::new(d).join("libcuda.so.1").exists())
        .map(|d| d.to_string())
}

/// On a normal distro the driver libraries share a directory with the distro's glibc. Putting that
/// whole directory on LD_LIBRARY_PATH makes the nix Python resolve libc.so.6 there instead of the
/// nix one it was linked against, and it dies with `GLIBC_ABI_GNU2_TLS not found`. So link only the
/// NVIDIA libraries into a directory of our own and put that on the path instead.
fn link_driver_libs(src: &Path, dst: &Path) -> bool {
    let _ = fs::create_dir_all(dst);
    let names = fs::read_dir(src).into_iter().flatten().flatten().map(|e| e.file_name().to_string_lossy().to_string());
    for n in names.filter(|n| n.starts_with("libcuda.so") || n.strip_prefix("libnvidia-").is_some_and(|r| r.contains(".so"))) {
        let (from, to) = (src.join(&n), dst.join(&n));
        if from.exists() {
            let _ = fs::remove_file(&to);
            let _ = symlink(&from, &to);
        }
    }
    dst.join("libcuda.so.1").exists()
}

/// PyTorch ships manylinux wheels that dlopen the system libstdc++. Under `nix develop` the nix
/// Python's linker cannot see one, so `import torch` dies with
/// "libstdc++.so.6: cannot open shared object file". Prefer nix's gcc lib, fall back to the distro's.
fn libstdcxx_dir() -> Option<String> {
    let has = |d: &str| Path::new(d).join("libstdc++.so.6").exists();
    let baked = env::var("LIBSTDCXX_DIR").ok().or(LIBSTDCXX_BAKED.map(str::to_string)).unwrap_or_default();
    if has(&baked) {
        return Some(baked);
    }
    let nix = Command::new("nix").args(["build", "--no-link", "--print-out-paths", "nixpkgs#stdenv.cc.cc.lib"]).stderr(std::process::Stdio::null()).output();
    if let Ok(o) = nix {
        let d = String::from_utf8_lossy(&o.stdout).lines().last().map(|l| format!("{l}/lib"));
        if let Some(d) = d.filter(|d| has(d)) {
            return Some(d);
        }
    }
    ["/usr/lib/x86_64-linux-gnu", "/usr/lib64", "/lib/x86_64-linux-gnu"].iter().find(|d| has(d)).map(|d| d.to_string())
}

/// The wrapper every caller uses instead of $VENV_ROOT/bin/python.
fn wrapper_body(venv: &str, libs: &str, drv: &str) -> String {
    format!(
        "#!/usr/bin/env bash\n\
         # Written by setup-ml-venv (nix-config/labs/cuda). Runs the venv Python with libstdc++ and the NVIDIA\n\
         # driver libraries on the search path: the manylinux wheels need the first, and without the\n\
         # second torch imports but reports cuda_available=False. A nix Python finds neither. The\n\
         # driver dir holds links to the NVIDIA libraries only — never a whole distro lib directory,\n\
         # whose glibc would shadow nix's own.\n\
         export LD_LIBRARY_PATH=\"{libs}:{drv}${{LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}}\"\n\
         exec \"{venv}/bin/python\" \"$@\"\n"
    )
}

/// Runs a child with inherited stdio; its failure ends this program with its exit code.
fn run(cmd: &mut Command) {
    match cmd.status() {
        Ok(s) if s.success() => {}
        Ok(s) => exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("setup-ml-venv: {:?}: {e}", cmd.get_program());
            exit(127);
        }
    }
}

fn die(msg: &str) -> ! {
    eprintln!("{msg}");
    exit(1)
}

fn main() {
    let home = var("HOME", "");
    let venv = var("VENV_ROOT", &format!("{home}/.ml-venv"));
    let index = var("CUDA_INDEX", "https://download.pytorch.org/whl/cu129");
    let py = var("PYTHON", "python3");
    let req = var("REQUIREMENTS", "");
    match env::args().nth(1).as_deref() {
        Some("--help" | "-h") => return print!("{USAGE}"),
        Some("--path") => return println!("{venv}/bin/{WRAPPER}"),
        _ => {}
    }
    let Some(requirements) = Some(req).filter(|r| Path::new(r).is_file()).and_then(|r| fs::read_to_string(r).ok()) else {
        eprintln!("setup-ml-venv: REQUIREMENTS=<file> is required\n{USAGE}");
        exit(2);
    };

    let vpy = format!("{venv}/bin/python");
    if !fs::metadata(&vpy).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0) {
        let v = Command::new(&py).arg("--version").output().map_or(String::new(), |o| String::from_utf8_lossy(&o.stdout).trim().to_string());
        println!("creating venv at {venv} ({v})");
        run(Command::new(&py).args(["-m", "venv", &venv]));
    }

    run(Command::new(&vpy).args(["-m", "pip", "install", "--quiet", "--upgrade", "pip"]));
    println!("installing torch from {index}");
    run(Command::new(&vpy).args(["-m", "pip", "install", "--index-url", &index, "torch"]));
    println!("installing the rest from PyPI");
    run(Command::new(&vpy).args(["-m", "pip", "install"]).args(pkgs_from_requirements(&requirements)));

    let libs = libstdcxx_dir().unwrap_or_else(|| die("setup-ml-venv: no libstdc++.so.6 found — torch's wheels cannot load without one"));
    let drvsrc = libcuda_dir().unwrap_or_else(|| die("setup-ml-venv: no libcuda.so.1 found — is the NVIDIA driver installed? (nvidia-smi)"));
    let drv = format!("{venv}/driver-lib");
    if !link_driver_libs(Path::new(&drvsrc), Path::new(&drv)) {
        exit(1);
    }
    let wrapper = format!("{venv}/bin/{WRAPPER}");
    if let Err(e) = fs::write(&wrapper, wrapper_body(&venv, &libs, &drv)).and_then(|_| fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755))) {
        die(&format!("setup-ml-venv: {wrapper}: {e}"));
    }
    println!("wrapper: {wrapper} (libstdc++ {libs}, driver {drv} -> {drvsrc})");

    println!();
    run(Command::new(&wrapper).args(["-c", CHECK]));
    println!("venv ready: {venv} — call bin/{WRAPPER}, not bin/python");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = env::temp_dir().join(format!("setup-ml-venv-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn requirements_skip_torch_comments_and_markers() {
        let p = pkgs_from_requirements("torch==2.8.0\ntransformers>=4.44\nnumpy; python_version >= \"3.9\"\n# a comment\n\ntorchvision\n");
        assert!(!p.iter().any(|x| x == "torch" || x.starts_with("torch=")), "torch is installed from the CUDA index first");
        assert_eq!(p, ["transformers>=4.44", "numpy", "torchvision"], "markers stripped, pip gets plain names");
    }

    #[test]
    fn libstdcxx_is_located_for_the_manylinux_wheels() {
        let d = libstdcxx_dir().expect("a libstdc++");
        assert!(Path::new(&d).join("libstdc++.so.6").exists());
    }

    #[test]
    fn wrapper_sets_the_library_path_and_execs_the_venv_python() {
        let w = wrapper_body("/venv", "/libs", "/drv");
        let n = |p: &str| w.lines().filter(|l| l.contains(p)).count();
        assert_eq!(n("LD_LIBRARY_PATH"), 1);
        assert_eq!(w.lines().filter(|l| l.starts_with("exec ")).count(), 1, "exec, not a shell per call");
        assert_eq!(n("/venv/bin/python"), 1);
        assert_eq!(n("/libs:/drv"), 1, "both libstdc++ and the driver dir are on the path");
    }

    #[test]
    fn driver_dir_finds_libcuda_when_there_is_a_gpu() {
        if let Some(d) = libcuda_dir() {
            assert!(Path::new(&d).join("libcuda.so.1").exists());
        } // else: no GPU/driver on this host
    }

    #[test]
    fn only_nvidia_libraries_are_linked_never_the_distro_glibc() {
        let src = tmp("src");
        let dst = src.join("link");
        for f in ["libcuda.so.1", "libnvidia-ml.so.1", "libc.so.6", "libstdc++.so.6"] {
            fs::write(src.join(f), "").unwrap();
        }
        assert!(link_driver_libs(&src, &dst), "succeeds when libcuda.so.1 is present");
        assert_eq!(fs::read_dir(&dst).unwrap().count(), 2);
        assert!(!dst.join("libc.so.6").exists(), "linking glibc breaks the nix Python");
        fs::remove_file(src.join("libcuda.so.1")).unwrap();
        assert!(!link_driver_libs(&src, &src.join("link2")), "a driver dir without libcuda.so.1 is rejected");
    }
}
