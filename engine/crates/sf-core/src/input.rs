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

/// The joystick tokens a script writes, in the order it writes them.
const JOY_TOKENS: [(Joy, &str); 6] = [
    (Joy::UP, "U"),
    (Joy::DOWN, "D"),
    (Joy::LEFT, "L"),
    (Joy::RIGHT, "R"),
    (Joy::FIRE, "F"),
    (Joy::PORT1_BUTTON, "M"),
];

/// One input as a script and a recording write it (bug-recorder DESIGN §2.6): `R F`, `K19`, `-`.
/// Joystick tokens `U D L R F M` in that order, then `K<hex>` per held key, lowest first; `-` for
/// nothing held.
pub fn format_tokens(input: FrameInput) -> String {
    let mut out: Vec<String> = JOY_TOKENS
        .iter()
        .filter(|(j, _)| input.joy.contains(*j))
        .map(|(_, n)| n.to_string())
        .collect();
    out.extend(input.keys.codes().map(|c| format!("K{c:02X}")));
    if out.is_empty() {
        "-".to_string()
    } else {
        out.join(" ")
    }
}

/// One token: `Some(Ok(what it holds))`, `Some(Err(why))` for a malformed key, `None` for a token
/// this format does not know (sf-conform's scripts add `C<entry>.<k>`, which needs the disks).
pub fn parse_token(t: &str) -> Option<Result<FrameInput, String>> {
    if t == "-" {
        return Some(Ok(FrameInput::NONE));
    }
    if let Some(&(joy, _)) = JOY_TOKENS.iter().find(|(_, n)| *n == t) {
        return Some(Ok(FrameInput::joy(joy)));
    }
    let hex = t.strip_prefix('K')?;
    Some(
        (hex.len() == 2 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| u8::from_str_radix(hex, 16).ok())
            .flatten()
            .filter(|&c| c < 0x80)
            .map(FrameInput::key)
            .ok_or_else(|| format!("`{t}`: a key is K and two hex digits, 00..7F")),
    )
}

/// The whitespace-separated tokens of [`format_tokens`] (nothing at all is nothing held).
pub fn parse_tokens(s: &str) -> Result<FrameInput, String> {
    let mut input = FrameInput::NONE;
    for t in s.split_whitespace() {
        let one = parse_token(t)
            .ok_or_else(|| format!("`{t}` is not an input (U D L R F M K<hex> -)"))??;
        input.joy = input.joy | one.joy;
        input.keys = KeySet(input.keys.0 | one.keys.0);
    }
    Ok(input)
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

    #[test]
    fn tokens_round_trip_and_refuse_what_they_do_not_know() {
        let i = FrameInput {
            joy: Joy::UP | Joy::DOWN | Joy::LEFT | Joy::PORT1_BUTTON,
            keys: KeySet::key(0x19).with(0x45),
        };
        assert_eq!(format_tokens(i), "U D L M K19 K45");
        assert_eq!(parse_tokens("U D L M K19 K45"), Ok(i));
        assert_eq!(format_tokens(FrameInput::NONE), "-");
        assert_eq!(parse_tokens("-"), Ok(FrameInput::NONE));
        assert_eq!(parse_tokens(""), Ok(FrameInput::NONE));
        for joy in 0..64 {
            for keys in [KeySet::NONE, KeySet::key(0), KeySet::key(0x7F).with(0x0a)] {
                let i = FrameInput {
                    joy: Joy(joy),
                    keys,
                };
                assert_eq!(parse_tokens(&format_tokens(i)), Ok(i));
            }
        }
        for (bad, says) in [
            ("Q", "not an input"),
            ("C8.0", "not an input"),
            ("K80", "00..7F"),
            ("K1", "two hex digits"),
            ("K+1", "two hex digits"),
            ("R K", "two hex digits"),
        ] {
            let e = parse_tokens(bad).unwrap_err();
            assert!(e.contains(says), "{bad}: {e}");
        }
        assert!(parse_token("C8.0").is_none());
    }
}
