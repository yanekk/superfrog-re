//! The one point where host input becomes the original's input (DESIGN §2.6): winit's key and
//! mouse events, the pads' events (`pad.rs`) and a script's events (DESIGN §5) are all turned into
//! [`HostInput`] and handed to [`Input::event`]; each frame [`Input::frame`] gives the
//! [`FrameInput`] the game steps on, through the pure [`map_input`].
//!
//! Which host key feeds which of the original's inputs is the [`KeyMap`], and which pad input the
//! [`PadMap`], both remembered in `settings.toml` (DESIGN §2.9) and rebound in the Controls window
//! (`controls_window.rs`). By default the arrows are the joystick's directions, Space and either
//! Ctrl its fire button, and letters, digits, Return, Escape, Backspace, Tab, Delete and F1–F10 the
//! Amiga's own keys; the pad's defaults are in `pad.rs`. The left mouse button is port 1's button
//! (the crack intro waits for it) and is not remapped. With ⌘ held a key is the app's, never the
//! game's: ⌃⌘F is fullscreen and ⌘Q quits, the menu bar's shortcuts (`menu.rs`) as a script's keys
//! arrive at them.

use std::collections::BTreeMap;

use sf_core::input::{FrameInput, Joy, KeySet};
use winit::keyboard::KeyCode;

use crate::menu::Command;
use crate::pad::{PadEvent, PadInput, PadMap, PadState};

/// One of the original's inputs: a joystick direction, its fire button, or an Amiga key (raw code).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OriginalInput {
    Up,
    Down,
    Left,
    Right,
    Fire,
    Key(u8),
}

/// The Amiga keys the game reads, by raw code and the label on the key.
pub const AMIGA_KEYS: &[(u8, &str)] = &[
    (0x01, "1"),
    (0x02, "2"),
    (0x03, "3"),
    (0x04, "4"),
    (0x05, "5"),
    (0x06, "6"),
    (0x07, "7"),
    (0x08, "8"),
    (0x09, "9"),
    (0x0A, "0"),
    (0x10, "Q"),
    (0x11, "W"),
    (0x12, "E"),
    (0x13, "R"),
    (0x14, "T"),
    (0x15, "Y"),
    (0x16, "U"),
    (0x17, "I"),
    (0x18, "O"),
    (0x19, "P"),
    (0x20, "A"),
    (0x21, "S"),
    (0x22, "D"),
    (0x23, "F"),
    (0x24, "G"),
    (0x25, "H"),
    (0x26, "J"),
    (0x27, "K"),
    (0x28, "L"),
    (0x31, "Z"),
    (0x32, "X"),
    (0x33, "C"),
    (0x34, "V"),
    (0x35, "B"),
    (0x36, "N"),
    (0x37, "M"),
    (0x41, "Backspace"),
    (0x42, "Tab"),
    (0x44, "Return"),
    (0x45, "Esc"),
    (0x46, "Del"),
    (0x50, "F1"),
    (0x51, "F2"),
    (0x52, "F3"),
    (0x53, "F4"),
    (0x54, "F5"),
    (0x55, "F6"),
    (0x56, "F7"),
    (0x57, "F8"),
    (0x58, "F9"),
    (0x59, "F10"),
];

impl OriginalInput {
    /// Its name in `settings.toml`: `up`, `down`, `left`, `right`, `fire`, or `key <label>`.
    pub fn name(self) -> String {
        match self {
            OriginalInput::Up => "up".into(),
            OriginalInput::Down => "down".into(),
            OriginalInput::Left => "left".into(),
            OriginalInput::Right => "right".into(),
            OriginalInput::Fire => "fire".into(),
            OriginalInput::Key(k) => {
                let label = AMIGA_KEYS.iter().find(|a| a.0 == k).map(|a| a.1);
                match label {
                    Some(l) => format!("key {l}"),
                    None => format!("key ${k:02x}"),
                }
            }
        }
    }

    pub fn named(name: &str) -> Option<OriginalInput> {
        Some(match name {
            "up" => OriginalInput::Up,
            "down" => OriginalInput::Down,
            "left" => OriginalInput::Left,
            "right" => OriginalInput::Right,
            "fire" => OriginalInput::Fire,
            _ => {
                let label = name.strip_prefix("key ")?;
                let code = match label.strip_prefix('$') {
                    Some(hex) => u8::from_str_radix(hex, 16).ok().filter(|&c| c < 0x80)?,
                    None => AMIGA_KEYS.iter().find(|a| a.1 == label)?.0,
                };
                OriginalInput::Key(code)
            }
        })
    }

    /// Its name in the Controls window.
    pub fn label(self) -> String {
        match self {
            OriginalInput::Up => "Up".into(),
            OriginalInput::Down => "Down".into(),
            OriginalInput::Left => "Left".into(),
            OriginalInput::Right => "Right".into(),
            OriginalInput::Fire => "Fire".into(),
            OriginalInput::Key(k) => match AMIGA_KEYS.iter().find(|a| a.0 == k) {
                Some(a) => format!("the Amiga's {} key", a.1),
                None => format!("Amiga key ${k:02x}"),
            },
        }
    }

    fn apply(self, mut f: FrameInput) -> FrameInput {
        match self {
            OriginalInput::Up => f.joy = f.joy | Joy::UP,
            OriginalInput::Down => f.joy = f.joy | Joy::DOWN,
            OriginalInput::Left => f.joy = f.joy | Joy::LEFT,
            OriginalInput::Right => f.joy = f.joy | Joy::RIGHT,
            OriginalInput::Fire => f.joy = f.joy | Joy::FIRE,
            OriginalInput::Key(k) => f.keys = KeySet(f.keys.0 | KeySet::key(k).0),
        }
        f
    }
}

/// Every host key the app knows, with the name a script and `settings.toml` use for it (winit's).
pub const HOST_KEYS: &[(KeyCode, &str)] = {
    use KeyCode::*;
    &[
        (ArrowUp, "ArrowUp"),
        (ArrowDown, "ArrowDown"),
        (ArrowLeft, "ArrowLeft"),
        (ArrowRight, "ArrowRight"),
        (Space, "Space"),
        (ControlLeft, "ControlLeft"),
        (ControlRight, "ControlRight"),
        (ShiftLeft, "ShiftLeft"),
        (ShiftRight, "ShiftRight"),
        (AltLeft, "AltLeft"),
        (AltRight, "AltRight"),
        (SuperLeft, "SuperLeft"),
        (SuperRight, "SuperRight"),
        (Digit1, "Digit1"),
        (Digit2, "Digit2"),
        (Digit3, "Digit3"),
        (Digit4, "Digit4"),
        (Digit5, "Digit5"),
        (Digit6, "Digit6"),
        (Digit7, "Digit7"),
        (Digit8, "Digit8"),
        (Digit9, "Digit9"),
        (Digit0, "Digit0"),
        (KeyQ, "KeyQ"),
        (KeyW, "KeyW"),
        (KeyE, "KeyE"),
        (KeyR, "KeyR"),
        (KeyT, "KeyT"),
        (KeyY, "KeyY"),
        (KeyU, "KeyU"),
        (KeyI, "KeyI"),
        (KeyO, "KeyO"),
        (KeyP, "KeyP"),
        (KeyA, "KeyA"),
        (KeyS, "KeyS"),
        (KeyD, "KeyD"),
        (KeyF, "KeyF"),
        (KeyG, "KeyG"),
        (KeyH, "KeyH"),
        (KeyJ, "KeyJ"),
        (KeyK, "KeyK"),
        (KeyL, "KeyL"),
        (KeyZ, "KeyZ"),
        (KeyX, "KeyX"),
        (KeyC, "KeyC"),
        (KeyV, "KeyV"),
        (KeyB, "KeyB"),
        (KeyN, "KeyN"),
        (KeyM, "KeyM"),
        (Backspace, "Backspace"),
        (Tab, "Tab"),
        (Enter, "Enter"),
        (Escape, "Escape"),
        (Delete, "Delete"),
        (F1, "F1"),
        (F2, "F2"),
        (F3, "F3"),
        (F4, "F4"),
        (F5, "F5"),
        (F6, "F6"),
        (F7, "F7"),
        (F8, "F8"),
        (F9, "F9"),
        (F10, "F10"),
        (F11, "F11"),
        (F12, "F12"),
    ]
};

/// The winit code a script's or `settings.toml`'s key name stands for.
pub fn key_named(name: &str) -> Option<KeyCode> {
    HOST_KEYS.iter().find(|k| k.1 == name).map(|k| k.0)
}

/// The name of a host key, for `settings.toml`.
pub fn key_name(code: KeyCode) -> Option<&'static str> {
    HOST_KEYS.iter().find(|k| k.0 == code).map(|k| k.1)
}

/// The label the Controls window shows for a host key: what is printed on a Mac's key.
pub fn key_label(code: KeyCode) -> String {
    use KeyCode::*;
    let fixed = match code {
        ArrowUp => "↑",
        ArrowDown => "↓",
        ArrowLeft => "←",
        ArrowRight => "→",
        Space => "Space",
        ControlLeft => "Left Ctrl",
        ControlRight => "Right Ctrl",
        ShiftLeft => "Left Shift",
        ShiftRight => "Right Shift",
        AltLeft => "Left Option",
        AltRight => "Right Option",
        SuperLeft => "Left ⌘",
        SuperRight => "Right ⌘",
        Backspace => "Delete",
        Delete => "Forward Delete",
        Enter => "Return",
        Escape => "Esc",
        Tab => "Tab",
        _ => "",
    };
    if !fixed.is_empty() {
        return fixed.into();
    }
    let name = key_name(code).unwrap_or("?");
    name.strip_prefix("Key")
        .or_else(|| name.strip_prefix("Digit"))
        .unwrap_or(name)
        .into()
}

/// Which host keys feed each of the original's inputs (DESIGN §2.6). A key may feed more than one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyMap(pub BTreeMap<OriginalInput, Vec<KeyCode>>);

impl Default for KeyMap {
    fn default() -> KeyMap {
        use KeyCode::*;
        let mut m = BTreeMap::new();
        m.insert(OriginalInput::Up, vec![ArrowUp]);
        m.insert(OriginalInput::Down, vec![ArrowDown]);
        m.insert(OriginalInput::Left, vec![ArrowLeft]);
        m.insert(OriginalInput::Right, vec![ArrowRight]);
        m.insert(OriginalInput::Fire, vec![Space, ControlLeft, ControlRight]);
        // The Amiga key with the same label on the Mac's keyboard.
        for &(code, label) in AMIGA_KEYS {
            let host = match label {
                "Return" => Enter,
                "Esc" => Escape,
                "Del" => Delete,
                "Backspace" => Backspace,
                "Tab" => Tab,
                l if l.len() == 1 && l.as_bytes()[0].is_ascii_digit() => {
                    key_named(&format!("Digit{l}")).expect("a digit key")
                }
                l if l.len() == 1 => key_named(&format!("Key{l}")).expect("a letter key"),
                l => key_named(l).expect("a function key"),
            };
            m.insert(OriginalInput::Key(code), vec![host]);
        }
        KeyMap(m)
    }
}

impl KeyMap {
    /// The original's inputs `code` feeds.
    pub fn targets(&self, code: KeyCode) -> impl Iterator<Item = OriginalInput> + '_ {
        self.0
            .iter()
            .filter(move |(_, keys)| keys.contains(&code))
            .map(|(o, _)| *o)
    }
}

/// A host input event, from winit or from a script.
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

/// The host keys held, one entry a press.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyState(pub Vec<KeyCode>);

/// The original's inputs held by what the keys and pads hold (DESIGN §2.6). Pure: the device
/// that feeds an input makes no difference to what the game sees.
pub fn map_input(keys: &KeyState, pads: &PadState, km: &KeyMap, pm: &PadMap) -> FrameInput {
    let mut f = FrameInput::NONE;
    for &code in &keys.0 {
        for t in km.targets(code) {
            f = t.apply(f);
        }
    }
    for p in pads.held() {
        for t in pm.targets(p) {
            f = t.apply(f);
        }
    }
    f
}

/// What is held, and what was pressed since the last frame.
#[derive(Debug, Default)]
pub struct Input {
    pub map: KeyMap,
    pub pad_map: PadMap,
    keys: KeyState,
    pads: PadState,
    mouse: bool,
    /// Pressed since the last frame, so a press released within one frame is still seen once.
    tapped: FrameInput,
}

fn is_command(code: KeyCode) -> bool {
    matches!(code, KeyCode::SuperLeft | KeyCode::SuperRight)
}

impl Input {
    pub fn new(map: KeyMap, pad_map: PadMap) -> Input {
        Input {
            map,
            pad_map,
            ..Input::default()
        }
    }

    fn holding(&self, f: impl Fn(KeyCode) -> bool) -> bool {
        self.keys.0.iter().any(|&c| f(c))
    }

    /// Feed `p`'s targets into this frame, so a press released within one frame is still seen.
    fn tap_pad(&mut self, p: PadInput) {
        for t in self.pad_map.targets(p) {
            self.tapped = t.apply(self.tapped);
        }
    }

    /// Take one host event. A key pressed with ⌘ held goes to the app, not the game: `Some` when
    /// it is one of the menu bar's shortcuts.
    pub fn event(&mut self, e: HostInput) -> Option<Command> {
        match e {
            HostInput::Key { code, pressed: true } => {
                if is_command(code) {
                    self.keys.0.push(code);
                    return None;
                }
                if self.holding(is_command) {
                    let ctrl = self.holding(|c| matches!(c, KeyCode::ControlLeft | KeyCode::ControlRight));
                    return match (ctrl, code) {
                        (true, KeyCode::KeyF) => Some(Command::ToggleFullscreen),
                        (false, KeyCode::KeyQ) => Some(Command::Quit),
                        _ => None,
                    };
                }
                self.keys.0.push(code);
                for t in self.map.targets(code) {
                    self.tapped = t.apply(self.tapped);
                }
            }
            HostInput::Key { code, pressed: false } => {
                if let Some(i) = self.keys.0.iter().position(|&h| h == code) {
                    self.keys.0.remove(i);
                }
            }
            HostInput::MouseLeft { pressed } => {
                self.mouse = pressed;
                if pressed {
                    self.tapped.joy = self.tapped.joy | Joy::PORT1_BUTTON;
                }
            }
            HostInput::FocusLost => {
                self.keys.0.clear();
                self.mouse = false;
            }
            HostInput::Pad(e) => {
                for p in self.pads.apply(e) {
                    self.tap_pad(p);
                }
            }
        }
        None
    }

    /// What the pads hold.
    pub fn pads(&self) -> &PadState {
        &self.pads
    }

    /// Let go of every key held, as the window losing focus does, without a frame's worth of
    /// taps being lost: the Controls window taking the keyboard.
    pub fn release_keys(&mut self) {
        self.keys.0.clear();
    }

    /// This frame's input: what is held, plus anything pressed and released since the last frame.
    pub fn frame(&mut self) -> FrameInput {
        let tapped = std::mem::take(&mut self.tapped);
        let held = map_input(&self.keys, &self.pads, &self.map, &self.pad_map);
        let mut f = FrameInput {
            joy: tapped.joy | held.joy,
            keys: KeySet(tapped.keys.0 | held.keys.0),
        };
        if self.mouse {
            f.joy = f.joy | Joy::PORT1_BUTTON;
        }
        f
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, pressed: bool) -> HostInput {
        HostInput::Key { code, pressed }
    }

    fn input() -> Input {
        Input::new(KeyMap::default(), PadMap::default())
    }

    #[test]
    fn arrows_and_space_are_the_joystick_and_letters_are_amiga_keys() {
        let mut i = input();
        i.event(key(KeyCode::ArrowLeft, true));
        i.event(key(KeyCode::Space, true));
        i.event(key(KeyCode::KeyP, true));
        let f = i.frame();
        assert_eq!(f.joy, Joy::LEFT | Joy::FIRE);
        assert_eq!(f.keys, KeySet::key(0x19));
        assert_eq!(i.frame(), f, "held keys stay held");
    }

    #[test]
    fn either_ctrl_is_fire_too() {
        for ctrl in [KeyCode::ControlLeft, KeyCode::ControlRight] {
            let mut i = input();
            i.event(key(ctrl, true));
            assert_eq!(i.frame(), FrameInput::joy(Joy::FIRE));
        }
    }

    #[test]
    fn a_tap_inside_one_frame_is_seen_once() {
        let mut i = input();
        i.event(key(KeyCode::ArrowUp, true));
        i.event(key(KeyCode::ArrowUp, false));
        assert_eq!(i.frame().joy, Joy::UP);
        assert_eq!(i.frame(), FrameInput::NONE);
    }

    #[test]
    fn the_mouse_is_port_1_s_button_and_unmapped_keys_do_nothing() {
        let mut i = input();
        i.event(HostInput::MouseLeft { pressed: true });
        i.event(key(KeyCode::CapsLock, true));
        i.event(key(KeyCode::F12, true));
        assert_eq!(i.frame(), FrameInput::joy(Joy::PORT1_BUTTON));
        i.event(HostInput::MouseLeft { pressed: false });
        assert_eq!(i.frame(), FrameInput::NONE);
    }

    #[test]
    fn two_keys_on_one_direction_hold_it_until_both_are_up() {
        let mut i = input();
        i.event(key(KeyCode::ArrowLeft, true));
        i.event(key(KeyCode::ArrowLeft, true));
        i.event(key(KeyCode::ArrowLeft, false));
        i.frame();
        assert_eq!(i.frame().joy, Joy::LEFT);
        let mut i = input();
        i.event(key(KeyCode::Space, true));
        i.event(key(KeyCode::ControlLeft, true));
        i.event(key(KeyCode::Space, false));
        i.frame();
        assert_eq!(i.frame().joy, Joy::FIRE, "Ctrl still holds fire");
    }

    #[test]
    fn losing_focus_lets_go_of_everything_held() {
        let mut i = input();
        i.event(key(KeyCode::ArrowLeft, true));
        i.event(HostInput::MouseLeft { pressed: true });
        i.frame();
        i.event(HostInput::FocusLost);
        assert_eq!(i.frame(), FrameInput::NONE);
    }

    #[test]
    fn a_remapped_key_feeds_its_new_input_and_the_old_key_nothing() {
        let mut map = KeyMap::default();
        map.0.insert(OriginalInput::Fire, vec![KeyCode::KeyX]);
        map.0.insert(OriginalInput::Key(0x32), vec![]);
        let mut i = Input::new(map, PadMap::default());
        i.event(key(KeyCode::Space, true));
        i.event(key(KeyCode::KeyX, true));
        assert_eq!(i.frame(), FrameInput::joy(Joy::FIRE));
    }

    #[test]
    fn ctrl_cmd_f_is_fullscreen_cmd_q_quits_and_keys_with_cmd_never_reach_the_game() {
        let mut i = input();
        assert_eq!(i.event(key(KeyCode::SuperLeft, true)), None);
        assert_eq!(i.event(key(KeyCode::KeyQ, true)), Some(Command::Quit));
        assert_eq!(i.event(key(KeyCode::KeyF, true)), None, "⌘F alone is nothing");
        i.event(key(KeyCode::SuperLeft, false));
        assert_eq!(i.event(key(KeyCode::ControlLeft, true)), None);
        assert_eq!(i.event(key(KeyCode::SuperLeft, true)), None);
        assert_eq!(i.event(key(KeyCode::KeyF, true)), Some(Command::ToggleFullscreen));
        assert_eq!(i.event(key(KeyCode::KeyQ, true)), None, "⌃⌘Q is not Quit");
        // Ctrl is fire; F and Q, pressed with ⌘, are not Amiga keys here.
        assert_eq!(i.frame(), FrameInput::joy(Joy::FIRE));
        i.event(key(KeyCode::SuperLeft, false));
        i.event(key(KeyCode::ControlLeft, false));
        assert_eq!(i.event(key(KeyCode::KeyF, true)), None, "F alone is the game's");
        assert_eq!(i.frame().keys, KeySet::key(0x23));
    }

    #[test]
    fn every_name_is_unique_and_found_both_ways() {
        for (code, name) in HOST_KEYS {
            assert_eq!(key_named(name), Some(*code), "{name}");
            assert_eq!(key_name(*code), Some(*name));
            assert_eq!(HOST_KEYS.iter().filter(|k| k.1 == *name).count(), 1);
        }
        assert_eq!(key_named("NoSuchKey"), None);
        for o in KeyMap::default().0.keys() {
            assert_eq!(OriginalInput::named(&o.name()), Some(*o), "{}", o.name());
        }
        assert_eq!(OriginalInput::named("key $60"), Some(OriginalInput::Key(0x60)));
        assert_eq!(OriginalInput::named("key Nope"), None);
    }

    #[test]
    fn the_default_map_is_t22_s_fixed_mapping_plus_ctrl() {
        // Every Amiga key the T22 table mapped, from the same host key.
        let m = KeyMap::default();
        assert_eq!(m.0[&OriginalInput::Key(0x19)], vec![KeyCode::KeyP]);
        assert_eq!(m.0[&OriginalInput::Key(0x0A)], vec![KeyCode::Digit0]);
        assert_eq!(m.0[&OriginalInput::Key(0x44)], vec![KeyCode::Enter]);
        assert_eq!(m.0[&OriginalInput::Key(0x59)], vec![KeyCode::F10]);
        assert_eq!(m.0.len(), 5 + AMIGA_KEYS.len());
    }

    fn pads(events: &[PadEvent]) -> PadState {
        let mut s = PadState::default();
        for &e in events {
            s.apply(e);
        }
        s
    }

    fn button(button: PadInput) -> PadEvent {
        PadEvent::Button { pad: 0, button, pressed: true }
    }

    fn stick(x: i16, y: i16) -> [PadEvent; 2] {
        use crate::pad::StickAxis;
        [
            PadEvent::Stick { pad: 0, axis: StickAxis::X, value: x },
            PadEvent::Stick { pad: 0, axis: StickAxis::Y, value: y },
        ]
    }

    fn mapped(keys: &[KeyCode], pad: &PadState) -> FrameInput {
        map_input(&KeyState(keys.to_vec()), pad, &KeyMap::default(), &PadMap::default())
    }

    #[test]
    fn map_input_every_default_key_binding() {
        let none = PadState::default();
        let m = KeyMap::default();
        for (o, codes) in &m.0 {
            for &code in codes {
                let want = o.apply(FrameInput::NONE);
                assert_eq!(mapped(&[code], &none), want, "{code:?} for {}", o.name());
            }
        }
        assert_eq!(mapped(&[KeyCode::ArrowUp], &none), FrameInput::joy(Joy::UP));
        assert_eq!(mapped(&[KeyCode::Space], &none), FrameInput::joy(Joy::FIRE));
        assert_eq!(mapped(&[KeyCode::KeyX], &none), FrameInput::key(0x32));
        assert_eq!(mapped(&[], &none), FrameInput::NONE);
    }

    #[test]
    fn map_input_every_default_pad_binding() {
        use PadInput::*;
        for (p, joy) in [
            (DPadUp, Joy::UP),
            (DPadDown, Joy::DOWN),
            (DPadLeft, Joy::LEFT),
            (DPadRight, Joy::RIGHT),
            (South, Joy::FIRE),
            (East, Joy::FIRE),
        ] {
            assert_eq!(mapped(&[], &pads(&[button(p)])), FrameInput::joy(joy), "{p:?}");
        }
        for (x, y, joy) in [(0, 800, Joy::UP), (0, -800, Joy::DOWN), (-800, 0, Joy::LEFT), (800, 0, Joy::RIGHT)] {
            assert_eq!(mapped(&[], &pads(&stick(x, y))), FrameInput::joy(joy), "stick {x},{y}");
        }
        for p in [West, North, LeftTrigger, RightTrigger2, Select, Start, Mode, LeftThumb, RightThumb] {
            assert_eq!(mapped(&[], &pads(&[button(p)])), FrameInput::NONE, "{p:?} is unbound");
        }
    }

    #[test]
    fn map_input_key_and_pad_held_together() {
        let pad = pads(&[button(PadInput::South), button(PadInput::DPadRight)]);
        let f = mapped(&[KeyCode::ArrowUp, KeyCode::KeyP], &pad);
        assert_eq!(f.joy, Joy::UP | Joy::RIGHT | Joy::FIRE);
        assert_eq!(f.keys, KeySet::key(0x19));
        // The same input from both is the same input, once.
        assert_eq!(mapped(&[KeyCode::Space], &pads(&[button(PadInput::South)])), FrameInput::joy(Joy::FIRE));
    }

    #[test]
    fn map_input_ignores_the_stick_inside_the_dead_zone() {
        for (x, y) in [(0, 0), (500, 0), (-500, 0), (0, 500), (0, -500), (300, -450), (499, 499)] {
            assert_eq!(mapped(&[], &pads(&stick(x, y))), FrameInput::NONE, "stick {x},{y}");
        }
    }

    #[test]
    fn map_input_a_diagonal_holds_both_directions() {
        for (x, y, joy) in [
            (707, 707, Joy::UP | Joy::RIGHT),
            (-707, 707, Joy::UP | Joy::LEFT),
            (707, -707, Joy::DOWN | Joy::RIGHT),
            (-707, -707, Joy::DOWN | Joy::LEFT),
            (900, 300, Joy::RIGHT),
        ] {
            assert_eq!(mapped(&[], &pads(&stick(x, y))), FrameInput::joy(joy), "stick {x},{y}");
        }
        let pad = pads(&[button(PadInput::DPadUp), button(PadInput::DPadLeft)]);
        assert_eq!(mapped(&[], &pad), FrameInput::joy(Joy::UP | Joy::LEFT), "the d-pad's diagonal");
    }

    #[test]
    fn a_pad_tap_inside_one_frame_is_seen_once_and_unplugging_releases_only_that_pad() {
        let mut i = input();
        i.event(HostInput::Pad(button(PadInput::South)));
        i.event(HostInput::Pad(PadEvent::Button { pad: 0, button: PadInput::South, pressed: false }));
        assert_eq!(i.frame(), FrameInput::joy(Joy::FIRE));
        assert_eq!(i.frame(), FrameInput::NONE);

        i.event(HostInput::Pad(PadEvent::Connected(0)));
        i.event(HostInput::Pad(PadEvent::Connected(1)));
        for e in stick(-900, 0) {
            i.event(HostInput::Pad(e));
        }
        i.event(HostInput::Pad(PadEvent::Button { pad: 1, button: PadInput::DPadUp, pressed: true }));
        i.event(key(KeyCode::Space, true));
        assert_eq!(i.frame().joy, Joy::LEFT | Joy::UP | Joy::FIRE);
        i.event(HostInput::Pad(PadEvent::Disconnected(0)));
        assert_eq!(i.frame().joy, Joy::UP | Joy::FIRE, "pad 1 and the key still hold");
        // A pad is not the window's: losing focus lets go of the keys only.
        i.event(HostInput::FocusLost);
        assert_eq!(i.frame().joy, Joy::UP);
    }

    #[test]
    fn a_remapped_pad_input_feeds_its_new_input() {
        let mut pm = PadMap::default();
        pm.0.insert(OriginalInput::Fire, vec![PadInput::North]);
        let mut i = Input::new(KeyMap::default(), pm);
        i.event(HostInput::Pad(button(PadInput::South)));
        assert_eq!(i.frame(), FrameInput::NONE);
        i.event(HostInput::Pad(button(PadInput::North)));
        assert_eq!(i.frame(), FrameInput::joy(Joy::FIRE));
    }

    #[test]
    fn key_labels_are_what_the_mac_s_keys_show() {
        assert_eq!(key_label(KeyCode::KeyX), "X");
        assert_eq!(key_label(KeyCode::Digit7), "7");
        assert_eq!(key_label(KeyCode::ArrowLeft), "←");
        assert_eq!(key_label(KeyCode::ControlLeft), "Left Ctrl");
        assert_eq!(key_label(KeyCode::F10), "F10");
        assert_eq!(key_label(KeyCode::Backspace), "Delete");
        for (code, _) in HOST_KEYS {
            assert!(!key_label(*code).is_empty() && key_label(*code) != "?", "{code:?}");
        }
    }
}
