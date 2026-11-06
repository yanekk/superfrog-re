//! A bug recording (DESIGN §2.8): the level started from the Start Level window, every engine
//! step's input from the level's frame 0, where it stopped and the owner's note, as readable text.
//! Pure, so the app writes it and `fd-conform bug replay` reads it with one parser.
//!
//! ```text
//! forest-dumb recording 1
//! build forest-dumb 0.1.0 (0123456789abcdef)
//! level W1-1
//! frames 18342
//! stopped-at W1-1:42,35
//! note the character fell through the floor
//! inputs
//! 0: -
//! 112: R
//! 130: R F
//! ```
//!
//! The recording, its text form's shared rules, the recorder and the zip are the toolkit's
//! (`amiga_record`); this file is Forest Dumb's header in it ([`Header`]). The header is fixed:
//! every field, in this order, one per line. There is no `seed` line: the engine holds no random
//! state, so a level start is the same every time (DESIGN §2.3) and the seed is always empty.
//!
//! Replay refuses a recording from another build or level and names the difference (DESIGN §2.12):
//! [`check_build`] and the reader's `level` line.

use core::fmt;

use amiga_chips::Picture;
use amiga_input::FrameInput;
use amiga_record::{GameRecording, Head, HeaderReader, HeaderWriter, Replayable};

use crate::Game;

pub use amiga_record::{MAX_FRAMES, RecordingError, ZIP_RECORDING, ZIP_SCREENSHOT};

/// One recording: the toolkit's, with Forest Dumb's [`Header`]. It reads as its header too
/// (`rec.level`, `rec.stopped_at`).
pub type Recording = amiga_record::Recording<ForestDumb>;

/// The levels the Start Level window offers (DESIGN §2.7): level 1 only, until later plans build
/// more.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LevelId {
    #[default]
    W1_1,
}

impl LevelId {
    pub const ALL: [LevelId; 1] = [LevelId::W1_1];

    /// Its name, as a spot is written (`games/forest-dumb/docs/LEVEL-INDEX.md`).
    pub fn name(self) -> &'static str {
        match self {
            LevelId::W1_1 => "W1-1",
        }
    }
}

impl fmt::Display for LevelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl core::str::FromStr for LevelId {
    type Err = String;
    fn from_str(s: &str) -> Result<LevelId, String> {
        LevelId::ALL
            .into_iter()
            .find(|l| l.name() == s)
            .ok_or_else(|| {
                let names: Vec<&str> = LevelId::ALL.iter().map(|l| l.name()).collect();
                format!(
                    "level `{s}` is not one this engine starts ({})",
                    names.join(", ")
                )
            })
    }
}

/// Where play is: the character's cell, `<level-id>:<col>,<row>` (16-pixel cells, top-left
/// origin, as `games/forest-dumb/CLAUDE.md` names a spot).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Location {
    pub level: LevelId,
    pub col: u16,
    pub row: u16,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{},{}", self.level, self.col, self.row)
    }
}

impl core::str::FromStr for Location {
    type Err = String;

    /// What [`Location`]'s `Display` prints: `W1-1:42,35`.
    fn from_str(s: &str) -> Result<Location, String> {
        let bad = || format!("`{s}` is not a place: a level cell like W1-1:42,35");
        let (level, cell) = s.split_once(':').ok_or_else(bad)?;
        let level: LevelId = level.parse()?;
        let (col, row) = cell.split_once(',').ok_or_else(bad)?;
        Ok(Location {
            level,
            col: number(col).ok_or_else(bad)?,
            row: number(row).ok_or_else(bad)?,
        })
    }
}

/// The character's offset to the point a spot names, in pixels: the cell under `char_x + $e`
/// (the column `h0_24ce` and the item checks read, docs/10-object-system.md) and `char_y + 9`.
/// Checked against both spots docs/10 measured: the start, (`$70`, `$70`), is `W1-1:7,7`, and the
/// slide's end, (669, 553), is `W1-1:42,35`.
const SPOT_DX: u32 = 0xe;
const SPOT_DY: u32 = 9;

impl Location {
    /// The cell a character at (`char_x`, `char_y`) of `level` is at.
    pub fn of_character(level: LevelId, char_x: u32, char_y: u32) -> Location {
        Location {
            level,
            col: ((char_x + SPOT_DX) / 16) as u16,
            row: ((char_y + SPOT_DY) / 16) as u16,
        }
    }
}

impl Game {
    /// Where play is ([`Location`]): the cell the character is at.
    pub fn location(&self) -> Location {
        let s = self.state();
        Location::of_character(LevelId::W1_1, s.char_x, s.char_y)
    }
}

/// A recording is replayed on the game as the app plays it: one [`Game::step`] a frame, the
/// events it raises taken (the app logs them; they change nothing), and the picture after it.
impl Replayable for Game {
    fn picture(&self) -> Picture {
        Game::picture(self)
    }

    fn step(&mut self, _frame: u32, input: FrameInput) -> Picture {
        Game::step(self, input);
        self.take_events();
        Game::picture(self)
    }
}

/// Decimal digits only (`parse` alone would take a leading `+`).
fn number<T: core::str::FromStr>(s: &str) -> Option<T> {
    (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .then(|| s.parse().ok())
        .flatten()
}

/// Forest Dumb's part of a recording ([`GameRecording`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForestDumb {}

/// Forest Dumb's header: the level started, and where it stopped.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    pub level: LevelId,
    /// Where Stop was pressed; `None` (written `-`) before it was.
    pub stopped_at: Option<Location>,
}

impl GameRecording for ForestDumb {
    type Header = Header;
    type Stop = Location;
    const MAGIC: &'static str = "forest-dumb recording";
    const VERSION: &'static str = "1";

    fn write_header(rec: &Recording, w: &mut HeaderWriter) {
        w.build(&rec.build);
        w.field("level", rec.level.name());
        w.frames(rec.inputs.len());
        match &rec.stopped_at {
            Some(at) => w.field("stopped-at", &at.to_string()),
            None => w.field("stopped-at", "-"),
        }
        w.note(&rec.note);
    }

    fn read_header(
        n: usize,
        version: &str,
        r: &mut HeaderReader<'_>,
    ) -> Result<Head<ForestDumb>, RecordingError> {
        let err = HeaderReader::error;
        if version != Self::VERSION {
            return Err(err(
                n,
                format!(
                    "recording format `{version}` is not one this engine reads ({})",
                    Self::VERSION
                ),
            ));
        }
        let build = r.build()?;
        let (n, level) = r.next("level")?;
        let level: LevelId = level.parse().map_err(|e| err(n, e))?;
        let frames = r.frames()?;
        let (n, stopped) = r.next("stopped-at")?;
        let stopped_at = match stopped {
            "-" => None,
            s => Some(s.parse().map_err(|e| err(n, e))?),
        };
        let note = r.note()?;
        Ok(Head {
            build,
            seed: Vec::new(),
            frames,
            note,
            header: Header { level, stopped_at },
        })
    }

    /// No seed: the engine has no random state to start from.
    fn validate(rec: &Recording) -> Result<(), String> {
        match rec.seed.is_empty() {
            true => Ok(()),
            false => Err(format!(
                "a Forest Dumb recording has no seed, and this one has {} bytes",
                rec.seed.len()
            )),
        }
    }

    fn stop(rec: &mut Recording, at: Location) {
        rec.stopped_at = Some(at);
    }
}

/// A recording of `level` from its start, by `build`, nothing stepped yet.
pub fn recorder(level: LevelId, build: &str) -> amiga_record::Recorder<ForestDumb> {
    amiga_record::Recorder::start(
        Header {
            level,
            stopped_at: None,
        },
        Vec::new(),
        build,
    )
}

/// `Err` naming both builds unless `rec` was made by `build` (DESIGN §2.12): another build's
/// engine may step the same inputs differently, so its replay would say nothing about this one.
pub fn check_build(rec: &Recording, build: &str) -> Result<(), String> {
    match rec.build == build {
        true => Ok(()),
        false => Err(format!(
            "recorded by engine build `{}`, and this is `{build}`: replay it with that build, or record it again",
            rec.build
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amiga_input::{Joy, KeySet};

    fn full() -> Recording {
        let kinds = [
            FrameInput::NONE,
            FrameInput::joy(Joy::RIGHT),
            FrameInput::joy(Joy::RIGHT | Joy::FIRE),
            FrameInput::joy(Joy::UP | Joy::LEFT),
            FrameInput {
                joy: Joy::LEFT,
                keys: KeySet::key(0x19),
            },
        ];
        let mut r = recorder(LevelId::W1_1, "forest-dumb 0.1.0 (0123456789abcdef)");
        for f in 0..10_000u32 {
            r.step(kinds[(f / 37 % kinds.len() as u32) as usize]);
        }
        r.finish(
            Location {
                level: LevelId::W1_1,
                col: 42,
                row: 35,
            },
            "fell through the floor # not a comment".into(),
        )
        .unwrap()
    }

    #[test]
    fn a_recording_reads_back_as_written() {
        let rec = full();
        assert!(rec.validate().is_ok());
        let text = rec.to_text();
        assert!(
            text.starts_with(
                "forest-dumb recording 1\nbuild forest-dumb 0.1.0 (0123456789abcdef)\nlevel W1-1\n\
                 frames 10000\nstopped-at W1-1:42,35\nnote fell through the floor # not a comment\n\
                 inputs\n0: -\n37: R\n"
            ),
            "{text}"
        );
        assert_eq!(Recording::parse(&text).unwrap(), rec);
        let zip = amiga_record::zip_bytes(&rec, b"png").unwrap();
        assert_eq!(
            amiga_record::read_zip::<ForestDumb>(&zip).unwrap(),
            (rec, b"png".to_vec())
        );
    }

    #[test]
    fn an_empty_recording_and_an_empty_note_read_back() {
        let rec = recorder(LevelId::W1_1, "b")
            .finish(
                Location {
                    level: LevelId::W1_1,
                    col: 7,
                    row: 7,
                },
                String::new(),
            )
            .unwrap();
        assert_eq!(rec.inputs.len(), 0);
        let text = rec.to_text();
        assert_eq!(
            text,
            "forest-dumb recording 1\nbuild b\nlevel W1-1\nframes 0\nstopped-at W1-1:7,7\nnote\ninputs\n"
        );
        assert_eq!(Recording::parse(&text).unwrap(), rec);
    }

    #[test]
    fn another_version_or_level_is_refused_naming_it() {
        let text = full().to_text();
        let v2 = text.replacen("recording 1", "recording 2", 1);
        let e = Recording::parse(&v2).unwrap_err();
        assert_eq!(e.line, 1);
        assert!(
            e.why
                .contains("format `2` is not one this engine reads (1)"),
            "{e}"
        );
        let w12 = text.replacen("level W1-1", "level W1-2", 1);
        let e = Recording::parse(&w12).unwrap_err();
        assert_eq!(e.line, 3);
        assert!(
            e.why
                .contains("level `W1-2` is not one this engine starts (W1-1)"),
            "{e}"
        );
        let place = text.replacen("stopped-at W1-1:42,35", "stopped-at W1-2:1,1", 1);
        assert!(Recording::parse(&place).unwrap_err().why.contains("W1-2"));
        let magic = text.replacen("forest-dumb", "superfrog", 1);
        assert!(Recording::parse(&magic).is_err());
    }

    #[test]
    fn another_build_is_refused_naming_both() {
        let rec = full();
        assert_eq!(
            check_build(&rec, "forest-dumb 0.1.0 (0123456789abcdef)"),
            Ok(())
        );
        let e = check_build(&rec, "forest-dumb 0.1.0 (fedcba9876543210)").unwrap_err();
        assert!(
            e.contains("`forest-dumb 0.1.0 (0123456789abcdef)`")
                && e.contains("`forest-dumb 0.1.0 (fedcba9876543210)`"),
            "{e}"
        );
    }

    #[test]
    fn a_seed_is_refused() {
        let mut rec = full();
        rec.seed = vec![1, 2];
        assert!(rec.validate().unwrap_err().contains("no seed"));
    }

    /// Both spots docs/10-object-system.md measured: the start and the slide's end. The slide's end
    /// is where each offset decides the cell (669 / 16 is 41, 553 / 16 is 34).
    #[test]
    fn the_character_s_cell_is_the_one_docs_10_measured() {
        let at = |x, y| Location::of_character(LevelId::W1_1, x, y).to_string();
        assert_eq!(at(0x70, 0x70), "W1-1:7,7");
        assert_eq!(at(669, 553), "W1-1:42,35");
    }

    #[test]
    fn a_place_reads_back_and_a_bad_one_says_why() {
        let at: Location = "W1-1:42,35".parse().unwrap();
        assert_eq!((at.col, at.row), (42, 35));
        assert_eq!(at.to_string(), "W1-1:42,35");
        for bad in ["W1-1", "W1-1:4", "W1-1:+4,5", "W1-1:a,5", "title"] {
            assert!(bad.parse::<Location>().is_err(), "{bad}");
        }
    }
}
