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
//! 75: occlude on         # the window cannot be drawn (covered, minimised, the screen locked) until
//!                       # `occlude off`: what macOS's withheld drawables do, on demand
//! 80: continue          # Game → Continue from World N (`menu continue`)
//! 90: menu pause        # a menu command, by its item's id (menu.rs `Command::id`, `-` as a space):
//!                       # import, continue, scale 1|2|3|auto, fullscreen, pause, reset, controls, quit,
//!                       # export W1-1 (any level id), play-level-file, save state 1..5, load state 1..5
//! 95: controls fire keys  # a click in the Controls window: an input's `keys` or `pad`, `defaults`,
//!                       # or `close` (its close button). While it is open, keys go to it.
//! 100: pad connect 1    # gamepad 1 plugged in (pad.rs; a script's run reads no real pad)
//! 101: pad down 1 South # a pad button (names: pad.rs PAD_INPUTS); also `up`, `press`
//! 102: pad stick 1 0.8 -0.3  # the left stick's x and y, −1 to 1, up positive
//! 103: pad disconnect 1
//! ```
//!
//! Before the game, the launch's questions (`import.rs`) take the script's answers in order:
//!
//! ```text
//! pick /d/one.zip; /d/two.zip   # what the disk picker returns (`;` between files)
//! pick cancel                   # the picker cancelled
//! answer choose                 # the missing-disks message: "Choose Disks…" (or `quit`)
//! ```
//!
//! and so do the panels of File → Export Level and Play Level File… (`levelfile.rs`), each its own
//! queue:
//!
//! ```text
//! save /tmp/out/W1-1.sflevel    # the export's save panel (or `save cancel`)
//! open /tmp/out/W1-1.sflevel    # Play Level File…'s open panel (or `open cancel`)
//! answer disk-level             # a refused level file: play its disk level (or `answer cancel`)
//! ```
//!
//! The bug recorder's (bug-recorder T06). `start-level` is Game ▸ Start Level… chosen at its frame
//! with its dialog answered so (`menu start-level` alone takes the next answer, none being cancel);
//! the other two answer their questions in order, whenever they are asked:
//!
//! ```text
//! 5: start-level W1-2 lives=7 points=1000 energy=3 difficulty=easy spud wings record
//!                               # any value left out is the dialog's default (W1-1's new game);
//!                               # `record` turns the recorder on; `5: start-level cancel`
//! answer-recording save         # "Recording in progress": `save`, `discard` or `cancel`
//! note black screen here        # the save window's note, and Save (`note` alone: an empty note;
//!                               # `note discard`: Discard)
//! 9: menu stop-recording        # Game ▸ Stop Recording, ⌘.
//! native-ui                     # the OS layer's: those two questions, the notices and the
//!                               # clipboard are the real windows, answered by real clicks and
//!                               # keys (bug-recorder T10); the log still says what was answered
//! ```
//!
//! The Start Level window driven as clicks drive it (bug-recorder T08): `form` lines, in order,
//! answer the next Game ▸ Start Level… (`menu start-level`) through the window's model, each a
//! change or a button, until a Start it takes or a Cancel. Start while Points is out of range is
//! refused (`superfrog: start-level: Start is disabled`) and the window stays open:
//!
//! ```text
//! form level W1-2               # also lives 3|5|7, energy 1..4, difficulty easy|normal
//! form points 1000000           # what the Points field holds (`form points` alone: empty)
//! form start                    # refused: out of range
//! form points 450000
//! form wings on                 # spud, wings, record: on|off (the recorder starts on)
//! form start                    # or `form cancel`
//! ```
//!
//! A `start-level` line is the same window filled in with its values and Start pressed.
//!
//! Test-only: `N: exit` sets the level's exit as reaching it does (`level_state` 1, as conformance
//! cases force it), so a test passes a level's end without a route that plays it.

use std::path::PathBuf;

use winit::keyboard::KeyCode;

use sf_core::recording::{Difficulty, LevelSetup, Lives};

use crate::controls_window::ControlsAction;
use crate::import::Pick;
use crate::input::{HostInput, key_named};
use crate::menu::Command;
use crate::recorder::EndChoice;
use crate::start_level_window::{FormAction, StartLevelForm};
use crate::pad::{PadEvent, PadInput, StickAxis};
use crate::settings::Scale;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Input(HostInput),
    Hang,
    /// The window cannot be drawn until `false` (a covered window, a locked screen).
    Occlude(bool),
    Resize(u32, u32),
    Shot,
    /// What a menu item does (`menu.rs`), as though it were chosen.
    Command(Command),
    /// A click in the Controls window, as though it were made.
    Controls(ControlsAction),
    /// Test-only: the level's exit set, as reaching it sets it.
    Exit,
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
    /// The export's save panel's answers, in order: `None` for cancelled.
    pub saves: Vec<Option<PathBuf>>,
    /// Play Level File…'s open panel's answers, in order.
    pub opens: Vec<Option<PathBuf>>,
    /// A refused level file's answers, in order: `true` for playing the disk level.
    pub level_answers: Vec<bool>,
    /// Start Level's answers, in order: what is done in the window each time it shows, ending
    /// in Start or Cancel (`start_level_window.rs`).
    pub starts: Vec<Vec<FormAction>>,
    /// "Recording in progress"'s answers, in order.
    pub endings: Vec<EndChoice>,
    /// The save window's answers, in order: the note, `None` for Discard.
    pub notes: Vec<Option<String>>,
    /// `native-ui`: the recorder's windows, the notices and the clipboard are the real ones.
    pub native_ui: bool,
}

impl Script {
    pub fn parse(text: &str) -> Result<Script, String> {
        let mut s = Script::default();
        // The `form` lines of the Start Level window not yet closed by a Start it takes or a
        // Cancel, run through its model to find where they end.
        let mut form: Option<(StartLevelForm, Vec<FormAction>)> = None;
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
                    (Some("disk-level"), 2) => s.level_answers.push(true),
                    (Some("cancel"), 2) => s.level_answers.push(false),
                    _ => return Err(err("`answer` takes `choose`, `quit`, `disk-level` or `cancel`")),
                }
                continue;
            }
            if line == "native-ui" {
                s.native_ui = true;
                continue;
            }
            if words[0] == "answer-recording" {
                s.endings.push(match (words.get(1).copied(), words.len()) {
                    (Some("save"), 2) => EndChoice::Save,
                    (Some("discard"), 2) => EndChoice::Discard,
                    (Some("cancel"), 2) => EndChoice::Cancel,
                    _ => return Err(err("`answer-recording` takes `save`, `discard` or `cancel`")),
                });
                continue;
            }
            if words[0] == "form" {
                let a = FormAction::named(&words[1..]).ok_or_else(|| {
                    err("`form` takes level ID, lives 3|5|7, points N, energy 1..4, difficulty easy|normal, \
                         spud|wings|record on|off, start or cancel")
                })?;
                let (model, actions) = form.get_or_insert_with(|| (StartLevelForm::new(), Vec::new()));
                actions.push(a.clone());
                if model.apply(a).is_some() {
                    s.starts.push(form.take().expect("open").1);
                }
                continue;
            }
            if words[0] == "note" {
                s.notes.push(match line["note".len()..].trim() {
                    "discard" => None,
                    note => Some(note.to_string()),
                });
                continue;
            }
            if words[0] == "save" || words[0] == "open" {
                let rest = line[words[0].len()..].trim();
                let answer = match rest {
                    "" => return Err(err("a path or `cancel` is missing")),
                    "cancel" => None,
                    p => Some(PathBuf::from(p)),
                };
                if words[0] == "save" {
                    s.saves.push(answer);
                } else {
                    s.opens.push(answer);
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
                (Some("occlude"), 3) => match words[2] {
                    "on" => vec![Event::Occlude(true)],
                    "off" => vec![Event::Occlude(false)],
                    _ => return Err(err("`occlude` takes `on` or `off`")),
                },
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
                (Some("exit"), 2) => vec![Event::Exit],
                (Some("start-level"), 3..) => {
                    if form.is_some() {
                        return Err(err("the `form` lines above are not closed by `form start` or `form cancel`"));
                    }
                    s.starts.push(match level_start(&words[2..]).map_err(|why| err(&why))? {
                        Some((setup, record)) => FormAction::filling(&setup, record),
                        None => vec![FormAction::Cancel],
                    });
                    vec![Event::Command(Command::StartLevel)]
                }
                (Some("controls"), 3..) => match ControlsAction::named(&words[2..]) {
                    Some(a) => vec![Event::Controls(a)],
                    None => return Err(err("`controls` takes an input and `keys` or `pad`, `defaults` or `close`")),
                },
                (Some("pad"), 4..) => pad_events(&words[2..]).ok_or_else(|| err("bad `pad` event"))?,
                (Some("menu"), 3..) => match Command::from_id(&words[2..].join("-")) {
                    Some(c) => vec![Event::Command(c)],
                    None => return Err(err("no menu command is named so")),
                },
                _ => return Err(err("unknown event")),
            };
            s.events.extend(events.into_iter().map(|e| (frame, e)));
        }
        if form.is_some() {
            return Err("script: the last `form` lines are not closed by `form start` or `form cancel`".into());
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

/// A `start-level` line's answer: `cancel`, or a level id then any of `lives=3|5|7`,
/// `points=N`, `energy=1..4`, `difficulty=easy|normal`, `spud`, `wings`, `record`.
fn level_start(words: &[&str]) -> Result<Option<(LevelSetup, bool)>, String> {
    if words == ["cancel"] {
        return Ok(None);
    }
    let mut setup = LevelSetup {
        level: words[0].parse()?,
        ..LevelSetup::default()
    };
    let mut record = false;
    for w in &words[1..] {
        let bad = || format!("`{w}`: start-level takes lives=3|5|7, points=N, energy=1..4, difficulty=easy|normal, spud, wings, record");
        match w.split_once('=') {
            None => match *w {
                "spud" => setup.spud = true,
                "wings" => setup.wings = true,
                "record" => record = true,
                _ => return Err(bad()),
            },
            Some(("lives", n)) => {
                setup.lives = match n {
                    "3" => Lives::Three,
                    "5" => Lives::Five,
                    "7" => Lives::Seven,
                    _ => return Err(bad()),
                }
            }
            Some(("points", n)) => setup.points = n.parse().map_err(|_| bad())?,
            Some(("energy", n)) => setup.energy = n.parse().map_err(|_| bad())?,
            Some(("difficulty", "easy")) => setup.difficulty = Difficulty::Easy,
            Some(("difficulty", "normal")) => setup.difficulty = Difficulty::Normal,
            _ => return Err(bad()),
        }
    }
    setup.validate()?;
    Ok(Some((setup, record)))
}

/// A `pad …` line's events: `connect N`, `disconnect N`, `down|up|press N BUTTON`, `stick N X Y`.
fn pad_events(words: &[&str]) -> Option<Vec<Event>> {
    let pad: usize = words.get(1)?.parse().ok()?;
    let ev = |e| Event::Input(HostInput::Pad(e));
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
             40: mouse down\n50: hang\n60: occlude on\n61: occlude off\n",
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
        assert_eq!(s.at(60).collect::<Vec<_>>(), vec![Event::Occlude(true)]);
        assert_eq!(s.at(61).collect::<Vec<_>>(), vec![Event::Occlude(false)]);
        assert!(Script::parse("1: occlude maybe\n").is_err());
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
    fn the_level_file_panels_and_their_menu_commands_parse() {
        let s = Script::parse(
            "save /t/My Levels/W1-1.sflevel\nsave cancel\nopen /t/a.sflevel\nopen cancel\n\
             answer disk-level\nanswer cancel\nanswer choose\n1: menu export W6-2alt\n2: menu play-level-file\n",
        )
        .unwrap();
        assert_eq!(s.saves, vec![Some("/t/My Levels/W1-1.sflevel".into()), None]);
        assert_eq!(s.opens, vec![Some("/t/a.sflevel".into()), None]);
        assert_eq!(s.level_answers, vec![true, false]);
        assert_eq!(s.answers, vec![true]);
        assert_eq!(
            s.at(1).collect::<Vec<_>>(),
            vec![Event::Command(Command::ExportLevel("W6-2alt".parse().unwrap()))]
        );
        assert_eq!(s.at(2).collect::<Vec<_>>(), vec![Event::Command(Command::PlayLevelFile)]);
    }

    #[test]
    fn controls_and_pad_lines_parse() {
        use crate::controls_window::Device;
        use crate::input::OriginalInput;
        let s = Script::parse(
            "1: menu controls\n2: controls fire keys\n3: controls up pad\n4: controls defaults\n\
             5: controls close\n6: pad connect 2\n7: pad press 2 South\n8: pad stick 2 -0.8 0.25\n\
             9: pad down 0 DPadLeft\n10: pad up 0 DPadLeft\n11: pad disconnect 2\n",
        )
        .unwrap();
        let at = |n| s.at(n).collect::<Vec<_>>();
        let pad = |e| Event::Input(HostInput::Pad(e));
        assert_eq!(at(1), vec![Event::Command(Command::Controls)]);
        assert_eq!(at(2), vec![Event::Controls(ControlsAction::Rebind(OriginalInput::Fire, Device::Keys))]);
        assert_eq!(at(3), vec![Event::Controls(ControlsAction::Rebind(OriginalInput::Up, Device::Pad))]);
        assert_eq!(at(4), vec![Event::Controls(ControlsAction::Defaults)]);
        assert_eq!(at(5), vec![Event::Controls(ControlsAction::Close)]);
        assert_eq!(at(6), vec![pad(PadEvent::Connected(2))]);
        assert_eq!(
            at(7),
            vec![
                pad(PadEvent::Button { pad: 2, button: PadInput::South, pressed: true }),
                pad(PadEvent::Button { pad: 2, button: PadInput::South, pressed: false }),
            ]
        );
        assert_eq!(
            at(8),
            vec![
                pad(PadEvent::Stick { pad: 2, axis: StickAxis::X, value: -800 }),
                pad(PadEvent::Stick { pad: 2, axis: StickAxis::Y, value: 250 }),
            ]
        );
        assert_eq!(at(9), vec![pad(PadEvent::Button { pad: 0, button: PadInput::DPadLeft, pressed: true })]);
        assert_eq!(at(10), vec![pad(PadEvent::Button { pad: 0, button: PadInput::DPadLeft, pressed: false })]);
        assert_eq!(at(11), vec![pad(PadEvent::Disconnected(2))]);
    }

    #[test]
    fn the_recorder_s_events_and_answers_parse() {
        let s = Script::parse(
            "5: start-level W1-2 lives=7 points=1000 energy=3 difficulty=easy spud wings record\n\
             6: start-level cancel\n7: start-level WB-1\n8: menu stop-recording\n9: menu start-level\n\
             answer-recording save\nanswer-recording discard\nanswer-recording cancel\n\
             note black screen # after the slot\nnote\nnote discard\n10: exit\n",
        )
        .unwrap();
        let at = |n| s.at(n).collect::<Vec<_>>();
        assert_eq!(at(5), vec![Event::Command(Command::StartLevel)]);
        assert_eq!(at(8), vec![Event::Command(Command::StopRecording)]);
        assert_eq!(at(9), vec![Event::Command(Command::StartLevel)]);
        assert_eq!(at(10), vec![Event::Exit]);
        let w1_2 = LevelSetup {
            level: "W1-2".parse().unwrap(),
            lives: Lives::Seven,
            points: 1000,
            energy: 3,
            difficulty: Difficulty::Easy,
            spud: true,
            wings: true,
        };
        let wb_1 = LevelSetup { level: "WB-1".parse().unwrap(), ..LevelSetup::default() };
        assert_eq!(
            s.starts,
            vec![FormAction::filling(&w1_2, true), vec![FormAction::Cancel], FormAction::filling(&wb_1, false)]
        );
        assert_eq!(s.endings, vec![EndChoice::Save, EndChoice::Discard, EndChoice::Cancel]);
        assert_eq!(s.notes, vec![Some("black screen".into()), Some(String::new()), None]);
        assert!(!s.native_ui);
        assert!(Script::parse("native-ui\n").unwrap().native_ui);
        assert!(Script::parse("native-ui now\n").is_err());
    }

    #[test]
    fn form_lines_answer_the_window_through_its_model_until_a_start_it_takes_or_a_cancel() {
        let s = Script::parse(
            "5: menu start-level\nform level W1-2\nform points 1000000\nform start\nform points 450000\n\
             form wings on\nform start\n9: menu start-level\nform record off\nform cancel\n\
             12: start-level W1-3\nform start\n",
        )
        .unwrap();
        let level = |id: &str| FormAction::Level(id.parse().unwrap());
        let points = |p: &str| FormAction::Points(p.into());
        assert_eq!(
            s.starts,
            vec![
                vec![level("W1-2"), points("1000000"), FormAction::Start, points("450000"), FormAction::Wings(true), FormAction::Start],
                vec![FormAction::Record(false), FormAction::Cancel],
                FormAction::filling(&LevelSetup { level: "W1-3".parse().unwrap(), ..LevelSetup::default() }, false),
                vec![FormAction::Start],
            ]
        );
        for bad in [
            "form",
            "form level W6-2alt",
            "form lives 4",
            "form energy 5",
            "form difficulty hard",
            "form wings yes",
            "form jump",
            "form points 1000000\nform start",
            "form level W1-2\n5: start-level W1-1",
        ] {
            let e = Script::parse(&format!("\n{bad}")).unwrap_err();
            assert!(e.contains("line 2") || e.contains("not closed"), "{bad}: {e}");
        }
    }

    #[test]
    fn a_start_level_out_of_the_dialog_s_range_is_refused() {
        for bad in [
            "5: start-level",
            "5: start-level W7-1",
            "5: start-level W6-2alt",
            "5: start-level W1-1 lives=4",
            "5: start-level W1-1 energy=0",
            "5: start-level W1-1 energy=5",
            "5: start-level W1-1 points=1000000",
            "5: start-level W1-1 points=-1",
            "5: start-level W1-1 difficulty=hard",
            "5: start-level W1-1 recorder",
            "5: start-level W1-1 lives",
            "answer-recording",
            "answer-recording keep",
            "5: exit now",
        ] {
            let e = Script::parse(&format!("\n{bad}")).unwrap_err();
            assert!(e.contains("line 2"), "{bad}: {e}");
        }
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
            "5: menu export W7-1",
            "save",
            "open",
            "answer play",
            "5: controls",
            "5: controls jump keys",
            "5: controls fire mouse",
            "5: pad connect",
            "5: pad connect x",
            "5: pad press 1 Nope",
            "5: pad press 1 StickUp",
            "5: pad stick 1 2 0",
            "5: pad stick 1 0.5",
            "5: pad wiggle 1",
        ] {
            let e = Script::parse(&format!("\n{bad}")).unwrap_err();
            assert!(e.contains("line 2"), "{bad}: {e}");
        }
    }
}
