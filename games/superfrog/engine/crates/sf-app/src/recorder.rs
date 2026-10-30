//! The bug recorder (bug-recorder DESIGN §2.2, §2.4, §2.5): a recording held in memory from the
//! level's first frame of a Start Level with the recorder on, one input a stepped frame, until STOP
//! or a menu action that would change the game ends it; then written as one zip into the bugs
//! folder. The rules for what ends it are `menu.rs`'s ([`crate::App::on_command`]); the questions
//! go through [`crate::import::Ui`].
//!
//! The other way round, [`Replay`] is `--replay`'s source (§2.7): a bug zip's inputs fed one a
//! step in place of the player's, then nothing, so the app holds on the last frame.
//!
//! The recorder and replay state machines and the zip's layout are the toolkit's (`amiga_record`,
//! toolkit-extract DESIGN §2.8); widescreen's toggles, the zip's name and folder are kept here.

use std::io;
use std::path::{Path, PathBuf};

use sf_core::recording::{Header, LevelSetup, Location, Recording, Superfrog, seed_bytes};
use sf_core::{LevelId, Rng};

use amiga_app::atomic_write::write as atomic_write;

/// Whether a recording is running, and the recording (`amiga_record`'s, with Superfrog's header).
pub type Recorder = amiga_record::Recorder<Superfrog>;

/// `--replay`: a recording's inputs, fed to the game one a step in order.
pub type Replay = amiga_record::Replay<Superfrog>;

/// A recording of a level started with `setup` from `seed`, in widescreen when `widescreen`, no
/// frame stepped yet.
pub fn start(setup: LevelSetup, seed: Rng, build: &str, widescreen: bool) -> Recorder {
    Recorder::start(Header::new(setup, widescreen), seed_bytes(seed), build)
}

/// Widescreen switched before the next step (real-widescreen DESIGN §2.8). Switched twice before
/// one step, the two cancel: the game steps in the mode it had.
pub fn toggle(recorder: &mut Recorder) {
    if let Some(rec) = recorder.recording_mut() {
        let at = rec.inputs.len() as u32;
        match rec.toggles.last() {
            Some(&t) if t == at => {
                rec.toggles.pop();
            }
            _ => rec.toggles.push(at),
        }
    }
}

/// Widescreen for the replay's next step, as recorded (real-widescreen DESIGN §2.8): the mode at
/// frame 0 before any step, and once every input is played the mode the last step played in.
pub fn replay_widescreen(r: &Replay) -> bool {
    let rec = r.recording();
    rec.widescreen_at(r.played().min(rec.inputs.len().saturating_sub(1)) as u32)
}

/// The recording in a bug zip's bytes (`amiga_record::read_zip`), or why there is none. The app
/// replays without the screenshot, so it is not decoded.
pub fn open_zip(bytes: &[u8]) -> Result<Recording, String> {
    amiga_record::read_zip::<Superfrog>(bytes).map(|(rec, _)| rec)
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

/// The bugs folder (DESIGN §2.4): `--bugs-dir`, else `games/superfrog/bugs/` in the main checkout this app was
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
        Some(root) => root.join("games/superfrog/bugs"),
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
    let zip = amiga_record::zip_bytes(rec, png).map_err(io::Error::other)?;
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
    use sf_core::input::{FrameInput, Joy};
    use sf_core::recording::{MAX_FRAMES, RecordingRng, ZIP_RECORDING, ZIP_SCREENSHOT};

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
        let mut r = start(LevelSetup::default(), Rng::seeded(1, 2), "test build", false);
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
        assert_eq!(r.frames(), None);
        let mut on = start(LevelSetup::default(), Rng::seeded(1, 2), "b", false);
        assert_eq!(on.frames(), Some(0));
        on.step(FrameInput::NONE);
        assert_eq!(on.frames(), Some(1));
        let rec = recording(5);
        assert_eq!(rec.inputs.len(), 5);
        assert_eq!(rec.inputs[0].joy, Joy::RIGHT);
        assert_eq!(rec.stopped_at, Some(Location::Screen("tally")));
        assert_eq!((rec.note.as_str(), rec.build.as_str()), ("a note", "test build"));
        assert_eq!(rec.rng(), Rng::seeded(1, 2));
        assert_eq!(rec.setup, LevelSetup::default());
    }

    /// real-widescreen DESIGN §2.8: the mode at the start, and a toggle before each step a switch
    /// came before; two before one step cancel, and one after the last step is dropped.
    #[test]
    fn a_recording_holds_its_widescreen_and_every_switch_before_a_step() {
        let mut r = start(LevelSetup::default(), Rng::seeded(1, 2), "b", true);
        toggle(&mut r);
        for n in 0..10 {
            if n == 3 || n == 7 {
                toggle(&mut r);
            }
            if n == 5 {
                toggle(&mut r);
                toggle(&mut r);
            }
            r.step(FrameInput::NONE);
        }
        toggle(&mut r);
        let rec = r.finish(Location::Screen("title"), String::new()).unwrap();
        assert!(rec.widescreen);
        assert_eq!(rec.toggles, [0, 3, 7]);
        assert!(rec.validate().is_ok());
        assert_eq!(Recording::parse(&rec.to_text()).unwrap(), rec);
        let modes: Vec<bool> = (0..10).map(|k| rec.widescreen_at(k)).collect();
        assert_eq!(modes, [false, false, false, true, true, true, true, false, false, false]);
        let mut off = Recorder::default();
        toggle(&mut off);
        assert_eq!(off, Recorder::Off);
    }

    #[test]
    fn a_replay_says_the_mode_of_the_step_it_feeds_next() {
        let mut rec = recording(6);
        rec.widescreen = true;
        rec.toggles = vec![2, 4];
        let mut r = Replay::new(&rec);
        let mut modes = Vec::new();
        while !r.finished() {
            modes.push(replay_widescreen(&r));
            r.next();
        }
        assert_eq!(modes, [true, true, false, false, true, true]);
        assert!(replay_widescreen(&r), "held on the last step's mode");
        assert!(!replay_widescreen(&Replay::new(&recording(0))));
    }

    #[test]
    fn a_full_recording_holds_no_more_than_a_day() {
        let mut r = start(LevelSetup::default(), Rng::seeded(0, 0), "b", false);
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
        let built = main.join("games/superfrog/engine/crates/sf-app");
        std::fs::create_dir_all(&built).unwrap();
        std::fs::create_dir_all(main.join(".git/worktrees/wt")).unwrap();
        assert_eq!(bugs_dir_in(None, &built, home.clone()), main.join("games/superfrog/bugs"));
        // A worktree: `.git` a file naming its git dir, whose `commondir` is the main one.
        let wt = main.join(".claude/worktrees/wt");
        let wt_built = wt.join("games/superfrog/engine/crates/sf-app");
        std::fs::create_dir_all(&wt_built).unwrap();
        let own = main.join(".git/worktrees/wt");
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", own.display())).unwrap();
        std::fs::write(own.join("commondir"), "../..\n").unwrap();
        let main_real = std::fs::canonicalize(&main).unwrap();
        assert_eq!(bugs_dir_in(None, &wt_built, home.clone()), main_real.join("games/superfrog/bugs"));
        // A relative `gitdir:` resolves from the worktree.
        std::fs::write(wt.join(".git"), "gitdir: ../../../.git/worktrees/wt\n").unwrap();
        assert_eq!(bugs_dir_in(None, &wt_built, home), main_real.join("games/superfrog/bugs"));
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
        assert_eq!(open_zip(&std::fs::read(d.join("z.zip")).unwrap()), Ok(rec.clone()));
        assert!(open_zip(b"hello").unwrap_err().starts_with("not a bug zip"));
        let only_shot = sf_disk::zip::write_stored(&[(ZIP_SCREENSHOT, b"png".as_slice())]);
        assert!(open_zip(&only_shot).unwrap_err().contains("no recording.txt"));
        let text = rec.to_text();
        let only_rec = sf_disk::zip::write_stored(&[(ZIP_RECORDING, text.as_bytes())]);
        assert!(open_zip(&only_rec).unwrap_err().contains("no screenshot.png"));
        let bad = sf_disk::zip::write_stored(&[(ZIP_RECORDING, b"not a recording".as_slice()), (ZIP_SCREENSHOT, b"png")]);
        assert!(open_zip(&bad).unwrap_err().starts_with("recording.txt: "), "{:?}", open_zip(&bad));
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// toolkit-extract T09: every bug zip the owner saved (the main checkout's `games/superfrog/bugs/`, git-ignored,
    /// so none on a fresh clone) reads back and is written again as it was, text and zip. A version
    /// 1 file is written as version 2, as before the move: the magic's number and `widescreen off`
    /// after `wings`, nothing else.
    #[test]
    fn every_saved_bug_zip_reads_back_byte_for_byte() {
        let Ok(dir) = std::fs::read_dir(bugs_dir(None)) else { return };
        let mut n = 0;
        for p in dir.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "zip")) {
            let bytes = std::fs::read(&p).unwrap();
            let (rec, png) = amiga_record::read_zip::<Superfrog>(&bytes).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            let entries = sf_disk::zip::read_entries(&bytes, |_| true, 1 << 28).unwrap();
            let text = String::from_utf8(entries[0].bytes.clone()).unwrap();
            let want = match text.strip_prefix("superfrog recording 1\n") {
                Some(v1) => format!("superfrog recording 2\n{}", v1.replacen("\nwings off\n", "\nwings off\nwidescreen off\n", 1).replacen("\nwings on\n", "\nwings on\nwidescreen off\n", 1)),
                None => text.clone(),
            };
            assert_eq!(rec.to_text(), want, "{}: the text", p.display());
            let zip = sf_disk::zip::write_stored(&[(ZIP_RECORDING, want.as_bytes()), (ZIP_SCREENSHOT, &png)]);
            assert_eq!(amiga_record::zip_bytes(&rec, &png).unwrap(), zip, "{}: the zip", p.display());
            if want == text {
                assert_eq!(zip, bytes, "{}: the zip as saved", p.display());
            }
            n += 1;
        }
        eprintln!("{n} saved bug zips read back as saved");
    }

    #[test]
    fn the_zip_is_named_for_where_play_is_or_the_level_started() {
        let rec = recording(1);
        let w12: LevelId = "W1-2".parse().unwrap();
        assert_eq!(named_level(&Location::Level { id: w12, col: 1, row: 2 }, &rec), w12);
        assert_eq!(named_level(&Location::Screen("slot machine"), &rec), w1_1());
    }
}
