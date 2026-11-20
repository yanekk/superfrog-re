//! The command line, and the seatbelts it enforces (DESIGN §5.2).
//!
//! ```text
//! fd-app [--home DIR] [--disks DIR] [--script FILE] [--capture DIR [--capture-hashes]] [--max-frames N]
//!        [--timeout S] [--scale N] [--bugs-dir DIR] [--engine translated|hand]
//!        [--level 1-6 [--cheat]] [--stop-after N] [--allow-fullscreen]
//! ```

use std::ffi::OsString;
use std::path::PathBuf;

use fd_core::recording::LevelId;

/// `--timeout` under `--script` when none is given (DESIGN §5.2).
pub const SCRIPT_TIMEOUT_S: u64 = 120;

/// The most frames a run may ask for together with `--allow-fullscreen`, so a fullscreen run
/// always ends by itself (as the first game's app).
pub const FULLSCREEN_MAX_FRAMES: u64 = 300;

pub const USAGE: &str = "usage: fd-app [--home DIR] [--disks DIR] [--script FILE] [--capture DIR [--capture-hashes]] \
[--max-frames N] [--timeout S] [--scale N] [--bugs-dir DIR] [--engine translated|hand] \
[--level 1-6 [--cheat]] [--stop-after N] [--allow-fullscreen]";

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
    /// `--level N`, 1 to 6: start at world N's first frame, the translated program run headless
    /// through the title with Start pressed and the world set as `load_world` is entered (title
    /// plan DESIGN §2.5). The hand engine always starts in W1-1, its one level.
    pub level: Option<LevelId>,
    /// `--cheat`: the cheat on for `--level N`'s start (`cheat_flag` set at `load_world`).
    pub cheat: bool,
    /// `--stop-after N`, a test's seatbelt-style hook (translator T39): the translated program
    /// stops at step N as if it had reached a stop, so the app's stop path runs without a defect.
    pub stop_after: Option<u64>,
    /// Lets a scripted run go fullscreen (View → Fullscreen, a saved `fullscreen = true`).
    pub allow_fullscreen: bool,
}

impl Args {
    /// Is this run for an agent or a test? Then it takes neither the keyboard from the person nor
    /// a real pad, and shows no native message.
    pub fn unattended(&self) -> bool {
        self.script.is_some()
    }

    /// May this run go fullscreen? Never under `--script` without `--allow-fullscreen`.
    pub fn fullscreen_allowed(&self) -> bool {
        !self.unattended() || self.allow_fullscreen
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
            "--allow-fullscreen" => a.allow_fullscreen = true,
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
            "--level" => {
                let v = value("a level")?.to_string_lossy().into_owned();
                let world = v.parse::<usize>().ok().filter(|n| (1..=6).contains(n));
                a.level = Some(world.map(|n| LevelId::ALL[n - 1]).ok_or_else(|| {
                    format!("--level {v:?}: the level is a world from 1 to 6\n{USAGE}")
                })?);
            }
            "--cheat" => a.cheat = true,
            "--stop-after" => a.stop_after = Some(number(&name, value("a number")?)?),
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
    if a.stop_after.is_some() && a.engine == EngineChoice::Hand {
        return Err(format!(
            "--stop-after: only the translated program stops; drop --engine hand\n{USAGE}"
        ));
    }
    if a.engine == EngineChoice::Hand
        && let Some(l) = a.level.filter(|&l| l != LevelId::W1_1)
    {
        return Err(format!(
            "--level {}: the hand engine has level 1 only; drop --engine hand\n{USAGE}",
            l.world_index() + 1
        ));
    }
    if a.cheat && a.engine == EngineChoice::Hand {
        return Err(format!(
            "--cheat: the hand engine has no title to set the cheat on; drop --engine hand\n{USAGE}"
        ));
    }
    if a.cheat && a.level.is_none() {
        return Err(format!(
            "--cheat needs --level N: the cheat is set as a world starts, and power-on starts \
             on the title, where its code is typed\n{USAGE}"
        ));
    }
    if a.allow_fullscreen && !a.max_frames.is_some_and(|n| n <= FULLSCREEN_MAX_FRAMES) {
        return Err(format!(
            "--allow-fullscreen needs --max-frames {FULLSCREEN_MAX_FRAMES} or fewer, so a \
             fullscreen run always ends by itself\n{USAGE}"
        ));
    }
    if a.stop_after == Some(0) {
        return Err(format!("--stop-after 0: the first step is 1\n{USAGE}"));
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
    fn a_script_stays_windowed_unless_it_allows_fullscreen_with_a_short_cap() {
        assert!(!p(&["--script", "s", "--max-frames", "5"]).unwrap().fullscreen_allowed());
        assert!(p(&[]).unwrap().fullscreen_allowed());
        let a = p(&["--script", "s", "--max-frames", "300", "--allow-fullscreen"]).unwrap();
        assert!(a.allow_fullscreen && a.fullscreen_allowed());
        for bad in [
            &["--script", "s", "--max-frames", "301", "--allow-fullscreen"][..],
            &["--allow-fullscreen"],
            &["--script", "s", "--allow-fullscreen"],
        ] {
            let e = p(bad).unwrap_err();
            assert!(e.contains("--max-frames 300 or fewer") && e.contains(USAGE), "{bad:?}: {e}");
        }
    }

    #[test]
    fn the_translated_program_plays_unless_the_hand_engine_is_asked_for() {
        let a = p(&[]).unwrap();
        assert_eq!((a.engine, a.level, a.cheat), (EngineChoice::Translated, None, false));
        assert_eq!(p(&["--engine", "hand"]).unwrap().engine, EngineChoice::Hand);
        assert_eq!(
            p(&["--engine", "translated"]).unwrap().engine,
            EngineChoice::Translated
        );
        assert_eq!(p(&["--level", "1"]).unwrap().level, Some(LevelId::W1_1));
        assert_eq!(p(&["--stop-after", "120"]).unwrap().stop_after, Some(120));
        assert_eq!(p(&[]).unwrap().stop_after, None);
        assert_eq!(
            p(&["--engine", "hand", "--level", "1"]).unwrap().level,
            Some(LevelId::W1_1)
        );
        for bad in [
            &["--engine", "oracle"][..],
            &["--engine"],
            &["--level"],
            &["--stop-after", "0"],
            &["--stop-after", "x"],
            &["--engine", "hand", "--stop-after", "5"],
        ] {
            let e = p(bad).unwrap_err();
            assert!(e.contains(USAGE), "{bad:?}: {e}");
        }
    }

    #[test]
    fn level_takes_worlds_1_to_6_and_cheat_needs_a_translated_world_start() {
        for (n, l) in LevelId::ALL.iter().enumerate() {
            let a = p(&["--level", &(n + 1).to_string()]).unwrap();
            assert_eq!((a.level, a.cheat), (Some(*l), false));
        }
        for bad in ["0", "7", "three", "-1", ""] {
            let e = p(&["--level", bad]).unwrap_err();
            assert!(e.contains("a world from 1 to 6") && e.contains(USAGE), "{bad}: {e}");
        }
        for n in 2..=6 {
            let e = p(&["--engine", "hand", "--level", &n.to_string()]).unwrap_err();
            assert!(e.contains(&format!("--level {n}: the hand engine has level 1 only")), "{e}");
        }
        let a = p(&["--level", "3", "--cheat"]).unwrap();
        assert_eq!((a.level, a.cheat), (Some(LevelId::W3_1), true));
        assert!(p(&["--cheat", "--level", "1"]).unwrap().cheat, "either order");
        let e = p(&["--cheat"]).unwrap_err();
        assert!(e.contains("--cheat needs --level N") && e.contains(USAGE), "{e}");
        let e = p(&["--engine", "hand", "--level", "1", "--cheat"]).unwrap_err();
        assert!(e.contains("the hand engine has no title"), "{e}");
    }
}
