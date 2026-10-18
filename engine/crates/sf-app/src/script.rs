//! `--script FILE`: host events by frame, fed in where winit's are translated (`input.rs`), so
//! everything after that point is the real code path (DESIGN §5, the script layer).
//!
//! One directive per line; `#` starts a comment. `N:` is the frame the event lands before: the
//! game's step N (counting from 1) sees it.
//!
//! ```text
//! seed 1 2              # the power-on RNG seed (default 0 0 under a script)
//! 10: down ArrowLeft    # a key goes down (names: input.rs HOST_KEYS)
//! 20: up ArrowLeft
//! 30: press Space       # down, and up again before the next frame
//! 40: mouse down        # the left mouse button
//! 41: mouse up
//! 45: blur              # the window loses focus
//! 50: hang              # the main thread stops dead: proves --timeout
//! 60: resize 700 600    # the window's inner size, in points, as a person dragging it
//! 61: scale 3           # the scale setting: a whole number or `auto` (`menu scale 3`)
//! 70: shot              # write the window as presented to the capture folder (window-NNNNN.png)
//! 80: continue          # Game → Continue from World N (`menu continue`)
//! 90: menu pause        # a menu command, by its item's id (menu.rs `Command::id`, `-` as a space):
//!                       # import, continue, scale 1|2|3|auto, fullscreen, pause, reset, quit,
//!                       # save state 1..5, load state 1..5
//! ```
//!
//! Before the game, the launch's questions (`import.rs`) take the script's answers in order:
//!
//! ```text
//! pick /d/one.zip; /d/two.zip   # what the disk picker returns (`;` between files)
//! pick cancel                   # the picker cancelled
//! answer choose                 # the missing-disks message: "Choose Disks…" (or `quit`)
//! ```

use std::path::PathBuf;

use winit::keyboard::KeyCode;

use crate::import::Pick;
use crate::input::{HostInput, key_named};
use crate::menu::Command;
use crate::settings::Scale;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Input(HostInput),
    Hang,
    Resize(u32, u32),
    Shot,
    /// What a menu item does (`menu.rs`), as though it were chosen.
    Command(Command),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Script {
    pub seed: Option<(u32, u32)>,
    /// `(frame, event)`, in file order; a frame's events are applied in that order.
    pub events: Vec<(u64, Event)>,
    /// The disk picker's answers, in order.
    pub picks: Vec<Pick>,
    /// The missing-disks message's answers, in order: `true` for "Choose Disks…".
    pub answers: Vec<bool>,
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
            if words[0] == "pick" {
                let rest = line["pick".len()..].trim();
                s.picks.push(match rest {
                    "cancel" => Pick::Cancel,
                    _ => Pick::Files(
                        rest.split(';')
                            .map(str::trim)
                            .filter(|p| !p.is_empty())
                            .map(PathBuf::from)
                            .collect(),
                    ),
                });
                continue;
            }
            if words[0] == "answer" {
                match (words.get(1).copied(), words.len()) {
                    (Some("choose"), 2) => s.answers.push(true),
                    (Some("quit"), 2) => s.answers.push(false),
                    _ => return Err(err("`answer` takes `choose` or `quit`")),
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
                (Some("resize"), 4) => {
                    let num = |i: usize| words[i].parse::<u32>().ok().filter(|&n| n > 0);
                    match (num(2), num(3)) {
                        (Some(w), Some(h)) => vec![Event::Resize(w, h)],
                        _ => return Err(err("`resize` takes a width and a height in points")),
                    }
                }
                (Some("scale"), 3) => vec![Event::Command(Command::Scale(match words[2] {
                    "auto" => Scale::Auto,
                    n => match n.parse::<u32>() {
                        Ok(n) if (1..=16).contains(&n) => Scale::N(n),
                        _ => return Err(err("`scale` takes `auto` or a whole number 1 to 16")),
                    },
                }))],
                (Some("shot"), 2) => vec![Event::Shot],
                (Some("continue"), 2) => vec![Event::Command(Command::ContinueFromProgress)],
                (Some("menu"), 3..) => match Command::from_id(&words[2..].join("-")) {
                    Some(c) => vec![Event::Command(c)],
                    None => return Err(err("no menu command is named so")),
                },
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
    fn window_events_and_the_launch_s_answers_parse() {
        let s = Script::parse(
            "pick /d/Disk 1.zip; /d/two.adf\npick cancel\npick\nanswer choose\nanswer quit\n\
             5: resize 700 600\n6: scale auto\n6: scale 3\n7: shot\n8: continue\n",
        )
        .unwrap();
        assert_eq!(
            s.picks,
            vec![
                Pick::Files(vec!["/d/Disk 1.zip".into(), "/d/two.adf".into()]),
                Pick::Cancel,
                Pick::Files(vec![]),
            ]
        );
        assert_eq!(s.answers, vec![true, false]);
        assert_eq!(s.at(5).collect::<Vec<_>>(), vec![Event::Resize(700, 600)]);
        assert_eq!(
            s.at(6).collect::<Vec<_>>(),
            vec![Event::Command(Command::Scale(Scale::Auto)), Event::Command(Command::Scale(Scale::N(3)))]
        );
        assert_eq!(s.at(7).collect::<Vec<_>>(), vec![Event::Shot]);
        assert_eq!(s.at(8).collect::<Vec<_>>(), vec![Event::Command(Command::ContinueFromProgress)]);
    }

    #[test]
    fn a_menu_event_names_its_command() {
        let s = Script::parse("1: menu pause
2: menu scale 2
3: menu scale auto
4: menu quit
5: menu import
")
            .unwrap();
        let at = |n| s.at(n).collect::<Vec<_>>();
        assert_eq!(at(1), vec![Event::Command(Command::Pause)]);
        assert_eq!(at(2), vec![Event::Command(Command::Scale(Scale::N(2)))]);
        assert_eq!(at(3), vec![Event::Command(Command::Scale(Scale::Auto))]);
        assert_eq!(at(4), vec![Event::Command(Command::Quit)]);
        assert_eq!(at(5), vec![Event::Command(Command::ImportDisks)]);
    }

    #[test]
    fn a_bad_line_is_refused_with_its_number() {
        for bad in [
            "10: down Nope",
            "x: hang",
            "0: hang",
            "5: jump",
            "seed 1",
            "5: down",
            "5: resize 0 10",
            "5: resize 10",
            "5: scale 17",
            "5: scale big",
            "answer maybe",
            "5: menu",
            "5: menu jump",
            "5: menu scale 0",
        ] {
            let e = Script::parse(&format!("\n{bad}")).unwrap_err();
            assert!(e.contains("line 2"), "{bad}: {e}");
        }
    }
}
