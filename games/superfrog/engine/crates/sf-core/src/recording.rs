//! A bug recording (bug-recorder DESIGN §2.6): how a level was started from the Start Level dialog
//! (§2.1), the start's random state, every engine step's input from the level's frame 0, where it
//! stopped and the owner's note, as readable text. Pure, so the app writes it and sf-conform reads
//! it with one parser (D9).
//!
//! ```text
//! superfrog recording 2
//! build 0.1.0 abc1234
//! level W1-1
//! lives 3
//! points 0
//! energy 4
//! difficulty normal
//! spud off
//! wings off
//! widescreen off
//! seed 0000a5c3 1f00e2b7
//! frames 18342
//! stopped-at W1-2:94,51
//! note black screen after the slot machine
//! inputs
//! 0: -
//! 112: R
//! 130: R F
//! 900: R W
//! 901: R
//! ```
//!
//! The recording, its text form's shared rules, the recorder and the zip are the toolkit's
//! (`amiga_record`, toolkit-extract DESIGN §2.8); this file is Superfrog's header in it
//! ([`Header`]): the Start Level setup, where STOP was pressed, widescreen and its toggles, and the
//! magic. The header is fixed: every field, in this order, one per line, the value the rest of the
//! line (a note may hold `#`). The inputs are the conformance script's tokens and rule
//! ([`crate::input::format_tokens`]): a line holds from its frame until the next, frames strictly
//! increase, a `#` starts a comment; before the first line nothing is held. `frames` is how many
//! steps were recorded, so held input past the last line is still counted.
//!
//! Version 2 (real-widescreen DESIGN §2.8) adds `widescreen`, the mode at frame 0, and the token
//! `W`: the mode toggles once, before its line's first frame is stepped. Unlike the other tokens it
//! is not held for the line's later frames, so the writer starts a line at every toggle frame and
//! writes `W` after the held tokens. Version 1 reads as widescreen off with no toggles; the writer
//! always writes 2.

use core::fmt;

use amiga_record::{GameRecording, Head, HeaderReader, HeaderWriter};
use sf_assets::LevelId;

use crate::rng::Rng;

pub use amiga_record::{MAX_FRAMES, RecordingError, ZIP_RECORDING, ZIP_SCREENSHOT};

/// One recording (DESIGN §2.2, §2.6): the toolkit's, with Superfrog's [`Header`]. It reads as its
/// header too (`rec.setup`, `rec.stopped_at`).
pub type Recording = amiga_record::Recording<Superfrog>;

/// The input token that toggles widescreen once (DESIGN §2.8).
const TOGGLE: &str = "W";

pub const MAX_POINTS: u32 = 999_999;

/// An extra life comes at every multiple of this score (`frog_next_life`, h0+$880).
const LIFE_EVERY: u32 = 200_000;

/// The screens [`Location::Screen`] names (DESIGN §2.6; `Game::location`, T02). `world end` is the
/// end-of-world picture after a world's fourth level (owner, T02, 2026-10-20).
pub const SCREENS: [&str; 9] = [
    "title",
    "intro",
    "world map",
    "world end",
    "tally",
    "slot machine",
    "game over",
    "story",
    "end sequence",
];

/// `opt_game_mode` (h0+$9e6) as the options screen sets it. No hard: the options screen cannot
/// select it (`options_input` stops at 1; D12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Difficulty {
    Easy,
    Normal,
}

impl Difficulty {
    /// `opt_game_mode`'s value.
    pub fn opt_game_mode(self) -> u8 {
        match self {
            Difficulty::Easy => 0,
            Difficulty::Normal => 1,
        }
    }

    fn word(self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Normal => "normal",
        }
    }
}

/// `opt_lives` as the options screen sets it: 3, 5 or 7 lives (`frog_new_game` stores 2, 4, 6
/// spare).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lives {
    Three,
    Five,
    Seven,
}

impl Lives {
    /// `opt_lives`' value.
    pub fn opt_lives(self) -> u8 {
        match self {
            Lives::Three => 0,
            Lives::Five => 1,
            Lives::Seven => 2,
        }
    }

    /// The lives the player starts with, as the options screen shows them.
    pub fn count(self) -> u8 {
        match self {
            Lives::Three => 3,
            Lives::Five => 5,
            Lives::Seven => 7,
        }
    }

    fn from_count(n: &str) -> Option<Lives> {
        match n {
            "3" => Some(Lives::Three),
            "5" => Some(Lives::Five),
            "7" => Some(Lives::Seven),
            _ => None,
        }
    }
}

/// The Start Level dialog's values (DESIGN §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LevelSetup {
    pub level: LevelId,
    pub lives: Lives,
    /// `score` (h0+$834), `0..=MAX_POINTS`.
    pub points: u32,
    /// `frog_hearts` (h0+$830), `1..=4`.
    pub energy: u8,
    pub difficulty: Difficulty,
    /// `frog_spud_on`.
    pub spud: bool,
    /// `frog_flight`.
    pub wings: bool,
}

/// A new game's values (owner, 2026-10-20; D12): W1-1, 3 lives, 0 points, 4 hearts, normal.
impl Default for LevelSetup {
    fn default() -> LevelSetup {
        LevelSetup {
            level: LevelId {
                world: sf_assets::World::W1,
                level: 1,
                alt: false,
            },
            lives: Lives::Three,
            points: 0,
            energy: 4,
            difficulty: Difficulty::Normal,
            spud: false,
            wings: false,
        }
    }
}

/// The levels a setup starts: the 26 a game plays, W1-1..W6-4, the bonus level and the final level.
/// W6's second map set (`W6-2alt`..) is left out: a game started as a player starts one
/// (`Game::start_level`) never loads it (owner, T02, 2026-10-20).
pub fn levels() -> Vec<LevelId> {
    use sf_assets::World;
    let mut v: Vec<LevelId> = World::ALL[..6]
        .iter()
        .flat_map(|&world| (1..=4).map(move |level| LevelId { world, level, alt: false }))
        .collect();
    for world in [World::Bonus, World::Final] {
        v.push(LevelId { world, level: 1, alt: false });
    }
    v
}

impl LevelSetup {
    /// Whether the values are in the dialog's ranges.
    pub fn validate(&self) -> Result<(), String> {
        if !levels().contains(&self.level) {
            return Err(format!(
                "level {} is not one a game plays from its start (W1-1..W6-4, WB-1, WW-1)",
                self.level
            ));
        }
        if self.points > MAX_POINTS {
            return Err(format!(
                "points {} is more than the score's six digits hold ({MAX_POINTS})",
                self.points
            ));
        }
        if !(1..=4).contains(&self.energy) {
            return Err(format!("energy {} is not 1 to 4 hearts", self.energy));
        }
        Ok(())
    }

    /// `frog_next_life`: the next multiple of 200 000 above the points, where a real game would give
    /// the next extra life.
    pub fn next_life(&self) -> u32 {
        (self.points / LIFE_EVERY + 1) * LIFE_EVERY
    }
}

/// Where play is: the frog's cell in a level (DESIGN §2.6, T00 Q3), or a screen's name from
/// [`SCREENS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    Level { id: LevelId, col: u16, row: u16 },
    Screen(&'static str),
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Location::Level { id, col, row } => write!(f, "{id}:{col},{row}"),
            Location::Screen(name) => f.write_str(name),
        }
    }
}

/// As its text: a save state carries the last place a game showed (`Game::location`).
impl serde::Serialize for Location {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for Location {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Location, D::Error> {
        let s = <String as serde::Deserialize>::deserialize(de)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl core::str::FromStr for Location {
    type Err = String;

    /// What [`Location`]'s `Display` prints: `W1-2:94,51` or a name from [`SCREENS`].
    fn from_str(s: &str) -> Result<Location, String> {
        if let Some(name) = SCREENS.iter().find(|&&n| n == s) {
            return Ok(Location::Screen(name));
        }
        let bad = || {
            format!(
                "`{s}` is not a place: a level cell like W1-2:94,51 or a screen ({})",
                SCREENS.join(", ")
            )
        };
        let (id, cell) = s.split_once(':').ok_or_else(bad)?;
        let id: LevelId = id.parse()?;
        let (col, row) = cell.split_once(',').ok_or_else(bad)?;
        Ok(Location::Level {
            id,
            col: number(col).ok_or_else(bad)?,
            row: number(row).ok_or_else(bad)?,
        })
    }
}

/// Decimal digits only (`parse` alone would take a leading `+`).
fn number<T: core::str::FromStr>(s: &str) -> Option<T> {
    (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .then(|| s.parse().ok())
        .flatten()
}

/// Superfrog's part of a recording ([`GameRecording`]): its header, its text and its magic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Superfrog {}

/// Superfrog's header: where the level was started, where it stopped, and widescreen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    pub setup: LevelSetup,
    /// Where STOP was pressed; `None` (written `-`) before it was.
    pub stopped_at: Option<Location>,
    /// Widescreen at frame 0 (real-widescreen DESIGN §2.8).
    pub widescreen: bool,
    /// The frames before whose step widescreen toggles, strictly increasing, each below
    /// the recording's frame count.
    pub toggles: Vec<u32>,
}

impl Header {
    /// A level started with `setup` in widescreen when `widescreen`, not stopped, no toggles.
    pub fn new(setup: LevelSetup, widescreen: bool) -> Header {
        Header { setup, stopped_at: None, widescreen, toggles: Vec::new() }
    }

    /// Whether any of the game was played in widescreen: on at frame 0, or toggled on later. Such a
    /// game has no original to compare with (DESIGN §2.8, D9).
    pub fn has_widescreen(&self) -> bool {
        self.widescreen || !self.toggles.is_empty()
    }

    /// Widescreen while frame `frame` is stepped: the header's mode, flipped by every toggle at or
    /// before it.
    pub fn widescreen_at(&self, frame: u32) -> bool {
        self.widescreen ^ (self.toggles.partition_point(|&t| t <= frame) % 2 == 1)
    }
}

/// The seed's bytes in a recording: `lo` then `hi`, each big-endian.
pub fn seed_bytes(rng: Rng) -> Vec<u8> {
    [rng.lo.to_be_bytes(), rng.hi.to_be_bytes()].concat()
}

/// [`seed_bytes`] read back; `None` unless eight bytes.
fn rng_of(seed: &[u8]) -> Option<Rng> {
    let long = |at: usize| Some(u32::from_be_bytes(seed.get(at..at + 4)?.try_into().ok()?));
    (seed.len() == 8).then(|| Some(Rng { lo: long(0)?, hi: long(4)? })).flatten()
}

/// The random state a recording starts from.
pub trait RecordingRng {
    /// The seed as the game's [`Rng`]. Panics on a seed not eight bytes, which
    /// [`amiga_record::Recording::validate`] refuses and the parser never makes.
    fn rng(&self) -> Rng;
}

impl RecordingRng for Recording {
    fn rng(&self) -> Rng {
        rng_of(&self.seed).unwrap_or_else(|| panic!("a recording's seed is 8 bytes, not {}", self.seed.len()))
    }
}

impl GameRecording for Superfrog {
    type Header = Header;
    type Stop = Location;
    const MAGIC: &'static str = "superfrog recording";
    const VERSION: &'static str = "2";
    const TOKENS: &'static [&'static str] = &[TOGGLE];

    fn write_header(rec: &Recording, w: &mut HeaderWriter) {
        let s = &rec.setup;
        let on = |b: bool| if b { "on" } else { "off" };
        w.build(&rec.build);
        w.field("level", &s.level.to_string());
        w.field("lives", &s.lives.count().to_string());
        w.field("points", &s.points.to_string());
        w.field("energy", &s.energy.to_string());
        w.field("difficulty", s.difficulty.word());
        w.field("spud", on(s.spud));
        w.field("wings", on(s.wings));
        w.field("widescreen", on(rec.widescreen));
        let seed = rng_of(&rec.seed).unwrap_or_default();
        w.field("seed", &format!("{:08x} {:08x}", seed.lo, seed.hi));
        w.frames(rec.inputs.len());
        match &rec.stopped_at {
            Some(at) => w.field("stopped-at", &at.to_string()),
            None => w.field("stopped-at", "-"),
        }
        w.note(&rec.note);
    }

    fn read_header(n: usize, version: &str, r: &mut HeaderReader<'_>) -> Result<Head<Superfrog>, RecordingError> {
        let err = HeaderReader::error;
        let v2 = match version {
            "1" => false,
            "2" => true,
            _ => {
                return Err(err(n, format!("recording format `{version}` is not one this engine reads (1, 2)")));
            }
        };
        let build = r.build()?;
        let (n, level) = r.next("level")?;
        let level: LevelId = level.parse().map_err(|e| err(n, e))?;
        let (n, lives) = r.next("lives")?;
        let lives = Lives::from_count(lives).ok_or_else(|| err(n, format!("lives `{lives}` is not 3, 5 or 7")))?;
        let (n, points) = r.next("points")?;
        let points: u32 = number(points)
            .filter(|&p| p <= MAX_POINTS)
            .ok_or_else(|| err(n, format!("points `{points}` is not 0 to {MAX_POINTS}")))?;
        let (n, energy) = r.next("energy")?;
        let energy: u8 = number(energy)
            .filter(|e| (1..=4).contains(e))
            .ok_or_else(|| err(n, format!("energy `{energy}` is not 1 to 4 hearts")))?;
        let (n, difficulty) = r.next("difficulty")?;
        let difficulty = match difficulty {
            "easy" => Difficulty::Easy,
            "normal" => Difficulty::Normal,
            d => return Err(err(n, format!("difficulty `{d}` is not easy or normal"))),
        };
        let spud = r.flag("spud")?;
        let wings = r.flag("wings")?;
        let widescreen = if v2 { r.flag("widescreen")? } else { false };
        let (n, seed) = r.next("seed")?;
        let seed = seed
            .split_once(' ')
            .and_then(|(lo, hi)| Some(Rng { lo: hex32(lo)?, hi: hex32(hi)? }))
            .ok_or_else(|| err(n, format!("seed `{seed}` is not two hex longs, `lo hi`")))?;
        let frames = r.frames()?;
        let (n, stopped) = r.next("stopped-at")?;
        let stopped_at = match stopped {
            "-" => None,
            s => Some(s.parse().map_err(|e| err(n, e))?),
        };
        let note = r.note()?;
        let setup = LevelSetup { level, lives, points, energy, difficulty, spud, wings };
        Ok(Head {
            build,
            seed: seed_bytes(seed),
            frames,
            note,
            header: Header { setup, stopped_at, widescreen, toggles: Vec::new() },
        })
    }

    /// The setup in range, the seed two longs, and the toggles strictly increasing, each before
    /// the last step.
    fn validate(rec: &Recording) -> Result<(), String> {
        rec.setup.validate()?;
        if rng_of(&rec.seed).is_none() {
            return Err(format!("the seed is two longs, 8 bytes, not {}", rec.seed.len()));
        }
        if !rec.toggles.windows(2).all(|w| w[0] < w[1]) {
            return Err("widescreen toggles must be at strictly increasing frames".into());
        }
        if let Some(&last) = rec.toggles.last()
            && last as usize >= rec.inputs.len()
        {
            return Err(format!(
                "a widescreen toggle at frame {last} is past the recording's {} frames",
                rec.inputs.len()
            ));
        }
        Ok(())
    }

    fn frame_tokens(h: &Header, frame: u32) -> Vec<&'static str> {
        match h.toggles.binary_search(&frame) {
            Ok(_) => vec![TOGGLE],
            Err(_) => Vec::new(),
        }
    }

    fn read_frame_tokens(h: &mut Header, version: &str, frame: u32, tokens: &[&str]) -> Result<(), String> {
        match tokens.len() {
            0 => return Ok(()),
            1 => {}
            _ => return Err(format!("`{TOGGLE}` twice in one line: a line toggles once")),
        }
        if version == "1" {
            return Err(format!("`{TOGGLE}` is a version 2 token; this recording is version 1"));
        }
        h.toggles.push(frame);
        Ok(())
    }

    /// Stopped at `at`; a widescreen switch after the last step played nothing in its mode, so it
    /// is dropped.
    fn stop(rec: &mut Recording, at: Location) {
        if rec.toggles.last().is_some_and(|&t| t as usize >= rec.inputs.len()) {
            rec.toggles.pop();
        }
        rec.stopped_at = Some(at);
    }
}

/// One to eight hex digits.
fn hex32(s: &str) -> Option<u32> {
    (1..=8)
        .contains(&s.len())
        .then_some(())
        .filter(|_| s.bytes().all(|b| b.is_ascii_hexdigit()))
        .and_then(|_| u32::from_str_radix(s, 16).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{FrameInput, Joy, KeySet};

    fn id(s: &str) -> LevelId {
        s.parse().unwrap()
    }

    /// Every field set away from its default, every token kind, 100 000 frames.
    fn full() -> Recording {
        let kinds = [
            FrameInput::NONE,
            FrameInput::joy(Joy::RIGHT),
            FrameInput::joy(Joy::RIGHT | Joy::FIRE),
            FrameInput::joy(Joy::UP | Joy::DOWN | Joy::LEFT | Joy::PORT1_BUTTON),
            FrameInput {
                joy: Joy::LEFT,
                keys: KeySet::key(0x19).with(0x7F),
            },
            FrameInput::key(0),
        ];
        let inputs = (0..100_000u32)
            .map(|f| kinds[(f / 37 % kinds.len() as u32) as usize])
            .collect();
        Recording {
            build: "0.1.0 abc1234 dirty".into(),
            seed: seed_bytes(Rng {
                lo: 0x0000_a5c3,
                hi: 0xffff_0001,
            }),
            note: "black screen after the slot machine # not a comment".into(),
            inputs,
            header: Header {
                setup: LevelSetup {
                    level: id("W4-3"),
                    lives: Lives::Seven,
                    points: 999_999,
                    energy: 1,
                    difficulty: Difficulty::Easy,
                    spud: true,
                    wings: true,
                },
                stopped_at: Some(Location::Level {
                    id: id("W1-2"),
                    col: 94,
                    row: 51,
                }),
                widescreen: true,
                toggles: vec![0, 38, 900, 99_999],
            },
        }
    }

    #[test]
    fn a_full_recording_round_trips() {
        let r = full();
        assert_eq!(r.validate(), Ok(()));
        assert_eq!(Recording::parse(&r.to_text()), Ok(r.clone()));
        let mut s = r.clone();
        s.stopped_at = Some(Location::Screen("slot machine"));
        s.setup.lives = Lives::Five;
        assert_eq!(Recording::parse(&s.to_text()), Ok(s.clone()));
        s.stopped_at = None;
        s.inputs.clear();
        s.toggles.clear();
        assert_eq!(Recording::parse(&s.to_text()), Ok(s));
    }

    #[test]
    fn the_header_is_in_design_order_and_inputs_only_where_they_change() {
        let mut r = Recording {
            build: "b".into(),
            seed: seed_bytes(Rng { lo: 1, hi: 0xabc }),
            note: "n".into(),
            inputs: vec![FrameInput::NONE; 200],
            header: Header {
                stopped_at: Some(Location::Screen("tally")),
                ..Header::new(LevelSetup::default(), false)
            },
        };
        r.inputs[112..130].fill(FrameInput::joy(Joy::RIGHT));
        r.inputs[130..].fill(FrameInput::joy(Joy::RIGHT | Joy::FIRE));
        assert_eq!(
            r.to_text(),
            "superfrog recording 2\nbuild b\nlevel W1-1\nlives 3\npoints 0\nenergy 4\n\
             difficulty normal\nspud off\nwings off\nwidescreen off\nseed 00000001 00000abc\n\
             frames 200\nstopped-at tally\nnote n\ninputs\n0: -\n112: R\n130: R F\n"
        );
        r.toggles = vec![0, 120, 130, 131];
        r.widescreen = true;
        let text = r.to_text();
        assert!(text.contains("\nwings off\nwidescreen on\nseed"), "{text}");
        assert!(
            text.ends_with("inputs\n0: W\n112: R\n120: R W\n130: R F W\n131: R F W\n"),
            "{text}"
        );
        assert_eq!(Recording::parse(&text), Ok(r));
    }

    #[test]
    fn held_input_and_comments_in_the_inputs() {
        let mut r = full();
        r.toggles.clear();
        let text = r.to_text().replace("frames 100000", "frames 10");
        let head = &text[..text.find("inputs\n").unwrap() + 7];
        let r = Recording::parse(&format!("{head}# a comment\n\n3: R # right\n5: K19\n")).unwrap();
        assert_eq!(r.inputs.len(), 10);
        assert_eq!(r.inputs[..3], [FrameInput::NONE; 3]);
        assert_eq!(r.inputs[3..5], [FrameInput::joy(Joy::RIGHT); 2]);
        assert_eq!(r.inputs[5..], [FrameInput::key(0x19); 5]);
    }

    #[test]
    fn refusals_name_their_line() {
        let good = full().to_text();
        let swap = |from: &str, to: &str| {
            assert!(good.contains(from), "{from}");
            good.replacen(from, to, 1)
        };
        let inputs_at = good.lines().position(|l| l == "inputs").unwrap() + 1;
        for (text, line, says) in [
            (
                swap("superfrog recording 2", "superfrog recording 3"),
                1,
                "format `3`",
            ),
            (
                swap("superfrog recording 2", "superfrog recording 1"),
                10,
                "missing `seed`",
            ),
            (
                swap("superfrog recording 2", "superfrog recordings 2"),
                1,
                "missing `superfrog recording`",
            ),
            (swap("energy 1\n", ""), 6, "missing `energy`"),
            (swap("level W4-3\n", ""), 3, "missing `level`"),
            (
                good[..good.find("note").unwrap()].to_string(),
                14,
                "missing `note`",
            ),
            (swap("widescreen on\n", ""), 10, "missing `widescreen`"),
            (swap("widescreen on", "wide-screen on"), 10, "missing `widescreen`"),
            (swap("widescreen on", "widescreen yes"), 10, "not on or off"),
            (swap("\n38: R W\n", "\n38: R W W\n"), inputs_at + 3, "twice in one line"),
            (
                swap("\n38: R W\n", "\n38: R V\n"),
                inputs_at + 3,
                "not an input",
            ),
            (swap("lives 7", "lives 4"), 4, "not 3, 5 or 7"),
            (swap("energy 1", "energy 0"), 6, "not 1 to 4"),
            (swap("energy 1", "energy 5"), 6, "not 1 to 4"),
            (
                swap("points 999999", "points 1000000"),
                5,
                "not 0 to 999999",
            ),
            (swap("points 999999", "points +5"), 5, "not 0 to 999999"),
            (
                swap("difficulty easy", "difficulty hard"),
                7,
                "not easy or normal",
            ),
            (swap("spud on", "spud yes"), 8, "not on or off"),
            (
                swap("seed 0000a5c3 ffff0001", "seed 0000a5c3"),
                11,
                "two hex longs",
            ),
            (
                swap("seed 0000a5c3 ffff0001", "seed 0000a5c3 1ffff0001"),
                11,
                "two hex longs",
            ),
            (swap("frames 100000", "frames -1"), 12, "not a frame count"),
            (
                swap("frames 100000", "frames 4320001"),
                12,
                "longest recording",
            ),
            (
                swap("frames 100000", "frames 4294967295"),
                12,
                "longest recording",
            ),
            (swap("level W4-3", "level W9-1"), 3, "not a level id"),
            (
                swap("stopped-at W1-2:94,51", "stopped-at W9-2:94,51"),
                13,
                "not a level id",
            ),
            (
                swap("stopped-at W1-2:94,51", "stopped-at the moon"),
                13,
                "not a place",
            ),
            (swap("inputs\n", "inputs x\n"), inputs_at, "stands alone"),
            (
                swap("\n37: R\n", "\n37: R\n36: R F\n"),
                inputs_at + 3,
                "strictly increase",
            ),
            (
                swap("\n37: R\n", "\n37: R\n37: R F\n"),
                inputs_at + 3,
                "strictly increase",
            ),
            (
                swap("\n37: R\n", "\n37: Q\n"),
                inputs_at + 2,
                "not an input",
            ),
            (swap("\n37: R\n", "\n37: K80\n"), inputs_at + 2, "00..7F"),
            (
                swap("\n37: R\n", "\nx: R\n"),
                inputs_at + 2,
                "not a frame number",
            ),
            (swap("\n37: R\n", "\n37 R\n"), inputs_at + 2, "expected"),
            (
                format!("{good}100000: R\n"),
                good.lines().count() + 1,
                "past the recording's",
            ),
        ] {
            let e = Recording::parse(&text).unwrap_err();
            assert_eq!((e.line, e.why.contains(says)), (line, true), "{says}: {e}");
        }
        assert_eq!(Recording::parse("").unwrap_err().line, 1);
        assert!(Recording::parse("PK\u{3}\u{4}junk").is_err());
    }

    #[test]
    fn a_version_1_recording_reads_as_widescreen_off_with_no_toggles() {
        let mut r = full();
        r.widescreen = false;
        r.toggles.clear();
        let v1 = r
            .to_text()
            .replace("superfrog recording 2", "superfrog recording 1")
            .replace("widescreen off\n", "");
        assert_eq!(Recording::parse(&v1), Ok(r.clone()));
        let with_w = v1.replacen("\n37: R\n", "\n37: R W\n", 1);
        let e = Recording::parse(&with_w).unwrap_err();
        assert!(e.why.contains("version 2 token"), "{e}");
        // Read back and written again, a version 1 file becomes version 2, the game unchanged.
        assert!(Recording::parse(&v1).unwrap().to_text().starts_with("superfrog recording 2\n"));
        let mut r = full();
        r.toggles.clear();
        assert_eq!(Recording::parse(&r.to_text()), Ok(r));
    }

    #[test]
    fn a_toggle_is_written_after_the_held_tokens_and_read_in_any_order() {
        let mut r = full();
        r.inputs.truncate(10);
        r.inputs.fill(FrameInput {
            joy: Joy::RIGHT | Joy::FIRE,
            keys: KeySet::key(0x19),
        });
        r.toggles = vec![4];
        let text = r.to_text();
        assert!(text.ends_with("inputs\n0: R F K19\n4: R F K19 W\n"), "{text}");
        let shuffled = text.replace("4: R F K19 W", "4: W K19 F R");
        assert_eq!(Recording::parse(&shuffled), Ok(r.clone()));
        let dash = text.replace("4: R F K19 W", "4: - W R F K19");
        assert_eq!(Recording::parse(&dash), Ok(r));
    }

    #[test]
    fn a_line_holding_w_for_several_frames_toggles_once() {
        let head = full().to_text().replace("frames 100000", "frames 20");
        let head = &head[..head.find("inputs\n").unwrap() + 7];
        let r = Recording::parse(&format!("{head}0: -\n5: R W\n15: -\n")).unwrap();
        assert_eq!(r.toggles, [5]);
        assert_eq!(r.inputs[5..15], [FrameInput::joy(Joy::RIGHT); 10]);
        assert!(r.widescreen);
        let at: Vec<bool> = (0..20).map(|f| r.widescreen_at(f)).collect();
        assert_eq!(at[..5], [true; 5]);
        assert_eq!(at[5..], [false; 15]);
        assert!(r.has_widescreen());
    }

    #[test]
    fn the_mode_at_each_frame_and_whether_any_was_wide() {
        let mut r = full();
        r.widescreen = false;
        r.toggles.clear();
        assert!(!r.has_widescreen() && !r.widescreen_at(0) && !r.widescreen_at(99_999));
        r.toggles = vec![0, 10, 11];
        assert!(r.has_widescreen());
        let at: Vec<bool> = (0..13).map(|f| r.widescreen_at(f)).collect();
        assert_eq!(at, [[true; 10].as_slice(), &[false, true, true]].concat());
        r.toggles = vec![10, 10];
        assert!(r.validate().unwrap_err().contains("strictly increasing"));
        r.toggles = vec![100_000];
        assert!(r.validate().unwrap_err().contains("past the recording"));
        r.toggles = vec![99_999];
        assert_eq!(r.validate(), Ok(()));
    }

    #[test]
    fn the_seed_is_its_two_longs_and_a_wrong_length_is_refused() {
        let r = full();
        assert_eq!(r.seed, [0, 0, 0xa5, 0xc3, 0xff, 0xff, 0, 1]);
        assert_eq!(r.rng(), Rng { lo: 0xa5c3, hi: 0xffff_0001 });
        let mut short = r.clone();
        short.seed.pop();
        assert!(short.validate().unwrap_err().contains("8 bytes"));
    }

    #[test]
    fn every_cut_of_a_recording_is_refused_or_read_never_a_panic() {
        let mut r = full();
        r.inputs.truncate(400);
        let text = r.to_text();
        for cut in 0..text.len() {
            if text.is_char_boundary(cut) {
                let _ = Recording::parse(&text[..cut]);
            }
        }
    }

    #[test]
    fn notes_survive_and_a_newline_is_refused() {
        for note in ["", "a # b", "żaba ☃ 🐸", " leading and trailing ", "#"] {
            let mut r = full();
            r.note = note.into();
            assert_eq!(r.validate(), Ok(()));
            assert_eq!(Recording::parse(&r.to_text()).unwrap().note, note);
        }
        let mut r = full();
        r.note = "one\ntwo".into();
        assert!(r.validate().unwrap_err().contains("note"));
        r.note = "one\rtwo".into();
        assert!(r.validate().is_err());
        r.note.clear();
        r.build = "b\n".into();
        assert!(r.validate().unwrap_err().contains("build"));
        r.build.clear();
        r.inputs = vec![FrameInput::NONE; MAX_FRAMES as usize + 1];
        assert!(r.validate().unwrap_err().contains("longest recording"));
        r.inputs.pop();
        assert_eq!(r.validate(), Ok(()));
    }

    #[test]
    fn setup_ranges_and_defaults() {
        let d = LevelSetup::default();
        assert_eq!(
            (
                d.level,
                d.lives,
                d.points,
                d.energy,
                d.difficulty,
                d.spud,
                d.wings
            ),
            (
                id("W1-1"),
                Lives::Three,
                0,
                4,
                Difficulty::Normal,
                false,
                false
            )
        );
        assert_eq!(d.validate(), Ok(()));
        for (points, energy, ok) in [
            (999_999, 4, true),
            (1_000_000, 4, false),
            (0, 0, false),
            (0, 5, false),
            (0, 1, true),
        ] {
            let s = LevelSetup {
                points,
                energy,
                ..d
            };
            assert_eq!(s.validate().is_ok(), ok, "{points} {energy}");
        }
        assert_eq!(
            [Lives::Three, Lives::Five, Lives::Seven].map(|l| (l.count(), l.opt_lives())),
            [(3, 0), (5, 1), (7, 2)]
        );
        assert_eq!(
            [Difficulty::Easy, Difficulty::Normal].map(Difficulty::opt_game_mode),
            [0, 1]
        );
    }

    #[test]
    fn next_life_is_the_next_multiple_of_200_000() {
        for (points, next) in [
            (0, 200_000),
            (199_999, 200_000),
            (200_000, 400_000),
            (999_999, 1_000_000),
        ] {
            assert_eq!(
                LevelSetup {
                    points,
                    ..LevelSetup::default()
                }
                .next_life(),
                next
            );
        }
    }

    #[test]
    fn locations_display_and_parse_back() {
        let at = Location::Level {
            id: id("W1-2"),
            col: 94,
            row: 51,
        };
        assert_eq!(at.to_string(), "W1-2:94,51");
        assert_eq!(Location::Screen("slot machine").to_string(), "slot machine");
        assert_eq!(
            Location::Level {
                id: id("WB-1"),
                col: 0,
                row: 0
            }
            .to_string(),
            "WB-1:0,0"
        );
        assert_eq!("W1-2:94,51".parse(), Ok(at));
        for name in SCREENS {
            assert_eq!(name.parse(), Ok(Location::Screen(name)));
        }
        for bad in [
            "",
            "W1-2",
            "W1-2:94",
            "W1-2:94,",
            "W1-2:+1,2",
            "W1-2:1,70000",
            "Title",
        ] {
            assert!(bad.parse::<Location>().is_err(), "{bad}");
        }
    }

    #[test]
    fn a_setup_starts_the_26_levels_a_game_plays_and_no_second_map() {
        let all = levels();
        assert_eq!(all.len(), 26);
        assert_eq!((all[0], all[23], all[24], all[25]), (id("W1-1"), id("W6-4"), id("WB-1"), id("WW-1")));
        for level in all {
            assert_eq!(LevelSetup { level, ..LevelSetup::default() }.validate(), Ok(()));
        }
        for alt in ["W6-2alt", "W6-3alt", "W6-4alt"] {
            let s = LevelSetup { level: id(alt), ..LevelSetup::default() };
            assert!(s.validate().unwrap_err().contains(alt), "{alt}");
        }
    }

    #[test]
    fn a_location_is_carried_as_its_text() {
        for l in [
            Location::Screen("world end"),
            Location::Level { id: id("W6-4"), col: 299, row: 83 },
        ] {
            let bytes = bincode::serialize(&l).unwrap();
            assert_eq!(bincode::deserialize::<Location>(&bytes).unwrap(), l);
        }
        let bad = bincode::serialize("nowhere").unwrap();
        assert!(bincode::deserialize::<Location>(&bad).is_err());
    }
}
