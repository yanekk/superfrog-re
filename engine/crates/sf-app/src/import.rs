//! The first-run disk import and the launch decision (DESIGN §2.2, §2.11, §3.4): use the cache,
//! import again from the disks remembered last time, or ask for the disks with the native picker.
//! The decision is T09's [`sf_assets::decide`]; this gathers its facts and acts on its answer.
//!
//! Every question to the player goes through [`Ui`]: the native panels in normal play,
//! [`ScriptedUi`] under `--script`, whose answers the script gives (`script.rs`: `pick`, `answer`)
//! and which logs every prompt, so a scripted run never opens a panel that would wait for a person.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sf_assets::{GameData, LaunchAction, LaunchFacts, cache_decode, cache_encode, decide};
use sf_disk::{DiskSlot, ImportError, Offered};
use winit::window::Window;

use crate::atomic_write::atomic_write;
use crate::home::{build, cache_path};

/// The two buttons of the missing-disks message (owner, 2026-10-10).
pub const CHOOSE: &str = "Choose Disks…";
pub const QUIT: &str = "Quit";
/// The refused level file's other button.
pub const CANCEL: &str = "Cancel";

/// The questions the app asks the player.
pub trait Ui {
    /// The disk picker, showing `message`: the files chosen, `None` when it was cancelled.
    fn pick_disks(&mut self, message: &str) -> Option<Vec<PathBuf>>;
    /// `message` with "Choose Disks…" and "Quit": `true` for Choose Disks.
    fn disks_missing(&mut self, message: &str) -> bool;
    /// `message` with an OK button.
    fn notice(&mut self, message: &str);
    /// The save panel for an exported level, `name` filled in: where to write it, `None` when it
    /// was cancelled (`levelfile.rs`).
    fn save_level(&mut self, name: &str) -> Option<PathBuf>;
    /// The open panel for a level file: the file chosen, `None` when it was cancelled.
    fn open_level(&mut self) -> Option<PathBuf>;
    /// A level file refused, `message` saying why. With `offer` (the disk level's button, `Play
    /// W1-1 from the Disks`), `true` when the player chose it; without, a notice.
    fn level_refused(&mut self, message: &str, offer: Option<&str>) -> bool;
}

/// The macOS panels, through rfd: sheets on `parent` when there is a window (with none, rfd shows
/// a system alert from another process, and the picker stays on screen behind it).
pub struct NativeUi {
    pub parent: Option<Arc<Window>>,
}

impl NativeUi {
    fn message(&self, level: rfd::MessageLevel, text: &str, buttons: rfd::MessageButtons) -> rfd::MessageDialogResult {
        let d = rfd::MessageDialog::new()
            .set_level(level)
            .set_title("Superfrog")
            .set_description(text)
            .set_buttons(buttons);
        match &self.parent {
            Some(w) => d.set_parent(&**w),
            None => d,
        }
        .show()
    }
}

impl Ui for NativeUi {
    fn pick_disks(&mut self, message: &str) -> Option<Vec<PathBuf>> {
        let d = rfd::FileDialog::new()
            .set_title(message)
            .add_filter("Amiga disk images", &["zip", "adf", "ZIP", "ADF"]);
        match &self.parent {
            Some(w) => d.set_parent(&**w),
            None => d,
        }
        .pick_files()
    }

    fn disks_missing(&mut self, message: &str) -> bool {
        let r = self.message(
            rfd::MessageLevel::Warning,
            message,
            rfd::MessageButtons::OkCancelCustom(CHOOSE.into(), QUIT.into()),
        );
        matches!(r, rfd::MessageDialogResult::Custom(ref b) if b == CHOOSE)
            || r == rfd::MessageDialogResult::Ok
    }

    fn notice(&mut self, message: &str) {
        self.message(rfd::MessageLevel::Info, message, rfd::MessageButtons::Ok);
    }

    fn save_level(&mut self, name: &str) -> Option<PathBuf> {
        let d = rfd::FileDialog::new()
            .set_title("Export Level")
            .set_file_name(name)
            .add_filter("Superfrog level", &["sflevel"]);
        match &self.parent {
            Some(w) => d.set_parent(&**w),
            None => d,
        }
        .save_file()
    }

    fn open_level(&mut self) -> Option<PathBuf> {
        let d = rfd::FileDialog::new()
            .set_title("Play Level File")
            .add_filter("Superfrog level", &["sflevel"]);
        match &self.parent {
            Some(w) => d.set_parent(&**w),
            None => d,
        }
        .pick_file()
    }

    fn level_refused(&mut self, message: &str, offer: Option<&str>) -> bool {
        let Some(offer) = offer else {
            self.message(rfd::MessageLevel::Warning, message, rfd::MessageButtons::Ok);
            return false;
        };
        let r = self.message(
            rfd::MessageLevel::Warning,
            message,
            rfd::MessageButtons::OkCancelCustom(offer.into(), CANCEL.into()),
        );
        matches!(r, rfd::MessageDialogResult::Custom(ref b) if b == offer)
            || r == rfd::MessageDialogResult::Ok
    }
}

/// What the script answers the picker with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pick {
    Files(Vec<PathBuf>),
    Cancel,
}

/// The script's answers, in order, and a log of every prompt. With no answer left, every panel
/// is cancelled and the missing-disks message answered Quit, so a run always ends.
#[derive(Debug, Default)]
pub struct ScriptedUi {
    pub picks: VecDeque<Pick>,
    /// `true` for "Choose Disks…".
    pub answers: VecDeque<bool>,
    /// The save panel's answers: a path, or `None` for cancelled.
    pub saves: VecDeque<Option<PathBuf>>,
    /// The level-file open panel's answers.
    pub opens: VecDeque<Option<PathBuf>>,
    /// The refused level file's answers: `true` for playing the disk level.
    pub level_answers: VecDeque<bool>,
    /// Every prompt, one line each: `picker: …`, `missing: … -> choose|quit`, `notice: …`,
    /// `save: <name> -> <path>|cancel`, `open: -> <path>|cancel`, `refused: … -> disk-level|cancel`
    /// (`refused: …` alone when nothing was offered).
    pub log: Vec<String>,
    /// Where each line is appended as it is shown (the capture folder's `prompts.txt`).
    pub log_file: Option<PathBuf>,
}

impl ScriptedUi {
    fn record(&mut self, line: String) {
        let line = line.replace('\n', " ");
        eprintln!("superfrog: {line}");
        if let Some(f) = &self.log_file {
            use std::io::Write;
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(f)
                .and_then(|mut f| writeln!(f, "{line}"));
        }
        self.log.push(line);
    }
}

impl Ui for ScriptedUi {
    fn pick_disks(&mut self, message: &str) -> Option<Vec<PathBuf>> {
        self.record(format!("picker: {message}"));
        match self.picks.pop_front() {
            Some(Pick::Files(f)) => Some(f),
            Some(Pick::Cancel) | None => None,
        }
    }

    fn disks_missing(&mut self, message: &str) -> bool {
        let choose = self.answers.pop_front().unwrap_or(false);
        self.record(format!("missing: {message} -> {}", if choose { "choose" } else { "quit" }));
        choose
    }

    fn notice(&mut self, message: &str) {
        self.record(format!("notice: {message}"));
    }

    fn save_level(&mut self, name: &str) -> Option<PathBuf> {
        let p = self.saves.pop_front().flatten();
        self.record(format!("save: {name} -> {}", shown(&p)));
        p
    }

    fn open_level(&mut self) -> Option<PathBuf> {
        let p = self.opens.pop_front().flatten();
        self.record(format!("open: -> {}", shown(&p)));
        p
    }

    fn level_refused(&mut self, message: &str, offer: Option<&str>) -> bool {
        if offer.is_none() {
            self.record(format!("refused: {message}"));
            return false;
        }
        let play = self.level_answers.pop_front().unwrap_or(false);
        self.record(format!("refused: {message} -> {}", if play { "disk-level" } else { "cancel" }));
        play
    }
}

fn shown(p: &Option<PathBuf>) -> String {
    p.as_ref().map_or("cancel".into(), |p| p.display().to_string())
}

/// `disks.toml`: the disk images the cache was last imported from (DESIGN §3.5).
pub fn disks_path(home: &Path) -> PathBuf {
    home.join("disks.toml")
}

#[derive(Serialize, Deserialize)]
struct Disks {
    disks: Vec<PathBuf>,
}

/// The remembered disk paths; `None` when none are, or the file cannot be read.
fn remembered(home: &Path) -> Option<Vec<PathBuf>> {
    let text = std::fs::read_to_string(disks_path(home)).ok()?;
    toml::from_str::<Disks>(&text).ok().map(|d| d.disks)
}

/// The files at `paths` that can be read, offered under their file names; with the reasons the
/// others could not be read.
fn read_offered(paths: &[PathBuf]) -> (Vec<Offered>, Vec<String>) {
    let mut offered = Vec::new();
    let mut unread = Vec::new();
    for p in paths {
        let name = p.file_name().unwrap_or(p.as_os_str()).to_string_lossy().into_owned();
        match std::fs::read(p) {
            Ok(bytes) => offered.push(Offered { name, bytes }),
            Err(e) => unread.push(format!("“{name}” could not be read: {e}.")),
        }
    }
    (offered, unread)
}

/// Why offered disk images give no game data.
#[derive(Debug)]
enum Unusable {
    /// They are not the four disks.
    Disks(Vec<ImportError>),
    /// They are, and decoding them failed.
    Decode(String),
}

/// Decode identified disk images into the game data.
fn decode(offered: &[Offered]) -> Result<GameData, Unusable> {
    let set = sf_disk::identify(offered).map_err(Unusable::Disks)?;
    let chunks = sf_assets::unpack(&set).map_err(|e| Unusable::Decode(e.to_string()))?;
    sf_assets::decode(&chunks).map_err(|e| Unusable::Decode(e.to_string()))
}

/// `a`, `a and b`, `a, b and c`.
fn and_list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

/// One sentence naming every missing disk (owner, 2026-10-18: not one sentence each); `None`
/// when none is.
fn missing_sentence(slots: &[DiskSlot]) -> Option<String> {
    match slots.len() {
        0 => None,
        1 => Some(ImportError::Missing(slots[0]).to_string()),
        4 => Some(
            "Superfrog needs all four: Disk 1, Disk 2, Disk 3 and Disk 4 of 4, zipped or as .adf \
             files."
                .to_string(),
        ),
        _ => {
            let names: Vec<String> = slots.iter().map(|s| format!("Disk {} of 4", s.number())).collect();
            Some(format!(
                "Superfrog {} are missing. Choose them too, zipped or as .adf files.",
                and_list(&names)
            ))
        }
    }
}

/// What is wrong with the disks offered, for the player: each other problem in its own words,
/// then the missing disks in one sentence (left out when all four are and `four_asked` says the
/// message already asks for four).
fn explain(errors: &[ImportError], four_asked: bool) -> Vec<String> {
    let mut lines: Vec<String> = errors
        .iter()
        .filter(|e| !matches!(e, ImportError::Missing(_)))
        .map(ToString::to_string)
        .collect();
    let missing: Vec<DiskSlot> = errors
        .iter()
        .filter_map(|e| match e {
            ImportError::Missing(s) => Some(*s),
            _ => None,
        })
        .collect();
    if !(four_asked && missing.len() == 4) {
        lines.extend(missing_sentence(&missing));
    }
    lines
}

fn write_cache(home: &Path, data: &GameData) -> Result<(), String> {
    let path = cache_path(home);
    atomic_write(&path, &cache_encode(data, build()))
        .map_err(|e| format!("The game data could not be written to {}: {e}", path.display()))
}

/// The game data for this launch: the cache when it is good; else imported again from the disks
/// remembered last time, silently; else from the disks the player chooses. `Ok(None)` when the
/// player chose Quit (nothing is written then). `cache_from_disks` names a folder whose disk
/// images are imported without asking (the tests' seed import). `Err` is fatal, for the player.
pub fn launch(
    home: &Path,
    cache_from_disks: Option<&Path>,
    ui: &mut dyn Ui,
) -> Result<Option<GameData>, String> {
    let cache = match std::fs::read(cache_path(home)) {
        Ok(bytes) => match cache_decode(&bytes) {
            Ok(data) => return Ok(Some(data)),
            Err(e) => Some(Err(e)),
        },
        Err(_) => None,
    };
    if let Some(dir) = cache_from_disks {
        let data = crate::home::import(dir)?;
        write_cache(home, &data)?;
        return Ok(Some(data));
    }
    let remembered_paths = remembered(home);
    let remembered_disks = remembered_paths.as_deref().map(|p| read_offered(p).0);
    let facts = LaunchFacts {
        cache,
        remembered_disks,
    };
    let reason = match decide(&facts) {
        LaunchAction::UseCache => unreachable!("the cache was not usable"),
        LaunchAction::Reimport { reason } => {
            let offered = facts.remembered_disks.as_deref().unwrap_or_default();
            match decode(offered) {
                Ok(data) => {
                    eprintln!("superfrog: {reason}");
                    write_cache(home, &data)?;
                    return Ok(Some(data));
                }
                Err(Unusable::Decode(why)) => format!("{reason} {why}"),
                Err(Unusable::Disks(errors)) => format!("{reason} {}", explain(&errors, false).join(" ")),
            }
        }
        // The disks remembered are gone or changed: T09's reason lists each problem after asking
        // for the four again; the app words the list as its message does.
        LaunchAction::AskForDisks { reason } => match (
            reason.split_once(" again. "),
            facts.remembered_disks.as_deref().map(sf_disk::identify),
        ) {
            (Some((head, _)), Some(Err(errors))) => {
                let mut text = format!("{head} again.");
                for line in explain(&errors, true) {
                    text.push(' ');
                    text.push_str(&line);
                }
                text
            }
            _ => reason,
        },
    };
    ask(home, ui, reason)
}

/// The picker, until the player chooses four good disks or Quit.
fn ask(home: &Path, ui: &mut dyn Ui, first: String) -> Result<Option<GameData>, String> {
    let mut message = first;
    loop {
        let problems = match ui.pick_disks(&message) {
            None => format!(
                "No disk images were chosen. {}",
                missing_sentence(&DiskSlot::ALL).expect("four are missing")
            ),
            Some(paths) => match take(home, &paths)? {
                Ok(data) => return Ok(Some(data)),
                Err(problems) => problems,
            },
        };
        if !ui.disks_missing(&problems) {
            return Ok(None);
        }
        message = CHOOSE_FOUR.to_string();
    }
}

const CHOOSE_FOUR: &str = "Choose your four Superfrog disk images, zipped or as .adf files.";

/// File → Import Disks… (`menu.rs`): the picker once. Four good disks replace the cache and are
/// remembered; `Ok(None)` when it was cancelled, or when the disks cannot be used, which a message
/// then says (the game carries on: nothing is replaced).
pub fn choose(home: &Path, ui: &mut dyn Ui) -> Result<Option<GameData>, String> {
    let Some(paths) = ui.pick_disks(CHOOSE_FOUR) else { return Ok(None) };
    match take(home, &paths)? {
        Ok(data) => Ok(Some(data)),
        Err(problems) => {
            ui.notice(&problems);
            Ok(None)
        }
    }
}

/// The disk images at `paths`: their game data, written to the cache and the paths remembered;
/// `Ok(Err(..))` says what is wrong with them, for the player; `Err` is fatal.
fn take(home: &Path, paths: &[PathBuf]) -> Result<Result<GameData, String>, String> {
    let (offered, unread) = read_offered(paths);
    Ok(Err(match decode(&offered) {
        Ok(data) if unread.is_empty() => {
            write_cache(home, &data)?;
            let paths = paths.iter().map(|p| std::path::absolute(p).unwrap_or(p.clone()));
            let text = toml::to_string(&Disks {
                disks: paths.collect(),
            })
            .expect("disk paths serialise");
            // Losing the remembered paths only costs a question next time.
            if let Err(e) = atomic_write(&disks_path(home), text.as_bytes()) {
                eprintln!("superfrog: could not remember the disks: {e}");
            }
            return Ok(Ok(data));
        }
        Ok(_) => format!("These disk images cannot be used:\n{}", unread.join("\n")),
        Err(Unusable::Decode(why)) => {
            format!("These disk images cannot be used: {why}")
        }
        // Only disks missing: say which; anything else wrong comes first.
        Err(Unusable::Disks(errors))
            if unread.is_empty() && errors.iter().all(|e| matches!(e, ImportError::Missing(_))) =>
        {
            explain(&errors, false).join("\n")
        }
        Err(Unusable::Disks(errors)) => {
            let all: Vec<String> = unread.into_iter().chain(explain(&errors, false)).collect();
            format!("These disk images cannot be used:\n{}", all.join("\n"))
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_missing_disks_are_named_in_one_sentence() {
        use DiskSlot::*;
        assert_eq!(missing_sentence(&[]), None);
        assert_eq!(
            missing_sentence(&[D3]).unwrap(),
            "Superfrog Disk 3 of 4 is missing. Choose its disk image too, zipped or as an .adf file."
        );
        assert_eq!(
            missing_sentence(&[D3, D4]).unwrap(),
            "Superfrog Disk 3 of 4 and Disk 4 of 4 are missing. Choose them too, zipped or as .adf \
             files."
        );
        assert_eq!(
            missing_sentence(&[D1, D2, D4]).unwrap(),
            "Superfrog Disk 1 of 4, Disk 2 of 4 and Disk 4 of 4 are missing. Choose them too, zipped \
             or as .adf files."
        );
        assert!(missing_sentence(&DiskSlot::ALL).unwrap().starts_with("Superfrog needs all four"));
        // Other problems keep their own words, before the missing disks.
        let lines = explain(&[ImportError::Missing(D2), ImportError::Duplicate(D1)], false);
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("Disk 1 of 4 was chosen more than once"), "{lines:?}");
        assert!(lines[1].contains("Disk 2 of 4 is missing"), "{lines:?}");
        assert!(explain(&DiskSlot::ALL.map(ImportError::Missing), true).is_empty());
    }

    fn home(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sf-app-import-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn a_cancelled_picker_names_all_four_disks_and_quit_writes_nothing() {
        let h = home("cancel");
        let mut ui = ScriptedUi::default();
        assert_eq!(launch(&h, None, &mut ui), Ok(None));
        assert_eq!(ui.log.len(), 2, "{:?}", ui.log);
        assert!(ui.log[0].starts_with("picker: The game data has not been imported yet"), "{:?}", ui.log);
        assert_eq!(
            ui.log[1],
            "missing: No disk images were chosen. Superfrog needs all four: Disk 1, Disk 2, Disk 3 \
             and Disk 4 of 4, zipped or as .adf files. -> quit"
        );
        assert!(!h.exists(), "Quit wrote into the home");
    }

    #[test]
    fn choose_disks_reopens_the_picker() {
        let h = home("again");
        let mut ui = ScriptedUi {
            answers: [true, false].into(),
            ..Default::default()
        };
        assert_eq!(launch(&h, None, &mut ui), Ok(None));
        let kinds: Vec<&str> = ui.log.iter().map(|l| l.split(':').next().unwrap()).collect();
        assert_eq!(kinds, ["picker", "missing", "picker", "missing"]);
        assert!(ui.log[2].contains("Choose your four Superfrog disk images"));
    }

    #[test]
    fn import_disks_cancelled_or_with_unusable_disks_replaces_nothing() {
        let h = home("choose");
        std::fs::create_dir_all(&h).unwrap();
        let mut ui = ScriptedUi {
            picks: [Pick::Cancel, Pick::Files(vec![h.join("gone.adf")])].into(),
            ..Default::default()
        };
        assert_eq!(choose(&h, &mut ui), Ok(None));
        assert_eq!(ui.log, [format!("picker: {CHOOSE_FOUR}")]);
        assert_eq!(choose(&h, &mut ui), Ok(None));
        assert_eq!(ui.log.len(), 3, "{:?}", ui.log);
        assert!(ui.log[2].starts_with("notice: These disk images cannot be used"), "{:?}", ui.log);
        assert!(!cache_path(&h).exists() && !disks_path(&h).exists());
        std::fs::remove_dir_all(&h).unwrap();
    }

    #[test]
    fn a_file_that_is_no_disk_and_one_that_is_gone_are_both_named() {
        let h = home("junk");
        std::fs::create_dir_all(&h).unwrap();
        let junk = h.join("junk.zip");
        std::fs::write(&junk, b"not a zip").unwrap();
        let mut ui = ScriptedUi {
            picks: [Pick::Files(vec![junk, h.join("gone.adf")])].into(),
            ..Default::default()
        };
        assert_eq!(launch(&h, None, &mut ui), Ok(None));
        let m = &ui.log[1];
        assert!(m.contains("“gone.adf” could not be read"), "{m}");
        assert!(m.contains("“junk.zip” could not be read as a disk image"), "{m}");
        assert!(m.contains("Superfrog needs all four"), "{m}");
        assert!(!cache_path(&h).exists());
        std::fs::remove_dir_all(&h).unwrap();
    }
}
