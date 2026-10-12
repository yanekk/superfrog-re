//! The one point where host input becomes the original's input (DESIGN §2.6): winit's key and
//! mouse events, and a script's events (DESIGN §5), are both turned into [`HostInput`] and handed
//! to [`Input::event`]; each frame [`Input::frame`] gives the [`FrameInput`] the game steps on.
//!
//! The mapping is fixed here; T23 makes it remappable. Arrows are the joystick's directions and
//! Space its fire button; the left mouse button is port 1's button (the crack intro waits for it);
//! letters, digits, Return, Escape, Backspace, Tab, Delete and F1–F10 are the Amiga's own keys.

use sf_core::input::{FrameInput, Joy, KeySet};
use winit::keyboard::KeyCode;

/// What a host key does in the original.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Joy(Joy),
    /// An Amiga raw key code.
    Key(u8),
}

/// Every mapped key: the winit code, the name a script uses for it, and what it does.
pub const KEYS: &[(KeyCode, &str, Target)] = {
    use KeyCode::*;
    use Target::{Joy as J, Key as K};
    &[
        (ArrowUp, "ArrowUp", J(Joy::UP)),
        (ArrowDown, "ArrowDown", J(Joy::DOWN)),
        (ArrowLeft, "ArrowLeft", J(Joy::LEFT)),
        (ArrowRight, "ArrowRight", J(Joy::RIGHT)),
        (Space, "Space", J(Joy::FIRE)),
        (Digit1, "Digit1", K(0x01)),
        (Digit2, "Digit2", K(0x02)),
        (Digit3, "Digit3", K(0x03)),
        (Digit4, "Digit4", K(0x04)),
        (Digit5, "Digit5", K(0x05)),
        (Digit6, "Digit6", K(0x06)),
        (Digit7, "Digit7", K(0x07)),
        (Digit8, "Digit8", K(0x08)),
        (Digit9, "Digit9", K(0x09)),
        (Digit0, "Digit0", K(0x0A)),
        (KeyQ, "KeyQ", K(0x10)),
        (KeyW, "KeyW", K(0x11)),
        (KeyE, "KeyE", K(0x12)),
        (KeyR, "KeyR", K(0x13)),
        (KeyT, "KeyT", K(0x14)),
        (KeyY, "KeyY", K(0x15)),
        (KeyU, "KeyU", K(0x16)),
        (KeyI, "KeyI", K(0x17)),
        (KeyO, "KeyO", K(0x18)),
        (KeyP, "KeyP", K(0x19)),
        (KeyA, "KeyA", K(0x20)),
        (KeyS, "KeyS", K(0x21)),
        (KeyD, "KeyD", K(0x22)),
        (KeyF, "KeyF", K(0x23)),
        (KeyG, "KeyG", K(0x24)),
        (KeyH, "KeyH", K(0x25)),
        (KeyJ, "KeyJ", K(0x26)),
        (KeyK, "KeyK", K(0x27)),
        (KeyL, "KeyL", K(0x28)),
        (KeyZ, "KeyZ", K(0x31)),
        (KeyX, "KeyX", K(0x32)),
        (KeyC, "KeyC", K(0x33)),
        (KeyV, "KeyV", K(0x34)),
        (KeyB, "KeyB", K(0x35)),
        (KeyN, "KeyN", K(0x36)),
        (KeyM, "KeyM", K(0x37)),
        (Backspace, "Backspace", K(0x41)),
        (Tab, "Tab", K(0x42)),
        (Enter, "Enter", K(0x44)),
        (Escape, "Escape", K(0x45)),
        (Delete, "Delete", K(0x46)),
        (F1, "F1", K(0x50)),
        (F2, "F2", K(0x51)),
        (F3, "F3", K(0x52)),
        (F4, "F4", K(0x53)),
        (F5, "F5", K(0x54)),
        (F6, "F6", K(0x55)),
        (F7, "F7", K(0x56)),
        (F8, "F8", K(0x57)),
        (F9, "F9", K(0x58)),
        (F10, "F10", K(0x59)),
    ]
};

/// A host input event, from winit or from a script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostInput {
    Key { code: KeyCode, pressed: bool },
    /// The left mouse button over the window.
    MouseLeft { pressed: bool },
    /// The window lost focus. macOS sends no key-up for a key held across it (winit resets only
    /// the modifiers), so everything held is let go.
    FocusLost,
}

/// The winit code a script's key name stands for.
pub fn key_named(name: &str) -> Option<KeyCode> {
    KEYS.iter().find(|k| k.1 == name).map(|k| k.0)
}

fn target(code: KeyCode) -> Option<Target> {
    KEYS.iter().find(|k| k.0 == code).map(|k| k.2)
}

/// What is held, and what was pressed since the last frame.
#[derive(Debug, Default)]
pub struct Input {
    held: Vec<Target>,
    mouse: bool,
    /// Pressed since the last frame, so a press released within one frame is still seen once.
    tapped: FrameInput,
}

impl Input {
    pub fn event(&mut self, e: HostInput) {
        match e {
            HostInput::Key { code, pressed } => {
                let Some(t) = target(code) else { return };
                if pressed {
                    self.held.push(t);
                    self.tapped = with(self.tapped, t);
                } else if let Some(i) = self.held.iter().position(|&h| h == t) {
                    self.held.remove(i);
                }
            }
            HostInput::MouseLeft { pressed } => {
                self.mouse = pressed;
                if pressed {
                    self.tapped.joy = self.tapped.joy | Joy::PORT1_BUTTON;
                }
            }
            HostInput::FocusLost => {
                self.held.clear();
                self.mouse = false;
            }
        }
    }

    /// This frame's input: what is held, plus anything pressed and released since the last frame.
    pub fn frame(&mut self) -> FrameInput {
        let mut f = std::mem::take(&mut self.tapped);
        for &t in &self.held {
            f = with(f, t);
        }
        if self.mouse {
            f.joy = f.joy | Joy::PORT1_BUTTON;
        }
        f
    }
}

fn with(mut f: FrameInput, t: Target) -> FrameInput {
    match t {
        Target::Joy(j) => f.joy = f.joy | j,
        Target::Key(k) => f.keys = KeySet(f.keys.0 | KeySet::key(k).0),
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, pressed: bool) -> HostInput {
        HostInput::Key { code, pressed }
    }

    #[test]
    fn arrows_and_space_are_the_joystick_and_letters_are_amiga_keys() {
        let mut i = Input::default();
        i.event(key(KeyCode::ArrowLeft, true));
        i.event(key(KeyCode::Space, true));
        i.event(key(KeyCode::KeyP, true));
        let f = i.frame();
        assert_eq!(f.joy, Joy::LEFT | Joy::FIRE);
        assert_eq!(f.keys, KeySet::key(0x19));
        assert_eq!(i.frame(), f, "held keys stay held");
    }

    #[test]
    fn a_tap_inside_one_frame_is_seen_once() {
        let mut i = Input::default();
        i.event(key(KeyCode::ArrowUp, true));
        i.event(key(KeyCode::ArrowUp, false));
        assert_eq!(i.frame().joy, Joy::UP);
        assert_eq!(i.frame(), FrameInput::NONE);
    }

    #[test]
    fn the_mouse_is_port_1_s_button_and_unmapped_keys_do_nothing() {
        let mut i = Input::default();
        i.event(HostInput::MouseLeft { pressed: true });
        i.event(key(KeyCode::CapsLock, true));
        assert_eq!(i.frame(), FrameInput::joy(Joy::PORT1_BUTTON));
        i.event(HostInput::MouseLeft { pressed: false });
        assert_eq!(i.frame(), FrameInput::NONE);
    }

    #[test]
    fn two_keys_on_one_direction_hold_it_until_both_are_up() {
        let mut i = Input::default();
        i.event(key(KeyCode::ArrowLeft, true));
        i.event(key(KeyCode::ArrowLeft, true));
        i.event(key(KeyCode::ArrowLeft, false));
        i.frame();
        assert_eq!(i.frame().joy, Joy::LEFT);
    }

    #[test]
    fn losing_focus_lets_go_of_everything_held() {
        let mut i = Input::default();
        i.event(key(KeyCode::ArrowLeft, true));
        i.event(HostInput::MouseLeft { pressed: true });
        i.frame();
        i.event(HostInput::FocusLost);
        assert_eq!(i.frame(), FrameInput::NONE);
    }

    #[test]
    fn every_key_name_is_unique_and_found() {
        for (code, name, _) in KEYS {
            assert_eq!(key_named(name), Some(*code), "{name}");
            assert_eq!(KEYS.iter().filter(|k| k.1 == *name).count(), 1);
        }
        assert_eq!(key_named("NoSuchKey"), None);
    }
}
