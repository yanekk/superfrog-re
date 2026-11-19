//! The command line, and the seatbelts it enforces (DESIGN §5.2).
//!
//! ```text
//! fd-app [--home DIR] [--disks DIR] [--script FILE] [--capture DIR [--capture-hashes]] [--max-frames N]
//!        [--timeout S] [--scale N] [--bugs-dir DIR] [--engine translated|hand] [--level 1]
//! ```

use std::ffi::OsString;
use std::path::PathBuf;

/// `--timeout` under `--script` when none is given (DESIGN §5.2).
pub const SCRIPT_TIMEOUT_S: u64 = 120;

pub const USAGE: &str = "usage: fd-app [--home DIR] [--disks DIR] [--script FILE] [--capture DIR [--capture-hashes]] \
[--max-frames N] [--timeout S] [--scale N] [--bugs-dir DIR] [--engine translated|hand] [--level 1]";

/// Which engine plays (translator DESIGN §2.10).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EngineChoice {
    /// The translated program from power-on, level 1's hand engine mounted through the seam.
    #[default]
    Translated,
    /// Level 1's hand engine alone, straight into W1-1, as before the translator plan: kept for
    /// comparison, and for the tests that run where no translation is generated.
    Hand,
}

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
    /// The capture without its frames' pictures: a whole level's PNGs would be gigabytes and take
    /// most of the run to encode (T28).
    pub capture_hashes: bool,
    /// Exit with status 0 after this many steps.
    pub max_frames: Option<u64>,
    /// Stop the app and its whole process group after this many seconds of wall clock.
    pub timeout_s: Option<u64>,
    /// The window's integer scale for this run, in place of the remembered setting.
    pub scale: Option<u32>,
    /// Where a bug recording is saved (DESIGN §2.8, §5.2). Taken now so tests always pass one;
    /// the recorder that writes there is T14's.
    pub bugs_dir: Option<PathBuf>,
    /// `--engine`.
    pub engine: EngineChoice,
    /// `--level 1`: start at level 1's first frame, the translated program run headless through
    /// the title with Start pressed. The hand engine always starts there.
    pub level_1: bool,
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
            it.next()
                .ok_or_else(|| format!("{name} needs {what}\n{USAGE}"))
        };
        match name.as_str() {
            "--home" => a.home = Some(value("a folder")?.into()),
            "--disks" => a.disks = Some(value("a folder")?.into()),
            "--script" => a.script = Some(value("a file")?.into()),
            "--capture" => a.capture = Some(value("a folder")?.into()),
            "--capture-hashes" => a.capture_hashes = true,
            "--bugs-dir" => a.bugs_dir = Some(value("a folder")?.into()),
            "--engine" => {
                a.engine = match value("translated or hand")?.to_string_lossy().as_ref() {
                    "translated" => EngineChoice::Translated,
                    "hand" => EngineChoice::Hand,
                    other => {
                        return Err(format!(
                            "--engine {other:?}: the engine is translated or hand\n{USAGE}"
                        ));
                    }
                }
            }
            "--level" => match value("a level")?.to_string_lossy().as_ref() {
                "1" => a.level_1 = true,
                other => {
                    return Err(format!(
                        "--level {other:?}: only level 1 can be started\n{USAGE}"
                    ));
                }
            },
            "--max-frames" => a.max_frames = Some(number(&name, value("a number")?)?),
            "--timeout" => a.timeout_s = Some(number(&name, value("a number of seconds")?)?),
            "--scale" => {
                let s = number(&name, value("a number")?)?;
                if !(1..=16).contains(&s) {
                    return Err(format!(
                        "--scale {s}: the scale is a whole number from 1 to 16"
                    ));
                }
                a.scale = Some(s as u32);
            }
            _ => return Err(format!("unknown argument {name:?}\n{USAGE}")),
        }
    }
    if a.capture_hashes && a.capture.is_none() {
        return Err(format!(
            "--capture-hashes needs --capture DIR to write in\n{USAGE}"
        ));
    }
    // A scripted run always ends: the watchdog is on unless it names its own limit.
    if a.script.is_some() && a.timeout_s.is_none() {
        a.timeout_s = Some(SCRIPT_TIMEOUT_S);
    }
    Ok(a)
}

fn number(name: &str, v: OsString) -> Result<u64, String> {
    v.to_string_lossy().parse().map_err(|_| {
        format!(
            "{name} {:?}: not a whole number\n{USAGE}",
            v.to_string_lossy()
        )
    })
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
            "--home",
            "/h",
            "--disks",
            "/d",
            "--script",
            "/s",
            "--capture",
            "/c",
            "--max-frames",
            "5",
            "--timeout",
            "9",
            "--scale",
            "3",
            "--bugs-dir",
            "/b",
        ])
        .unwrap();
        assert_eq!(a.home, Some("/h".into()));
        assert_eq!(a.disks, Some("/d".into()));
        assert_eq!(a.script, Some("/s".into()));
        assert_eq!(a.capture, Some("/c".into()));
        assert_eq!(a.bugs_dir, Some("/b".into()));
        assert_eq!(
            (a.max_frames, a.timeout_s, a.scale),
            (Some(5), Some(9), Some(3))
        );
    }

    #[test]
    fn a_script_gets_the_watchdog_unless_it_names_one() {
        assert_eq!(
            p(&["--script", "/s"]).unwrap().timeout_s,
            Some(SCRIPT_TIMEOUT_S)
        );
        assert_eq!(p(&[]).unwrap().timeout_s, None);
    }

    #[test]
    fn bad_arguments_are_refused_with_the_usage() {
        for bad in [
            &["--scale", "0"][..],
            &["--scale", "x"],
            &["--home"],
            &["--fly"],
            &["--max-frames", "-1"],
        ] {
            assert!(p(bad).is_err(), "{bad:?}");
        }
        assert!(p(&["--fly"]).unwrap_err().contains(USAGE));
        assert!(
            p(&["--capture-hashes"])
                .unwrap_err()
                .contains("--capture DIR")
        );
        assert!(
            p(&["--capture", "c", "--capture-hashes"])
                .unwrap()
                .capture_hashes
        );
        assert!(!p(&["--capture", "c"]).unwrap().capture_hashes);
    }

    #[test]
    fn the_translated_program_plays_unless_the_hand_engine_is_asked_for() {
        let a = p(&[]).unwrap();
        assert_eq!((a.engine, a.level_1), (EngineChoice::Translated, false));
        assert_eq!(p(&["--engine", "hand"]).unwrap().engine, EngineChoice::Hand);
        assert_eq!(
            p(&["--engine", "translated"]).unwrap().engine,
            EngineChoice::Translated
        );
        assert!(p(&["--level", "1"]).unwrap().level_1);
        assert!(p(&["--engine", "hand", "--level", "1"]).unwrap().level_1);
        for bad in [&["--engine", "oracle"][..], &["--engine"], &["--level", "2"], &["--level"]] {
            let e = p(bad).unwrap_err();
            assert!(e.contains(USAGE), "{bad:?}: {e}");
        }
    }
}
