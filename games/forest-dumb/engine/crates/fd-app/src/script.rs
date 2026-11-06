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
//!
//! The recorder's (T14). `menu start-level` and `menu stop-recording` are Game ▸ Start Level… and
//! Game ▸ Stop Recording at their frame. The windows they open are answered by lines that hold for
//! no frame, in order, whenever the window is shown; with none left, a window is cancelled (Start
//! Level), cancelled ("Recording in progress": the recording goes on) or discarded (the save window):
//!
//! ```text
//! form level W1-1               # the Start Level window as clicks drive it, until a `form start`
//! form record off               # it takes or a `form cancel` (the recorder starts on)
//! form start
//! answer-recording save         # "Recording in progress": `save`, `discard` or `cancel`
//! note black screen here        # the save window's note, and Save (`note` alone: an empty note;
//!                               # `note discard`: Discard)
//! ```

use std::ops::Deref;

use amiga_app::controls_window::ControlsAction;
use amiga_app::script_core::{
    CoreScript, HostInput, Line, ScriptError, ScriptGame, parse_script_core,
};
use amiga_app::start_level_window::{FieldAction, Form};

use crate::menu::Command;
use crate::recorder::{self, EndChoice};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Input(HostInput),
    Command(Command),
    Controls(ControlsAction),
    Shot,
    Resize(u32, u32),
    Hang,
}

/// The answers to the recorder's windows, in the order they are asked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answers {
    /// Each showing of the Start Level window: what is done in it, until Start or Cancel.
    pub starts: Vec<Vec<FieldAction>>,
    /// "Recording in progress".
    pub endings: Vec<EndChoice>,
    /// The save window: the note, or `None` for Discard.
    pub notes: Vec<Option<String>>,
}

/// A script: its events by frame and the windows' answers.
#[derive(Debug, Clone)]
pub struct Script {
    core: CoreScript<Event>,
    pub answers: Answers,
}

impl Deref for Script {
    type Target = CoreScript<Event>;
    fn deref(&self) -> &CoreScript<Event> {
        &self.core
    }
}

#[derive(Default)]
struct Game {
    answers: Answers,
    /// The `form` lines of a Start Level window not yet closed by a Start it takes or a Cancel,
    /// with the window's model, which says when that is.
    form: Option<(Form, Vec<FieldAction>)>,
}

impl ScriptGame for Game {
    type Event = Event;

    fn host(&self, input: HostInput) -> Event {
        Event::Input(input)
    }

    fn directive(&mut self, line: &Line) -> Result<bool, ScriptError> {
        let words = &line.words;
        match words.first().copied() {
            Some("form") => {
                let spec = recorder::spec();
                let a = FieldAction::named(&spec, &words[1..]).ok_or_else(|| {
                    line.error("`form` takes level W1-1, record on|off, start or cancel")
                })?;
                let (model, actions) = self
                    .form
                    .get_or_insert_with(|| (Form::new(spec), Vec::new()));
                actions.push(a.clone());
                if model.apply(a).is_some() {
                    let (_, actions) = self.form.take().expect("open");
                    self.answers.starts.push(actions);
                }
                Ok(true)
            }
            Some("answer-recording") => {
                self.answers.endings.push(match &words[1..] {
                    ["save"] => EndChoice::Save,
                    ["discard"] => EndChoice::Discard,
                    ["cancel"] => EndChoice::Cancel,
                    _ => {
                        return Err(line.error(
                            "`answer-recording` takes `save`, `discard` or `cancel`",
                        ));
                    }
                });
                Ok(true)
            }
            Some("note") => {
                self.answers.notes.push(match line.text["note".len()..].trim() {
                    "discard" => None,
                    note => Some(note.to_string()),
                });
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn finish(&mut self) -> Result<(), ScriptError> {
        match self.form {
            Some(_) => Err(ScriptError(
                "script: the last `form` lines are not closed by `form start` or `form cancel`"
                    .into(),
            )),
            None => Ok(()),
        }
    }

    fn event(&mut self, line: &Line, words: &[&str]) -> Result<Option<Vec<Event>>, ScriptError> {
        Ok(Some(vec![match words {
            ["menu", id] => Event::Command(
                Command::from_id(id)
                    .ok_or_else(|| line.error(&format!("no menu item is called {id}")))?,
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
    let mut g = Game::default();
    let core = parse_script_core(text, &mut g)?;
    Ok(Script {
        core,
        answers: g.answers,
    })
}

/// Step `n`'s events, in file order.
pub fn at(s: &Script, n: u64) -> impl Iterator<Item = Event> + '_ {
    s.events
        .iter()
        .filter(move |(f, _)| *f == n)
        .map(|(_, e)| *e)
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
                (
                    11,
                    Event::Controls(ControlsAction::Rebind(OriginalInput::Up, Device::Keys))
                ),
                (
                    12,
                    Event::Input(HostInput::Key {
                        code: KeyCode::KeyW,
                        pressed: true
                    })
                ),
                (
                    12,
                    Event::Input(HostInput::Key {
                        code: KeyCode::KeyW,
                        pressed: false
                    })
                ),
                (13, Event::Shot),
                (14, Event::Resize(640, 600)),
                (15, Event::Hang),
            ]
        );
        assert_eq!(at(&s, 12).count(), 2);
    }

    #[test]
    fn the_recorder_s_lines_answer_its_windows_in_order() {
        let s = parse(
            "1: menu start-level\nform record off\nform start\nform cancel\n\
             9: menu stop-recording\nanswer-recording save\nanswer-recording discard\n\
             answer-recording cancel\nnote black screen # here\nnote\nnote discard\n",
        )
        .unwrap();
        assert_eq!(
            s.events,
            vec![
                (1, Event::Command(Command::StartLevel)),
                (9, Event::Command(Command::StopRecording)),
            ]
        );
        let spec = recorder::spec();
        let record = spec.field("record").unwrap();
        assert_eq!(
            s.answers,
            Answers {
                starts: vec![
                    vec![
                        FieldAction::Set(
                            record,
                            amiga_app::start_level_window::Value::On(false)
                        ),
                        FieldAction::Start
                    ],
                    vec![FieldAction::Cancel],
                ],
                endings: vec![EndChoice::Save, EndChoice::Discard, EndChoice::Cancel],
                notes: vec![Some("black screen".into()), Some(String::new()), None],
            }
        );
    }

    #[test]
    fn bad_lines_name_themselves() {
        for (bad, why) in [
            ("5: menu start", "no menu item is called start"),
            ("form level W1-2", "`form` takes"),
            ("form record maybe", "`form` takes"),
            ("form record off", "not closed by `form start`"),
            ("answer-recording keep", "`answer-recording` takes"),
            ("5: controls jump keys", "bad `controls` event"),
            ("5: resize big 3", "`resize` takes"),
            ("5: fly", "unknown event"),
        ] {
            let e = parse(bad).unwrap_err().0;
            assert!(e.contains(why), "{bad}: {e}");
        }
    }
}
