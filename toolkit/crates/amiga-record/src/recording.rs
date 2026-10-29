//! A recording and its text form. The text is a header, one field a line, then `inputs` and an
//! input line wherever the input changes:
//!
//! ```text
//! <magic> <version>
//! build 0.1.0 abc1234
//! <the game's own fields>
//! seed <the game's form>
//! frames 18342
//! note black screen after the bonus
//! inputs
//! 0: -
//! 112: R
//! 130: R F
//! ```
//!
//! The game writes and reads every header line between the first line and `inputs`, in its own
//! order, through [`HeaderWriter`] and [`HeaderReader`], which hold the shared fields' form
//! (`build`, `frames`, `note`): so a game's format already in use stays byte for byte as it was. A
//! field's value is the rest of its line (a note may hold `#`).
//!
//! The inputs are the input tokens' rule ([`amiga_input::format_tokens`]): a line holds from its
//! frame until the next, frames strictly increase, a `#` starts a comment; before the first line
//! nothing is held. `frames` is how many steps were recorded, so held input past the last line is
//! still counted. A game may add tokens of its own ([`GameRecording::TOKENS`]) that mark a frame
//! rather than hold for the line's later frames; the writer starts a line at every marked frame and
//! writes them after the held tokens.

use core::fmt;
use core::ops::{Deref, DerefMut};

use amiga_input::{FrameInput, format_tokens, parse_tokens};

/// The longest recording: a day of play at one PAL frame a step (138 MB held expanded). The parser
/// holds `frames` inputs, so without a bound a header of a few bytes asks for 128 GB.
pub const MAX_FRAMES: u32 = 50 * 60 * 60 * 24;

/// A game's part of a recording: its header (where play started and stopped, its toggles), the
/// header's text and the first line's magic.
pub trait GameRecording: Sized {
    /// The start point, the stop point and any toggles of the game's.
    type Header: Clone + fmt::Debug + PartialEq + Eq;
    /// Where play was when the recorder was stopped, as the app knows it.
    type Stop;
    /// The first line's key; the line is `MAGIC VERSION`.
    const MAGIC: &'static str;
    /// The format version written.
    const VERSION: &'static str;
    /// Tokens of the game's own on an input line: each marks its line's frame only
    /// ([`GameRecording::frame_tokens`], [`GameRecording::read_frame_tokens`]).
    const TOKENS: &'static [&'static str] = &[];

    /// Every header line between the first line and `inputs`, in the game's order: its own fields
    /// with [`HeaderWriter::field`], the shared ones with [`HeaderWriter::build`],
    /// [`HeaderWriter::frames`] and [`HeaderWriter::note`], and the seed in the game's form.
    fn write_header(rec: &Recording<Self>, w: &mut HeaderWriter);

    /// [`GameRecording::write_header`]'s lines read back, `version` the first line's (line
    /// `version_line`). Every refusal names its line ([`HeaderReader::error`]).
    fn read_header(
        version_line: usize,
        version: &str,
        r: &mut HeaderReader<'_>,
    ) -> Result<Head<Self>, RecordingError>;

    /// Whether the game's part can be written and read back. Called before the shared checks.
    fn validate(rec: &Recording<Self>) -> Result<(), String> {
        let _ = rec;
        Ok(())
    }

    /// The game's tokens on frame `frame`'s line. Any forces a line at that frame.
    fn frame_tokens(header: &Self::Header, frame: u32) -> Vec<&'static str> {
        let _ = (header, frame);
        Vec::new()
    }

    /// The game's tokens read on a line for frame `frame` (none is an empty slice), into
    /// `header`; `version` is the first line's.
    fn read_frame_tokens(
        header: &mut Self::Header,
        version: &str,
        frame: u32,
        tokens: &[&str],
    ) -> Result<(), String> {
        let _ = (header, version, frame, tokens);
        Ok(())
    }

    /// The recording stopped at `at`, after its last step.
    fn stop(rec: &mut Recording<Self>, at: Self::Stop);
}

/// One recording.
pub struct Recording<G: GameRecording> {
    /// The engine build that made it; informational, never refused.
    pub build: String,
    /// The random state the start boots with, in the game's byte order.
    pub seed: Vec<u8>,
    /// The owner's one-line note; may be empty.
    pub note: String,
    /// The game's header.
    pub header: G::Header,
    /// One per engine step from the start's frame 0. Held expanded: the app appends one a step.
    pub inputs: Vec<FrameInput>,
}

/// A recording reads as its game's header too (`rec.stop_point`), so a game's code names its
/// fields as it did before the shared half moved out.
impl<G: GameRecording> Deref for Recording<G> {
    type Target = G::Header;
    fn deref(&self) -> &G::Header {
        &self.header
    }
}

impl<G: GameRecording> DerefMut for Recording<G> {
    fn deref_mut(&mut self) -> &mut G::Header {
        &mut self.header
    }
}

// By hand, not derived: a derive would ask `G` itself, a marker type, for each.
impl<G: GameRecording> Clone for Recording<G> {
    fn clone(&self) -> Self {
        Recording {
            build: self.build.clone(),
            seed: self.seed.clone(),
            note: self.note.clone(),
            header: self.header.clone(),
            inputs: self.inputs.clone(),
        }
    }
}

impl<G: GameRecording> PartialEq for Recording<G> {
    fn eq(&self, o: &Self) -> bool {
        (&self.build, &self.seed, &self.note, &self.header, &self.inputs)
            == (&o.build, &o.seed, &o.note, &o.header, &o.inputs)
    }
}

impl<G: GameRecording> Eq for Recording<G> {}

impl<G: GameRecording> fmt::Debug for Recording<G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Recording")
            .field("build", &self.build)
            .field("seed", &self.seed)
            .field("note", &self.note)
            .field("header", &self.header)
            .field("inputs", &self.inputs.len())
            .finish()
    }
}

/// A header as [`GameRecording::read_header`] reads it: the shared fields and the game's.
pub struct Head<G: GameRecording> {
    pub build: String,
    pub seed: Vec<u8>,
    /// How many inputs the recording holds, at most [`MAX_FRAMES`].
    pub frames: u32,
    pub note: String,
    pub header: G::Header,
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

/// The header's lines as they are written.
pub struct HeaderWriter {
    out: String,
}

impl HeaderWriter {
    /// One whole line.
    pub fn line(&mut self, l: &str) {
        self.out.push_str(l);
        self.out.push('\n');
    }

    /// `key value`, or the bare key when the value is empty (so no line ends in a space).
    pub fn field(&mut self, key: &str, value: &str) {
        match value.is_empty() {
            true => self.line(key),
            false => self.line(&format!("{key} {value}")),
        }
    }

    pub fn build(&mut self, build: &str) {
        self.field("build", build);
    }

    pub fn frames(&mut self, frames: usize) {
        self.field("frames", &frames.to_string());
    }

    pub fn note(&mut self, note: &str) {
        self.field("note", note);
    }
}

/// The header's lines as they are read, one field at a time in the order written.
pub struct HeaderReader<'a> {
    lines: core::iter::Enumerate<core::str::Lines<'a>>,
    /// How many lines the text has: a field missing at the end is missing on the line after.
    last: usize,
}

impl<'a> HeaderReader<'a> {
    fn new(text: &'a str) -> HeaderReader<'a> {
        HeaderReader {
            lines: text.lines().enumerate(),
            last: text.lines().count(),
        }
    }

    /// A refusal at line `line`.
    pub fn error(line: usize, why: String) -> RecordingError {
        RecordingError { line, why }
    }

    /// The next line, which must be `key` alone or `key value`: its number and the value (empty
    /// for the bare key).
    pub fn next(&mut self, key: &str) -> Result<(usize, &'a str), RecordingError> {
        let missing = |line: usize| RecordingError {
            line,
            why: format!("missing `{key}`, which comes next in the header"),
        };
        let (n, l) = self
            .lines
            .next()
            .map(|(i, l)| (i + 1, l))
            .ok_or_else(|| missing(self.last + 1))?;
        if l == key {
            return Ok((n, ""));
        }
        l.strip_prefix(key)
            .and_then(|r| r.strip_prefix(' '))
            .map(|r| (n, r))
            .ok_or_else(|| missing(n))
    }

    /// The next line, `key on` or `key off`.
    pub fn flag(&mut self, key: &str) -> Result<bool, RecordingError> {
        match self.next(key)? {
            (_, "on") => Ok(true),
            (_, "off") => Ok(false),
            (n, v) => Err(Self::error(n, format!("{key} `{v}` is not on or off"))),
        }
    }

    /// The `build` line's value.
    pub fn build(&mut self) -> Result<String, RecordingError> {
        Ok(self.next("build")?.1.to_string())
    }

    /// The `frames` line's count, refused past [`MAX_FRAMES`].
    pub fn frames(&mut self) -> Result<u32, RecordingError> {
        let (n, frames) = self.next("frames")?;
        let frames: u32 = number(frames).ok_or_else(|| {
            Self::error(n, format!("frames `{frames}` is not a frame count"))
        })?;
        if frames > MAX_FRAMES {
            return Err(Self::error(
                n,
                format!("frames {frames} is past the longest recording, {MAX_FRAMES} (a day)"),
            ));
        }
        Ok(frames)
    }

    /// The `note` line's value.
    pub fn note(&mut self) -> Result<String, RecordingError> {
        Ok(self.next("note")?.1.to_string())
    }
}

impl<G: GameRecording> Recording<G> {
    /// Whether the recording can be written and read back: the game's part
    /// ([`GameRecording::validate`]), at most [`MAX_FRAMES`] inputs, and the build and note each
    /// one line.
    pub fn validate(&self) -> Result<(), String> {
        G::validate(self)?;
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

    /// The recording as text: the first line, the header in the game's order, then an input line
    /// wherever the input changes or the game marks a frame, the first at frame 0.
    pub fn to_text(&self) -> String {
        let mut w = HeaderWriter { out: String::new() };
        w.line(&format!("{} {}", G::MAGIC, G::VERSION));
        G::write_header(self, &mut w);
        w.line("inputs");
        let mut held = None;
        for (frame, &input) in self.inputs.iter().enumerate() {
            let marks = G::frame_tokens(&self.header, frame as u32);
            if held != Some(input) || !marks.is_empty() {
                let tokens = match (marks.is_empty(), input == FrameInput::NONE) {
                    (true, _) => format_tokens(input),
                    (false, true) => marks.join(" "),
                    (false, false) => format!("{} {}", format_tokens(input), marks.join(" ")),
                };
                w.line(&format!("{frame}: {tokens}"));
                held = Some(input);
            }
        }
        w.out
    }

    /// Reads [`Recording::to_text`]'s format back. Every refusal names its line; never a panic.
    pub fn parse(text: &str) -> Result<Self, RecordingError> {
        let err = HeaderReader::error;
        let mut r = HeaderReader::new(text);
        let (n, version) = r.next(G::MAGIC)?;
        let head = G::read_header(n, version, &mut r)?;
        let (n, rest) = r.next("inputs")?;
        if !rest.is_empty() {
            return Err(err(n, "`inputs` stands alone on its line".into()));
        }
        let frames = head.frames;
        let mut header = head.header;
        let mut inputs = Vec::with_capacity(frames as usize);
        let (mut held, mut prev) = (FrameInput::NONE, None);
        for (n, raw) in r.lines.map(|(i, l)| (i + 1, l)) {
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
            let (marks, held_tokens): (Vec<&str>, Vec<&str>) =
                tokens.split_whitespace().partition(|t| G::TOKENS.contains(t));
            G::read_frame_tokens(&mut header, version, frame, &marks).map_err(|e| err(n, e))?;
            let input = parse_tokens(&held_tokens.join(" ")).map_err(|e| err(n, e))?;
            inputs.resize(frame as usize, held);
            inputs.push(input);
            held = input;
            prev = Some(frame);
        }
        inputs.resize(frames as usize, held);
        Ok(Recording {
            build: head.build,
            seed: head.seed,
            note: head.note,
            header,
            inputs,
        })
    }
}

/// Decimal digits only (`parse` alone would take a leading `+`).
fn number<T: core::str::FromStr>(s: &str) -> Option<T> {
    (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .then(|| s.parse().ok())
        .flatten()
}
