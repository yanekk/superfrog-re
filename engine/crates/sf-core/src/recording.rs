//! A bug recording (bug-recorder DESIGN §2.6): how a level was started from the Start Level dialog
//! (§2.1), the start's random state, every engine step's input from the level's frame 0, where it
//! stopped and the owner's note, as readable text. Pure, so the app writes it and sf-conform reads
//! it with one parser (D9).
//!
//! ```text
//! superfrog recording 1
//! build 0.1.0 abc1234
//! level W1-1
//! lives 3
//! points 0
//! energy 4
//! difficulty normal
//! spud off
//! wings off
//! seed 0000a5c3 1f00e2b7
//! frames 18342
//! stopped-at W1-2:94,51
//! note black screen after the slot machine
//! inputs
//! 0: -
//! 112: R
//! 130: R F
//! ```
//!
//! The header is fixed: every field, in this order, one per line, the value the rest of the line
//! (a note may hold `#`). The inputs are the conformance script's tokens and rule
//! ([`crate::input::format_tokens`]): a line holds from its frame until the next, frames strictly
//! increase, a `#` starts a comment; before the first line nothing is held. `frames` is how many
//! steps were recorded, so held input past the last line is still counted.

use core::fmt;

use sf_assets::LevelId;

use crate::input::{FrameInput, format_tokens, parse_tokens};
use crate::rng::Rng;

/// The bug zip's entries (DESIGN §2.5).
pub const ZIP_RECORDING: &str = "recording.txt";
pub const ZIP_SCREENSHOT: &str = "screenshot.png";

/// The first line of every recording; the number is the format's version.
const MAGIC: &str = "superfrog recording 1";

/// The highest score the dialog takes: the score shows six digits.
pub const MAX_POINTS: u32 = 999_999;

/// An extra life comes at every multiple of this score (`frog_next_life`, h0+$880).
const LIFE_EVERY: u32 = 200_000;

/// The longest recording: a day of play at one PAL frame a step (138 MB held expanded). The parser
/// holds `frames` inputs, so without a bound a header of a few bytes asks for 128 GB.
pub const MAX_FRAMES: u32 = 50 * 60 * 60 * 24;

/// The screens [`Location::Screen`] names (DESIGN §2.6; `Game::location`, T02).
pub const SCREENS: [&str; 8] = [
    "title",
    "intro",
    "world map",
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

impl LevelSetup {
    /// Whether the values are in the dialog's ranges.
    pub fn validate(&self) -> Result<(), String> {
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

/// One recording (DESIGN §2.2, §2.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recording {
    /// The engine build that made it; informational, never refused.
    pub build: String,
    pub setup: LevelSetup,
    /// The random state the start (b) boots with (`Start::PowerOn { seed }`, T00/T02).
    pub seed: Rng,
    /// Where STOP was pressed; `None` (written `-`) before it was.
    pub stopped_at: Option<Location>,
    /// The owner's one-line note; may be empty.
    pub note: String,
    /// One per engine step from the level's frame 0 (the first step after which the scene is a
    /// level or the bonus level). Held expanded: the app appends one a step.
    pub inputs: Vec<FrameInput>,
}

/// Why a recording's text was refused: the 1-based line and what is wrong with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingError {
    pub line: usize,
    pub why: String,
}

impl fmt::Display for RecordingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.why)
    }
}

impl Recording {
    /// Whether the recording can be written and read back: the setup in range, at most
    /// [`MAX_FRAMES`] inputs, and the build and note each one line.
    pub fn validate(&self) -> Result<(), String> {
        self.setup.validate()?;
        if self.inputs.len() > MAX_FRAMES as usize {
            return Err(format!(
                "{} frames is past the longest recording, {MAX_FRAMES} (a day)",
                self.inputs.len()
            ));
        }
        for (what, text) in [("build", &self.build), ("note", &self.note)] {
            if text.contains(['\n', '\r']) {
                return Err(format!("the {what} must be one line"));
            }
        }
        Ok(())
    }

    /// The recording as text (DESIGN §2.6): the header in order, then an input line wherever the
    /// input changes, the first at frame 0.
    pub fn to_text(&self) -> String {
        let s = &self.setup;
        let on = |b: bool| if b { "on" } else { "off" };
        let mut out = String::new();
        let mut line = |l: &str| {
            out.push_str(l);
            out.push('\n');
        };
        line(MAGIC);
        line(&with("build", &self.build));
        line(&format!("level {}", s.level));
        line(&format!("lives {}", s.lives.count()));
        line(&format!("points {}", s.points));
        line(&format!("energy {}", s.energy));
        line(&format!("difficulty {}", s.difficulty.word()));
        line(&format!("spud {}", on(s.spud)));
        line(&format!("wings {}", on(s.wings)));
        line(&format!("seed {:08x} {:08x}", self.seed.lo, self.seed.hi));
        line(&format!("frames {}", self.inputs.len()));
        match &self.stopped_at {
            Some(at) => line(&format!("stopped-at {at}")),
            None => line("stopped-at -"),
        }
        line(&with("note", &self.note));
        line("inputs");
        let mut held = None;
        for (frame, &input) in self.inputs.iter().enumerate() {
            if held != Some(input) {
                line(&format!("{frame}: {}", format_tokens(input)));
                held = Some(input);
            }
        }
        out
    }

    /// Reads [`Recording::to_text`]'s format back. Every refusal names its line; never a panic.
    pub fn parse(text: &str) -> Result<Recording, RecordingError> {
        let mut lines = text.lines().enumerate().map(|(i, l)| (i + 1, l));
        let last = text.lines().count();
        let mut next = |key: &str| -> Result<(usize, &str), RecordingError> {
            let missing = |line: usize| RecordingError {
                line,
                why: format!("missing `{key}`, which comes next in the header"),
            };
            let (n, l) = lines.next().ok_or_else(|| missing(last + 1))?;
            if l == key {
                return Ok((n, ""));
            }
            l.strip_prefix(key)
                .and_then(|r| r.strip_prefix(' '))
                .map(|r| (n, r))
                .ok_or_else(|| missing(n))
        };
        let err = |line: usize, why: String| RecordingError { line, why };

        let (n, version) = next("superfrog recording")?;
        if version != "1" {
            return Err(err(
                n,
                format!("recording format `{version}` is not one this engine reads (1)"),
            ));
        }
        let (_, build) = next("build")?;
        let (n, level) = next("level")?;
        let level: LevelId = level.parse().map_err(|e| err(n, e))?;
        let (n, lives) = next("lives")?;
        let lives = Lives::from_count(lives)
            .ok_or_else(|| err(n, format!("lives `{lives}` is not 3, 5 or 7")))?;
        let (n, points) = next("points")?;
        let points: u32 = number(points)
            .filter(|&p| p <= MAX_POINTS)
            .ok_or_else(|| err(n, format!("points `{points}` is not 0 to {MAX_POINTS}")))?;
        let (n, energy) = next("energy")?;
        let energy: u8 = number(energy)
            .filter(|e| (1..=4).contains(e))
            .ok_or_else(|| err(n, format!("energy `{energy}` is not 1 to 4 hearts")))?;
        let (n, difficulty) = next("difficulty")?;
        let difficulty = match difficulty {
            "easy" => Difficulty::Easy,
            "normal" => Difficulty::Normal,
            d => return Err(err(n, format!("difficulty `{d}` is not easy or normal"))),
        };
        let mut flag = |key: &str| -> Result<bool, RecordingError> {
            match next(key)? {
                (_, "on") => Ok(true),
                (_, "off") => Ok(false),
                (n, v) => Err(err(n, format!("{key} `{v}` is not on or off"))),
            }
        };
        let spud = flag("spud")?;
        let wings = flag("wings")?;
        let (n, seed) = next("seed")?;
        let seed = seed
            .split_once(' ')
            .and_then(|(lo, hi)| {
                Some(Rng {
                    lo: hex32(lo)?,
                    hi: hex32(hi)?,
                })
            })
            .ok_or_else(|| err(n, format!("seed `{seed}` is not two hex longs, `lo hi`")))?;
        let (n, frames) = next("frames")?;
        let frames: u32 = number(frames)
            .ok_or_else(|| err(n, format!("frames `{frames}` is not a frame count")))?;
        if frames > MAX_FRAMES {
            return Err(err(
                n,
                format!("frames {frames} is past the longest recording, {MAX_FRAMES} (a day)"),
            ));
        }
        let (n, stopped) = next("stopped-at")?;
        let stopped_at = match stopped {
            "-" => None,
            s => Some(s.parse().map_err(|e| err(n, e))?),
        };
        let (_, note) = next("note")?;
        let (n, rest) = next("inputs")?;
        if !rest.is_empty() {
            return Err(err(n, "`inputs` stands alone on its line".into()));
        }

        let mut inputs = Vec::with_capacity(frames as usize);
        let (mut held, mut prev) = (FrameInput::NONE, None);
        for (n, raw) in lines {
            let body = raw.split('#').next().unwrap_or("").trim();
            if body.is_empty() {
                continue;
            }
            let (frame, tokens) = body
                .split_once(':')
                .ok_or_else(|| err(n, format!("`{body}`: expected `<frame>: <inputs>`")))?;
            let frame: u32 = number(frame.trim())
                .ok_or_else(|| err(n, format!("`{}` is not a frame number", frame.trim())))?;
            if let Some(prev) = prev
                && frame <= prev
            {
                return Err(err(
                    n,
                    format!(
                        "frame {frame} comes after frame {prev}: frames must strictly increase"
                    ),
                ));
            }
            if frame >= frames {
                return Err(err(
                    n,
                    format!("frame {frame} is past the recording's {frames} frames"),
                ));
            }
            let input = parse_tokens(tokens).map_err(|e| err(n, e))?;
            inputs.resize(frame as usize, held);
            inputs.push(input);
            held = input;
            prev = Some(frame);
        }
        inputs.resize(frames as usize, held);
        Ok(Recording {
            build: build.to_string(),
            setup: LevelSetup {
                level,
                lives,
                points,
                energy,
                difficulty,
                spud,
                wings,
            },
            seed,
            stopped_at,
            note: note.to_string(),
            inputs,
        })
    }
}

/// `key value`, or the bare key when the value is empty (so no line ends in a space).
fn with(key: &str, value: &str) -> String {
    if value.is_empty() {
        key.to_string()
    } else {
        format!("{key} {value}")
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
    use crate::input::{Joy, KeySet};

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
            setup: LevelSetup {
                level: id("W4-3"),
                lives: Lives::Seven,
                points: 999_999,
                energy: 1,
                difficulty: Difficulty::Easy,
                spud: true,
                wings: true,
            },
            seed: Rng {
                lo: 0x0000_a5c3,
                hi: 0xffff_0001,
            },
            stopped_at: Some(Location::Level {
                id: id("W1-2"),
                col: 94,
                row: 51,
            }),
            note: "black screen after the slot machine # not a comment".into(),
            inputs,
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
        assert_eq!(Recording::parse(&s.to_text()), Ok(s));
    }

    #[test]
    fn the_header_is_in_design_order_and_inputs_only_where_they_change() {
        let mut r = Recording {
            build: "b".into(),
            setup: LevelSetup::default(),
            seed: Rng { lo: 1, hi: 0xabc },
            stopped_at: Some(Location::Screen("tally")),
            note: "n".into(),
            inputs: vec![FrameInput::NONE; 200],
        };
        r.inputs[112..130].fill(FrameInput::joy(Joy::RIGHT));
        r.inputs[130..].fill(FrameInput::joy(Joy::RIGHT | Joy::FIRE));
        assert_eq!(
            r.to_text(),
            "superfrog recording 1\nbuild b\nlevel W1-1\nlives 3\npoints 0\nenergy 4\n\
             difficulty normal\nspud off\nwings off\nseed 00000001 00000abc\nframes 200\n\
             stopped-at tally\nnote n\ninputs\n0: -\n112: R\n130: R F\n"
        );
    }

    #[test]
    fn held_input_and_comments_in_the_inputs() {
        let text = full().to_text().replace("frames 100000", "frames 10");
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
                swap("superfrog recording 1", "superfrog recording 2"),
                1,
                "format `2`",
            ),
            (
                swap("superfrog recording 1", "superfrog recordings 1"),
                1,
                "missing `superfrog recording`",
            ),
            (swap("energy 1\n", ""), 6, "missing `energy`"),
            (swap("level W4-3\n", ""), 3, "missing `level`"),
            (
                good[..good.find("note").unwrap()].to_string(),
                13,
                "missing `note`",
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
                10,
                "two hex longs",
            ),
            (
                swap("seed 0000a5c3 ffff0001", "seed 0000a5c3 1ffff0001"),
                10,
                "two hex longs",
            ),
            (swap("frames 100000", "frames -1"), 11, "not a frame count"),
            (
                swap("frames 100000", "frames 4320001"),
                11,
                "longest recording",
            ),
            (
                swap("frames 100000", "frames 4294967295"),
                11,
                "longest recording",
            ),
            (swap("level W4-3", "level W9-1"), 3, "not a level id"),
            (
                swap("stopped-at W1-2:94,51", "stopped-at W9-2:94,51"),
                12,
                "not a level id",
            ),
            (
                swap("stopped-at W1-2:94,51", "stopped-at the moon"),
                12,
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
}
