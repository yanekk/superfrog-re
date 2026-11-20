//! The keyboard as the original reads it: CIA-A's serial port and `kbd_poll` (h0+$15ff2), one
//! adapter for every handler that polls it (the level's vertical blank, the front end's, game
//! over's borrowed level blank). The keyboard sends a byte for each key going down or up; CIA-A's
//! serial data register holds the last one and its serial-port flag says it came in; every second
//! vertical blank `kbd_poll` decodes the register into `key_down`/`key_up` and, once it has
//! handled a new byte, writes `$ff` there, which it then reads (T191: P let go reads as no key
//! after). The keyboard and CIA-A's serial port themselves ([`CiaKeyboard`], [`key_byte`]) are the
//! Amiga's and live in the toolkit's `amiga-input`; `kbd_poll` at Superfrog's addresses stays here.

pub use amiga_input::{CiaKeyboard, key_byte};

use crate::layout::h0;

/// `kbd_toggle`, `key_down`, `key_up` (h0+$16076..$16079): the original's own, compared.
pub const KBD_TOGGLE: u32 = h0(0x16076);
pub const KEY_DOWN: u32 = h0(0x16078);
pub const KEY_UP: u32 = h0(0x16079);

/// INTENA, INTREQ: `kbd_poll` masks the keyboard's interrupt (`$0008`) around its work and
/// hands the line back after.
const INTENA: u16 = 0x9a;
const INTREQ: u16 = 0x9c;

/// Whether the next [`kbd_poll`] decodes the register (it does every second blank).
pub fn polls(kbd_toggle: u8) -> bool {
    kbd_toggle == 0
}

/// What `kbd_poll` runs on: the original's memory, the keyboard, and the custom registers its
/// interrupt writes go to, in the caller's own write order (the level's `custom`, the front end's
/// `wr`).
pub trait KbdHost {
    fn b(&self, addr: u32) -> u8;
    fn set_b(&mut self, addr: u32, v: u8);
    fn keyboard(&mut self) -> &mut CiaKeyboard;
    fn custom(&mut self, reg: u16, value: u16);
}

/// `kbd_poll` (h0+$15ff2): every second vertical blank the register's byte into `key_down` or
/// `key_up` (the other cleared), the keyboard's interrupt masked around it; a new byte handled,
/// the register reads `$ff` from then on.
pub fn kbd_poll(h: &mut impl KbdHost) {
    let t = !h.b(KBD_TOGGLE);
    h.set_b(KBD_TOGGLE, t);
    if t == 0 {
        return;
    }
    h.custom(INTENA, 0x0008);
    let k = h.keyboard();
    let v = !k.sdr;
    if std::mem::take(&mut k.new) {
        k.sdr = 0xff;
    }
    let code = (v >> 1) & 0x7f;
    if v & 1 == 0 {
        h.set_b(KEY_DOWN, code);
        h.set_b(KEY_UP, 0);
    } else {
        h.set_b(KEY_UP, code);
        h.set_b(KEY_DOWN, 0);
    }
    h.custom(INTREQ, 0x0008);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::KeySet;

    fn decoded(k: &CiaKeyboard) -> (u8, u8) {
        (!k.sdr >> 1 & 0x7f, !k.sdr & 1)
    }

    #[test]
    fn the_keyboard_sends_a_byte_per_change_and_reads_ff_once_handled() {
        struct Mem(Vec<u8>, CiaKeyboard, Vec<(u16, u16)>);
        impl KbdHost for Mem {
            fn b(&self, a: u32) -> u8 {
                self.0[(a - KBD_TOGGLE) as usize]
            }
            fn set_b(&mut self, a: u32, v: u8) {
                self.0[(a - KBD_TOGGLE) as usize] = v;
            }
            fn keyboard(&mut self) -> &mut CiaKeyboard {
                &mut self.1
            }
            fn custom(&mut self, reg: u16, value: u16) {
                self.2.push((reg, value));
            }
        }
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

        // The poll: every second blank, the byte decoded, then `$ff` once handled.
        let mut m = Mem(vec![0xff, 0, 0, 0], k, Vec::new());
        kbd_poll(&mut m);
        assert_eq!((m.0[0], m.2.len()), (0, 0), "the toggle's off blank polls nothing");
        kbd_poll(&mut m);
        assert_eq!((m.0[2], m.0[3]), (0, 0x10));
        assert_eq!(m.2, vec![(INTENA, 8), (INTREQ, 8)]);
        assert_eq!((m.1.sdr, m.1.new), (0xff, false));
        kbd_poll(&mut m);
        kbd_poll(&mut m);
        assert_eq!((m.0[2], m.0[3]), (0, 0), "a handled byte reads as no key");
    }
}
