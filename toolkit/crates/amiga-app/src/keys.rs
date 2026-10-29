//! The Amiga keys and which host key feeds which of the original's inputs: [`OriginalInput`] (a
//! joystick direction, its fire button or an Amiga key), [`AMIGA_KEYS`] (raw code and label),
//! [`HOST_KEYS`] (winit's codes and the names scripts and settings files use) and the [`KeyMap`].
//!
//! By default the arrows are the joystick's directions, Space and either Ctrl its fire button, and
//! letters, digits, Return, Escape, Backspace, Tab, Delete and F1–F10 the Amiga's own keys.

use std::collections::BTreeMap;

use amiga_input::{FrameInput, Joy, KeySet};
use winit::keyboard::KeyCode;

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
    /// Its name in settings files: `up`, `down`, `left`, `right`, `fire`, or `key <label>`.
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

    /// Its name in a controls window.
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

    /// `f` with this input held too.
    pub fn apply(self, mut f: FrameInput) -> FrameInput {
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

/// Every host key the app knows, with the name a script and a settings file use for it (winit's).
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

/// The winit code a script's or a settings file's key name stands for.
pub fn key_named(name: &str) -> Option<KeyCode> {
    HOST_KEYS.iter().find(|k| k.1 == name).map(|k| k.0)
}

/// The name of a host key, for a settings file.
pub fn key_name(code: KeyCode) -> Option<&'static str> {
    HOST_KEYS.iter().find(|k| k.0 == code).map(|k| k.1)
}

/// The label a controls window shows for a host key: what is printed on a Mac's key.
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

/// Which host keys feed each of the original's inputs. A key may feed more than one.
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn the_default_map_is_the_arrows_space_ctrl_and_the_same_labelled_keys() {
        // Every Amiga key, from the host key with the same label.
        let m = KeyMap::default();
        assert_eq!(m.0[&OriginalInput::Key(0x19)], vec![KeyCode::KeyP]);
        assert_eq!(m.0[&OriginalInput::Key(0x0A)], vec![KeyCode::Digit0]);
        assert_eq!(m.0[&OriginalInput::Key(0x44)], vec![KeyCode::Enter]);
        assert_eq!(m.0[&OriginalInput::Key(0x59)], vec![KeyCode::F10]);
        assert_eq!(m.0.len(), 5 + AMIGA_KEYS.len());
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
