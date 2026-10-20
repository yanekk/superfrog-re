//! File → Export Level and Play Level File… (DESIGN §2.10, §2.11): a disk level written to a
//! `.sflevel` where the player chooses, and a `.sflevel` played in place of the level it names.
//! The format is `sf_assets::sflevel` (docs/14).
//!
//! A file that cannot be played is refused with its reason; the disk level it names is played
//! instead only when the player chooses that in the message.

use std::path::Path;

use sf_assets::{LevelData, LevelId, SflevelError, sflevel_decode, sflevel_encode, sflevel_replaces};
use sf_core::{Game, Start};
use sf_paula::Paula;

use crate::App;
use crate::atomic_write::atomic_write;

/// The name the save panel suggests.
pub fn export_name(id: LevelId) -> String {
    format!("{id}.sflevel")
}

/// The button that plays the disk level in place of a refused file.
pub fn disk_level_button(id: LevelId) -> String {
    format!("Play {id} from the Disks")
}

/// Why a level file cannot be played, as the message says it.
pub fn refusal(file: &Path, why: &str) -> String {
    let name = file.file_name().unwrap_or(file.as_os_str()).to_string_lossy();
    format!("“{name}” cannot be played: {why}.")
}

/// The level in `text`, checked against the levels the disks hold: `Err` with the reason it
/// cannot be played, and the disk level to offer instead when the file names one.
pub fn read_level(
    text: &str,
    disk: &std::collections::BTreeMap<LevelId, LevelData>,
) -> Result<LevelData, (SflevelError, Option<LevelId>)> {
    let offer = sflevel_replaces(text).filter(|id| disk.contains_key(id));
    match sflevel_decode(text) {
        Ok(l) if disk.contains_key(&l.id) => Ok(l),
        Ok(l) => Err((SflevelError::UnknownLevel(l.id.to_string()), None)),
        Err(e) => Err((e, offer)),
    }
}

impl App {
    /// File → Export Level ▸ `id`: the save panel, then the disk level written where the player
    /// chose, and nowhere else. Cancelled, nothing is written.
    pub(crate) fn export_level(&mut self, id: LevelId) {
        let Some(level) = self.run.as_ref().and_then(|r| r.levels.get(&id)).cloned() else {
            eprintln!("superfrog: {id} is not on these disks");
            return;
        };
        let path = self.ui.save_level(&export_name(id));
        self.after_panel();
        let Some(path) = path else { return };
        match atomic_write(&path, sflevel_encode(&level).as_bytes()) {
            Ok(()) => eprintln!("superfrog: exported {id} to {}", path.display()),
            Err(e) => {
                let m = format!("{id} could not be exported to “{}”: {e}.", path.display());
                eprintln!("superfrog: {m}");
                self.ui.notice(&m);
            }
        }
    }

    /// File → Play Level File…: the open panel, then the file's level played from its start in
    /// place of the disk level it names. Refused, the game carries on, or the disk level starts
    /// if the player chooses it.
    pub(crate) fn play_level_file(&mut self) {
        let path = self.ui.open_level();
        self.after_panel();
        let Some(path) = path else { return };
        let read = std::fs::read(&path).map_err(|e| e.to_string()).and_then(|bytes| {
            String::from_utf8(bytes).map_err(|_| SflevelError::NotJson.to_string())
        });
        let text = match read {
            Ok(t) => t,
            Err(why) => {
                let m = refusal(&path, &why);
                eprintln!("superfrog: {m}");
                self.ui.level_refused(&m, None);
                return self.after_panel();
            }
        };
        let run = self.run.as_ref().expect("running");
        match read_level(&text, &run.levels) {
            Ok(level) => {
                eprintln!("superfrog: playing {} from {}", level.id, path.display());
                let run = self.run.as_mut().expect("running");
                run.game = Game::with_override(run.display.clone(), level);
                self.restarted();
            }
            Err((why, offer)) => {
                let m = refusal(&path, &why.to_string());
                eprintln!("superfrog: {m}");
                let button = offer.map(disk_level_button);
                let play = self.ui.level_refused(&m, button.as_deref());
                self.after_panel();
                if let (true, Some(id)) = (play, offer) {
                    eprintln!("superfrog: playing {id} from the disks");
                    let run = self.run.as_mut().expect("running");
                    run.game = Game::with_display(run.display.clone(), Start::Level(id));
                    self.restarted();
                }
            }
        }
    }

    /// A new game in place: drawn for the widescreen setting, a new sound chip, not paused.
    fn restarted(&mut self) {
        let run = self.run.as_mut().expect("running");
        run.game.set_view(crate::window::view(run.settings.widescreen));
        run.paula = Paula::booted(run.rate);
        run.paused = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_messages_name_the_file_the_reason_and_the_level() {
        let w = "W1-1".parse().unwrap();
        assert_eq!(export_name(w), "W1-1.sflevel");
        assert_eq!(disk_level_button(w), "Play W1-1 from the Disks");
        assert_eq!(
            refusal(Path::new("/x/My Level.sflevel"), &SflevelError::Version(2).to_string()),
            "“My Level.sflevel” cannot be played: it is a version 2 level file; this Superfrog \
             reads version 1."
        );
    }

    #[test]
    fn a_file_for_a_level_the_disks_lack_is_refused_and_offers_nothing() {
        let id: LevelId = "W1-1".parse().unwrap();
        let level = LevelData {
            id,
            chunk: sf_assets::ChunkId::new(sf_disk::DiskSlot::D1, 21, 0),
            t7mp: sf_assets::t7mp_decode(&tiny_map(), sf_assets::ChunkId::new(sf_disk::DiskSlot::D1, 0, 0), "t")
                .unwrap(),
            extra: Default::default(),
        };
        let text = sflevel_encode(&level);
        let none = Default::default();
        assert_eq!(
            read_level(&text, &none).map(|_| ()),
            Err((SflevelError::UnknownLevel("W1-1".into()), None))
        );
        let disk = [(id, level.clone())].into_iter().collect();
        assert_eq!(read_level(&text, &disk), Ok(level));
        let v2 = text.replace("\"version\": 1", "\"version\": 2");
        assert_eq!(read_level(&v2, &disk).map(|_| ()), Err((SflevelError::Version(2), Some(id))));
        assert_eq!(read_level(&v2, &none).map(|_| ()), Err((SflevelError::Version(2), None)));
    }

    /// A 300×84 T7MP of empty cells.
    fn tiny_map() -> Vec<u8> {
        let mut b = Vec::new();
        let tag = |b: &mut Vec<u8>, t: &[u8], p: &[u8]| {
            b.extend_from_slice(t);
            b.extend_from_slice(&(p.len() as u32).to_be_bytes());
            b.extend_from_slice(p);
        };
        let body = vec![0; 300 * 84 * 2];
        let mut rest = Vec::new();
        tag(&mut rest, b"XBLK", &300u32.to_be_bytes());
        tag(&mut rest, b"YBLK", &84u32.to_be_bytes());
        tag(&mut rest, b"BODY", &body);
        tag(&mut b, b"T7MP", &((rest.len() + 12) as u32).to_be_bytes());
        b.extend(rest);
        b
    }
}
