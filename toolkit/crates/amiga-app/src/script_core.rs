//! The game-free half of an app's `--script FILE`: host events by frame, fed in where the window's
//! events are translated, so everything after that point is the real code path. A game's own
//! lines (its menu commands, its windows' answers) are its [`ScriptGame`]'s; this reads the rest.
//!
//! One directive per line; `#` starts a comment. `N:` is the frame the event lands before: the
//! game's step N (counting from 1) sees it.
//!
//! ```text
//! seed 1 2              # the power-on RNG seed
//! fast-until 11183      # steps up to 11183 run unpaced and silent, then the clock takes over
//! 10: down ArrowLeft    # a key goes down (names: keys.rs)
//! 20: up ArrowLeft
//! 30: press Space       # down, and up again before the next frame
//! 40: mouse down        # the left mouse button; also `mouse up`
//! 45: blur              # the window loses focus
//! 100: pad connect 1    # gamepad 1 plugged in (pad.rs; a script's run reads no real pad)
//! 101: pad down 1 South # a pad button (names: pad.rs); also `up`, `press`
//! 102: pad stick 1 0.8 -0.3  # the left stick's x and y, −1 to 1, up positive
//! 103: pad disconnect 1
//! ```

use winit::keyboard::KeyCode;

use crate::keys::key_named;
use crate::pad::{PadEvent, PadInput, StickAxis};

/// A host input event, from the window or from a script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostInput {
    Key { code: KeyCode, pressed: bool },
    /// The left mouse button over the window.
    MouseLeft { pressed: bool },
    /// The window lost focus. macOS sends no key-up for a key held across it (winit resets only
    /// the modifiers), so every key and the mouse are let go. A pad is not: it is not the
    /// window's.
    FocusLost,
    /// A gamepad's event (`pad.rs`).
    Pad(PadEvent),
}

/// What is wrong with a script, naming its line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptError(pub String);

impl std::fmt::Display for ScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<ScriptError> for String {
    fn from(e: ScriptError) -> String {
        e.0
    }
}

/// One line of a script with something on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line<'a> {
    /// Counting from 1.
    pub number: usize,
    /// As written, comment included.
    pub raw: &'a str,
    /// Without its comment, trimmed.
    pub text: &'a str,
    /// `text`'s words; never empty.
    pub words: Vec<&'a str>,
}

impl Line<'_> {
    /// `why` this line is refused: `script line 3: unknown event: "3: jump"`.
    pub fn error(&self, why: &str) -> ScriptError {
        ScriptError(format!("script line {}: {why}: {:?}", self.number, self.raw))
    }
}

/// Every line of `text` with something on it.
pub fn lines(text: &str) -> Vec<Line<'_>> {
    text.lines()
        .enumerate()
        .filter_map(|(n, raw)| {
            let text = raw.split('#').next().unwrap_or("").trim();
            let words: Vec<&str> = text.split_whitespace().collect();
            (!words.is_empty()).then_some(Line { number: n + 1, raw, text, words })
        })
        .collect()
}

/// A game's half of the script.
pub trait ScriptGame {
    /// What a frame's line becomes.
    type Event;

    /// A host input as the game's event.
    fn host(&self, input: HostInput) -> Self::Event;

    /// A line with no frame: `Ok(true)` when it is the game's and taken, `Ok(false)` when not.
    fn directive(&mut self, line: &Line) -> Result<bool, ScriptError>;

    /// A frame's event: `words` are the line's words after `N:`. `Ok(None)` when it is not the
    /// game's.
    fn event(&mut self, line: &Line, words: &[&str]) -> Result<Option<Vec<Self::Event>>, ScriptError>;

    /// After the last line: anything the game left open.
    fn finish(&mut self) -> Result<(), ScriptError> {
        Ok(())
    }
}

/// What the toolkit reads of a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreScript<E> {
    /// `seed A B`.
    pub seed: Option<(u32, u32)>,
    /// `fast-until N`: steps up to N run as fast as the machine allows, not on the clock, and make
    /// no sound; from N on the game is paced again from that moment.
    pub fast_until: Option<u64>,
    /// `(frame, event)`, in file order; a frame's events are applied in that order.
    pub events: Vec<(u64, E)>,
}

/// Read `text`, the game's lines through `game`.
pub fn parse_script_core<G: ScriptGame>(text: &str, game: &mut G) -> Result<CoreScript<G::Event>, ScriptError> {
    let mut s = CoreScript { seed: None, fast_until: None, events: Vec::new() };
    for line in lines(text) {
        let words = &line.words;
        if words[0] == "seed" {
            let num = |i: usize| words.get(i).and_then(|w| w.parse::<u32>().ok());
            match (num(1), num(2), words.len()) {
                (Some(a), Some(b), 3) => s.seed = Some((a, b)),
                _ => return Err(line.error("`seed` takes two whole numbers")),
            }
            continue;
        }
        if words[0] == "fast-until" {
            match (words.get(1).and_then(|w| w.parse::<u64>().ok()), words.len()) {
                (Some(n), 2) => s.fast_until = Some(n),
                _ => return Err(line.error("`fast-until` takes a step number")),
            }
            continue;
        }
        if game.directive(&line)? {
            continue;
        }
        let Some(frame) = words[0].strip_suffix(':').and_then(|f| f.parse::<u64>().ok()) else {
            return Err(line.error("expected `N: event` or `seed A B`"));
        };
        if frame == 0 {
            return Err(line.error("frames count from 1 (the first step)"));
        }
        let events = match host_events(&line, &words[1..])? {
            Some(inputs) => inputs.into_iter().map(|i| game.host(i)).collect(),
            None => match game.event(&line, &words[1..])? {
                Some(events) => events,
                None => return Err(line.error("unknown event")),
            },
        };
        s.events.extend(events.into_iter().map(|e| (frame, e)));
    }
    game.finish()?;
    Ok(s)
}

/// A frame's host input: `down|up|press KEY`, `mouse down|up`, `blur`, `pad …`. `Ok(None)` for
/// anything else.
pub fn host_events(line: &Line, words: &[&str]) -> Result<Option<Vec<HostInput>>, ScriptError> {
    let key = |name: Option<&&str>| -> Result<KeyCode, ScriptError> {
        let name = name.ok_or_else(|| line.error("a key name is missing"))?;
        key_named(name).ok_or_else(|| line.error(&format!("no key is named {name}")))
    };
    let k = |code, pressed| HostInput::Key { code, pressed };
    Ok(Some(match (words.first().copied(), words.len()) {
        (Some("down"), 2) => vec![k(key(words.get(1))?, true)],
        (Some("up"), 2) => vec![k(key(words.get(1))?, false)],
        (Some("press"), 2) => {
            let code = key(words.get(1))?;
            vec![k(code, true), k(code, false)]
        }
        (Some("mouse"), 2) if matches!(words[1], "down" | "up") => {
            vec![HostInput::MouseLeft { pressed: words[1] == "down" }]
        }
        (Some("blur"), 1) => vec![HostInput::FocusLost],
        (Some("pad"), 3..) => pad_events(&words[1..]).ok_or_else(|| line.error("bad `pad` event"))?,
        _ => return Ok(None),
    }))
}

/// A `pad …` line's events: `connect N`, `disconnect N`, `down|up|press N BUTTON`, `stick N X Y`.
fn pad_events(words: &[&str]) -> Option<Vec<HostInput>> {
    let pad: usize = words.get(1)?.parse().ok()?;
    let ev = HostInput::Pad;
    let button = |pressed| {
        let button = PadInput::named(words.get(2)?).filter(|b| b.is_button())?;
        Some(ev(PadEvent::Button { pad, button, pressed }))
    };
    let axis = |i: usize| -> Option<i16> {
        let v: f32 = words.get(i)?.parse().ok()?;
        (-1.0..=1.0).contains(&v).then(|| (v * 1000.0).round() as i16)
    };
    Some(match (words[0], words.len()) {
        ("connect", 2) => vec![ev(PadEvent::Connected(pad))],
        ("disconnect", 2) => vec![ev(PadEvent::Disconnected(pad))],
        ("down", 3) => vec![button(true)?],
        ("up", 3) => vec![button(false)?],
        ("press", 3) => vec![button(true)?, button(false)?],
        ("stick", 4) => vec![
            ev(PadEvent::Stick { pad, axis: StickAxis::X, value: axis(2)? }),
            ev(PadEvent::Stick { pad, axis: StickAxis::Y, value: axis(3)? }),
        ],
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up game: `N: jump` is its one event, `cheat` its one directive, and `open` must be
    /// closed by `close` before the script ends.
    #[derive(Debug, Default)]
    struct Game {
        cheats: usize,
        open: bool,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Ev {
        Host(HostInput),
        Jump,
    }

    impl ScriptGame for Game {
        type Event = Ev;
        fn host(&self, input: HostInput) -> Ev {
            Ev::Host(input)
        }
        fn directive(&mut self, line: &Line) -> Result<bool, ScriptError> {
            match line.words[..] {
                ["cheat"] => self.cheats += 1,
                ["cheat", ..] => return Err(line.error("`cheat` takes nothing")),
                ["open"] => self.open = true,
                ["close"] => self.open = false,
                _ => return Ok(false),
            }
            Ok(true)
        }
        fn event(&mut self, _: &Line, words: &[&str]) -> Result<Option<Vec<Ev>>, ScriptError> {
            Ok((words == ["jump"]).then(|| vec![Ev::Jump]))
        }
        fn finish(&mut self) -> Result<(), ScriptError> {
            if self.open { Err(ScriptError("script: `open` is not closed".into())) } else { Ok(()) }
        }
    }

    fn parse(text: &str) -> Result<(CoreScript<Ev>, Game), ScriptError> {
        let mut g = Game::default();
        parse_script_core(text, &mut g).map(|s| (s, g))
    }

    #[test]
    fn host_events_and_the_game_s_lines_land_on_their_frames_in_order() {
        let (s, g) = parse(
            "seed 1 2  # comment\n\nfast-until 40\ncheat\n10: press Space\n10: jump\n11: mouse down\n12: blur\n\
             13: pad stick 1 0.5 -1\n14: pad down 2 South\ncheat\n",
        )
        .unwrap();
        assert_eq!((s.seed, s.fast_until, g.cheats), (Some((1, 2)), Some(40), 2));
        let key = |pressed| Ev::Host(HostInput::Key { code: KeyCode::Space, pressed });
        assert_eq!(
            s.events,
            vec![
                (10, key(true)),
                (10, key(false)),
                (10, Ev::Jump),
                (11, Ev::Host(HostInput::MouseLeft { pressed: true })),
                (12, Ev::Host(HostInput::FocusLost)),
                (13, Ev::Host(HostInput::Pad(PadEvent::Stick { pad: 1, axis: StickAxis::X, value: 500 }))),
                (13, Ev::Host(HostInput::Pad(PadEvent::Stick { pad: 1, axis: StickAxis::Y, value: -1000 }))),
                (14, Ev::Host(HostInput::Pad(PadEvent::Button { pad: 2, button: PadInput::South, pressed: true }))),
            ]
        );
    }

    #[test]
    fn a_bad_line_is_refused_naming_its_number_and_why() {
        for (bad, why) in [
            ("seed 1", "`seed` takes two whole numbers"),
            ("fast-until soon", "`fast-until` takes a step number"),
            ("jump", "expected `N: event` or `seed A B`"),
            ("0: jump", "frames count from 1"),
            ("5: down NoSuchKey", "no key is named NoSuchKey"),
            ("5: fly", "unknown event"),
            ("5: down", "unknown event"),
            ("5: pad down 1 Stick", "bad `pad` event"),
            ("5: pad stick 1 2 0", "bad `pad` event"),
            ("5: pad", "unknown event"),
            ("cheat now", "`cheat` takes nothing"),
        ] {
            let e = parse(&format!("# first\n{bad}  # why\n")).unwrap_err().0;
            assert!(e.starts_with("script line 2: "), "{bad}: {e}");
            assert!(e.contains(why), "{bad}: {e}");
            assert!(e.ends_with(&format!("{:?}", format!("{bad}  # why"))), "{bad}: {e}");
        }
        assert_eq!(parse("open\n").unwrap_err().0, "script: `open` is not closed");
        assert!(parse("open\nclose\n").is_ok());
    }
}
