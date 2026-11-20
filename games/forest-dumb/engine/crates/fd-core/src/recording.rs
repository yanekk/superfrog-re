//! A bug recording (DESIGN §2.8; translator DESIGN §2.10): where play started, the engine, every
//! engine step's input from the start's frame 0, where it stopped, the translated program's stop
//! and the owner's note, as readable text. Pure, so the app writes it and `fd-conform bug replay`
//! reads it with one parser.
//!
//! ```text
//! forest-dumb recording 2
//! build forest-dumb 0.1.0 (0123456789abcdef)
//! start level-1
//! engine translated
//! level W1-1
//! frames 18342
//! stopped-at W1-1:42,35
//! stop untranslated: h0+$1a2b4
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
//! state, so a start is the same every time (DESIGN §2.3) and the seed is always empty.
//!
//! `start` is where play started (translator T39): `level-1`, level 1's first frame (the hand
//! engine's `Start::Level1`, or the translated program run from power-on through the title on
//! [`to_level_1_input`] until [`in_level_1`]), or `power-on`, the translated program's first frame.
//! `engine` is which engine played. `stopped-at` is `-` when play was in no level (the title).
//! `stop` is how the translated program stopped, `-` when it did not: a recording the app saved
//! by itself on a stop (translator DESIGN §2.4, §2.10) ends on the stop's frame. Version 1, before
//! T39, had none of `start`, `engine` and `stop`: it reads as `level-1` on the hand engine, no stop.
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

/// Where play started ([`Header::start`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum StartPoint {
    /// Level 1's first frame.
    #[default]
    Level1,
    /// The translated program's first frame, from its entry (`fd-app` with no `--level`).
    PowerOn,
}

impl StartPoint {
    pub fn name(self) -> &'static str {
        match self {
            StartPoint::Level1 => "level-1",
            StartPoint::PowerOn => "power-on",
        }
    }
}

impl core::str::FromStr for StartPoint {
    type Err = String;
    fn from_str(s: &str) -> Result<StartPoint, String> {
        match s {
            "level-1" => Ok(StartPoint::Level1),
            "power-on" => Ok(StartPoint::PowerOn),
            _ => Err(format!("start `{s}` is not one a recording names (level-1, power-on)")),
        }
    }
}

/// Which engine played ([`Header::engine`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EngineKind {
    /// Level 1's hand engine alone (`fd-app --engine hand`, and every version 1 recording).
    #[default]
    Hand,
    /// The translated program, level 1's hand engine mounted through the seam.
    Translated,
}

impl EngineKind {
    pub fn name(self) -> &'static str {
        match self {
            EngineKind::Hand => "hand",
            EngineKind::Translated => "translated",
        }
    }
}

impl core::str::FromStr for EngineKind {
    type Err = String;
    fn from_str(s: &str) -> Result<EngineKind, String> {
        match s {
            "hand" => Ok(EngineKind::Hand),
            "translated" => Ok(EngineKind::Translated),
            _ => Err(format!("engine `{s}` is not one a recording names (hand, translated)")),
        }
    }
}

/// Forest Dumb's header: where play started and on which engine, the level, where it stopped,
/// and the translated program's stop.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Header {
    pub start: StartPoint,
    pub engine: EngineKind,
    pub level: LevelId,
    /// Where play was when the recording stopped; `None` (written `-`) before it stopped, or when
    /// play was in no level (the translated program's title).
    pub stopped_at: Option<Location>,
    /// How the translated program stopped (`pcode_rt::Stop`'s text: `untranslated: h0+$1a2b4`),
    /// on the recording's last frame; `None` (written `-`) when it did not.
    pub stop: Option<String>,
}

/// The `stop` line's text for a stop: one line, never `-` alone.
pub fn stop_text(stop: &str) -> String {
    let s = stop.replace(['\n', '\r'], " ");
    match s.as_str() {
        "" | "-" => format!("({s})"),
        _ => s,
    }
}

/// What `fd-app --stop-after N` stops with (translator T39): a test's stop, raised at step N
/// with no defect behind it. A replay raises it again at the recording's end
/// ([`is_test_stop`]), as the app did.
pub fn test_stop(step: u64) -> String {
    format!("{TEST_STOP} --stop-after {step}")
}

const TEST_STOP: &str = "test-stop:";

/// Is `stop` [`test_stop`]'s?
pub fn is_test_stop(stop: &str) -> bool {
    stop.starts_with(TEST_STOP)
}

impl GameRecording for ForestDumb {
    type Header = Header;
    type Stop = Option<Location>;
    const MAGIC: &'static str = "forest-dumb recording";
    const VERSION: &'static str = "2";

    fn write_header(rec: &Recording, w: &mut HeaderWriter) {
        w.build(&rec.build);
        w.field("start", rec.start.name());
        w.field("engine", rec.engine.name());
        w.field("level", rec.level.name());
        w.frames(rec.inputs.len());
        match &rec.stopped_at {
            Some(at) => w.field("stopped-at", &at.to_string()),
            None => w.field("stopped-at", "-"),
        }
        match &rec.stop {
            Some(s) => w.field("stop", &stop_text(s)),
            None => w.field("stop", "-"),
        }
        w.note(&rec.note);
    }

    fn read_header(
        n: usize,
        version: &str,
        r: &mut HeaderReader<'_>,
    ) -> Result<Head<ForestDumb>, RecordingError> {
        let err = HeaderReader::error;
        // Version 1 (before translator T39): level 1 on the hand engine, no stop lines.
        let v1 = match version {
            "1" => true,
            v if v == Self::VERSION => false,
            _ => {
                return Err(err(
                    n,
                    format!(
                        "recording format `{version}` is not one this engine reads (1, {})",
                        Self::VERSION
                    ),
                ));
            }
        };
        let build = r.build()?;
        let (start, engine) = match v1 {
            true => (StartPoint::Level1, EngineKind::Hand),
            false => {
                let (n, start) = r.next("start")?;
                let start: StartPoint = start.parse().map_err(|e| err(n, e))?;
                let (n, engine) = r.next("engine")?;
                let engine: EngineKind = engine.parse().map_err(|e| err(n, e))?;
                if (start, engine) == (StartPoint::PowerOn, EngineKind::Hand) {
                    return Err(err(
                        n,
                        "the hand engine starts at level-1 only, never power-on".into(),
                    ));
                }
                (start, engine)
            }
        };
        let (n, level) = r.next("level")?;
        let level: LevelId = level.parse().map_err(|e| err(n, e))?;
        let frames = r.frames()?;
        let (n, stopped) = r.next("stopped-at")?;
        let stopped_at = match stopped {
            "-" => None,
            s => Some(s.parse().map_err(|e| err(n, e))?),
        };
        let stop = match v1 {
            true => None,
            false => match r.next("stop")? {
                (_, "-") => None,
                (n, "") => return Err(err(n, "stop is empty: a stop's text, or `-`".into())),
                (_, s) => Some(s.to_string()),
            },
        };
        let note = r.note()?;
        Ok(Head {
            build,
            seed: Vec::new(),
            frames,
            note,
            header: Header {
                start,
                engine,
                level,
                stopped_at,
                stop,
            },
        })
    }

    /// No seed: the engine has no random state to start from.
    fn validate(rec: &Recording) -> Result<(), String> {
        if !rec.seed.is_empty() {
            return Err(format!(
                "a Forest Dumb recording has no seed, and this one has {} bytes",
                rec.seed.len()
            ));
        }
        if (rec.start, rec.engine) == (StartPoint::PowerOn, EngineKind::Hand) {
            return Err("the hand engine starts at level-1 only, never power-on".into());
        }
        Ok(())
    }

    fn stop(rec: &mut Recording, at: Option<Location>) {
        rec.stopped_at = at;
    }
}

/// A recording of `level` from its start on the hand engine, by `build`, nothing stepped yet.
pub fn recorder(level: LevelId, build: &str) -> amiga_record::Recorder<ForestDumb> {
    recorder_from(StartPoint::Level1, EngineKind::Hand, level, build)
}

/// A recording from `start` on `engine`, by `build`, nothing stepped yet.
pub fn recorder_from(
    start: StartPoint,
    engine: EngineKind,
    level: LevelId,
    build: &str,
) -> amiga_record::Recorder<ForestDumb> {
    amiga_record::Recorder::start(
        Header {
            start,
            engine,
            level,
            stopped_at: None,
            stop: None,
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

/// The translated program's level-1 start (translator DESIGN §2.10): from power-on, fire pressed
/// for ten frames and released for ten, over and over. It passes the disk 2 prompt, presses START
/// on the title and fire on the SCENA 1 screen; frame `n` is the `n`th the program runs, from 0.
/// `fd-app --level 1`, its Start Level… and `fd-conform bug replay` all start this one way.
pub fn to_level_1_input(n: u64) -> FrameInput {
    let mut i = FrameInput::default();
    if n / 10 % 2 == 1 {
        i.joy = amiga_input::Joy::FIRE;
    }
    i
}

/// The level-1 start's seatbelt: the frames the program may take to reach play from power-on (it
/// takes about 500 on [`to_level_1_input`], translator T37).
pub const TO_LEVEL_1_FRAMES: u64 = 3_000;

/// Where slow RAM starts in the A500's address space.
const SLOW_BASE: usize = 0xC0_0000;

/// A variable of [`crate::LAYOUT`] read out of chip RAM and slow RAM at a frame's end.
fn ram_var(chip: &[u8], slow: &[u8], name: &str) -> u32 {
    let v = crate::layout_var(name).expect("in LAYOUT");
    let at = crate::HUNK_BASES[v.hunk as usize] + v.offset;
    (0..v.width.bytes() as u32).fold(0u32, |acc, k| {
        let a = ((at + k) & 0xFF_FFFF) as usize;
        let b = match a.checked_sub(SLOW_BASE) {
            Some(i) => slow.get(i),
            None => chip.get(a),
        };
        acc << 8 | b.copied().unwrap_or(0) as u32
    })
}

/// Whether the translated program is playing world 1, from chip RAM and slow RAM at a frame's end:
/// `irq_count` counts the passes of play from 1 at its first frame and is 0 before (docs/09), and
/// `world_index` is 0 in world 1.
pub fn in_level_1(chip: &[u8], slow: &[u8]) -> bool {
    ram_var(chip, slow, "irq_count") != 0 && ram_var(chip, slow, "world_index") == 0
}

/// Where the translated program's play is ([`Location`]), from RAM at a frame's end: the
/// character's cell in level 1, `None` anywhere else (the title, later worlds no recording names).
pub fn location_in_ram(chip: &[u8], slow: &[u8]) -> Option<Location> {
    in_level_1(chip, slow).then(|| {
        Location::of_character(
            LevelId::W1_1,
            ram_var(chip, slow, "char_x"),
            ram_var(chip, slow, "char_y"),
        )
    })
}

/// The display window the translated program's frames are shown through, saved in a bug zip's
/// screenshot and replayed through: left, top, width, height. The machine's picture is the
/// program's own window, which changes from screen to screen (measured 2026-11-19, translator
/// T37: the disk 2 prompt 320×200 from line 64, the title 320×238 from 42, play's first frame
/// 320×255 from 42, play 320×250 from 47, all from column 129); this is the least window holding
/// every one, so the app's window keeps one size.
pub const CANVAS: (i32, u16, u16, u16) = (129, 42, 320, 256);

/// `p` through [`CANVAS`]: what the app shows of a translated frame.
pub fn canvas(p: &Picture) -> Picture {
    let (left, top, width, height) = CANVAS;
    p.rewindow(left, top, width, height)
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
            Some(Location {
                level: LevelId::W1_1,
                col: 42,
                row: 35,
            }),
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
                "forest-dumb recording 2\nbuild forest-dumb 0.1.0 (0123456789abcdef)\nstart level-1\n\
                 engine hand\nlevel W1-1\nframes 10000\nstopped-at W1-1:42,35\nstop -\nnote fell through the floor # not a comment\n\
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
                Some(Location {
                    level: LevelId::W1_1,
                    col: 7,
                    row: 7,
                }),
                String::new(),
            )
            .unwrap();
        assert_eq!(rec.inputs.len(), 0);
        let text = rec.to_text();
        assert_eq!(
            text,
            "forest-dumb recording 2\nbuild b\nstart level-1\nengine hand\nlevel W1-1\nframes 0\n\
             stopped-at W1-1:7,7\nstop -\nnote\ninputs\n"
        );
        assert_eq!(Recording::parse(&text).unwrap(), rec);
    }

    #[test]
    fn another_version_or_level_is_refused_naming_it() {
        let text = full().to_text();
        let v3 = text.replacen("recording 2", "recording 3", 1);
        let e = Recording::parse(&v3).unwrap_err();
        assert_eq!(e.line, 1);
        assert!(
            e.why
                .contains("format `3` is not one this engine reads (1, 2)"),
            "{e}"
        );
        let w12 = text.replacen("level W1-1", "level W1-2", 1);
        let e = Recording::parse(&w12).unwrap_err();
        assert_eq!(e.line, 5);
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

    /// A recording made before translator T39: no `start`, `engine` or `stop` line.
    const V1: &str = "forest-dumb recording 1\nbuild b\nlevel W1-1\nframes 3\n\
                      stopped-at W1-1:7,7\nnote old\ninputs\n0: -\n1: R\n";

    #[test]
    fn a_version_1_recording_reads_as_level_1_on_the_hand_engine_with_no_stop() {
        let rec = Recording::parse(V1).unwrap();
        assert_eq!(
            (rec.start, rec.engine, rec.stop.clone()),
            (StartPoint::Level1, EngineKind::Hand, None)
        );
        assert_eq!(rec.inputs.len(), 3);
        assert_eq!(rec.note, "old");
        assert_eq!(rec.stopped_at.unwrap().to_string(), "W1-1:7,7");
        // Written again, it is version 2.
        assert!(rec.to_text().starts_with("forest-dumb recording 2\nbuild b\nstart level-1\n"));
    }

    #[test]
    fn a_translated_stop_reads_back_with_its_start_and_no_place() {
        let mut r = recorder_from(StartPoint::PowerOn, EngineKind::Translated, LevelId::W1_1, "b");
        r.step(FrameInput::NONE);
        r.step(FrameInput::joy(Joy::FIRE));
        let mut rec = r.finish(None, String::new()).unwrap();
        rec.stop = Some(stop_text("unsupported: odd\nop at h0+$10"));
        let text = rec.to_text();
        assert!(
            text.starts_with(
                "forest-dumb recording 2\nbuild b\nstart power-on\nengine translated\nlevel W1-1\n\
                 frames 2\nstopped-at -\nstop unsupported: odd op at h0+$10\nnote\ninputs\n"
            ),
            "{text}"
        );
        assert_eq!(Recording::parse(&text).unwrap(), rec);
        assert_eq!(stop_text("-"), "(-)");
        assert!(is_test_stop(&test_stop(120)) && !is_test_stop("untranslated: h0+$10"));
        assert_eq!(test_stop(120), "test-stop: --stop-after 120");
    }

    #[test]
    fn a_bad_start_engine_or_stop_is_refused_naming_its_line() {
        let text = full().to_text();
        let e = Recording::parse(&text.replacen("start level-1", "start title", 1)).unwrap_err();
        assert_eq!(e.line, 3);
        assert!(e.why.contains("start `title`"), "{e}");
        let e = Recording::parse(&text.replacen("engine hand", "engine oracle", 1)).unwrap_err();
        assert_eq!(e.line, 4);
        let e = Recording::parse(&text.replacen("start level-1", "start power-on", 1)).unwrap_err();
        assert!(e.why.contains("hand engine starts at level-1 only"), "{e}");
        let e = Recording::parse(&text.replacen("stop -", "stop", 1)).unwrap_err();
        assert_eq!(e.line, 8);
        let mut rec = full();
        rec.start = StartPoint::PowerOn;
        assert!(rec.validate().is_err());
    }

    #[test]
    fn the_way_to_level_1_presses_fire_ten_frames_in_twenty() {
        let fire: Vec<bool> = (0..40)
            .map(|n| to_level_1_input(n).joy.contains(Joy::FIRE))
            .collect();
        assert!(fire[..10].iter().all(|f| !f) && fire[10..20].iter().all(|&f| f));
        assert!(fire[20..30].iter().all(|f| !f) && fire[30..].iter().all(|&f| f));
    }

    /// Writes `value` into `name` in a 512 KB chip and 512 KB slow RAM pair.
    fn put(chip: &mut [u8], slow: &mut [u8], name: &str, value: u32) {
        let v = crate::layout_var(name).unwrap();
        let at = (crate::HUNK_BASES[v.hunk as usize] + v.offset) as usize;
        let n = v.width.bytes();
        for k in 0..n {
            let b = (value >> (8 * (n - 1 - k))) as u8;
            match (at + k).checked_sub(SLOW_BASE) {
                Some(i) => slow[i] = b,
                None => chip[at + k] = b,
            }
        }
    }

    #[test]
    fn level_1_is_play_counted_in_world_1_and_has_the_character_s_cell() {
        let (mut chip, mut slow) = (vec![0u8; 512 << 10], vec![0u8; 512 << 10]);
        assert!(!in_level_1(&chip, &slow));
        assert_eq!(location_in_ram(&chip, &slow), None);
        put(&mut chip, &mut slow, "irq_count", 1);
        put(&mut chip, &mut slow, "char_x", 669);
        put(&mut chip, &mut slow, "char_y", 553);
        assert!(in_level_1(&chip, &slow));
        assert_eq!(location_in_ram(&chip, &slow).unwrap().to_string(), "W1-1:42,35");
        put(&mut chip, &mut slow, "world_index", 1);
        assert!(!in_level_1(&chip, &slow), "world 2 is not level 1");
        assert_eq!(location_in_ram(&chip, &slow), None);
    }

    #[test]
    fn the_canvas_holds_every_window_the_program_shows() {
        for (left, top, w, h) in [(129, 64, 320, 200), (129, 42, 320, 238), (129, 42, 320, 255), (129, 47, 320, 250)] {
            let (cl, ct, cw, ch) = CANVAS;
            assert!(left >= cl && left + w <= cl + cw as i32, "{left} {w}");
            assert!(top >= ct && top + h <= ct + ch, "{top} {h}");
        }
        let p = canvas(&Picture::blank(129, 47, 320, 250));
        assert_eq!((p.width, p.height), (CANVAS.2, CANVAS.3));
    }
}
