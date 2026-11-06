//! Input scripts: what the player holds, frame by frame. Inputs only, never disk data.
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
//! held. Frames count from the case's start and must strictly increase. Tokens are the shared ones
//! (`amiga_input::parse_token`: `U D L R F` for the port-2 joystick, `M` for the port-1 mouse
//! button, `K<hex>` for a raw key code 00..7F, `-` or nothing for nothing), and whatever a game's
//! hook takes before them ([`parse_with`]).

use std::fmt;

use amiga_input::{FrameInput, KeySet, format_tokens, parse_token};

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

/// A script of the shared tokens only.
pub fn parse(text: &str) -> Result<InputScript, ScriptError> {
    parse_with(text, |_| None)
}

/// A script whose tokens `game_token` sees first: `None` for a token that is not the game's (it
/// is then read as a shared token), else the input it holds or why it is wrong.
pub fn parse_with(
    text: &str,
    game_token: impl Fn(&str) -> Option<Result<FrameInput, String>>,
) -> Result<InputScript, ScriptError> {
    parse_with_forms(text, "", game_token)
}

/// [`parse_with`], naming the game's token forms in the error for a token nobody takes: `forms`
/// goes after `K<hex>` in `(U D L R F M K<hex> -)`, with its own leading space.
pub fn parse_with_forms(
    text: &str,
    forms: &str,
    game_token: impl Fn(&str) -> Option<Result<FrameInput, String>>,
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
            let one = game_token(t)
                .or_else(|| parse_token(t))
                .ok_or_else(|| err(format!("`{t}` is not an input (U D L R F M K<hex>{forms} -)")))?
                .map_err(err)?;
            input.joy = input.joy | one.joy;
            input.keys = KeySet(input.keys.0 | one.keys.0);
        }
        lines.push((frame, input));
    }
    Ok(InputScript { lines })
}

/// What `lines` hold on `frame`.
pub fn input_at(lines: &[(u32, FrameInput)], frame: u32) -> FrameInput {
    match lines.partition_point(|l| l.0 <= frame) {
        0 => FrameInput::NONE,
        i => lines[i - 1].1,
    }
}

/// `lines` as played from their frame `from` on: frame 0 holds what `from` held.
pub fn from_frame(lines: &[(u32, FrameInput)], from: u32) -> Vec<(u32, FrameInput)> {
    let mut out = vec![(0, input_at(lines, from))];
    out.extend(lines.iter().filter(|l| l.0 > from).map(|&(f, i)| (f - from, i)));
    out
}

impl InputScript {
    /// Nothing held, ever.
    pub fn idle() -> InputScript {
        InputScript::default()
    }

    /// What is held on `frame`.
    pub fn input_at(&self, frame: u32) -> FrameInput {
        input_at(&self.lines, frame)
    }

    /// The script as played from its frame `from` on: its frame 0 holds what `from` held.
    pub fn from_frame(&self, from: u32) -> InputScript {
        InputScript { lines: from_frame(&self.lines, from) }
    }

    /// A recording's inputs, one a step, as a script of steps: a line where the input changes.
    pub fn from_steps(inputs: &[FrameInput]) -> InputScript {
        let mut s = InputScript::default();
        for (k, &i) in inputs.iter().enumerate() {
            if s.lines.last().map(|l| l.1) != Some(i) {
                s.lines.push((k as u32, i));
            }
        }
        s
    }

    /// The script as its text: one `frame: tokens` line each ([`InputScript::format_input`]),
    /// which [`parse`] reads back.
    pub fn to_text(&self) -> String {
        self.lines.iter().map(|&(f, i)| format!("{f}: {}\n", InputScript::format_input(i))).collect()
    }

    /// One input as a script writes it: `R F`, `K19`, `-` ([`amiga_input::format_tokens`]).
    pub fn format_input(input: FrameInput) -> String {
        format_tokens(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amiga_input::Joy;

    #[test]
    fn holds_and_releases_last_until_the_next_line() {
        let s = parse("# walk right, jump, let go\n120: R F   # right and fire\n150: R\n\n200: -\n")
            .unwrap();
        assert_eq!(s.input_at(119), FrameInput::NONE);
        assert_eq!(s.input_at(149).joy, Joy::RIGHT | Joy::FIRE);
        assert_eq!(s.input_at(150).joy, Joy::RIGHT);
        assert_eq!(s.input_at(1_000_000), FrameInput::NONE);
        assert_eq!(s.lines.len(), 3);
        assert_eq!(InputScript::format_input(s.input_at(120)), "R F");
        let from = s.from_frame(130);
        assert_eq!(from.lines, vec![(0, s.input_at(130)), (20, s.input_at(150)), (70, FrameInput::NONE)]);
    }

    #[test]
    fn steps_become_lines_where_the_input_changes() {
        let r = FrameInput { joy: Joy::RIGHT, ..FrameInput::NONE };
        let s = InputScript::from_steps(&[FrameInput::NONE, r, r, FrameInput::NONE]);
        assert_eq!(s.lines, vec![(0, FrameInput::NONE), (1, r), (3, FrameInput::NONE)]);
        assert_eq!(s.to_text(), "0: -\n1: R\n3: -\n");
        assert_eq!(parse(&s.to_text()).unwrap(), s);
        assert_eq!(InputScript::from_steps(&[]).lines, vec![]);
    }

    #[test]
    fn bad_lines_say_why_and_where() {
        for (text, says) in [
            ("R F", "expected"),
            ("x: R", "not a frame number"),
            ("1: Q", "`Q` is not an input (U D L R F M K<hex> -)"),
            ("1: K80", "00..7F"),
        ] {
            let e = parse(text).unwrap_err();
            assert!(e.why.contains(says), "{text}: {e}");
            assert_eq!(e.line, 1);
        }
        let e = parse("10: R\n# x\n10: L\n").unwrap_err();
        assert_eq!(e.line, 3);
        assert!(e.why.contains("strictly increase"), "{e}");
    }

    /// A game's token: `Z<n>` holds raw key `n` (a stand-in for a game's own form); the hook sees
    /// a token before the shared ones, and its error carries the line.
    #[test]
    fn a_games_token_goes_through_the_hook() {
        let hook = |t: &str| {
            t.strip_prefix('Z').map(|n| {
                n.parse::<u8>().ok().filter(|&k| k < 0x80).map(FrameInput::key).ok_or(format!("`{t}`: Z<key>"))
            })
        };
        let s = parse_with("1: Z25 F\n", hook).unwrap();
        assert_eq!(s.input_at(1).keys, KeySet::key(25));
        assert_eq!(s.input_at(1).joy, Joy::FIRE);
        let e = parse_with("1: -\n2: Zx\n", hook).unwrap_err();
        assert_eq!((e.line, e.why.as_str()), (2, "`Zx`: Z<key>"));
        // Without the hook, the shared tokens alone.
        assert!(parse("1: Z25\n").is_err());
        let e = parse_with_forms("1: Q\n", " Z<key>", hook).unwrap_err();
        assert_eq!(e.why, "`Q` is not an input (U D L R F M K<hex> Z<key> -)");
    }
}
