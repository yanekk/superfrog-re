//! The level's own vertical-blank and pass routines (`sf_core::level`, T14), apart from the
//! frog's, against the original's on the routine oracle (`sf_conform::routine`).

mod common;

use common::*;
use sf_conform::routine::{Gen, Setup};
use sf_core::layout::h0;
use sf_core::level::scroll;

#[test]
fn scroll_latch_matches_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(31);
    for case in 0..CASES {
        let mut s = Setup::default();
        s.poke_l(h0(0x16664), 0x10 + g.below(0x1200) as u32);
        s.poke_l(h0(0x16660), 0x20 + g.below(0x480) as u32);
        s.poke_w(h0(0x1665e), g.pick(&[1, 2]));
        if g.one_in(3) {
            s.poke_w(scroll::LEVEL_COPPER + 0x35c, g.pick(&[0xffdf, 0xffd7, 0x30df]));
            s.poke_w(scroll::LEVEL_COPPER + 0x35e, 0xfffe);
        }
        rig.check("scroll_latch", case, 0x1700a, &s, scroll::scroll_latch);
    }
}

/// The camera anywhere, moved a little since the scroller's last passes.
fn scrolled(g: &mut Gen, s: &mut Setup) {
    // Where the camera can be in W1-1 (camera_follow keeps it inside the level).
    let x = 0x10 + g.below(0x12c0 - 0x168 - 0x10 + 1) as u32;
    let y = 0x20 + g.below(0x540 - 0xf0 - 0x20 + 1) as u32;
    s.poke_l(h0(0x16664), x);
    s.poke_l(h0(0x16660), y);
    let (px, py) = (
        (x as u16).wrapping_add(g.below(65) as u16).wrapping_sub(32),
        (y as u16).wrapping_add(g.below(65) as u16).wrapping_sub(32),
    );
    for k in 0..17u32 {
        s.poke_w(h0(0x16606) + 2 * k, (py & 0xfff0).wrapping_add(16 * k as u16));
    }
    for k in 0..23u32 {
        s.poke_w(h0(0x1662c) + 2 * k, (px.wrapping_add(5) & 0xfff0).wrapping_add(16 * k as u16));
    }
    s.poke_w(h0(0x16674), g.pick(&[0, 0xff00]));
    s.poke_w(h0(0x1665e), g.pick(&[1, 2]));
}

#[test]
fn scroller_matches_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(32);
    for case in 0..CASES / 5 {
        let mut s = Setup::default();
        scrolled(&mut g, &mut s);
        rig.check("scroller", case, 0x1668c, &s, scroll::scroller);
    }
}

#[test]
fn copper_irq_blits_match_the_original() {
    use sf_core::layout::h3;
    let mut rig = rig();
    let mut g = Gen::new(33);
    for case in 0..CASES / 20 {
        let mut s = Setup::default();
        s.poke_w(h0(0x16688), g.pick(&[1, 1, 1, 0]));
        let n = g.below(6) as u32;
        for k in 0..23u32 {
            let e = h0(0x1675a) + 16 * k;
            if k >= n {
                s.poke_w(e, 0);
                continue;
            }
            let ring_row = g.below(19) as u32;
            let col = g.below(24) as u32 * 2;
            let src = h3(0x35860) + ring_row * 0x300 + col;
            s.poke_w(e, 1);
            if g.one_in(2) {
                // A row across the screen.
                let width = 0x18 - g.below(2) as u16;
                s.poke_w(e + 2, 0);
                s.poke_w(e + 4, width);
                s.poke_l(e + 0xa, h3(0x35860) + ring_row * 0x300);
            } else {
                // A column down it, which may wrap round the ring.
                s.poke_w(e + 2, 1);
                s.poke_w(e + 4, 0x14);
                s.poke_l(e + 6, col);
                s.poke_l(e + 0xa, src);
            }
        }
        rig.check("copper_irq_blits", case, 0x1693e, &s, scroll::copper_irq_blits);
    }
}
