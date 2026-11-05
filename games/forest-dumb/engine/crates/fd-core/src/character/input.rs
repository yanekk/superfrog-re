//! The joystick read (T51): `read_joystick` (`h0+$8dfc`), the first thing `game_frame` does each
//! pass. It reads JOY1DAT once into four flag words and appends them to `input_log` while the log
//! has room (docs/09-play-measured.md "Input"; docs/12-character-anim-input.md).

use std::sync::Arc;

use amiga_input::{FrameInput, Joy};

/// `input_log_index` stops here: `input_log` (`h0+$34c28`) holds `$2ee0` bytes, one a pass.
pub const LOG_FULL: u32 = 0x2ee0;

/// `joy_left`, `joy_right`, `joy_up`, `joy_down` (`h0+$8df2`..`$8df8`): 1 or 0, words.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct JoyFlags {
    pub left: u16,
    pub right: u16,
    pub up: u16,
    pub down: u16,
}

/// `input_log` and its index. Nothing in the original reads the log back (`re/symbols.toml`); it is
/// kept because the original writes it. Shared until written, so a clone of the game costs nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct InputLog {
    /// `input_log_index` (`h0+$8ec0`), long: the next byte to write.
    pub index: u32,
    /// The bytes written so far, `[0]` at `input_log`; empty until the first.
    bytes: Arc<Vec<u8>>,
}

impl InputLog {
    /// An index with nothing written behind it, as a transplant leaves it.
    pub fn at(index: u32) -> InputLog {
        InputLog {
            index,
            bytes: Arc::default(),
        }
    }

    /// The byte at `i`: 0 where nothing was written (a transplanted index has no bytes behind it).
    pub fn byte(&self, i: u32) -> u8 {
        self.bytes.get(i as usize).copied().unwrap_or(0)
    }
}

/// The JOY1DAT word a joystick in port 2 gives: horizontal count in bits 0-1 and 8-9 as the
/// hardware encodes direction (right sets bit 1, left bit 9, down flips bit 0 from bit 1, up flips
/// bit 8 from bit 9). The mouse counters a joystick leaves at 0. Fire is not on JOY1DAT.
pub fn joy1dat(i: &FrameInput) -> u16 {
    let (l, r) = (i.joy.contains(Joy::LEFT), i.joy.contains(Joy::RIGHT));
    let (u, d) = (i.joy.contains(Joy::UP), i.joy.contains(Joy::DOWN));
    let (b9, b1) = (l as u16, r as u16);
    b9 << 9 | (b9 ^ u as u16) << 8 | b1 << 1 | (b1 ^ d as u16)
}

impl JoyFlags {
    /// `read_joystick`: the flags from a pass's input, then appended to `log`.
    pub fn read_joystick(&mut self, log: &mut InputLog, i: &FrameInput) {
        self.read_joy1dat(log, joy1dat(i));
    }

    /// `read_joystick` on the JOY1DAT word it reads (once, as the original does).
    pub fn read_joy1dat(&mut self, log: &mut InputLog, dat: u16) {
        let bit = |w: u16, n: u32| (w >> n) & 1;
        let x = dat ^ (dat >> 1);
        *self = JoyFlags {
            left: bit(dat, 9),
            right: bit(dat, 1),
            up: bit(x, 8),
            down: bit(x, 0),
        };
        if log.index == LOG_FULL {
            return;
        }
        let byte = (self.left | self.right << 1 | self.up << 2 | self.down << 3) as u8;
        // The original writes `input_log + index` for any index but $2ee0. Past $2ee0 that lands
        // outside the log; the game never gets there (it counts up from 0 and stops at $2ee0), so
        // such a byte is not kept: an exclusion, docs/12-character-anim-input.md.
        if log.index < LOG_FULL {
            let bytes = Arc::make_mut(&mut log.bytes);
            let at = log.index as usize;
            if bytes.len() <= at {
                bytes.resize(at + 1, 0);
            }
            bytes[at] = byte;
        }
        log.index = log.index.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_direction_reads_back_as_its_flag() {
        for bits in 0..16u8 {
            let joy = Joy(bits);
            let mut f = JoyFlags::default();
            let mut log = InputLog::default();
            f.read_joystick(&mut log, &FrameInput::joy(joy | Joy::FIRE));
            let want = JoyFlags {
                left: joy.contains(Joy::LEFT) as u16,
                right: joy.contains(Joy::RIGHT) as u16,
                up: joy.contains(Joy::UP) as u16,
                down: joy.contains(Joy::DOWN) as u16,
            };
            assert_eq!(f, want, "{joy:?}");
            // Left is bit 0 of the log byte, right 1, up 2, down 3.
            let byte = f.left | f.right << 1 | f.up << 2 | f.down << 3;
            assert_eq!((log.index, log.byte(0)), (1, byte as u8));
        }
    }

    #[test]
    fn a_full_log_takes_nothing_and_flags_still_change() {
        let mut f = JoyFlags::default();
        let mut log = InputLog::at(LOG_FULL);
        // Left alone: bit 9, and bit 8 equal to it.
        f.read_joy1dat(&mut log, 0x0300);
        assert_eq!(
            (f.left, f.up, log.index, log.byte(LOG_FULL)),
            (1, 0, LOG_FULL, 0)
        );
        let mut log = InputLog::at(LOG_FULL - 1);
        // Bit 1 alone is right and down: down is bit 1 xor bit 0.
        f.read_joy1dat(&mut log, 0x0002);
        assert_eq!((log.index, log.byte(LOG_FULL - 1)), (LOG_FULL, 2 | 8));
    }

    #[test]
    fn a_clone_shares_the_log_until_one_writes() {
        let mut f = JoyFlags::default();
        let mut a = InputLog::default();
        f.read_joy1dat(&mut a, 3);
        let mut b = a.clone();
        assert!(Arc::ptr_eq(&a.bytes, &b.bytes));
        f.read_joy1dat(&mut b, 0x300);
        assert_eq!((a.index, a.byte(1)), (1, 0));
        assert_eq!((b.index, b.byte(0), b.byte(1)), (2, 2, 1));
    }
}
