//! `--script FILE`: host events by frame, fed in where winit's are translated (`input.rs`), so
//! everything after that point is the real code path (DESIGN §5, the script layer).
//!
//! One directive per line; `#` starts a comment. `N:` is the frame the event lands before: the
//! game's step N (counting from 1) sees it.
//!
//! ```text
//! seed 1 2              # the power-on RNG seed (default 0 0 under a script)
//! 10: down ArrowLeft    # a key goes down (names: input.rs KEYS)
//! 20: up ArrowLeft
//! 30: press Space       # down, and up again before the next frame
//! 40: mouse down        # the left mouse button
//! 41: mouse up
//! 45: blur              # the window loses focus
//! 50: hang              # the main thread stops dead: proves --timeout
//! ```

use winit::keyboard::KeyCode;

use crate::input::{HostInput, key_named};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Input(HostInput),
    Hang,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Script {
    pub seed: Option<(u32, u32)>,
    /// `(frame, event)`, in file order; a frame's events are applied in that order.
    pub events: Vec<(u64, Event)>,
}

impl Script {
    pub fn parse(text: &str) -> Result<Script, String> {
        let mut s = Script::default();
        for (n, raw) in text.lines().enumerate() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let err = |why: &str| format!("script line {}: {why}: {raw:?}", n + 1);
            let words: Vec<&str> = line.split_whitespace().collect();
            if words[0] == "seed" {
                let num = |i: usize| words.get(i).and_then(|w| w.parse::<u32>().ok());
                match (num(1), num(2), words.len()) {
                    (Some(a), Some(b), 3) => s.seed = Some((a, b)),
                    _ => return Err(err("`seed` takes two whole numbers")),
                }
                continue;
            }
            let Some(frame) = words[0].strip_suffix(':').and_then(|f| f.parse::<u64>().ok()) else {
                return Err(err("expected `N: event` or `seed A B`"));
            };
            if frame == 0 {
                return Err(err("frames count from 1 (the first step)"));
            }
            let key = |name: Option<&&str>| -> Result<KeyCode, String> {
                let name = name.ok_or_else(|| err("a key name is missing"))?;
                key_named(name).ok_or_else(|| err(&format!("no key is named {name}")))
            };
            let events: Vec<Event> = match (words.get(1).copied(), words.len()) {
                (Some("down"), 3) => vec![key_event(key(words.get(2))?, true)],
                (Some("up"), 3) => vec![key_event(key(words.get(2))?, false)],
                (Some("press"), 3) => {
                    let k = key(words.get(2))?;
                    vec![key_event(k, true), key_event(k, false)]
                }
                (Some("mouse"), 3) if matches!(words[2], "down" | "up") => {
                    vec![Event::Input(HostInput::MouseLeft {
                        pressed: words[2] == "down",
                    })]
                }
                (Some("blur"), 2) => vec![Event::Input(HostInput::FocusLost)],
                (Some("hang"), 2) => vec![Event::Hang],
                _ => return Err(err("unknown event")),
            };
            s.events.extend(events.into_iter().map(|e| (frame, e)));
        }
        Ok(s)
    }

    /// The events that land before step `frame`, in order.
    pub fn at(&self, frame: u64) -> impl Iterator<Item = Event> + '_ {
        self.events
            .iter()
            .filter(move |(f, _)| *f == frame)
            .map(|(_, e)| *e)
    }
}

fn key_event(code: KeyCode, pressed: bool) -> Event {
    Event::Input(HostInput::Key { code, pressed })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_script_parses_into_events_by_frame() {
        let s = Script::parse(
            "# a comment\nseed 1 2\n10: down ArrowLeft  # held\n20: up ArrowLeft\n30: press Space\n\
             40: mouse down\n50: hang\n",
        )
        .unwrap();
        assert_eq!(s.seed, Some((1, 2)));
        assert_eq!(
            s.at(10).collect::<Vec<_>>(),
            vec![key_event(KeyCode::ArrowLeft, true)]
        );
        assert_eq!(
            s.at(30).collect::<Vec<_>>(),
            vec![key_event(KeyCode::Space, true), key_event(KeyCode::Space, false)]
        );
        assert_eq!(
            s.at(40).collect::<Vec<_>>(),
            vec![Event::Input(HostInput::MouseLeft { pressed: true })]
        );
        assert_eq!(s.at(50).collect::<Vec<_>>(), vec![Event::Hang]);
        assert_eq!(s.at(11).count(), 0);
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number() {
        for bad in ["10: down Nope", "x: hang", "0: hang", "5: jump", "seed 1", "5: down"] {
            let e = Script::parse(&format!("\n{bad}")).unwrap_err();
            assert!(e.contains("line 2"), "{bad}: {e}");
        }
    }
}
