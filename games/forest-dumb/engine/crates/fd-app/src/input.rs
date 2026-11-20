//! The one point where host input becomes the original's input (DESIGN §2.7), as `sf-app`'s
//! `input.rs`: the window's keys, the pads' events and a script's events all arrive as
//! [`HostInput`]; each step [`Input::frame`] gives the [`FrameInput`] the game steps on.
//!
//! Esc quits (DESIGN §2.7), so it never reaches the game. With ⌘ held a key is the app's: ⌘Q
//! quits, as the menu's shortcut does when a script's keys arrive at it. Losing focus lets go of
//! every key held and play goes on; a pad is not the window's and keeps what it holds.

use amiga_app::keys::KeyMap;
use amiga_app::pad::{PadMap, PadState};
use amiga_input::FrameInput;
use fd_core::character::death::KEY_P;
use winit::keyboard::KeyCode;

pub use amiga_app::script_core::HostInput;

use crate::menu::Command;

#[derive(Debug, Default)]
pub struct Input {
    pub map: KeyMap,
    pub pad_map: PadMap,
    keys: Vec<KeyCode>,
    pads: PadState,
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

    /// Take one host event: `Some` when it is the app's, not the game's.
    pub fn event(&mut self, e: HostInput) -> Option<Command> {
        match e {
            HostInput::Key {
                code,
                pressed: true,
            } => {
                if code == KeyCode::Escape {
                    return Some(Command::Quit);
                }
                if !is_command(code) && self.keys.iter().any(|&k| is_command(k)) {
                    return (code == KeyCode::KeyQ).then_some(Command::Quit);
                }
                self.keys.push(code);
                for t in self.map.targets(code) {
                    self.tapped = t.apply(self.tapped);
                }
            }
            HostInput::Key {
                code,
                pressed: false,
            } => {
                if let Some(i) = self.keys.iter().position(|&h| h == code) {
                    self.keys.remove(i);
                }
            }
            // The game reads no mouse.
            HostInput::MouseLeft { .. } => {}
            HostInput::FocusLost => self.keys.clear(),
            HostInput::Pad(e) => {
                for p in self.pads.apply(e) {
                    for t in self.pad_map.targets(p) {
                        self.tapped = t.apply(self.tapped);
                    }
                }
            }
        }
        None
    }

    pub fn pads(&self) -> &PadState {
        &self.pads
    }

    /// Let go of every key held: the Controls window taking the keyboard.
    pub fn release_keys(&mut self) {
        self.keys.clear();
    }

    /// This frame's input: what is held, plus anything pressed and released since the last frame.
    ///
    /// Never the Amiga's P: the app has no pause (DESIGN §2.7; the owner kept it, 2026-11-05), and
    /// since T23 the engine pauses on P as the original's `play_keys` does, which cases need. The
    /// default map would otherwise give it the Mac's P.
    pub fn frame(&mut self) -> FrameInput {
        let mut f = std::mem::take(&mut self.tapped);
        for &code in &self.keys {
            for t in self.map.targets(code) {
                f = t.apply(f);
            }
        }
        for p in self.pads.held() {
            for t in self.pad_map.targets(p) {
                f = t.apply(f);
            }
        }
        f.keys = f.keys.without(KEY_P);
        f
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amiga_app::pad::{PadEvent, PadInput};
    use amiga_input::Joy;

    fn key(code: KeyCode, pressed: bool) -> HostInput {
        HostInput::Key { code, pressed }
    }

    fn input() -> Input {
        Input::new(crate::settings::default_keys(), PadMap::default())
    }

    #[test]
    fn space_is_up_and_left_option_fire() {
        let mut i = input();
        i.event(key(KeyCode::Space, true));
        i.event(key(KeyCode::AltLeft, true));
        i.event(key(KeyCode::ArrowRight, true));
        let f = i.frame();
        assert_eq!(f.joy, Joy::UP | Joy::FIRE | Joy::RIGHT);
        assert_eq!(i.frame(), f, "held keys stay held");
    }

    #[test]
    fn a_press_released_within_a_frame_is_seen_once() {
        let mut i = input();
        i.event(key(KeyCode::Space, true));
        i.event(key(KeyCode::Space, false));
        assert_eq!(i.frame().joy, Joy::UP);
        assert_eq!(i.frame().joy, Joy::NONE);
    }

    #[test]
    fn esc_and_command_q_quit_and_never_reach_the_game() {
        let mut i = input();
        assert_eq!(i.event(key(KeyCode::Escape, true)), Some(Command::Quit));
        i.event(key(KeyCode::SuperLeft, true));
        assert_eq!(i.event(key(KeyCode::KeyQ, true)), Some(Command::Quit));
        assert_eq!(i.event(key(KeyCode::Space, true)), None);
        assert_eq!(i.frame(), FrameInput::NONE, "a key with ⌘ is the app's");
    }

    #[test]
    fn p_never_reaches_the_game_though_the_default_map_names_it() {
        let mut i = input();
        assert!(
            i.map.targets(KeyCode::KeyP).next().is_some(),
            "the default map binds P"
        );
        i.event(key(KeyCode::KeyP, true));
        assert_eq!(i.frame(), FrameInput::NONE);
    }

    #[test]
    fn losing_focus_lets_go_of_keys_but_not_pads() {
        let mut i = input();
        i.event(HostInput::Pad(PadEvent::Connected(1)));
        i.event(HostInput::Pad(PadEvent::Button {
            pad: 1,
            button: PadInput::DPadLeft,
            pressed: true,
        }));
        i.event(key(KeyCode::Space, true));
        i.frame();
        i.event(HostInput::FocusLost);
        assert_eq!(i.frame().joy, Joy::LEFT);
    }
}
