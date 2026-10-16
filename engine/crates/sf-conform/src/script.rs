//! Input scripts: what the player holds, frame by frame (DESIGN §4). Committed under
//! `engine/conform/scripts/`; inputs only, never disk data.
//!
//! ```text
//! # comment to the end of the line
//! 0: -            nothing held from frame 0
//! 120: R F        right and fire held from frame 120 (until the next line)
//! 150: R          fire released, right still held
//! 200: K19        raw key 0x19 (P) held
//! 210: -          everything released
//! ```
//!
//! A line sets everything held from its frame until the next line; before the first line nothing is
//! held. Frames count from the case's sync point and must strictly increase. Tokens: `U D L R F`
//! for the port-2 joystick (up is jump), `M` for the port-1 mouse button (the crack intro waits for
//! it), `K<hex>` for a raw key code 00..7F, `C<entry>.<k>` for the raw key of digit `k` (0..5) of
//! `password_table` entry `entry` (0..27, h0+$24dc), `-` (or nothing) for nothing. A level code's
//! digits are the player's disks' (DESIGN §2.3): `C` reads them from the disks when the script is
//! parsed, so a script types a code without holding it.

use std::fmt;
use std::path::Path;

use sf_core::input::{FrameInput, Joy};

/// `password_table` (h0+$24dc): 28 entries of six ASCII digits and a handler.
const PASSWORD_TABLE: u32 = 0x24dc;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InputScript {
    /// (first frame, what is held from it), frames strictly increasing.
    pub lines: Vec<(u32, FrameInput)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptError {
    /// 1-based line in the text.
    pub line: usize,
    pub why: String,
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.why)
    }
}

impl std::error::Error for ScriptError {}

impl InputScript {
    /// Nothing held, ever.
    pub fn idle() -> InputScript {
        InputScript::default()
    }

    pub fn parse(text: &str) -> Result<InputScript, ScriptError> {
        InputScript::parse_with(text, &|entry, k| {
            crate::display_data()
                .front
                .h0_b(PASSWORD_TABLE + 10 * entry + k)
        })
    }

    /// [`InputScript::parse`], `digit(entry, k)` giving the ASCII digit `k` of a `password_table`
    /// entry (read only for a `C` token).
    pub fn parse_with(
        text: &str,
        digit: &dyn Fn(u32, u32) -> u8,
    ) -> Result<InputScript, ScriptError> {
        let mut lines: Vec<(u32, FrameInput)> = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let line = i + 1;
            let err = |why: String| ScriptError { line, why };
            let body = raw.split('#').next().unwrap_or("").trim();
            if body.is_empty() {
                continue;
            }
            let (frame, tokens) = body
                .split_once(':')
                .ok_or_else(|| err(format!("`{body}`: expected `<frame>: <inputs>`")))?;
            let frame = frame.trim();
            let frame: u32 = frame
                .bytes()
                .all(|b| b.is_ascii_digit())
                .then(|| frame.parse().ok())
                .flatten()
                .ok_or_else(|| err(format!("`{frame}` is not a frame number")))?;
            if let Some(&(prev, _)) = lines.last()
                && frame <= prev
            {
                return Err(err(format!(
                    "frame {frame} comes after frame {prev}: frames must strictly increase"
                )));
            }
            let mut input = FrameInput::NONE;
            for t in tokens.split_whitespace() {
                match t {
                    "-" => {}
                    "U" => input.joy = input.joy | Joy::UP,
                    "D" => input.joy = input.joy | Joy::DOWN,
                    "L" => input.joy = input.joy | Joy::LEFT,
                    "R" => input.joy = input.joy | Joy::RIGHT,
                    "F" => input.joy = input.joy | Joy::FIRE,
                    "M" => input.joy = input.joy | Joy::PORT1_BUTTON,
                    k if k.starts_with('K') => {
                        let hex = &k[1..];
                        let code = (hex.len() == 2 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
                            .then(|| u8::from_str_radix(hex, 16).ok())
                            .flatten()
                            .filter(|&c| c < 0x80)
                            .ok_or_else(|| {
                                err(format!("`{k}`: a key is K and two hex digits, 00..7F"))
                            })?;
                        input.keys = input.keys.with(code);
                    }
                    c if c.starts_with('C') => {
                        let bad = || {
                            err(format!(
                                "`{c}`: a code digit is C<entry 0..27>.<digit 0..5>"
                            ))
                        };
                        let (e, k) = c[1..].split_once('.').ok_or_else(bad)?;
                        let e: u32 = e.parse().ok().filter(|&e| e < 28).ok_or_else(bad)?;
                        let k: u32 = k.parse().ok().filter(|&k| k < 6).ok_or_else(bad)?;
                        // The number keys' raw codes: 1..9 are 01..09, 0 is 0A.
                        let code = match digit(e, k) {
                            b'0' => 0x0a,
                            d @ b'1'..=b'9' => d - b'0',
                            _ => {
                                return Err(err(format!(
                                    "`{c}`: password_table entry {e} is not digits"
                                )));
                            }
                        };
                        input.keys = input.keys.with(code);
                    }
                    other => {
                        return Err(err(format!(
                            "`{other}` is not an input (U D L R F M K<hex> C<entry>.<k> -)"
                        )));
                    }
                }
            }
            lines.push((frame, input));
        }
        Ok(InputScript { lines })
    }

    pub fn load(path: &Path) -> Result<InputScript, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        InputScript::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// What is held on `frame`.
    pub fn input_at(&self, frame: u32) -> FrameInput {
        match self.lines.partition_point(|l| l.0 <= frame) {
            0 => FrameInput::NONE,
            i => self.lines[i - 1].1,
        }
    }

    /// The same script for the oracle's trace cache.
    pub fn to_oracle(&self) -> sf_oracle::InputScript {
        sf_oracle::InputScript::new(self.lines.clone())
    }

    /// One input as a script writes it: `R F`, `K19`, `-`.
    pub fn format_input(input: FrameInput) -> String {
        let names = [
            (Joy::UP, "U"),
            (Joy::DOWN, "D"),
            (Joy::LEFT, "L"),
            (Joy::RIGHT, "R"),
            (Joy::FIRE, "F"),
            (Joy::PORT1_BUTTON, "M"),
        ];
        let mut out: Vec<String> = names
            .iter()
            .filter(|(j, _)| input.joy.contains(*j))
            .map(|(_, n)| n.to_string())
            .collect();
        out.extend(input.keys.codes().map(|c| format!("K{c:02X}")));
        if out.is_empty() {
            "-".to_string()
        } else {
            out.join(" ")
        }
    }
}
