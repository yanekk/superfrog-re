//! The command line, and the seatbelts it enforces (DESIGN §5.2).
//!
//! ```text
//! fd-app [--home DIR] [--disks DIR] [--script FILE] [--capture DIR] [--max-frames N] [--timeout S]
//!        [--scale N] [--bugs-dir DIR]
//! ```

use std::ffi::OsString;
use std::path::PathBuf;

/// `--timeout` under `--script` when none is given (DESIGN §5.2).
pub const SCRIPT_TIMEOUT_S: u64 = 120;

pub const USAGE: &str = "usage: fd-app [--home DIR] [--disks DIR] [--script FILE] [--capture DIR] \
[--max-frames N] [--timeout S] [--scale N] [--bugs-dir DIR]";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Args {
    /// The app-data folder, in place of `FOREST_DUMB_HOME` and the real one (DESIGN §2.7).
    pub home: Option<PathBuf>,
    /// The folder holding the two zips, in place of `fd-tools`' lookup (DESIGN §2.9).
    pub disks: Option<PathBuf>,
    /// Feed these events in place of the keyboard's (the script layer).
    pub script: Option<PathBuf>,
    /// Write every frame, its state hash and the inputs the game saw here.
    pub capture: Option<PathBuf>,
    /// Exit with status 0 after this many steps.
    pub max_frames: Option<u64>,
    /// Stop the app and its whole process group after this many seconds of wall clock.
    pub timeout_s: Option<u64>,
    /// The window's integer scale for this run, in place of the remembered setting.
    pub scale: Option<u32>,
    /// Where a bug recording is saved (DESIGN §2.8, §5.2). Taken now so tests always pass one;
    /// the recorder that writes there is T14's.
    pub bugs_dir: Option<PathBuf>,
}

impl Args {
    /// Is this run for an agent or a test? Then it takes neither the keyboard from the person nor
    /// a real pad, and shows no native message.
    pub fn unattended(&self) -> bool {
        self.script.is_some()
    }
}

/// Parse the arguments after the program name, and apply the seatbelts: `Err` says why the run is
/// refused, for the terminal.
pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Args, String> {
    let mut a = Args::default();
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        let name = arg.to_string_lossy().into_owned();
        let mut value = |what: &str| -> Result<OsString, String> {
            it.next().ok_or_else(|| format!("{name} needs {what}\n{USAGE}"))
        };
        match name.as_str() {
            "--home" => a.home = Some(value("a folder")?.into()),
            "--disks" => a.disks = Some(value("a folder")?.into()),
            "--script" => a.script = Some(value("a file")?.into()),
            "--capture" => a.capture = Some(value("a folder")?.into()),
            "--bugs-dir" => a.bugs_dir = Some(value("a folder")?.into()),
            "--max-frames" => a.max_frames = Some(number(&name, value("a number")?)?),
            "--timeout" => a.timeout_s = Some(number(&name, value("a number of seconds")?)?),
            "--scale" => {
                let s = number(&name, value("a number")?)?;
                if !(1..=16).contains(&s) {
                    return Err(format!("--scale {s}: the scale is a whole number from 1 to 16"));
                }
                a.scale = Some(s as u32);
            }
            _ => return Err(format!("unknown argument {name:?}\n{USAGE}")),
        }
    }
    // A scripted run always ends: the watchdog is on unless it names its own limit.
    if a.script.is_some() && a.timeout_s.is_none() {
        a.timeout_s = Some(SCRIPT_TIMEOUT_S);
    }
    Ok(a)
}

fn number(name: &str, v: OsString) -> Result<u64, String> {
    v.to_string_lossy()
        .parse()
        .map_err(|_| format!("{name} {:?}: not a whole number\n{USAGE}", v.to_string_lossy()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Args, String> {
        parse(args.iter().map(OsString::from))
    }

    #[test]
    fn every_flag_is_read() {
        let a = p(&[
            "--home", "/h", "--disks", "/d", "--script", "/s", "--capture", "/c", "--max-frames", "5",
            "--timeout", "9", "--scale", "3", "--bugs-dir", "/b",
        ])
        .unwrap();
        assert_eq!(a.home, Some("/h".into()));
        assert_eq!(a.disks, Some("/d".into()));
        assert_eq!(a.script, Some("/s".into()));
        assert_eq!(a.capture, Some("/c".into()));
        assert_eq!(a.bugs_dir, Some("/b".into()));
        assert_eq!((a.max_frames, a.timeout_s, a.scale), (Some(5), Some(9), Some(3)));
    }

    #[test]
    fn a_script_gets_the_watchdog_unless_it_names_one() {
        assert_eq!(p(&["--script", "/s"]).unwrap().timeout_s, Some(SCRIPT_TIMEOUT_S));
        assert_eq!(p(&[]).unwrap().timeout_s, None);
    }

    #[test]
    fn bad_arguments_are_refused_with_the_usage() {
        for bad in [&["--scale", "0"][..], &["--scale", "x"], &["--home"], &["--fly"], &["--max-frames", "-1"]] {
            assert!(p(bad).is_err(), "{bad:?}");
        }
        assert!(p(&["--fly"]).unwrap_err().contains(USAGE));
    }
}
