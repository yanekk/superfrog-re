//! The bug recorder (bug-recorder DESIGN §2.2, §2.4, §2.5): a recording held in memory from the
//! level's first frame of a Start Level with the recorder on, one input a stepped frame, until STOP
//! or a menu action that would change the game ends it; then written as one zip into the bugs
//! folder. The rules for what ends it are `menu.rs`'s ([`crate::App::on_command`]); the questions
//! go through [`crate::import::Ui`].
//!
//! The other way round, [`Replay`] is `--replay`'s source (§2.7): a bug zip's inputs fed one a
//! step in place of the player's, then nothing, so the app holds on the last frame.

use std::io;
use std::path::{Path, PathBuf};

use sf_core::input::FrameInput;
use sf_core::recording::{LevelSetup, Location, MAX_FRAMES, Recording, ZIP_RECORDING, ZIP_SCREENSHOT};
use sf_core::{LevelId, Rng};

use crate::atomic_write::atomic_write;

/// Whether a recording is running, and the recording.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Recorder {
    #[default]
    Off,
    Recording { rec: Recording },
}

impl Recorder {
    /// A recording of a level started with `setup` from `seed`, no frame stepped yet.
    pub fn start(setup: LevelSetup, seed: Rng, build: &str) -> Recorder {
        Recorder::Recording {
            rec: Recording {
                build: build.to_string(),
                setup,
                seed,
                stopped_at: None,
                note: String::new(),
                inputs: Vec::new(),
            },
        }
    }

    /// One stepped frame's input. Past [`MAX_FRAMES`] nothing more is held ([`Recorder::full`]):
    /// the app stops the recording there.
    pub fn step(&mut self, input: FrameInput) {
        if let Recorder::Recording { rec } = self
            && rec.inputs.len() < MAX_FRAMES as usize
        {
            rec.inputs.push(input);
        }
    }

    pub fn is_recording(&self) -> bool {
        matches!(self, Recorder::Recording { .. })
    }

    /// Has the recording reached the longest a recording may be (a day)?
    pub fn full(&self) -> bool {
        matches!(self, Recorder::Recording { rec } if rec.inputs.len() >= MAX_FRAMES as usize)
    }

    /// The recording, stopped at `at` with `note`; `None` when none was running.
    pub fn finish(self, at: Location, note: String) -> Option<Recording> {
        match self {
            Recorder::Off => None,
            Recorder::Recording { mut rec } => {
                rec.stopped_at = Some(at);
                rec.note = note;
                Some(rec)
            }
        }
    }
}

/// `--replay`: a recording's inputs, fed to the game one a step in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replay {
    inputs: Vec<FrameInput>,
    played: usize,
    /// [`Replay::just_finished`] has said so.
    told: bool,
}

impl Replay {
    pub fn new(rec: &Recording) -> Replay {
        Replay { inputs: rec.inputs.clone(), played: 0, told: false }
    }

    /// The input for the next step, `None` once every recorded one has been played.
    pub fn next(&mut self) -> Option<FrameInput> {
        let input = self.inputs.get(self.played).copied()?;
        self.played += 1;
        Some(input)
    }

    /// Has every recorded input been played?
    pub fn finished(&self) -> bool {
        self.played == self.inputs.len()
    }

    /// The frame the replay holds on, the first time it is asked once every input is played;
    /// `None` before and after.
    pub fn just_finished(&mut self) -> Option<usize> {
        (self.finished() && !std::mem::replace(&mut self.told, true)).then_some(self.played)
    }
}

/// The largest entry read out of a bug zip: `recording.txt` at [`MAX_FRAMES`] with the input
/// changing every frame is ~60 MB.
const MAX_ENTRY: usize = 256 << 20;

/// The recording in a bug zip's bytes, or why there is none: not a zip, no `recording.txt` or
/// `screenshot.png`, or a recording the parser or [`Recording::validate`] refuses (as
/// `sf-conform`'s `bug::read_zip`, which sf-app may not depend on, minus its PNG check: the app
/// replays without the screenshot).
pub fn read_zip(bytes: &[u8]) -> Result<Recording, String> {
    let entries = sf_disk::zip::read_entries(bytes, |n| n == ZIP_RECORDING || n == ZIP_SCREENSHOT, MAX_ENTRY)
        .map_err(|e| format!("not a bug zip: {e}"))?;
    let has = |name: &str| entries.iter().find(|e| e.name == name).ok_or_else(|| format!("not a bug zip: it has no {name}"));
    let text = has(ZIP_RECORDING)?;
    has(ZIP_SCREENSHOT)?;
    let text = std::str::from_utf8(&text.bytes).map_err(|_| format!("{ZIP_RECORDING}: it is not UTF-8 text"))?;
    let rec = Recording::parse(text).map_err(|e| format!("{ZIP_RECORDING}: {e}"))?;
    rec.validate().map_err(|e| format!("{ZIP_RECORDING}: {e}"))?;
    Ok(rec)
}

/// What the player answered "Recording in progress" with, before an action that would end it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndChoice {
    /// Save… : the save window, then the action.
    Save,
    /// The recording dropped, then the action.
    Discard,
    /// Back to play, still recording; the action is not taken.
    Cancel,
}

impl EndChoice {
    pub fn word(self) -> &'static str {
        match self {
            EndChoice::Save => "save",
            EndChoice::Discard => "discard",
            EndChoice::Cancel => "cancel",
        }
    }
}

/// The bugs folder (DESIGN §2.4): `--bugs-dir`, else `bugs/` in the main checkout this app was
/// built from, else `~/Documents/Superfrog Bugs/`.
pub fn bugs_dir(arg: Option<&Path>) -> PathBuf {
    bugs_dir_in(arg, Path::new(env!("CARGO_MANIFEST_DIR")), std::env::var_os("HOME"))
}

/// [`bugs_dir`] for a build made in `built_in`, the user's home `user_home`.
fn bugs_dir_in(arg: Option<&Path>, built_in: &Path, user_home: Option<std::ffi::OsString>) -> PathBuf {
    if let Some(a) = arg {
        return a.to_path_buf();
    }
    match main_checkout(built_in) {
        Some(root) => root.join("bugs"),
        None => PathBuf::from(user_home.unwrap_or_else(|| "/".into())).join("Documents/Superfrog Bugs"),
    }
}

/// The main checkout `dir` is in: the parent of the git common dir, so a build in any worktree
/// saves into one folder. Read from the files git leaves, not by running git: a worktree's `.git`
/// is a file naming its own git dir, whose `commondir` names the shared one.
fn main_checkout(dir: &Path) -> Option<PathBuf> {
    for d in dir.ancestors() {
        let git = d.join(".git");
        if git.is_dir() {
            return Some(d.to_path_buf());
        }
        if git.is_file() {
            let text = std::fs::read_to_string(&git).ok()?;
            let own = d.join(text.lines().find_map(|l| l.strip_prefix("gitdir:"))?.trim());
            let common = match std::fs::read_to_string(own.join("commondir")) {
                Ok(c) => own.join(c.trim()),
                Err(_) => own,
            };
            return std::fs::canonicalize(common).ok()?.parent().map(Path::to_path_buf);
        }
    }
    None
}

/// The zip's path in `dir` (DESIGN §2.4): `bug-<level>-<YYYY-MM-DD-HHMM>.zip`, with `-2`, `-3`, …
/// before `.zip` when that name is taken. `now` is the local time, `(year, month, day, hour,
/// minute)`.
pub fn zip_name(dir: &Path, at_level: LevelId, now: (i32, u8, u8, u8, u8)) -> PathBuf {
    let (y, mo, d, h, mi) = now;
    let stem = format!("bug-{at_level}-{y:04}-{mo:02}-{d:02}-{h:02}{mi:02}");
    let mut path = dir.join(format!("{stem}.zip"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem}-{n}.zip"));
        n += 1;
    }
    path
}

/// The bug zip (DESIGN §2.5): `recording.txt` and `screenshot.png`, stored, written whole or not
/// at all. A recording that cannot be read back is refused before anything is written.
pub fn write_zip(path: &Path, rec: &Recording, png: &[u8]) -> io::Result<()> {
    rec.validate().map_err(io::Error::other)?;
    let text = rec.to_text();
    let zip = sf_disk::zip::write_stored(&[(ZIP_RECORDING, text.as_bytes()), (ZIP_SCREENSHOT, png)]);
    atomic_write(path, &zip)
}

/// The local time now, as [`zip_name`] takes it.
pub fn local_now() -> (i32, u8, u8, u8, u8) {
    // SAFETY: `time` and `localtime_r` fill the values they are given.
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&t, &mut tm).is_null() {
            return (1970, 1, 1, 0, 0);
        }
        (tm.tm_year + 1900, tm.tm_mon as u8 + 1, tm.tm_mday as u8, tm.tm_hour as u8, tm.tm_min as u8)
    }
}

/// The level a zip is named for: where play is, or the level started when not in one.
pub fn named_level(at: &Location, rec: &Recording) -> LevelId {
    match at {
        Location::Level { id, .. } => *id,
        Location::Screen(_) => rec.setup.level,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sf_core::input::Joy;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sf-app-recorder-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn w1_1() -> LevelId {
        "W1-1".parse().unwrap()
    }

    fn recording(frames: usize) -> Recording {
        let mut r = Recorder::start(LevelSetup::default(), Rng::seeded(1, 2), "test build");
        for n in 0..frames {
            r.step(FrameInput { joy: if n % 3 == 0 { Joy::RIGHT } else { Joy::NONE }, ..FrameInput::NONE });
        }
        r.finish(Location::Screen("tally"), "a note".into()).unwrap()
    }

    #[test]
    fn a_recording_holds_its_start_every_step_and_where_it_stopped() {
        let mut r = Recorder::default();
        assert!(!r.is_recording());
        r.step(FrameInput::NONE);
        assert_eq!(r.clone().finish(Location::Screen("title"), String::new()), None);
        let rec = recording(5);
        assert_eq!(rec.inputs.len(), 5);
        assert_eq!(rec.inputs[0].joy, Joy::RIGHT);
        assert_eq!(rec.stopped_at, Some(Location::Screen("tally")));
        assert_eq!((rec.note.as_str(), rec.build.as_str()), ("a note", "test build"));
        assert_eq!(rec.seed, Rng::seeded(1, 2));
        assert_eq!(rec.setup, LevelSetup::default());
    }

    #[test]
    fn a_full_recording_holds_no_more_than_a_day() {
        let mut r = Recorder::start(LevelSetup::default(), Rng::seeded(0, 0), "b");
        if let Recorder::Recording { rec } = &mut r {
            rec.inputs = vec![FrameInput::NONE; MAX_FRAMES as usize - 1];
        }
        assert!(!r.full());
        r.step(FrameInput::NONE);
        assert!(r.full());
        r.step(FrameInput::NONE);
        let rec = r.finish(Location::Screen("title"), String::new()).unwrap();
        assert_eq!(rec.inputs.len(), MAX_FRAMES as usize);
        assert!(rec.validate().is_ok());
    }

    #[test]
    fn a_zip_is_named_for_the_level_and_minute_and_never_overwrites() {
        let d = temp("names");
        let now = (2026, 10, 20, 9, 5);
        let first = zip_name(&d, w1_1(), now);
        assert_eq!(first, d.join("bug-W1-1-2026-10-20-0905.zip"));
        std::fs::write(&first, b"x").unwrap();
        let second = zip_name(&d, w1_1(), now);
        assert_eq!(second, d.join("bug-W1-1-2026-10-20-0905-2.zip"));
        std::fs::write(&second, b"x").unwrap();
        assert_eq!(zip_name(&d, w1_1(), now), d.join("bug-W1-1-2026-10-20-0905-3.zip"));
        assert_eq!(
            zip_name(&d, "WB-1".parse().unwrap(), (2026, 1, 2, 23, 59)),
            d.join("bug-WB-1-2026-01-02-2359.zip")
        );
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn the_bugs_folder_is_the_flag_else_the_main_checkout_s_else_documents() {
        let d = temp("bugs-dir");
        let home = Some(std::ffi::OsString::from("/Users/u"));
        // The flag wins.
        assert_eq!(bugs_dir_in(Some(Path::new("/x")), &d, home.clone()), PathBuf::from("/x"));
        // No checkout above.
        let lone = d.join("lone/engine/crates/sf-app");
        std::fs::create_dir_all(&lone).unwrap();
        assert_eq!(bugs_dir_in(None, &lone, home.clone()), PathBuf::from("/Users/u/Documents/Superfrog Bugs"));
        // A checkout: `.git` a folder.
        let main = d.join("main");
        let built = main.join("engine/crates/sf-app");
        std::fs::create_dir_all(&built).unwrap();
        std::fs::create_dir_all(main.join(".git/worktrees/wt")).unwrap();
        assert_eq!(bugs_dir_in(None, &built, home.clone()), main.join("bugs"));
        // A worktree: `.git` a file naming its git dir, whose `commondir` is the main one.
        let wt = main.join(".claude/worktrees/wt");
        let wt_built = wt.join("engine/crates/sf-app");
        std::fs::create_dir_all(&wt_built).unwrap();
        let own = main.join(".git/worktrees/wt");
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", own.display())).unwrap();
        std::fs::write(own.join("commondir"), "../..\n").unwrap();
        let main_real = std::fs::canonicalize(&main).unwrap();
        assert_eq!(bugs_dir_in(None, &wt_built, home.clone()), main_real.join("bugs"));
        // A relative `gitdir:` resolves from the worktree.
        std::fs::write(wt.join(".git"), "gitdir: ../../../.git/worktrees/wt\n").unwrap();
        assert_eq!(bugs_dir_in(None, &wt_built, home), main_real.join("bugs"));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn the_zip_holds_the_recording_and_the_screenshot_stored() {
        let d = temp("zip");
        let rec = recording(40);
        let path = d.join("bugs/bug.zip");
        write_zip(&path, &rec, b"png bytes").unwrap();
        let zip = std::fs::read(&path).unwrap();
        let entries = sf_disk::zip::read_entries(&zip, |_| true, 1 << 20).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, [ZIP_RECORDING, ZIP_SCREENSHOT]);
        let text = String::from_utf8(entries[0].bytes.clone()).unwrap();
        assert_eq!(Recording::parse(&text).unwrap(), rec);
        assert_eq!(entries[1].bytes, b"png bytes");
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_write_that_fails_returns_the_error_and_the_recording_is_still_held() {
        use std::os::unix::fs::PermissionsExt;
        let d = temp("readonly");
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o555)).unwrap();
        let rec = recording(10);
        let path = zip_name(&d, w1_1(), (2026, 10, 20, 12, 0));
        let e = write_zip(&path, &rec, b"png").unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::PermissionDenied, "{e}");
        assert!(!path.exists());
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 0, "no half zip, no temporary");
        // Held: the same recording saves once the folder can be written.
        std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o755)).unwrap();
        write_zip(&path, &rec, b"png").unwrap();
        assert!(path.is_file());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_recording_that_could_not_be_read_back_is_not_written() {
        let d = temp("invalid");
        let mut rec = recording(3);
        rec.note = "two\nlines".into();
        assert!(write_zip(&d.join("x.zip"), &rec, b"").is_err());
        assert!(!d.join("x.zip").exists());
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn a_replay_feeds_the_inputs_in_order_then_nothing() {
        let rec = recording(4);
        let mut r = Replay::new(&rec);
        assert!(!r.finished());
        assert_eq!(r.just_finished(), None);
        let fed: Vec<FrameInput> = std::iter::from_fn(|| r.next()).collect();
        assert_eq!(fed, rec.inputs);
        assert!(r.finished());
        assert_eq!(r.next(), None);
        assert_eq!(r.just_finished(), Some(4));
        assert_eq!(r.just_finished(), None, "said once");
        let mut empty = Replay::new(&recording(0));
        assert!(empty.finished());
        assert_eq!(empty.just_finished(), Some(0));
    }

    #[test]
    fn a_zip_written_reads_back_and_a_bad_one_says_why() {
        let d = temp("read");
        let rec = recording(7);
        write_zip(&d.join("z.zip"), &rec, b"png").unwrap();
        assert_eq!(read_zip(&std::fs::read(d.join("z.zip")).unwrap()), Ok(rec.clone()));
        assert!(read_zip(b"hello").unwrap_err().starts_with("not a bug zip"));
        let only_shot = sf_disk::zip::write_stored(&[(ZIP_SCREENSHOT, b"png".as_slice())]);
        assert!(read_zip(&only_shot).unwrap_err().contains("no recording.txt"));
        let text = rec.to_text();
        let only_rec = sf_disk::zip::write_stored(&[(ZIP_RECORDING, text.as_bytes())]);
        assert!(read_zip(&only_rec).unwrap_err().contains("no screenshot.png"));
        let bad = sf_disk::zip::write_stored(&[(ZIP_RECORDING, b"not a recording".as_slice()), (ZIP_SCREENSHOT, b"png")]);
        assert!(read_zip(&bad).unwrap_err().starts_with("recording.txt: "), "{:?}", read_zip(&bad));
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn the_zip_is_named_for_where_play_is_or_the_level_started() {
        let rec = recording(1);
        let w12: LevelId = "W1-2".parse().unwrap();
        assert_eq!(named_level(&Location::Level { id: w12, col: 1, row: 2 }, &rec), w12);
        assert_eq!(named_level(&Location::Screen("slot machine"), &rec), w1_1());
    }
}
