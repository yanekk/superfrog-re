//! `--script FILE`: host events by frame, fed in where the window's are translated, so everything
//! after that point is the real code path. The game-free lines (`seed`, `fast-until`, keys, mouse,
//! `blur`, `pad …`) are `amiga_app::script_core`'s; this game adds:
//!
//! ```text
//! 10: menu controls      # a menu item, by its id (menu.rs)
//! 11: controls up keys   # a click in the Controls window: `up keys`, `fire pad`, `defaults`, `close`
//! 20: shot               # the window as presented, into the capture (`window-NNNNN.png`)
//! 30: resize 700 600     # ask for this inner size, in points
//! 40: hang               # stop answering for good: the watchdog's test
//! ```

use amiga_app::controls_window::ControlsAction;
use amiga_app::script_core::{CoreScript, HostInput, Line, ScriptError, ScriptGame, parse_script_core};

use crate::menu::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Input(HostInput),
    Command(Command),
    Controls(ControlsAction),
    Shot,
    Resize(u32, u32),
    Hang,
}

pub type Script = CoreScript<Event>;

struct Game;

impl ScriptGame for Game {
    type Event = Event;

    fn host(&self, input: HostInput) -> Event {
        Event::Input(input)
    }

    fn directive(&mut self, _: &Line) -> Result<bool, ScriptError> {
        Ok(false)
    }

    fn event(&mut self, line: &Line, words: &[&str]) -> Result<Option<Vec<Event>>, ScriptError> {
        Ok(Some(vec![match words {
            ["menu", id] => Event::Command(
                Command::from_id(id).ok_or_else(|| line.error(&format!("no menu item is called {id}")))?,
            ),
            ["controls", rest @ ..] => Event::Controls(
                ControlsAction::named(rest).ok_or_else(|| line.error("bad `controls` event"))?,
            ),
            ["shot"] => Event::Shot,
            ["resize", w, h] => match (w.parse(), h.parse()) {
                (Ok(w), Ok(h)) => Event::Resize(w, h),
                _ => return Err(line.error("`resize` takes a width and a height in points")),
            },
            ["hang"] => Event::Hang,
            _ => return Ok(None),
        }]))
    }
}

pub fn parse(text: &str) -> Result<Script, ScriptError> {
    parse_script_core(text, &mut Game)
}

/// Step `n`'s events, in file order.
pub fn at(s: &Script, n: u64) -> impl Iterator<Item = Event> + '_ {
    s.events.iter().filter(move |(f, _)| *f == n).map(|(_, e)| *e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use amiga_app::controls_window::Device;
    use amiga_app::keys::OriginalInput;
    use winit::keyboard::KeyCode;

    #[test]
    fn the_game_s_lines_and_the_toolkit_s_land_on_their_frames() {
        let s = parse("10: menu controls\n11: controls up keys\n12: press KeyW\n13: shot\n14: resize 640 600\n15: hang\n")
            .unwrap();
        assert_eq!(
            s.events,
            vec![
                (10, Event::Command(Command::Controls)),
                (11, Event::Controls(ControlsAction::Rebind(OriginalInput::Up, Device::Keys))),
                (12, Event::Input(HostInput::Key { code: KeyCode::KeyW, pressed: true })),
                (12, Event::Input(HostInput::Key { code: KeyCode::KeyW, pressed: false })),
                (13, Event::Shot),
                (14, Event::Resize(640, 600)),
                (15, Event::Hang),
            ]
        );
        assert_eq!(at(&s, 12).count(), 2);
    }

    #[test]
    fn bad_lines_name_themselves() {
        for (bad, why) in [
            ("5: menu start-level", "no menu item is called start-level"),
            ("5: controls jump keys", "bad `controls` event"),
            ("5: resize big 3", "`resize` takes"),
            ("5: fly", "unknown event"),
        ] {
            let e = parse(bad).unwrap_err().0;
            assert!(e.contains(why), "{bad}: {e}");
        }
    }
}
