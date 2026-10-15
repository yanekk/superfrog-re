//! Palette fades (docs/12 "Palettes"): four fade records step colour moves of a copper list
//! towards a palette (mode 2, the level's fade-in, h0+$153a4), to black (mode 3, h0+$155a0) or by
//! fixed steps (mode 1, the boss's flash). The vertical blank steps every record each frame.
//!
//! A record (`$1f2` bytes): `+0` mode (0 off), `+2` frames between steps, `+4` the frame count,
//! `+6` set when done, `+8` colours, `+$a` the target palette (words), `+$e` the copper list's
//! first colour move less 2, `+$12` mode 1's step bytes, `+$72` per component the step (its
//! value × 256 / 15), `+$132` per component the running total.

use crate::layout::h0;
use crate::level::LevelCtx;
use crate::mem::Bus;

/// `fades` (h0+$157d8): the four fade records.
pub const FADES: [u32; 4] = [h0(0x157d8), h0(0x159ca), h0(0x15bbc), h0(0x15dae)];

/// The masks and shifts of a colour's components, red first.
const COMPONENTS: [(u16, u32); 3] = [(0xf00, 8), (0x0f0, 4), (0x00f, 0)];

/// The vertical blank's fades (h0+$15456, $156aa, $1528c, in that order): fade-ins, fade-outs,
/// then stepped fades, each over the four records.
pub fn fades_tick(c: &mut LevelCtx) {
    fade_in_tick(c);
    fade_out_tick(c);
    stepped_tick(c);
}

/// h0+$15456: the fade-ins.
pub fn fade_in_tick(c: &mut LevelCtx) {
    for r in FADES {
        fade_in_step(&mut c.bus, r);
    }
}

/// h0+$156aa: the fade-outs.
pub fn fade_out_tick(c: &mut LevelCtx) {
    for r in FADES {
        fade_out_step(&mut c.bus, r);
    }
}

/// h0+$1528c: the stepped fades.
pub fn stepped_tick(c: &mut LevelCtx) {
    for r in FADES {
        stepped_step(&mut c.bus, r);
    }
}

/// Whether record `r` in `mode` is due a step this frame (counting the frame).
fn due(b: &mut Bus, r: u32, mode: u16) -> bool {
    if b.w(r) != mode || b.w(r + 6) != 0 {
        return false;
    }
    let n = b.w(r + 4).wrapping_add(1);
    b.set_w(r + 4, n);
    if !crate::level::n_clear(n.wrapping_sub(b.w(r + 2))) {
        return false;
    }
    b.set_w(r + 4, 0);
    true
}

fn done(b: &mut Bus, r: u32) {
    b.set_w(r + 6, 1);
    b.set_w(r, 0);
}

/// h0+$1546c: every component not yet at the target's grows by its step.
fn fade_in_step(b: &mut Bus, r: u32) {
    if !due(b, r, 2) {
        return;
    }
    let mut a0 = b.l(r + 0xe);
    let mut a1 = b.l(r + 0xa);
    let n = b.w(r + 8) as u32;
    let all = (n * 3) as u16;
    let mut left = all;
    let (mut step, mut total) = (r + 0x72, r + 0x132);
    for _ in 0..n {
        a0 = a0.wrapping_add(4);
        a1 = a1.wrapping_add(2);
        for (mask, shift) in COMPONENTS {
            if b.w(a0) & mask != b.w(a1) & mask {
                let t = b.w(total).wrapping_add(b.w(step));
                b.set_w(total, t);
                let v = (t & 0xf00) >> (8 - shift);
                b.set_w(a0, (b.w(a0) & !mask).wrapping_add(v));
                left = left.wrapping_sub(1);
            }
            step += 2;
            total += 2;
        }
    }
    if left == all {
        done(b, r);
    }
}

/// h0+$156c0: every component not yet black falls by its step.
fn fade_out_step(b: &mut Bus, r: u32) {
    if !due(b, r, 3) {
        return;
    }
    let mut a0 = b.l(r + 0xe);
    let n = b.w(r + 8) as u32;
    let mut lit = 0u16;
    let (mut step, mut total) = (r + 0x72, r + 0x132);
    for _ in 0..n {
        a0 = a0.wrapping_add(4);
        for (mask, shift) in COMPONENTS {
            if b.w(a0) & mask != 0 {
                let t = b.w(total).wrapping_sub(b.w(step));
                b.set_w(total, t);
                let v = (t & 0xf00) >> (8 - shift);
                b.set_w(a0, (b.w(a0) & !mask).wrapping_add(v));
                lit = lit.wrapping_add(1);
            }
            step += 2;
            total += 2;
        }
    }
    if lit == 0 {
        done(b, r);
    }
}

/// h0+$152a2: each component not at the target's moves by its step byte.
fn stepped_step(b: &mut Bus, r: u32) {
    if !due(b, r, 1) {
        return;
    }
    let n = b.w(r + 8);
    let mut a0 = b.l(r + 0xe).wrapping_add(2);
    let mut a1 = b.l(r + 0xa);
    let mut a3 = r + 0x12;
    let mut same = 0u8;
    let colours = match n as u8 {
        0 => 256,
        k => k as u32,
    };
    for _ in 0..colours {
        for (k, (mask, shift)) in COMPONENTS.into_iter().enumerate() {
            let (cur, want) = (b.w(a0) & mask, b.w(a1) & mask);
            if k == 2 {
                a1 = a1.wrapping_add(2);
            }
            if cur == want {
                same = same.wrapping_add(1);
                a3 = a3.wrapping_add(1);
                continue;
            }
            let d3 = b.b(a3);
            a3 = a3.wrapping_add(1);
            let v = (((cur >> shift) as u8).wrapping_add(d3) as u16) << shift;
            let keep = match k {
                0 => 0x00ff,
                1 => 0x0f0f,
                _ => 0x0ff0,
            };
            b.set_w(a0, (b.w(a0) & keep) | v);
        }
        a0 = a0.wrapping_add(4);
    }
    if (same as u16 / 3) == n {
        done(b, r);
    }
}

/// `screen_fade_toggle` (h0+$3ca0).
const SCREEN_FADE_TOGGLE: u32 = h0(0x3ca0);

/// h0+$151b2 (mode 1), as the witch's defeat calls it in a level: record `r` steps the `n`
/// colours of the copper list at `list` (its first MOVE) by one a component towards `target`, from
/// `from`'s colours, which the list gets at once. Interrupts are off while the record changes.
pub fn stepped_start(c: &mut LevelCtx, r: u32, from: u32, target: u32, list: u32, n: u16) {
    c.custom(0x9a, 0x4000);
    let b = &mut c.bus;
    b.set_w(SCREEN_FADE_TOGGLE, 0);
    b.set_w(r, 1);
    b.set_w(r + 8, n);
    b.set_l(r + 0xa, target);
    b.set_l(r + 0xe, list);
    let colours = match n as u8 {
        0 => 256,
        k => k as u32,
    };
    let mut d = r + 0x12;
    for k in 0..colours {
        let (a, t) = (b.w(from + 2 * k), b.w(target + 2 * k));
        for (mask, _) in COMPONENTS {
            let (a, t) = (a & mask, t & mask);
            b.set_b(d, if t < a { 0xff } else { (t > a) as u8 });
            d += 1;
        }
    }
    for k in 0..colours {
        b.set_w(list + 2 + 4 * k, b.w(from + 2 * k));
    }
    b.set_w(r + 6, 0);
    c.custom(0x9a, 0xc000);
}
