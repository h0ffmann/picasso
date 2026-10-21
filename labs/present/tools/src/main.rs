//! cam: webcam controls you can save and get back, over v4l2-ctl.
//!
//! v4l2-ctl is resolved from PATH (the lab's shell puts v4l-utils there); the flake deliberately
//! does not wrap it in, and the tests fake it behind the `Ctl` trait.
use std::fmt::Write as _;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};

const USAGE: &str = "\
cam — webcam controls you can save and get back, over v4l2-ctl.

Webcams forget brightness, zoom and manual focus on unplug and on suspend, and every camera
names its controls differently, so the thing you want two minutes before a call is \"the
settings from last time\", not the v4l2-ctl man page.

  cam list                                   devices and their /dev/video* nodes
  cam show [dev]                             every writable control: value, default, range
  cam set  [dev] name=value [name=value…]    one control at a time, so a bad one names itself
  cam save <profile> [dev]                   current values → ~/.config/present/cam/<profile>.conf
  cam load <profile> [dev]                   apply a saved profile (auto-* controls first)
  cam reset [dev]                            every writable control back to its default

dev defaults to $CAM_DEVICE, then /dev/video0. Profiles are plain `name=value` lines, one per
control — edit them by hand. v4l2-ctl is resolved from PATH (the lab's shell puts v4l-utils
there); the flake deliberately does not wrap it in.
";

/// The v4l2-ctl calls cam makes.
trait Ctl {
    fn list_devices(&self) -> Result<String, i32>;
    fn list_ctrls(&self, dev: &str) -> Result<String, i32>;
    fn set_ctrl(&self, dev: &str, kv: &str) -> bool;
}

struct V4l2;

impl V4l2 {
    fn stdout(args: &[&str]) -> Result<String, i32> {
        match Command::new("v4l2-ctl").args(args).stderr(Stdio::inherit()).output() {
            Ok(o) if o.status.success() => Ok(String::from_utf8_lossy(&o.stdout).into_owned()),
            Ok(o) => Err(o.status.code().unwrap_or(1)),
            Err(e) => {
                eprintln!("cam: v4l2-ctl: {e}");
                Err(127)
            }
        }
    }
}

impl Ctl for V4l2 {
    fn list_devices(&self) -> Result<String, i32> {
        Self::stdout(&["--list-devices"])
    }
    fn list_ctrls(&self, dev: &str) -> Result<String, i32> {
        Self::stdout(&["-d", dev, "--list-ctrls"])
    }
    fn set_ctrl(&self, dev: &str, kv: &str) -> bool {
        Command::new("v4l2-ctl").args(["-d", dev, &format!("--set-ctrl={kv}")]).status().is_ok_and(|s| s.success())
    }
}

struct Ctrl {
    name: String,
    value: String,
    default: String,
    flags: String,
}

/// `v4l2-ctl --list-ctrls` → one entry per control that has a value. Headers ("User Controls")
/// have no 0x id and are dropped; read-only and inactive controls are kept but flagged so
/// save/reset can skip them.
fn parse_ctrls(text: &str) -> Vec<Ctrl> {
    text.lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            if !f.get(1)?.starts_with("0x") {
                return None;
            }
            let field = |p: &str| f.iter().skip(3).find_map(|w| w.strip_prefix(p)).unwrap_or("-");
            // flags is last and may contain spaces ("flags=inactive, has-min-max" on v4l-utils 1.32)
            let flags = l.find("flags=").map_or("-".to_string(), |i| l[i + 6..].replace(' ', ""));
            let value = field("value=");
            (value != "-").then(|| Ctrl { name: f[0].into(), value: value.into(), default: field("default=").into(), flags })
        })
        .collect()
}

fn writable(cs: Vec<Ctrl>) -> Vec<Ctrl> {
    cs.into_iter().filter(|c| !["read-only", "inactive", "volatile"].iter().any(|f| c.flags.contains(f))).collect()
}

/// auto-* controls first: setting focus_absolute while focus_automatic_continuous=1 fails on most
/// cameras, and the same for exposure and white balance. The sort is stable.
fn auto_first(mut cs: Vec<Ctrl>) -> Vec<Ctrl> {
    cs.sort_by_key(|c| !c.name.contains("auto"));
    cs
}

struct Cam<'a> {
    ctl: &'a dyn Ctl,
    profiles: PathBuf,
    default_dev: String,
}

impl Cam<'_> {
    /// Splits an optional leading /dev/… argument off; returns the device and the rest.
    fn take_dev<'b>(&self, args: &'b [String]) -> (String, &'b [String]) {
        match args.first() {
            Some(a) if a.starts_with("/dev/") => (a.clone(), &args[1..]),
            _ => (self.default_dev.clone(), args),
        }
    }

    fn controls(&self, dev: &str) -> Result<Vec<Ctrl>, i32> {
        self.ctl.list_ctrls(dev).map(|t| parse_ctrls(&t))
    }

    fn set_ctrl(&self, dev: &str, kv: &str, out: &mut String, err: &mut String) -> bool {
        if !self.ctl.set_ctrl(dev, kv) {
            let _ = writeln!(err, "cam: {dev}: could not set {kv}");
            return false;
        }
        let _ = writeln!(out, "  {kv}");
        true
    }

    fn set_all(&self, dev: &str, kvs: impl Iterator<Item = String>, out: &mut String, err: &mut String) -> i32 {
        for kv in kvs {
            if !self.set_ctrl(dev, &kv, out, err) {
                return 1;
            }
        }
        0
    }

    /// Runs one command; stdout and stderr text are appended to `out` and `err`, the result is the exit code.
    fn run(&self, args: &[String], out: &mut String, err: &mut String) -> i32 {
        let cmd = args.first().map_or("", String::as_str);
        let args = args.get(1..).unwrap_or(&[]);
        match cmd {
            "list" => self.ctl.list_devices().map(|t| out.push_str(&t)).err().unwrap_or(0),
            "show" => {
                let (dev, _) = self.take_dev(args);
                let cs = match self.controls(&dev) {
                    Ok(cs) => cs,
                    Err(c) => return c,
                };
                let _ = writeln!(out, "{dev}");
                for c in cs {
                    let flags = if c.flags == "-" { String::new() } else { format!(", {}", c.flags) };
                    let _ = writeln!(out, "  {:<32} {:>8}  (default {}{flags})", c.name, c.value, c.default);
                }
                0
            }
            "set" => {
                let (dev, rest) = self.take_dev(args);
                if rest.is_empty() {
                    let _ = writeln!(err, "cam set: nothing to set");
                    return 2;
                }
                let _ = writeln!(out, "{dev}");
                self.set_all(&dev, rest.iter().cloned(), out, err)
            }
            "save" => {
                let Some(name) = args.first() else {
                    let _ = writeln!(err, "cam save: profile name required");
                    return 2;
                };
                let (dev, _) = self.take_dev(&args[1..]);
                let cs = match self.controls(&dev) {
                    Ok(cs) => cs,
                    Err(c) => return c,
                };
                let when = Command::new("date").args(["-u", "+%Y-%m-%dT%H:%MZ"]).output().map_or(String::new(), |o| String::from_utf8_lossy(&o.stdout).trim().to_string());
                let mut text = format!("# cam profile '{name}' from {dev}, {when}\n");
                for c in auto_first(writable(cs)) {
                    let _ = writeln!(text, "{}={}", c.name, c.value);
                }
                let file = self.profiles.join(format!("{name}.conf"));
                if let Err(e) = std::fs::create_dir_all(&self.profiles).and_then(|()| std::fs::write(&file, text)) {
                    let _ = writeln!(err, "cam save: {}: {e}", file.display());
                    return 1;
                }
                let _ = writeln!(out, "{}", file.display());
                0
            }
            "load" => {
                let Some(name) = args.first() else {
                    let _ = writeln!(err, "cam load: profile name required");
                    return 2;
                };
                let (dev, _) = self.take_dev(&args[1..]);
                let file = self.profiles.join(format!("{name}.conf"));
                let Ok(text) = std::fs::read_to_string(&file) else {
                    let _ = writeln!(err, "cam load: no profile {}", file.display());
                    return 1;
                };
                let _ = writeln!(out, "{dev} ← {}", file.display());
                let kvs = text.lines().filter(|l| !l.starts_with('#') && l.contains('=')).map(String::from);
                self.set_all(&dev, kvs, out, err)
            }
            "reset" => {
                let (dev, _) = self.take_dev(args);
                let cs = match self.controls(&dev) {
                    Ok(cs) => cs,
                    Err(c) => return c,
                };
                let _ = writeln!(out, "{dev} → defaults");
                let kvs = auto_first(writable(cs)).into_iter().filter(|c| c.default != "-").map(|c| format!("{}={}", c.name, c.default));
                self.set_all(&dev, kvs, out, err)
            }
            "-h" | "--help" | "" => {
                out.push_str(USAGE);
                0
            }
            _ => {
                let _ = writeln!(err, "cam: unknown command '{cmd}'\n{USAGE}");
                2
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    let config = var("XDG_CONFIG_HOME").unwrap_or_else(|| format!("{}/.config", var("HOME").unwrap_or_default()));
    let cam = Cam { ctl: &V4l2, profiles: Path::new(&config).join("present/cam"), default_dev: var("CAM_DEVICE").unwrap_or("/dev/video0".into()) };
    let (mut out, mut err) = (String::new(), String::new());
    let code = cam.run(&args, &mut out, &mut err);
    // Printed after the run, so the order between the two streams is lost; each stays whole.
    let _ = std::io::stdout().write_all(out.as_bytes());
    let _ = std::io::stderr().write_all(err.as_bytes());
    exit(code);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    const CTRLS: &str = "
User Controls

                     brightness 0x00980900 (int)    : min=0 max=255 step=1 default=128 value=140
                       contrast 0x00980901 (int)    : min=0 max=255 step=1 default=128 value=128
        white_balance_automatic 0x0098090c (bool)   : default=1 value=1
           power_line_frequency 0x00980918 (menu)   : min=0 max=2 default=2 value=2 (60 Hz)
      white_balance_temperature 0x0098091a (int)    : min=2000 max=6500 step=1 default=4000 value=4000 flags=inactive, has-min-max

Camera Controls

                  auto_exposure 0x009a0901 (menu)   : min=0 max=3 default=3 value=3 (Aperture Priority Mode)
                 focus_absolute 0x009a090a (int)    : min=0 max=250 step=5 default=0 value=30
     focus_automatic_continuous 0x009a090c (bool)   : default=1 value=0
                  zoom_absolute 0x009a090d (int)    : min=100 max=500 step=1 default=100 value=120
                     led1_mode 0x0a046d05 (menu)   : min=0 max=3 default=0 value=0 flags=read-only
";

    /// A fake v4l2-ctl: a real camera's output, every set recorded, one control refused so the error path runs.
    struct Fake(RefCell<Vec<String>>);

    impl Ctl for Fake {
        fn list_devices(&self) -> Result<String, i32> {
            Ok("HD Pro Webcam C920 (usb-0000:00:14.0-1):\n\t/dev/video0\n\t/dev/video1\n".into())
        }
        fn list_ctrls(&self, _: &str) -> Result<String, i32> {
            Ok(CTRLS.into())
        }
        fn set_ctrl(&self, _: &str, kv: &str) -> bool {
            if kv == "zoom_absolute=999" {
                return false;
            }
            self.0.borrow_mut().push(kv.into());
            true
        }
    }

    fn run(cam: &Cam, args: &[&str]) -> (i32, String, String) {
        let (mut out, mut err) = (String::new(), String::new());
        let code = cam.run(&args.iter().map(|a| a.to_string()).collect::<Vec<_>>(), &mut out, &mut err);
        (code, out, err)
    }

    fn setup(name: &str) -> (Fake, PathBuf) {
        let d = std::env::temp_dir().join(format!("cam-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        (Fake(RefCell::new(vec![])), d)
    }

    #[test]
    fn list_passes_v4l2_ctl_through() {
        let (f, d) = setup("list");
        let cam = Cam { ctl: &f, profiles: d, default_dev: "/dev/video0".into() };
        assert_eq!(run(&cam, &["list"]).1.lines().next(), Some("HD Pro Webcam C920 (usb-0000:00:14.0-1):"));
    }

    #[test]
    fn show_prints_every_valued_control_with_flags() {
        let (f, d) = setup("show");
        let cam = Cam { ctl: &f, profiles: d, default_dev: "/dev/video0".into() };
        let out = run(&cam, &["show"]).1;
        assert_eq!(out.lines().filter(|l| l.starts_with("  ")).count(), 10, "headers dropped");
        let col = |name: &str, n: usize| out.lines().find(|l| l.split_whitespace().next() == Some(name)).and_then(|l| l.split_whitespace().nth(n)).map(String::from);
        assert_eq!(col("focus_absolute", 1).as_deref(), Some("30"), "current value, not the default");
        assert!(out.lines().any(|l| l.contains("led1_mode") && l.contains("read-only")));
        assert!(out.lines().any(|l| l.contains("white_balance_temperature") && l.ends_with("inactive,has-min-max)")), "multi-word flags kept whole");
        assert_eq!(run(&cam, &["show", "/dev/video1"]).1.lines().next(), Some("/dev/video1"), "a leading /dev/… selects the device");
    }

    #[test]
    fn set_applies_each_control_in_order_and_names_a_rejected_one() {
        let (f, d) = setup("set");
        let cam = Cam { ctl: &f, profiles: d, default_dev: "/dev/video0".into() };
        let (code, out, _) = run(&cam, &["set", "brightness=150", "focus_absolute=40"]);
        assert_eq!((code, out.lines().last()), (0, Some("  focus_absolute=40")));
        assert_eq!(*f.0.borrow(), ["brightness=150", "focus_absolute=40"]);
        let (code, _, err) = run(&cam, &["set", "zoom_absolute=999"]);
        assert_eq!((code, err.as_str()), (1, "cam: /dev/video0: could not set zoom_absolute=999\n"));
        assert_eq!(run(&cam, &["set"]).0, 2);
    }

    #[test]
    fn save_load_reset_round_trip() {
        let (f, d) = setup("profile");
        let cam = Cam { ctl: &f, profiles: d.clone(), default_dev: "/dev/video0".into() };
        let file = d.join("meeting.conf");
        assert_eq!(run(&cam, &["save", "meeting"]).1, format!("{}\n", file.display()));
        let text = std::fs::read_to_string(&file).unwrap();
        let lines: Vec<&str> = text.lines().filter(|l| !l.starts_with('#')).collect();
        assert_eq!(lines[..3], ["white_balance_automatic=1", "auto_exposure=3", "focus_automatic_continuous=0"], "auto controls first");
        assert!(!text.contains("white_balance_temperature") && !text.contains("led1_mode"), "inactive and read-only are not saved");
        assert_eq!(lines.len(), 8);

        f.0.borrow_mut().clear();
        let (_, out, _) = run(&cam, &["load", "meeting"]);
        assert_eq!(out.lines().last(), Some("  zoom_absolute=120"));
        assert_eq!(f.0.borrow()[0], "white_balance_automatic=1");
        let (code, _, err) = run(&cam, &["load", "nope"]);
        assert_eq!((code, err), (1, format!("cam load: no profile {}\n", d.join("nope.conf").display())));

        f.0.borrow_mut().clear();
        run(&cam, &["reset"]);
        let log = f.0.borrow();
        assert_eq!(log.len(), 8);
        assert!(log.iter().any(|l| l == "focus_absolute=0"), "to the default, not the current value");
        assert!(!log.iter().any(|l| l.contains("led1_mode")), "read-only skipped");
    }
}
