//! Hardware-sprite records (h0+$1b208 and $1b23e): each puts one sprite, or an attached pair, on
//! screen by rewriting the sprite MOVEs of a copper list.
//!
//! A record (`$1c` bytes): `+0` x, `+2` y, `+4` height, `+6` the CTL bits to OR in (`$80`
//! attaches the pair), `+8` the copper list's SPRxPTH MOVE, `+$c` the sprite data, `+$10` an
//! animation (a list of (data, _, frames) entries of 8 bytes ending in a negative long; 0 for
//! none), `+$14` the current entry, `+$18` frames left on it.

use crate::state::State;

/// `$1b208`: point the copper list at the record's sprite (its animation's first frame, if any).
pub fn attach(s: &mut State, rec: u32) {
    let anim = s.l(rec + 0x10);
    let data = if anim != 0 {
        s.set_l(rec + 0x14, anim);
        let n = s.w(anim + 6);
        s.set_w(rec + 0x18, n);
        s.l(anim)
    } else {
        s.l(rec + 0xc)
    };
    let mv = s.l(rec + 8);
    let ctl = s.w(mv + 0xe) | s.w(rec + 6);
    s.set_w(mv + 0xe, ctl);
    s.set_w(mv + 6, data as u16);
    s.set_w(mv + 2, (data >> 16) as u16);
}

/// `$1b23e`: the record's position (and animation frame) into its copper MOVEs.
pub fn update(s: &mut State, rec: u32) {
    let mv = s.l(rec + 8);
    if s.l(rec + 0x10) != 0 {
        let left = s.w(rec + 0x18).wrapping_sub(1);
        s.set_w(rec + 0x18, left);
        if (left as i16) < 0 {
            s.set_l(rec + 0x14, s.l(rec + 0x14).wrapping_add(8));
            loop {
                let e = s.l(rec + 0x14);
                let d0 = s.l(e);
                if (d0 as i32) < 0 {
                    s.set_l(rec + 0x14, s.l(rec + 0x10));
                    continue;
                }
                let n = s.w(e + 6);
                s.set_w(rec + 0x18, n);
                s.set_w(mv + 6, d0 as u16);
                s.set_w(mv + 2, (d0 >> 16) as u16);
                break;
            }
        }
    }
    let ctl = s.w(mv + 0xe) & 0x80;
    s.set_w(mv + 0xe, ctl);
    let x = s.w(rec).wrapping_add(0x80);
    if x & 1 != 0 {
        s.set_w(mv + 0xe, ctl | 1);
    }
    s.set_b(mv + 0xb, (x >> 1) as u8);
    let y = s.w(rec + 2).wrapping_add(0x2c);
    let mut stop = y.wrapping_add(s.w(rec + 4));
    if stop as i16 >= 0x100 {
        stop = stop.wrapping_sub(0xff);
        let b = s.b(mv + 0xf) | 2;
        s.set_b(mv + 0xf, b);
    }
    s.set_b(mv + 0xe, stop as u8);
    let mut start = y;
    if start as i16 >= 0x100 {
        start = start.wrapping_sub(0xff);
        let b = s.b(mv + 0xf) | 4;
        s.set_b(mv + 0xf, b);
    }
    s.set_b(mv + 0xa, start as u8);
    if s.w(rec + 6) != 0 {
        s.set_w(mv + 0x1a, s.w(mv + 0xa));
        s.set_w(mv + 0x1e, s.w(mv + 0xe));
        let d0 = (s.w(mv + 2) as u32) << 16 | s.w(mv + 6) as u32;
        let h = s.w(rec + 4) as i16 as i32 as u32;
        let d0 = d0.wrapping_add(h.wrapping_mul(4).wrapping_add(4));
        s.set_w(mv + 0x16, d0 as u16);
        s.set_w(mv + 0x12, (d0 >> 16) as u16);
    }
}
