//! The keyboard as CIA-A's serial port presents it. The keyboard sends a byte for each key going
//! down or up; CIA-A's serial data register holds the last one and its serial-port flag says it
//! came in. A game's own poll routine (it reads the register and acknowledges it) stays with the
//! game; this is the hardware half every game's poll reads.

use crate::KeySet;

/// The byte the keyboard sends for `code` going down or up: the code shifted up one, bit 0 set
/// for a key going up, all inverted.
pub fn key_byte(code: u8, up: bool) -> u8 {
    !(code << 1 | up as u8)
}

/// The keyboard and CIA-A's serial port: the keys the keyboard has seen held, the register's
/// byte as a game's poll reads it, and the serial-port flag. Part of a game's state, so a stretch
/// run again from its start sees the keyboard as it was then.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct CiaKeyboard {
    held: KeySet,
    /// CIA-A's serial data register (`$BFEC01`).
    pub sdr: u8,
    /// The serial-port flag: a byte has come in that the game's poll has not handled.
    pub new: bool,
}

impl Default for CiaKeyboard {
    /// No byte a poll would decode as a key: what a game started from its own state reads.
    fn default() -> CiaKeyboard {
        CiaKeyboard {
            held: KeySet::NONE,
            sdr: 0xff,
            new: false,
        }
    }
}

impl CiaKeyboard {
    /// At power-on the register holds what the keyboard's start-up ends with: `$7e` released
    /// (measured), never acknowledged.
    pub fn power_on() -> CiaKeyboard {
        CiaKeyboard {
            sdr: key_byte(0x7e, true),
            ..CiaKeyboard::default()
        }
    }

    /// The register as a poll last left a key-down and a key-up byte in memory: the last byte it
    /// decoded, or `$ff` once it has handled one (both read 0). For a start from that memory,
    /// where no keyboard was carried over.
    pub fn from_keys(key_down: u8, key_up: u8) -> CiaKeyboard {
        let sdr = match (key_down, key_up) {
            (_, up) if up != 0 => key_byte(up, true),
            (down, _) if down != 0 => key_byte(down, false),
            _ => 0xff,
        };
        CiaKeyboard {
            sdr,
            ..CiaKeyboard::default()
        }
    }

    /// The keys held this frame: the keyboard sends a byte for each key gone down (by code), then
    /// each gone up, and the register keeps the last.
    pub fn feed(&mut self, keys: KeySet) {
        for (code, up) in keys
            .codes()
            .map(|c| (c, false))
            .chain(self.held.codes().map(|c| (c, true)))
        {
            let changed = if up {
                !keys.contains(code)
            } else {
                !self.held.contains(code)
            };
            if changed {
                self.sdr = key_byte(code, up);
                self.new = true;
            }
        }
        self.held = keys;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::Options;

    fn decoded(k: &CiaKeyboard) -> (u8, u8) {
        (!k.sdr >> 1 & 0x7f, !k.sdr & 1)
    }

    #[test]
    fn the_keyboard_sends_a_byte_per_change() {
        let mut k = CiaKeyboard::power_on();
        // Power-on: $7e released, never acknowledged.
        assert_eq!(decoded(&k), (0x7e, 1));
        assert!(!k.new);
        k.feed(KeySet::key(0x19));
        assert_eq!((decoded(&k), k.new), ((0x19, 0), true));
        k.feed(KeySet::key(0x19));
        assert!(k.new, "holding a key sends nothing more");
        k.feed(KeySet::NONE);
        assert_eq!(decoded(&k), (0x19, 1));
        // A key down and another up in one frame: the up byte is the last sent.
        k.feed(KeySet::key(0x45));
        k.feed(KeySet::key(0x10));
        assert_eq!(decoded(&k), (0x45, 1));
        k.feed(KeySet::key(0x45));
        assert_eq!(decoded(&k), (0x10, 1));
    }

    #[test]
    fn from_keys_puts_back_the_last_decoded_byte() {
        assert_eq!(decoded(&CiaKeyboard::from_keys(0x19, 0)), (0x19, 0));
        assert_eq!(decoded(&CiaKeyboard::from_keys(0x19, 0x45)), (0x45, 1));
        assert_eq!(CiaKeyboard::from_keys(0, 0).sdr, 0xff);
        assert!(!CiaKeyboard::from_keys(0x19, 0).new);
        assert_eq!(key_byte(0x7e, true), 0x02);
    }

    /// Save states hold these types: their bytes under fixed-width little-endian bincode are the
    /// ones written before the move to the toolkit, recorded from that build.
    #[test]
    fn save_state_bytes_are_unchanged() {
        let o = bincode::DefaultOptions::new()
            .with_fixint_encoding()
            .with_little_endian();
        let mut k = CiaKeyboard::power_on();
        k.feed(KeySet::key(0x19).with(0x7f));
        let mut want = vec![0u8; 16];
        want[3] = 0x02;
        want[15] = 0x80;
        want.extend([0x01, 0x01]);
        assert_eq!(o.serialize(&k).unwrap(), want);
        assert_eq!(o.deserialize::<CiaKeyboard>(&want).unwrap(), k);

        let mut want = vec![0u8; 16];
        want[0] = 0x01;
        want[8] = 0x20;
        want[15] = 0x80;
        let keys = KeySet::key(0).with(0x45).with(0x7f);
        assert_eq!(o.serialize(&keys).unwrap(), want);
        assert_eq!(o.deserialize::<KeySet>(&want).unwrap(), keys);

        let mut want = vec![0u8; 16];
        want.extend([0xff, 0x00]);
        assert_eq!(o.serialize(&CiaKeyboard::default()).unwrap(), want);
    }
}
