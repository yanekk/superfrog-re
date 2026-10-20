//! The level clock (`level_clock`, the frog's `+$a0`: four digits, minutes tens and units,
//! seconds tens and units) and the keyboard, both run by the vertical blank.

use crate::frog::FROG;
use crate::layout::h0;
use crate::level::{LevelCtx, n_clear};
use crate::mem::Bus;

/// `level_clock` (h0+$876): its four digit bytes.
const CLOCK: u32 = FROG + 0xa0;
/// `clock_ticks` (h0+$11dfc): vertical blanks into the current second.
const TICKS: u32 = h0(0x11dfc);
/// The clock's colour moves in the level copper list (its two colours, words at +$1f2, +$1f6).
const CLOCK_COLOURS: u32 = crate::layout::h2(0x6bd0) + 0x1f2;
/// `world` (h0+$40da).
const WORLD: u32 = h0(0x40da);

/// `level_clock_tick` (h0+$11d2c): the clock's colours, then a second off the clock every 50
/// vertical blanks (none in the bonus level).
pub fn level_clock_tick(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.l(WORLD) == 7 {
        return;
    }
    clock_colours(b);
    let t = b.w(TICKS).wrapping_add(1);
    b.set_w(TICKS, t);
    if !n_clear(t.wrapping_sub(0x32)) {
        return;
    }
    b.set_w(TICKS, 0);
    if b.l(CLOCK) == 0 {
        return;
    }
    // Seconds units 9..0, tens 5..0, minutes units 9..0, then the minutes' tens.
    for (k, wrap) in [(3, 9u8), (2, 5), (1, 9)] {
        let d = b.b(CLOCK + k).wrapping_sub(1);
        b.set_b(CLOCK + k, d);
        if (d as i8) >= 0 {
            return;
        }
        b.set_b(CLOCK + k, wrap);
    }
    b.set_b(CLOCK, b.b(CLOCK).wrapping_sub(1));
}

/// h0+$11da6: the clock shows in its normal colours, and in its last ten seconds flashes red for
/// the first half of each.
fn clock_colours(b: &mut Bus) {
    let normal = b.l(CLOCK) == 0 || b.w(CLOCK) != 0 || b.b(CLOCK + 2) != 0;
    if normal && b.l(CLOCK) != 0 {
        return;
    }
    b.set_w(CLOCK_COLOURS, 0x149);
    b.set_w(CLOCK_COLOURS + 4, 0xbf);
    if b.l(CLOCK) == 0 {
        return;
    }
    if n_clear(b.w(TICKS).wrapping_sub(0x19)) {
        return;
    }
    b.set_w(CLOCK_COLOURS, 0x800);
    b.set_w(CLOCK_COLOURS + 4, 0xf00);
}

/// `kbd_toggle`, `key_down`, `key_up` (h0+$16076..$16079).
const KBD_TOGGLE: u32 = h0(0x16076);
const KEY_DOWN: u32 = h0(0x16078);
const KEY_UP: u32 = h0(0x16079);

/// `kbd_poll` (h0+$15ff2): every second vertical blank the byte in CIA-A's serial register is
/// the keyboard's last ([`crate::level::Hw::sdr`]): a key going down or up. The original masks the keyboard
/// interrupt around it (INTENA, INTREQ `$0008`) and hands the line back when a byte came in,
/// writing `$ff` to the register, which it then reads (T191: P let go reads as no key after).
pub fn kbd_poll(c: &mut LevelCtx) {
    if c.hw.key_in {
        c.bus.st.kbd_flag = true;
        c.bus.st.kbd_acked = false;
    }
    let sdr = if c.bus.st.kbd_acked { 0xff } else { c.hw.sdr };
    let b = &mut c.bus;
    let t = !b.b(KBD_TOGGLE);
    b.set_b(KBD_TOGGLE, t);
    if t == 0 {
        return;
    }
    c.custom(0x9a, 0x0008);
    let v = !sdr;
    let code = (v >> 1) & 0x7f;
    let b = &mut c.bus;
    if v & 1 == 0 {
        b.set_b(KEY_DOWN, code);
        b.set_b(KEY_UP, 0);
    } else {
        b.set_b(KEY_UP, code);
        b.set_b(KEY_DOWN, 0);
    }
    if std::mem::take(&mut b.st.kbd_flag) {
        b.st.kbd_acked = true;
    }
    c.custom(0x9c, 0x0008);
}
