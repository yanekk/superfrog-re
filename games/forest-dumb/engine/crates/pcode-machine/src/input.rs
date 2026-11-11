//! One frame's input as the machine takes it: the joystick counters, the fire buttons and the keys
//! held. Its shape follows `amiga_input::FrameInput`, which the hand engine takes, so the app and
//! the cases feed both engines from one input ([`Input::from`]).

use amiga_input::{FrameInput, Joy, KeySet};

/// One frame's input.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Input {
    /// JOY1DAT: the port-2 joystick's counters.
    pub joy1: u16,
    /// Port 2's fire button (CIA-A PRA bit 7).
    pub fire: bool,
    /// The raw key codes held this frame (0 to `$7F`); the keyboard sends a byte for each change.
    pub keys: Vec<u8>,
    /// Port 1's button, the left mouse button (CIA-A PRA bit 6).
    pub mouse_left: bool,
}

impl Input {
    pub(crate) fn key_set(&self) -> KeySet {
        self.keys.iter().fold(KeySet::NONE, |s, &c| s.with(c))
    }
}

impl From<FrameInput> for Input {
    fn from(i: FrameInput) -> Input {
        Input {
            joy1: i.joy.joydat(),
            fire: i.joy.contains(Joy::FIRE),
            keys: i.keys.codes().collect(),
            mouse_left: i.joy.contains(Joy::PORT1_BUTTON),
        }
    }
}
