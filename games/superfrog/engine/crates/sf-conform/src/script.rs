//! Input scripts: what the player holds, frame by frame (DESIGN §4). Committed under
//! `games/superfrog/engine/conform/scripts/`; inputs only, never disk data.
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
//! parsed, so a script types a code without holding it. The lexer is `amiga_conform::script`'s;
//! `C` is Superfrog's token, through its hook.

use std::path::Path;

use sf_core::input::FrameInput;

// The lexer and its error moved to the toolkit (toolkit-extract T06); Superfrog's `C` token is its
// hook.
pub use amiga_conform::script::ScriptError;

/// `password_table` (h0+$24dc): 28 entries of six ASCII digits and a handler.
const PASSWORD_TABLE: u32 = 0x24dc;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InputScript {
    /// (first frame, what is held from it), frames strictly increasing.
    pub lines: Vec<(u32, FrameInput)>,
}

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
        amiga_conform::script::parse_with_forms(text, " C<entry>.<k>", |t| {
            t.strip_prefix('C').map(|c| code_digit(t, c, digit))
        })
        .map(|s| InputScript { lines: s.lines })
    }

    pub fn load(path: &Path) -> Result<InputScript, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        InputScript::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// What is held on `frame`.
    pub fn input_at(&self, frame: u32) -> FrameInput {
        amiga_conform::script::input_at(&self.lines, frame)
    }

    /// The script as played from its frame `from` on: its frame 0 holds what `from` held.
    pub fn from_frame(&self, from: u32) -> InputScript {
        InputScript { lines: amiga_conform::script::from_frame(&self.lines, from) }
    }

    /// The same script for the oracle's trace cache.
    pub fn to_oracle(&self) -> sf_oracle::InputScript {
        sf_oracle::InputScript::new(self.lines.clone())
    }

    /// One input as a script writes it: `R F`, `K19`, `-` ([`sf_core::input::format_tokens`]).
    pub fn format_input(input: FrameInput) -> String {
        amiga_conform::script::InputScript::format_input(input)
    }
}

/// Token `t`, `C` then `c`: the raw key of digit `k` of `password_table` entry `e`, `c` being
/// `<e>.<k>`.
fn code_digit(t: &str, c: &str, digit: &dyn Fn(u32, u32) -> u8) -> Result<FrameInput, String> {
    let bad = || format!("`{t}`: a code digit is C<entry 0..27>.<digit 0..5>");
    let (e, k) = c.split_once('.').ok_or_else(bad)?;
    let e: u32 = e.parse().ok().filter(|&e| e < 28).ok_or_else(bad)?;
    let k: u32 = k.parse().ok().filter(|&k| k < 6).ok_or_else(bad)?;
    // The number keys' raw codes: 1..9 are 01..09, 0 is 0A.
    let code = match digit(e, k) {
        b'0' => 0x0a,
        d @ b'1'..=b'9' => d - b'0',
        _ => return Err(format!("`{t}`: password_table entry {e} is not digits")),
    };
    Ok(FrameInput::key(code))
}
