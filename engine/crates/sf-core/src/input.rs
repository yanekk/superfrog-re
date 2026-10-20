//! What the player does in one frame, as the original sees it (DESIGN §2.6): the joystick and the
//! keys held. Both the engine and the oracle take this type, so a script drives either one.

/// Joystick bits. Directions and fire are the joystick in port 2, the game's controller.
/// `PORT1_BUTTON` is the left mouse button in port 1 (the same pin as a port-1 joystick's fire),
/// which the crack intro on disk 1 waits for before the game loads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Joy(pub u8);

impl Joy {
    pub const NONE: Joy = Joy(0);
    pub const UP: Joy = Joy(1);
    pub const DOWN: Joy = Joy(2);
    pub const LEFT: Joy = Joy(4);
    pub const RIGHT: Joy = Joy(8);
    pub const FIRE: Joy = Joy(16);
    pub const PORT1_BUTTON: Joy = Joy(32);

    /// Every bit this type defines.
    pub const ALL: Joy = Joy(63);

    pub fn contains(self, other: Joy) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn bits(self) -> u8 {
        self.0
    }
}

impl core::ops::BitOr for Joy {
    type Output = Joy;
    fn bitor(self, rhs: Joy) -> Joy {
        Joy(self.0 | rhs.0)
    }
}

/// The keys held, by the original's raw key code (0x00 to 0x7F, e.g. 0x19 = P, 0x40 = space).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct KeySet(pub u128);

impl KeySet {
    pub const NONE: KeySet = KeySet(0);

    /// The set holding exactly `code`. Codes are 7 bits; the top bit is ignored.
    pub fn key(code: u8) -> KeySet {
        KeySet(1u128 << (code & 0x7F))
    }

    pub fn contains(self, code: u8) -> bool {
        self.0 >> (code & 0x7F) & 1 == 1
    }

    pub fn with(self, code: u8) -> KeySet {
        KeySet(self.0 | KeySet::key(code).0)
    }

    pub fn without(self, code: u8) -> KeySet {
        KeySet(self.0 & !KeySet::key(code).0)
    }

    /// The held codes, lowest first.
    pub fn codes(self) -> impl Iterator<Item = u8> {
        (0u8..128).filter(move |&c| self.contains(c))
    }

    /// Bit k of byte k/8 set for each held code k.
    pub fn to_bytes(self) -> [u8; 16] {
        self.0.to_le_bytes()
    }
}

/// One frame's input.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FrameInput {
    pub joy: Joy,
    pub keys: KeySet,
}

impl FrameInput {
    pub const NONE: FrameInput = FrameInput {
        joy: Joy::NONE,
        keys: KeySet::NONE,
    };

    pub fn joy(joy: Joy) -> FrameInput {
        FrameInput {
            joy,
            keys: KeySet::NONE,
        }
    }

    pub fn key(code: u8) -> FrameInput {
        FrameInput {
            joy: Joy::NONE,
            keys: KeySet::key(code),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joy_bits_combine_and_test() {
        let j = Joy::UP | Joy::FIRE;
        assert!(j.contains(Joy::UP) && j.contains(Joy::FIRE) && !j.contains(Joy::DOWN));
        assert_eq!(j.bits(), 17);
        assert_eq!(
            Joy::ALL.bits(),
            (Joy::UP | Joy::DOWN | Joy::LEFT | Joy::RIGHT | Joy::FIRE | Joy::PORT1_BUTTON).bits()
        );
    }

    #[test]
    fn keyset_holds_raw_codes() {
        let k = KeySet::NONE.with(0x19).with(0x7F).with(0);
        assert!(k.contains(0x19) && k.contains(0x7F) && k.contains(0) && !k.contains(0x40));
        assert_eq!(k.codes().collect::<Vec<_>>(), vec![0, 0x19, 0x7F]);
        assert!(!k.without(0x19).contains(0x19));
        let b = KeySet::key(9).to_bytes();
        assert_eq!((b[1], b[0]), (2, 0));
        assert_eq!(FrameInput::key(0x19).keys, KeySet::key(0x19));
    }
}
