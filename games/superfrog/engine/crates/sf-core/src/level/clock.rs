//! The level clock (`level_clock`, the frog's `+$a0`: four digits, minutes tens and units,
//! seconds tens and units), and the level's side of the keyboard's poll, both run by the vertical
//! blank.

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

/// The level's side of [`crate::keyboard::kbd_poll`]: its memory through the bus, and its
/// interrupt writes among the frame's writes to the custom chips.
impl crate::keyboard::KbdHost for LevelCtx<'_> {
    fn b(&self, addr: u32) -> u8 {
        self.bus.b(addr)
    }
    fn set_b(&mut self, addr: u32, v: u8) {
        self.bus.set_b(addr, v);
    }
    fn keyboard(&mut self) -> &mut crate::keyboard::CiaKeyboard {
        &mut self.bus.st.kbd
    }
    fn custom(&mut self, reg: u16, value: u16) {
        LevelCtx::custom(self, reg, value);
    }
}
