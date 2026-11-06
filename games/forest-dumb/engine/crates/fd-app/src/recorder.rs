//! The bug recorder (DESIGN §2.7, §2.8), as Superfrog's: Game ▸ Start Level… starts W1-1 afresh
//! with "Enable recorder?" among its fields; with it on, a recording is held in memory from the
//! level's first frame, one input a stepped frame, until Stop Recording (⌘. or the bar's STOP) opens
//! the save window and writes one zip into the bugs folder. Start Level and Quit during one ask
//! "Recording in progress" first: Save…, Discard or Cancel. What ends a recording is `main.rs`'s
//! (`App::on_command`).
//!
//! The recorder, the recording's text and the zip are the toolkit's (`amiga_record`) with this
//! game's header (`fd_core::recording`); the three windows are the toolkit's (`amiga_app`). Here:
//! this game's Start Level fields, the bugs folder, the zip's name, and [`Ui`], the windows shown
//! for a person or answered by a script.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use amiga_app::atomic_write::write as atomic_write;
use amiga_app::save_window::{Outcome, SaveAction, SaveForm};
use amiga_app::start_level_window::{
    FieldAction, FieldKind, Form, LevelField, StartLevelSpec, Value, Values,
};
use fd_core::recording::{ForestDumb, LevelId, Location, Recording};
use winit::window::Window;

/// Whether a recording is running, and the recording.
pub type Recorder = amiga_record::Recorder<ForestDumb>;

/// The app's name in the log and the toolkit's windows.
const APP: &str = "fd-app";

/// The Start Level window's fields (DESIGN §2.7): a level selector offering level 1 only and
/// "Enable recorder?", on when the window opens, as Superfrog's is.
pub fn spec() -> StartLevelSpec {
    StartLevelSpec {
        fields: vec![
            LevelField {
                id: "level".into(),
                label: "Level".into(),
                kind: FieldKind::Dropdown(LevelId::ALL.iter().map(|l| l.to_string()).collect()),
                default: Value::Choice(0),
            },
            LevelField {
                id: "record".into(),
                label: "Enable recorder?".into(),
                kind: FieldKind::Checkbox,
                default: Value::On(true),
            },
        ],
    }
}

/// The level and whether to record, from the window's values.
pub fn chosen(v: &Values) -> (LevelId, bool) {
    let level = v
        .choice(0)
        .and_then(|i| LevelId::ALL.get(i).copied())
        .unwrap_or_default();
    (level, v.on(1).unwrap_or(true))
}

/// What the player answered "Recording in progress" with, before an action that would end it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndChoice {
    /// Save…: the save window, then the action.
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

/// The bugs folder (DESIGN §2.8): `--bugs-dir`; else, for a scripted run, `<home>/bugs`, so a test
/// never writes into the repo; else `games/forest-dumb/bugs/` in the main checkout this app was
/// built from (git-ignored); else `~/Documents/Forest Dumb Bugs/`.
pub fn bugs_dir(arg: Option<&Path>, scripted: bool, home: &Path) -> PathBuf {
    match (arg, scripted) {
        (Some(a), _) => a.to_path_buf(),
        (None, true) => home.join("bugs"),
        (None, false) => bugs_dir_in(
            Path::new(env!("CARGO_MANIFEST_DIR")),
            std::env::var_os("HOME"),
        ),
    }
}

/// The checkout's bugs folder for a build made in `built_in`, else the user's Documents one.
fn bugs_dir_in(built_in: &Path, user_home: Option<std::ffi::OsString>) -> PathBuf {
    match main_checkout(built_in) {
        Some(root) => root.join("games/forest-dumb/bugs"),
        None => PathBuf::from(user_home.unwrap_or_else(|| "/".into()))
            .join("Documents/Forest Dumb Bugs"),
    }
}

/// The main checkout `dir` is in: the parent of the git common dir, so a build in any worktree
/// saves into one folder. Read from the files git leaves, not by running git (the app does not
/// need git): a worktree's `.git` is a file naming its own git dir, whose `commondir` names the
/// shared one.
fn main_checkout(dir: &Path) -> Option<PathBuf> {
    for d in dir.ancestors() {
        let git = d.join(".git");
        if git.is_dir() {
            return Some(d.to_path_buf());
        }
        if git.is_file() {
            let text = std::fs::read_to_string(&git).ok()?;
            let own = d.join(
                text.lines()
                    .find_map(|l| l.strip_prefix("gitdir:"))?
                    .trim(),
            );
            let common = match std::fs::read_to_string(own.join("commondir")) {
                Ok(c) => own.join(c.trim()),
                Err(_) => own,
            };
            return std::fs::canonicalize(common)
                .ok()?
                .parent()
                .map(Path::to_path_buf);
        }
    }
    None
}

/// The zip's path in `dir`: `bug-<level>-<YYYY-MM-DD-HHMM>.zip`, with `-2`, `-3`, … before `.zip`
/// when that name is taken. `now` is the local time, `(year, month, day, hour, minute)`.
pub fn zip_name(dir: &Path, level: LevelId, now: (i32, u8, u8, u8, u8)) -> PathBuf {
    let (y, mo, d, h, mi) = now;
    let stem = format!("bug-{level}-{y:04}-{mo:02}-{d:02}-{h:02}{mi:02}");
    let mut path = dir.join(format!("{stem}.zip"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem}-{n}.zip"));
        n += 1;
    }
    path
}

/// The bug zip: `recording.txt` and `screenshot.png`, stored, written whole or not at all. A
/// recording that could not be read back is refused before anything is written.
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
        (
            tm.tm_year + 1900,
            tm.tm_mon as u8 + 1,
            tm.tm_mday as u8,
            tm.tm_hour as u8,
            tm.tm_min as u8,
        )
    }
}

/// The recorder's windows: shown to a person ([`NativeUi`]), or answered by a script
/// ([`ScriptedUi`]). Each is modal: no frame is stepped while one is open.
pub trait Ui {
    /// Game ▸ Start Level…: the level and whether to record, `None` for Cancel.
    fn start_level(&mut self) -> Option<(LevelId, bool)>;
    /// "Recording in progress" before `action`.
    fn recording_in_progress(&mut self, action: &str) -> EndChoice;
    /// The save window: `save` is called with each note the player saves with, until it succeeds
    /// (`true`) or the player discards (`false`).
    fn save_recording(
        &mut self,
        shot: &[u8],
        at: &Location,
        name: &str,
        save: &mut dyn FnMut(&str) -> Result<(), String>,
    ) -> bool;
    /// A plain message.
    fn notice(&mut self, message: &str);
    /// The game window, once it exists: the windows open over it.
    fn attach(&mut self, _window: Arc<Window>) {}
}

/// The real windows.
#[derive(Default)]
pub struct NativeUi {
    parent: Option<Arc<Window>>,
}

const IN_PROGRESS: &str = "Recording in progress";
const SAVE_FIRST: &str = "Save…";
const DISCARD: &str = "Discard";
const CANCEL: &str = "Cancel";

impl Ui for NativeUi {
    fn start_level(&mut self) -> Option<(LevelId, bool)> {
        amiga_app::start_level_window::run_modal(APP, &spec(), self.parent.as_deref())
            .map(|v| chosen(&v))
    }

    /// An alert with Save…, Discard and Cancel (Esc), as a sheet on the game window.
    fn recording_in_progress(&mut self, action: &str) -> EndChoice {
        let d = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Warning)
            .set_title(IN_PROGRESS)
            .set_description(format!("{action} will end the recording. Save it first?"))
            .set_buttons(rfd::MessageButtons::YesNoCancelCustom(
                SAVE_FIRST.into(),
                DISCARD.into(),
                CANCEL.into(),
            ));
        let r = match &self.parent {
            Some(w) => d.set_parent(&**w),
            None => d,
        }
        .show();
        match r {
            rfd::MessageDialogResult::Custom(b) if b == SAVE_FIRST => EndChoice::Save,
            rfd::MessageDialogResult::Custom(b) if b == DISCARD => EndChoice::Discard,
            rfd::MessageDialogResult::Yes => EndChoice::Save,
            rfd::MessageDialogResult::No => EndChoice::Discard,
            _ => EndChoice::Cancel,
        }
    }

    fn save_recording(
        &mut self,
        shot: &[u8],
        at: &Location,
        name: &str,
        save: &mut dyn FnMut(&str) -> Result<(), String>,
    ) -> bool {
        amiga_app::save_window::run(
            APP,
            self.parent.as_deref(),
            shot,
            SaveForm::new(name, at),
            save,
        )
    }

    fn notice(&mut self, message: &str) {
        let d = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Info)
            .set_title("Forest Dumb")
            .set_description(message)
            .set_buttons(rfd::MessageButtons::Ok);
        match &self.parent {
            Some(w) => d.set_parent(&**w),
            None => d,
        }
        .show();
    }

    fn attach(&mut self, window: Arc<Window>) {
        self.parent = Some(window);
    }
}

/// A script's answers ([`crate::script::Answers`]), taken in order, and a line in the log for every
/// window shown: `start-level: -> <level> [record]|cancel`, `recording: <action> -> save|discard|
/// cancel`, `save-recording: <name> at <place> -> note <text>|discard`, `notice: <text>`. With no
/// answer left a window is cancelled, or the save window discarded, so a run always goes on.
#[derive(Debug, Default)]
pub struct ScriptedUi {
    starts: std::collections::VecDeque<Vec<FieldAction>>,
    endings: std::collections::VecDeque<EndChoice>,
    notes: std::collections::VecDeque<Option<String>>,
    /// Where each line is appended as it is shown (the capture folder's `prompts.txt`).
    pub log_file: Option<PathBuf>,
}

impl ScriptedUi {
    pub fn new(a: crate::script::Answers, log_file: Option<PathBuf>) -> ScriptedUi {
        ScriptedUi {
            starts: a.starts.into(),
            endings: a.endings.into(),
            notes: a.notes.into(),
            log_file,
        }
    }

    fn record(&mut self, line: String) {
        let line = line.replace('\n', " ");
        eprintln!("{APP}: {line}");
        if let Some(f) = &self.log_file {
            use std::io::Write;
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(f)
                .and_then(|mut f| writeln!(f, "{line}"));
        }
    }
}

impl Ui for ScriptedUi {
    fn start_level(&mut self) -> Option<(LevelId, bool)> {
        let spec = spec();
        let mut form = Form::new(spec.clone());
        let mut answer = None;
        for action in self.starts.pop_front().unwrap_or_default() {
            eprintln!("{APP}: start-level form: {}", action.words(&spec));
            if let Some(done) = form.apply(action) {
                answer = done.map(|v| chosen(&v));
                break;
            }
        }
        let shown = match answer {
            None => "cancel".to_string(),
            Some((level, record)) => format!("{level}{}", if record { " record" } else { "" }),
        };
        self.record(format!("start-level: -> {shown}"));
        answer
    }

    fn recording_in_progress(&mut self, action: &str) -> EndChoice {
        let c = self.endings.pop_front().unwrap_or(EndChoice::Cancel);
        self.record(format!("recording: {action} -> {}", c.word()));
        c
    }

    /// The notes in order, each saved through the window's model (a newline becomes a space); one
    /// whose write fails is followed by the next, as a person types again in the window that stayed
    /// open. With none left, Discard.
    fn save_recording(
        &mut self,
        _shot: &[u8],
        at: &Location,
        name: &str,
        save: &mut dyn FnMut(&str) -> Result<(), String>,
    ) -> bool {
        let mut form = SaveForm::new(name, at);
        loop {
            let Some(note) = self.notes.pop_front().flatten() else {
                self.record(format!("save-recording: {name} at {at} -> discard"));
                return false;
            };
            form.action(SaveAction::Note(note));
            let Outcome::Save(note) = form.action(SaveAction::Save) else {
                unreachable!("Save saves")
            };
            self.record(format!("save-recording: {name} at {at} -> note {note}"));
            match save(&note) {
                Ok(()) => return true,
                Err(why) => self.record(format!("notice: {why}")),
            }
        }
    }

    fn notice(&mut self, message: &str) {
        self.record(format!("notice: {message}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amiga_input::{FrameInput, Joy};
    use fd_core::recording::{ZIP_RECORDING, ZIP_SCREENSHOT, recorder};

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("fd-app-recorder-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn at() -> Location {
        "W1-1:7,7".parse().unwrap()
    }

    fn recording(frames: usize) -> Recording {
        let mut r = recorder(LevelId::W1_1, "test build");
        for n in 0..frames {
            r.step(FrameInput::joy(if n % 3 == 0 { Joy::RIGHT } else { Joy::NONE }));
        }
        r.finish(at(), "a note".into()).unwrap()
    }

    #[test]
    fn the_window_offers_level_1_only_and_the_recorder_starts_on() {
        let s = spec();
        assert_eq!(
            s.fields[0].kind,
            FieldKind::Dropdown(vec!["W1-1".to_string()])
        );
        assert_eq!(s.fields[1].label, "Enable recorder?");
        let form = Form::new(s.clone());
        assert_eq!(chosen(&form.values), (LevelId::W1_1, true));
        assert!(FieldAction::named(&s, &["level", "W1-2"]).is_none());
        let mut f = Form::new(s.clone());
        f.apply(FieldAction::named(&s, &["record", "off"]).unwrap());
        let v = f.apply(FieldAction::Start).unwrap().unwrap();
        assert_eq!(chosen(&v), (LevelId::W1_1, false));
    }

    #[test]
    fn a_scripted_window_takes_its_answers_in_order_then_cancels() {
        let spec = spec();
        let mut ui = ScriptedUi::new(
            crate::script::Answers {
                starts: vec![
                    vec![
                        FieldAction::named(&spec, &["record", "off"]).unwrap(),
                        FieldAction::Start,
                    ],
                    vec![FieldAction::Cancel],
                ],
                endings: vec![EndChoice::Save],
                notes: vec![Some("two\nlines".into())],
            },
            None,
        );
        assert_eq!(ui.start_level(), Some((LevelId::W1_1, false)));
        assert_eq!(ui.start_level(), None);
        assert_eq!(ui.start_level(), None, "none left: cancel");
        assert_eq!(ui.recording_in_progress("Quit"), EndChoice::Save);
        assert_eq!(ui.recording_in_progress("Quit"), EndChoice::Cancel);
        let mut saved = Vec::new();
        let mut save = |n: &str| {
            saved.push(n.to_string());
            Ok(())
        };
        assert!(ui.save_recording(b"", &at(), "x.zip", &mut save));
        assert!(!ui.save_recording(b"", &at(), "x.zip", &mut save), "none left: discard");
        assert_eq!(saved, ["two lines"]);
    }

    #[test]
    fn a_save_that_fails_takes_the_next_note() {
        let mut ui = ScriptedUi::new(
            crate::script::Answers {
                notes: vec![Some("a".into()), Some("b".into())],
                ..Default::default()
            },
            None,
        );
        let mut tries = Vec::new();
        let mut save = |n: &str| {
            tries.push(n.to_string());
            if n == "a" { Err("disk full".into()) } else { Ok(()) }
        };
        assert!(ui.save_recording(b"", &at(), "x.zip", &mut save));
        assert_eq!(tries, ["a", "b"]);
    }

    #[test]
    fn a_zip_is_named_for_the_level_and_minute_and_never_overwrites() {
        let d = temp("names");
        let now = (2026, 11, 6, 9, 5);
        let first = zip_name(&d, LevelId::W1_1, now);
        assert_eq!(first, d.join("bug-W1-1-2026-11-06-0905.zip"));
        std::fs::write(&first, b"x").unwrap();
        assert_eq!(
            zip_name(&d, LevelId::W1_1, now),
            d.join("bug-W1-1-2026-11-06-0905-2.zip")
        );
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn the_bugs_folder_is_the_flag_else_home_for_a_script_else_the_checkout_s_else_documents() {
        let d = temp("bugs-dir");
        let home = Some(std::ffi::OsString::from("/Users/u"));
        assert_eq!(bugs_dir(Some(Path::new("/x")), true, &d), PathBuf::from("/x"));
        assert_eq!(bugs_dir(None, true, Path::new("/h")), PathBuf::from("/h/bugs"));
        let lone = d.join("lone/engine/crates/fd-app");
        std::fs::create_dir_all(&lone).unwrap();
        assert_eq!(
            bugs_dir_in(&lone, home.clone()),
            PathBuf::from("/Users/u/Documents/Forest Dumb Bugs")
        );
        let main = d.join("main");
        let built = main.join("games/forest-dumb/engine/crates/fd-app");
        std::fs::create_dir_all(&built).unwrap();
        std::fs::create_dir_all(main.join(".git/worktrees/wt")).unwrap();
        assert_eq!(
            bugs_dir_in(&built, home.clone()),
            main.join("games/forest-dumb/bugs")
        );
        // A worktree: `.git` a file naming its git dir, whose `commondir` is the main one.
        let wt = main.join(".claude/worktrees/wt");
        let wt_built = wt.join("games/forest-dumb/engine/crates/fd-app");
        std::fs::create_dir_all(&wt_built).unwrap();
        let own = main.join(".git/worktrees/wt");
        std::fs::write(wt.join(".git"), format!("gitdir: {}\n", own.display())).unwrap();
        std::fs::write(own.join("commondir"), "../..\n").unwrap();
        let main_real = std::fs::canonicalize(&main).unwrap();
        assert_eq!(
            bugs_dir_in(&wt_built, home),
            main_real.join("games/forest-dumb/bugs")
        );
        std::fs::remove_dir_all(&d).unwrap();
    }

    #[test]
    fn the_zip_holds_the_recording_and_the_screenshot_and_a_failed_write_leaves_nothing() {
        use std::os::unix::fs::PermissionsExt;
        let d = temp("zip");
        let rec = recording(40);
        let path = d.join("bugs/bug.zip");
        write_zip(&path, &rec, b"png bytes").unwrap();
        let zip = std::fs::read(&path).unwrap();
        let entries = amiga_disk::zip::read_entries(&zip, |_| true, 1 << 20).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, [ZIP_RECORDING, ZIP_SCREENSHOT]);
        let text = String::from_utf8(entries[0].bytes.clone()).unwrap();
        assert_eq!(Recording::parse(&text).unwrap(), rec);
        let ro = d.join("readonly");
        std::fs::create_dir_all(&ro).unwrap();
        std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o555)).unwrap();
        assert!(write_zip(&ro.join("x.zip"), &rec, b"png").is_err());
        assert_eq!(std::fs::read_dir(&ro).unwrap().count(), 0, "no half zip");
        std::fs::set_permissions(&ro, std::fs::Permissions::from_mode(0o755)).unwrap();
        let mut bad = recording(3);
        bad.note = "two\nlines".into();
        assert!(write_zip(&d.join("bad.zip"), &bad, b"").is_err());
        assert!(!d.join("bad.zip").exists());
        std::fs::remove_dir_all(&d).unwrap();
    }
}
