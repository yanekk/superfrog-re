//! The command line, and the seatbelts it enforces (DESIGN §5.2).
//!
//! ```text
//! superfrog [--home DIR] [--cache-from-disks DIR] [--script FILE [--pacing FILE]]
//!           [--capture DIR [--capture-hashes]] [--max-frames N] [--timeout S] [--scale N]
//!           [--allow-fullscreen] [--audio-device NAME] [--bugs-dir DIR] [--replay ZIP]
//! ```

use std::ffi::OsString;
use std::path::PathBuf;

/// `--timeout` under `--script` when none is given (DESIGN §5.2).
pub const SCRIPT_TIMEOUT_S: u64 = 120;

/// The most frames a run may ask for together with `--allow-fullscreen` (DESIGN §5.2).
pub const FULLSCREEN_MAX_FRAMES: u64 = 300;

pub const USAGE: &str = "usage: superfrog [--home DIR] [--cache-from-disks DIR] [--script FILE \
[--pacing FILE]] [--capture DIR [--capture-hashes]] [--max-frames N] [--timeout S] [--scale N] [--allow-fullscreen] \
[--audio-device NAME] [--bugs-dir DIR] [--replay ZIP]";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    /// The app-data folder, in place of the real one (DESIGN §3.5).
    pub home: Option<PathBuf>,
    /// Import the four disk images in this folder when the cache is missing or unusable.
    pub cache_from_disks: Option<PathBuf>,
    /// Feed these events in place of the keyboard's (DESIGN §5, the script layer).
    pub script: Option<PathBuf>,
    /// Test-only, with `--script`: step the game as `sf-conform`'s engine run alone is fed, from
    /// this pacing feed (`sf_core::feed`, T211). Normal play never reads one (DESIGN §2.5).
    pub pacing: Option<PathBuf>,
    /// Write every frame, the mixed audio and the inputs the game saw here.
    pub capture: Option<PathBuf>,
    /// With `--capture`: each frame's hashes (`hashes.txt`) in place of its PNG.
    pub capture_hashes: bool,
    /// Exit with status 0 after this many frames. 0 imports the cache and exits before any window.
    pub max_frames: Option<u64>,
    /// Stop the app and its whole process group after this many seconds of wall clock.
    pub timeout_s: Option<u64>,
    /// The window shows the game at this integer scale for this run, in place of the remembered
    /// setting (which it does not change).
    pub scale: Option<u32>,
    /// Lets a scripted run go fullscreen.
    pub allow_fullscreen: bool,
    /// Play through the output device of this name; one no device has leaves the audio output
    /// unavailable, the test hook for a Mac with none (DESIGN §2.11). A scripted run without it
    /// plays nothing.
    pub audio_device: Option<String>,
    /// Where a bug recording is saved, in place of the repo's `bugs/` (bug-recorder DESIGN §2.4,
    /// §5.2). A scripted run without it saves into `<home>/bugs`, never the repo.
    pub bugs_dir: Option<PathBuf>,
    /// Play this bug zip in the window, watch-only, holding on its last frame (bug-recorder
    /// DESIGN §2.7). For agents and tests.
    pub replay: Option<PathBuf>,
}

impl Default for Args {
    fn default() -> Args {
        Args {
            home: None,
            cache_from_disks: None,
            script: None,
            pacing: None,
            capture: None,
            capture_hashes: false,
            max_frames: None,
            timeout_s: None,
            scale: None,
            allow_fullscreen: false,
            audio_device: None,
            bugs_dir: None,
            replay: None,
        }
    }
}

impl Args {
    /// May this run go fullscreen? Never under `--script` or `--replay` without
    /// `--allow-fullscreen`.
    pub fn fullscreen_allowed(&self) -> bool {
        !self.unattended() || self.allow_fullscreen
    }

    /// Is this run for an agent or a test (`--script`, `--replay`)? Then it takes neither the
    /// keyboard from the person nor the speakers unless it names a device, and stays windowed.
    pub fn unattended(&self) -> bool {
        self.script.is_some() || self.replay.is_some()
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
            "--cache-from-disks" => a.cache_from_disks = Some(value("a folder")?.into()),
            "--script" => a.script = Some(value("a file")?.into()),
            "--pacing" => a.pacing = Some(value("a file")?.into()),
            "--capture" => a.capture = Some(value("a folder")?.into()),
            "--capture-hashes" => a.capture_hashes = true,
            "--max-frames" => a.max_frames = Some(number(&name, value("a number")?)?),
            "--timeout" => a.timeout_s = Some(number(&name, value("a number of seconds")?)?),
            "--scale" => {
                let s = number(&name, value("a number")?)?;
                if !(1..=16).contains(&s) {
                    return Err(format!("--scale {s}: the scale is a whole number from 1 to 16"));
                }
                a.scale = Some(s as u32);
            }
            "--audio-device" => {
                a.audio_device = Some(value("a device name")?.to_string_lossy().into_owned())
            }
            "--allow-fullscreen" => a.allow_fullscreen = true,
            "--bugs-dir" => a.bugs_dir = Some(value("a folder")?.into()),
            "--replay" => a.replay = Some(value("a bug zip")?.into()),
            "-h" | "--help" => return Err(USAGE.to_string()),
            _ => return Err(format!("unknown argument {name:?}\n{USAGE}")),
        }
    }
    if a.script.is_some() {
        if a.max_frames.is_none() {
            return Err("--script needs --max-frames N: a scripted run must end by itself \
                        (DESIGN §5.2)"
                .to_string());
        }
        a.timeout_s.get_or_insert(SCRIPT_TIMEOUT_S);
    }
    // A replay that is captured never ends by itself: it holds on its last frame (DESIGN §5.2).
    if a.replay.is_some() && a.capture.is_some() {
        a.timeout_s.get_or_insert(SCRIPT_TIMEOUT_S);
    }
    if a.replay.is_some() && a.pacing.is_some() {
        return Err("--replay plays the recording's own steps: it takes no --pacing".to_string());
    }
    if a.pacing.is_some() && a.script.is_none() {
        return Err("--pacing is for a scripted test run only: it needs --script (DESIGN §2.5)"
            .to_string());
    }
    if a.capture_hashes && a.capture.is_none() {
        return Err("--capture-hashes needs --capture DIR to write them in".to_string());
    }
    if a.allow_fullscreen && !a.max_frames.is_some_and(|n| n <= FULLSCREEN_MAX_FRAMES) {
        return Err(format!(
            "--allow-fullscreen needs --max-frames {FULLSCREEN_MAX_FRAMES} or fewer, so a \
             fullscreen run always ends by itself (DESIGN §5.2)"
        ));
    }
    if a.timeout_s == Some(0) {
        return Err("--timeout 0 would stop the app before it starts".to_string());
    }
    Ok(a)
}

fn number(name: &str, v: OsString) -> Result<u64, String> {
    let s = v.to_string_lossy();
    s.parse()
        .map_err(|_| format!("{name} {s:?}: not a whole number"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Result<Args, String> {
        parse(s.split_whitespace().map(OsString::from))
    }

    #[test]
    fn every_flag_parses() {
        let a = p("--home h --cache-from-disks d --script s --capture c --max-frames 9 --timeout 3 \
                   --scale 2 --audio-device none --pacing f --capture-hashes --bugs-dir b")
        .unwrap();
        assert_eq!(a.bugs_dir, Some("b".into()));
        assert_eq!(p("--replay z").unwrap().replay, Some("z".into()));
        assert_eq!(a.pacing, Some("f".into()));
        assert!(a.capture_hashes);
        assert_eq!(a.home, Some("h".into()));
        assert_eq!(a.cache_from_disks, Some("d".into()));
        assert_eq!(a.script, Some("s".into()));
        assert_eq!(a.capture, Some("c".into()));
        assert_eq!((a.max_frames, a.timeout_s, a.scale), (Some(9), Some(3), Some(2)));
        assert_eq!(a.audio_device.as_deref(), Some("none"));
        assert_eq!(p("").unwrap(), Args::default());
    }

    #[test]
    fn a_script_without_a_frame_cap_is_refused() {
        let e = p("--script s").unwrap_err();
        assert!(e.contains("--max-frames"), "{e}");
    }

    #[test]
    fn a_script_gets_a_120_second_timeout_unless_it_names_one() {
        assert_eq!(p("--script s --max-frames 5").unwrap().timeout_s, Some(120));
        assert_eq!(p("--script s --max-frames 5 --timeout 7").unwrap().timeout_s, Some(7));
        assert_eq!(p("--max-frames 5").unwrap().timeout_s, None);
    }

    #[test]
    fn a_script_stays_windowed_unless_it_allows_fullscreen_with_a_short_cap() {
        assert!(!p("--script s --max-frames 5").unwrap().fullscreen_allowed());
        assert!(p("").unwrap().fullscreen_allowed());
        let a = p("--script s --max-frames 300 --allow-fullscreen").unwrap();
        assert!(a.fullscreen_allowed());
        assert!(p("--script s --max-frames 301 --allow-fullscreen").is_err());
        assert!(p("--allow-fullscreen").is_err());
    }

    #[test]
    fn a_captured_replay_gets_a_120_second_timeout_and_stays_windowed() {
        assert_eq!(p("--replay z --capture c").unwrap().timeout_s, Some(120));
        assert_eq!(p("--replay z --capture c --timeout 9").unwrap().timeout_s, Some(9));
        assert_eq!(p("--replay z").unwrap().timeout_s, None);
        assert!(!p("--replay z").unwrap().fullscreen_allowed());
        assert!(p("--replay z").unwrap().unattended());
        assert!(!p("").unwrap().unattended());
        assert!(p("--replay").unwrap_err().contains("--replay"));
        assert!(p("--replay z --script s --max-frames 5 --pacing f").unwrap_err().contains("--pacing"));
    }

    #[test]
    fn a_pacing_feed_is_read_only_by_a_scripted_run() {
        let e = p("--pacing f").unwrap_err();
        assert!(e.contains("--script"), "{e}");
        assert!(p("--pacing f --max-frames 5 --capture c").is_err());
        assert_eq!(p("").unwrap().pacing, None);
        assert!(p("--script s --max-frames 5 --pacing f").is_ok());
    }

    #[test]
    fn hashes_are_captured_only_into_a_capture() {
        assert!(p("--capture-hashes").unwrap_err().contains("--capture DIR"));
        assert!(p("--capture c --capture-hashes").unwrap().capture_hashes);
    }

    #[test]
    fn bad_values_are_refused_with_the_flag_named() {
        assert!(p("--scale 0").unwrap_err().contains("--scale"));
        assert!(p("--max-frames x").unwrap_err().contains("--max-frames"));
        assert!(p("--home").unwrap_err().contains("--home"));
        assert!(p("--bugs-dir").unwrap_err().contains("--bugs-dir"));
        assert!(p("--timeout 0").is_err());
        assert!(p("--bogus").unwrap_err().contains("unknown"));
    }
}
