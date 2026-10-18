//! Gamepads (DESIGN §2.6): what a pad holds, which of the original's inputs each pad input feeds
//! (the [`PadMap`], remembered in `settings.toml`, DESIGN §2.9), and the source of pad events.
//!
//! A pad's events are [`PadEvent`]s: from gilrs in normal play ([`Gilrs`]), from a script's `pad …`
//! lines under `--script` (`script.rs`), so a scripted run never reads a real pad. Each pad is held
//! apart by its device number, so one unplugged mid-move lets go of what it held and nothing else
//! (DESIGN §2.6: a pad connected or disconnected changes only which device feeds the mapping).
//!
//! By default the d-pad and the left stick are the joystick's directions and A and B (the bottom
//! and right face buttons, gilrs's South and East) its fire button. The stick counts as a
//! direction once it is pushed more than halfway ([`STICK_THRESHOLD`]).

use std::collections::BTreeMap;

use crate::input::OriginalInput;

/// How far the stick must be pushed, in thousandths of full travel, to count as a direction:
/// half way. Anything at or inside it is the dead zone.
pub const STICK_THRESHOLD: i16 = 500;

/// A pad's button, or its left stick pushed one way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PadInput {
    South,
    East,
    West,
    North,
    LeftTrigger,
    LeftTrigger2,
    RightTrigger,
    RightTrigger2,
    Select,
    Start,
    Mode,
    LeftThumb,
    RightThumb,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    StickUp,
    StickDown,
    StickLeft,
    StickRight,
}

/// Every pad input, with its name in `settings.toml` and a script (gilrs's, `Stick…` for the left
/// stick) and the label the Controls window shows (an Xbox-layout pad's, as the 8BitDo's are).
pub const PAD_INPUTS: &[(PadInput, &str, &str)] = {
    use PadInput::*;
    &[
        (South, "South", "A"),
        (East, "East", "B"),
        (West, "West", "X"),
        (North, "North", "Y"),
        (LeftTrigger, "LeftTrigger", "LB"),
        (LeftTrigger2, "LeftTrigger2", "LT"),
        (RightTrigger, "RightTrigger", "RB"),
        (RightTrigger2, "RightTrigger2", "RT"),
        (Select, "Select", "Select"),
        (Start, "Start", "Start"),
        (Mode, "Mode", "Home"),
        (LeftThumb, "LeftThumb", "L3"),
        (RightThumb, "RightThumb", "R3"),
        (DPadUp, "DPadUp", "D-pad ↑"),
        (DPadDown, "DPadDown", "D-pad ↓"),
        (DPadLeft, "DPadLeft", "D-pad ←"),
        (DPadRight, "DPadRight", "D-pad →"),
        (StickUp, "StickUp", "Stick ↑"),
        (StickDown, "StickDown", "Stick ↓"),
        (StickLeft, "StickLeft", "Stick ←"),
        (StickRight, "StickRight", "Stick →"),
    ]
};

impl PadInput {
    pub fn name(self) -> &'static str {
        PAD_INPUTS.iter().find(|p| p.0 == self).expect("every pad input is listed").1
    }

    pub fn label(self) -> &'static str {
        PAD_INPUTS.iter().find(|p| p.0 == self).expect("every pad input is listed").2
    }

    pub fn named(name: &str) -> Option<PadInput> {
        PAD_INPUTS.iter().find(|p| p.1 == name).map(|p| p.0)
    }

    /// A button, as opposed to a stick direction.
    pub fn is_button(self) -> bool {
        !matches!(
            self,
            PadInput::StickUp | PadInput::StickDown | PadInput::StickLeft | PadInput::StickRight
        )
    }
}

/// Which pad inputs feed each of the original's inputs (DESIGN §2.6). One may feed more than one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PadMap(pub BTreeMap<OriginalInput, Vec<PadInput>>);

impl Default for PadMap {
    fn default() -> PadMap {
        use PadInput::*;
        let mut m = BTreeMap::new();
        m.insert(OriginalInput::Up, vec![DPadUp, StickUp]);
        m.insert(OriginalInput::Down, vec![DPadDown, StickDown]);
        m.insert(OriginalInput::Left, vec![DPadLeft, StickLeft]);
        m.insert(OriginalInput::Right, vec![DPadRight, StickRight]);
        m.insert(OriginalInput::Fire, vec![South, East]);
        PadMap(m)
    }
}

impl PadMap {
    /// The original's inputs `p` feeds.
    pub fn targets(&self, p: PadInput) -> impl Iterator<Item = OriginalInput> + '_ {
        self.0
            .iter()
            .filter(move |(_, ps)| ps.contains(&p))
            .map(|(o, _)| *o)
    }
}

/// The left stick's two axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickAxis {
    /// Right is positive.
    X,
    /// Up is positive (gilrs's convention).
    Y,
}

/// Something a pad did. Axis values are thousandths of full travel, −1000 to 1000.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadEvent {
    Connected(usize),
    Disconnected(usize),
    Button { pad: usize, button: PadInput, pressed: bool },
    Stick { pad: usize, axis: StickAxis, value: i16 },
}

/// One connected pad: the buttons it holds and where its stick is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Pad {
    buttons: Vec<PadInput>,
    x: i16,
    y: i16,
}

impl Pad {
    fn stick(&self) -> impl Iterator<Item = PadInput> {
        let t = STICK_THRESHOLD;
        [
            (self.y > t, PadInput::StickUp),
            (self.y < -t, PadInput::StickDown),
            (self.x < -t, PadInput::StickLeft),
            (self.x > t, PadInput::StickRight),
        ]
        .into_iter()
        .filter(|h| h.0)
        .map(|h| h.1)
    }

    fn held(&self) -> impl Iterator<Item = PadInput> + '_ {
        self.buttons.iter().copied().chain(self.stick())
    }
}

/// What every connected pad holds, by device.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PadState {
    pads: BTreeMap<usize, Pad>,
}

impl PadState {
    /// Every pad input held, on any pad (one entry for each pad holding it).
    pub fn held(&self) -> impl Iterator<Item = PadInput> + '_ {
        self.pads.values().flat_map(Pad::held)
    }

    /// The pads connected, by device number.
    #[cfg(test)]
    pub fn connected(&self) -> impl Iterator<Item = usize> + '_ {
        self.pads.keys().copied()
    }

    /// Take one event; the pad inputs it newly holds. A pad that sends anything before it is
    /// reported connected is taken as connected (gilrs reports pads present at start-up so, and
    /// a script may leave the line out). A disconnected pad lets go of everything it held.
    pub fn apply(&mut self, e: PadEvent) -> Vec<PadInput> {
        match e {
            PadEvent::Connected(pad) => {
                self.pads.entry(pad).or_default();
                Vec::new()
            }
            PadEvent::Disconnected(pad) => {
                self.pads.remove(&pad);
                Vec::new()
            }
            PadEvent::Button { pad, button, pressed } => {
                let p = self.pads.entry(pad).or_default();
                let was = p.buttons.contains(&button);
                p.buttons.retain(|&b| b != button);
                if pressed {
                    p.buttons.push(button);
                }
                if pressed && !was { vec![button] } else { Vec::new() }
            }
            PadEvent::Stick { pad, axis, value } => {
                let p = self.pads.entry(pad).or_default();
                let before: Vec<PadInput> = p.stick().collect();
                match axis {
                    StickAxis::X => p.x = value.clamp(-1000, 1000),
                    StickAxis::Y => p.y = value.clamp(-1000, 1000),
                }
                p.stick().filter(|s| !before.contains(s)).collect()
            }
        }
    }
}

/// gilrs's button as a pad input; `None` for one the mapping does not use (C, Z, Unknown).
pub fn from_gilrs_button(b: gilrs::Button) -> Option<PadInput> {
    use gilrs::Button as B;
    Some(match b {
        B::South => PadInput::South,
        B::East => PadInput::East,
        B::West => PadInput::West,
        B::North => PadInput::North,
        B::LeftTrigger => PadInput::LeftTrigger,
        B::LeftTrigger2 => PadInput::LeftTrigger2,
        B::RightTrigger => PadInput::RightTrigger,
        B::RightTrigger2 => PadInput::RightTrigger2,
        B::Select => PadInput::Select,
        B::Start => PadInput::Start,
        B::Mode => PadInput::Mode,
        B::LeftThumb => PadInput::LeftThumb,
        B::RightThumb => PadInput::RightThumb,
        B::DPadUp => PadInput::DPadUp,
        B::DPadDown => PadInput::DPadDown,
        B::DPadLeft => PadInput::DPadLeft,
        B::DPadRight => PadInput::DPadRight,
        B::C | B::Z | B::Unknown => return None,
    })
}

/// gilrs's event on pad `pad` as a [`PadEvent`]; `None` for what the mapping does not read (the
/// right stick, the analogue triggers' travel, force feedback, events a filter dropped).
pub fn from_gilrs(pad: usize, e: &gilrs::EventType) -> Option<PadEvent> {
    use gilrs::EventType as E;
    Some(match *e {
        E::Connected => PadEvent::Connected(pad),
        E::Disconnected => PadEvent::Disconnected(pad),
        E::ButtonPressed(b, _) => PadEvent::Button { pad, button: from_gilrs_button(b)?, pressed: true },
        E::ButtonReleased(b, _) => PadEvent::Button { pad, button: from_gilrs_button(b)?, pressed: false },
        E::AxisChanged(a, v, _) => from_gilrs_axis(pad, a, v)?,
        _ => return None,
    })
}

fn from_gilrs_axis(pad: usize, a: gilrs::Axis, v: f32) -> Option<PadEvent> {
    let axis = match a {
        gilrs::Axis::LeftStickX => StickAxis::X,
        gilrs::Axis::LeftStickY => StickAxis::Y,
        _ => return None,
    };
    let value = (v.clamp(-1.0, 1.0) * 1000.0).round() as i16;
    Some(PadEvent::Stick { pad, axis, value })
}

/// Where pad events come from.
pub trait PadSource {
    /// The next event since the last call, if any; never waits.
    fn next(&mut self) -> Option<PadEvent>;
}

/// The real pads, through gilrs (its default filters: the d-pad's axes become its buttons).
pub struct Gilrs(gilrs::Gilrs);

impl Gilrs {
    /// The pads, and a `Connected` for each one present already. `Err` says why there are none to
    /// be had; the game carries on with the keyboard.
    pub fn open() -> Result<(Gilrs, Vec<PadEvent>), String> {
        let g = gilrs::Gilrs::new().map_err(|e| format!("no gamepads: {e}"))?;
        let present = g.gamepads().map(|(id, _)| PadEvent::Connected(id.into())).collect();
        Ok((Gilrs(g), present))
    }
}

impl PadSource for Gilrs {
    fn next(&mut self) -> Option<PadEvent> {
        while let Some(e) = self.0.next_event() {
            if let Some(p) = from_gilrs(e.id.into(), &e.event) {
                return Some(p);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pad source that plays back the events it is given, as a test's stand-in for gilrs.
    #[derive(Default)]
    struct FakePads(std::collections::VecDeque<PadEvent>);

    impl PadSource for FakePads {
        fn next(&mut self) -> Option<PadEvent> {
            self.0.pop_front()
        }
    }

    fn drain(src: &mut dyn PadSource, state: &mut PadState) {
        while let Some(e) = src.next() {
            state.apply(e);
        }
    }

    fn held(s: &PadState) -> Vec<PadInput> {
        let mut h: Vec<PadInput> = s.held().collect();
        h.sort();
        h
    }

    #[test]
    fn every_name_is_unique_and_found_both_ways() {
        for &(p, name, label) in PAD_INPUTS {
            assert_eq!(PadInput::named(name), Some(p));
            assert_eq!(p.name(), name);
            assert_eq!(p.label(), label);
            assert_eq!(PAD_INPUTS.iter().filter(|q| q.1 == name).count(), 1, "{name}");
            assert_eq!(PAD_INPUTS.iter().filter(|q| q.2 == label).count(), 1, "{label}");
        }
        assert_eq!(PadInput::named("Nope"), None);
    }

    #[test]
    fn the_stick_counts_past_half_way_and_diagonals_hold_two_directions() {
        let mut s = PadState::default();
        let stick = |axis, value| PadEvent::Stick { pad: 0, axis, value };
        assert_eq!(s.apply(stick(StickAxis::X, -500)), vec![], "at the threshold: the dead zone");
        assert_eq!(held(&s), vec![]);
        assert_eq!(s.apply(stick(StickAxis::X, -501)), vec![PadInput::StickLeft]);
        assert_eq!(s.apply(stick(StickAxis::Y, 700)), vec![PadInput::StickUp]);
        assert_eq!(held(&s), vec![PadInput::StickUp, PadInput::StickLeft]);
        assert_eq!(s.apply(stick(StickAxis::X, -900)), vec![], "still left: nothing new");
        s.apply(stick(StickAxis::Y, -200));
        assert_eq!(held(&s), vec![PadInput::StickLeft]);
        s.apply(stick(StickAxis::X, 999));
        assert_eq!(held(&s), vec![PadInput::StickRight]);
    }

    #[test]
    fn a_pad_unplugged_mid_move_lets_go_of_what_it_held_and_only_that() {
        let mut src = FakePads(
            [
                PadEvent::Connected(0),
                PadEvent::Connected(1),
                PadEvent::Button { pad: 0, button: PadInput::DPadLeft, pressed: true },
                PadEvent::Stick { pad: 0, axis: StickAxis::Y, value: 800 },
                PadEvent::Button { pad: 1, button: PadInput::South, pressed: true },
            ]
            .into(),
        );
        let mut s = PadState::default();
        drain(&mut src, &mut s);
        assert_eq!(held(&s), vec![PadInput::South, PadInput::DPadLeft, PadInput::StickUp]);
        src.0.push_back(PadEvent::Disconnected(0));
        drain(&mut src, &mut s);
        assert_eq!(held(&s), vec![PadInput::South], "pad 1 still holds A");
        assert_eq!(s.connected().collect::<Vec<_>>(), vec![1]);
        // Plugged back in, it holds nothing until it says so.
        src.0.push_back(PadEvent::Connected(0));
        drain(&mut src, &mut s);
        assert_eq!(held(&s), vec![PadInput::South]);
        assert_eq!(s.connected().collect::<Vec<_>>(), vec![0, 1]);
    }

    #[test]
    fn a_press_is_new_once_and_a_release_of_nothing_held_is_harmless() {
        let mut s = PadState::default();
        let b = |pressed| PadEvent::Button { pad: 3, button: PadInput::East, pressed };
        assert_eq!(s.apply(b(true)), vec![PadInput::East]);
        assert_eq!(s.apply(b(true)), vec![], "a repeat is not a new press");
        assert_eq!(held(&s), vec![PadInput::East]);
        s.apply(b(false));
        s.apply(b(false));
        assert_eq!(held(&s), vec![]);
        assert_eq!(s.connected().collect::<Vec<_>>(), vec![3], "a pad heard from is connected");
    }

    #[test]
    fn gilrs_events_translate_and_the_rest_are_left_out() {
        use gilrs::{Axis, Button};
        assert_eq!(from_gilrs_button(Button::South), Some(PadInput::South));
        assert_eq!(from_gilrs_button(Button::DPadRight), Some(PadInput::DPadRight));
        assert_eq!(from_gilrs_button(Button::Mode), Some(PadInput::Mode));
        assert_eq!(from_gilrs_button(Button::C), None);
        assert_eq!(from_gilrs_button(Button::Unknown), None);
        assert_eq!(
            from_gilrs_axis(2, Axis::LeftStickY, -0.75),
            Some(PadEvent::Stick { pad: 2, axis: StickAxis::Y, value: -750 })
        );
        assert_eq!(
            from_gilrs_axis(2, Axis::LeftStickX, 1.5),
            Some(PadEvent::Stick { pad: 2, axis: StickAxis::X, value: 1000 })
        );
        assert_eq!(from_gilrs_axis(2, Axis::RightStickX, 1.0), None);
        assert_eq!(from_gilrs(4, &gilrs::EventType::Connected), Some(PadEvent::Connected(4)));
        assert_eq!(from_gilrs(4, &gilrs::EventType::Disconnected), Some(PadEvent::Disconnected(4)));
        assert_eq!(from_gilrs(4, &gilrs::EventType::Dropped), None);
    }

    #[test]
    fn the_default_map_is_the_d_pad_and_stick_for_directions_and_a_and_b_for_fire() {
        let m = PadMap::default();
        let t = |p| m.targets(p).collect::<Vec<_>>();
        assert_eq!(t(PadInput::DPadUp), vec![OriginalInput::Up]);
        assert_eq!(t(PadInput::StickUp), vec![OriginalInput::Up]);
        assert_eq!(t(PadInput::DPadDown), vec![OriginalInput::Down]);
        assert_eq!(t(PadInput::StickDown), vec![OriginalInput::Down]);
        assert_eq!(t(PadInput::DPadLeft), vec![OriginalInput::Left]);
        assert_eq!(t(PadInput::StickLeft), vec![OriginalInput::Left]);
        assert_eq!(t(PadInput::DPadRight), vec![OriginalInput::Right]);
        assert_eq!(t(PadInput::StickRight), vec![OriginalInput::Right]);
        assert_eq!(t(PadInput::South), vec![OriginalInput::Fire]);
        assert_eq!(t(PadInput::East), vec![OriginalInput::Fire]);
        for p in [PadInput::West, PadInput::North, PadInput::Start, PadInput::Select, PadInput::Mode] {
            assert_eq!(t(p), vec![], "{p:?}");
        }
    }
}
