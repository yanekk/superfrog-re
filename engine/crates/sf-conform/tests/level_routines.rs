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
            s.poke_w(
                scroll::LEVEL_COPPER + 0x35c,
                g.pick(&[0xffdf, 0xffd7, 0x30df]),
            );
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
        s.poke_w(
            h0(0x16606) + 2 * k,
            (py & 0xfff0).wrapping_add(16 * k as u16),
        );
    }
    for k in 0..23u32 {
        s.poke_w(
            h0(0x1662c) + 2 * k,
            (px.wrapping_add(5) & 0xfff0).wrapping_add(16 * k as u16),
        );
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
        rig.check(
            "copper_irq_blits",
            case,
            0x1693e,
            &s,
            scroll::copper_irq_blits,
        );
    }
}

/// Fade records in every mode, part way through, over the level copper list's palette moves.
fn fading(g: &mut Gen, s: &mut Setup, mode: u16) {
    use sf_core::level::fade::FADES;
    for r in FADES {
        s.poke_w(r, if g.one_in(3) { g.below(4) as u16 } else { mode });
        s.poke_w(r + 2, g.below(3) as u16);
        s.poke_w(r + 4, g.below(3) as u16);
        s.poke_w(r + 6, g.one_in(5) as u16);
        s.poke_w(r + 8, 1 + g.below(32) as u16);
        s.poke_l(r + 0xa, scroll::LEVEL_COPPER + 2 * g.below(0x100) as u32);
        s.poke_l(
            r + 0xe,
            scroll::LEVEL_COPPER + 0x2d2 - 4 * g.below(16) as u32,
        );
        for k in 0..0x60u32 {
            s.poke_b(r + 0x12 + k, g.pick(&[0, 1, 0xff, 2]));
        }
        for k in 0..0x60u32 {
            s.poke_w(r + 0x72 + 2 * k, g.below(0x120) as u16);
            s.poke_w(r + 0x132 + 2 * k, g.below(0x1000) as u16);
        }
    }
    for k in 0..0x60u32 {
        s.poke_w(
            scroll::LEVEL_COPPER + 0x2d4 + 4 * k - 0xc0,
            g.next() as u16 & 0xfff,
        );
    }
}

#[test]
fn fades_match_the_original() {
    use sf_core::level::fade;
    let mut rig = rig();
    let mut g = Gen::new(34);
    let kinds: [(u32, u16, fn(&mut sf_core::level::LevelCtx)); 3] = [
        (0x15456, 2, fade::fade_in_tick),
        (0x156aa, 3, fade::fade_out_tick),
        (0x1528c, 1, fade::stepped_tick),
    ];
    for case in 0..CASES {
        let (entry, mode, port) = kinds[case % 3];
        let mut s = Setup::default();
        fading(&mut g, &mut s, mode);
        rig.check("fades", case, entry, &s, port);
    }
}

#[test]
fn level_clock_tick_matches_the_original() {
    use sf_core::level::clock;
    let mut rig = rig();
    let mut g = Gen::new(35);
    for case in 0..CASES {
        let mut s = Setup::default();
        for k in 0..4u32 {
            let d = if g.one_in(3) { 0 } else { g.below(10) as u8 };
            s.poke_b(h0(0x876) + k, d);
        }
        let any = g.below(0x40) as u16;
        s.poke_w(h0(0x11dfc), g.pick(&[0, 0x18, 0x19, 0x31, 0x32, any]));
        s.poke_l(h0(0x40da), g.pick(&[1, 2, 7]));
        rig.check(
            "level_clock_tick",
            case,
            0x11d2c,
            &s,
            clock::level_clock_tick,
        );
    }
}

#[test]
fn kbd_poll_matches_the_original() {
    use sf_core::level::clock;
    let mut rig = rig();
    let mut g = Gen::new(36);
    for case in 0..CASES / 10 {
        let mut s = Setup::default();
        s.hw.sdr = g.next() as u8;
        s.poke_b(h0(0x16076), g.pick(&[0, 0xff]));
        s.poke_b(h0(0x16078), g.next() as u8);
        s.poke_b(h0(0x16079), g.next() as u8);
        rig.check("kbd_poll", case, 0x15ff2, &s, clock::kbd_poll);
    }
}

#[test]
fn buffer_flip_matches_the_original() {
    use sf_core::level::pass;
    let mut rig = rig();
    let mut g = Gen::new(37);
    for case in 0..CASES / 10 {
        let mut s = Setup::default();
        s.poke_w(h0(0x18be4), g.below(2) as u16);
        s.poke_w(h0(0x9da), g.below(2) as u16);
        s.poke_w(h0(0x1665e), g.pick(&[1, 2]));
        rig.check("buffer_flip", case, 0x172b4, &s, pass::buffer_flip);
    }
}

/// A cell on or near the screen, in the map, with the camera about it.
fn near_cell(g: &mut Gen, s: &mut Setup) -> (u32, u32) {
    let cx = 0x10 + g.below(0x12c0 - 0x168 - 0x10 + 1) as u32;
    let cy = 0x20 + g.below(0x540 - 0xf0 - 0x20 + 1) as u32;
    s.poke_l(h0(0x16664), cx);
    s.poke_l(h0(0x16660), cy);
    let col = ((cx as i32 - 0x30 + g.below(0x1b0) as i32) / 16).clamp(2, 297) as u32;
    let row = ((cy as i32 - 0x20 + g.below(0x140) as i32) / 16).clamp(2, 81) as u32;
    (col, row)
}

#[test]
fn tile_anim_draw_matches_the_original() {
    use sf_core::level::pass;
    let mut rig = rig();
    let mut g = Gen::new(38);
    for case in 0..CASES / 20 {
        let mut s = Setup::default();
        for k in 0..4u32 {
            let cell_addr = if g.one_in(3) {
                0
            } else {
                let (c, r) = near_cell(&mut g, &mut s);
                s.poke_w(cell(c, r), (g.below(838) as u16) << 6 | g.below(64) as u16);
                cell(c, r)
            };
            s.poke_l(h0(0xfa68) + 4 * k, cell_addr);
        }
        rig.check("tile_anim_draw", case, 0xfa7c, &s, pass::tile_anim_draw);
    }
}

#[test]
fn passage_step_matches_the_original() {
    use sf_core::frog::{FROG, f};
    use sf_core::level::pass;
    let mut rig = rig();
    let mut g = Gen::new(39);
    for case in 0..CASES / 10 {
        let mut s = Setup::default();
        let (col, row) = near_cell(&mut g, &mut s);
        for r in row.saturating_sub(4)..(row + 5).min(84) {
            for c in col.saturating_sub(3)..(col + 4).min(300) {
                let m = g.pick(&[0, 0x26, 0x26, 0x3e, 0x3d, 0x3f, 0x3e]);
                s.poke_w(cell(c, r), (g.below(838) as u16) << 6 | m);
            }
        }
        s.poke_l(h0(0x776c), cell(col, row));
        s.poke_w(FROG + f::IN_PASSAGE, (!g.one_in(8)) as u16);
        s.poke_w(h0(0x7770), g.pick(&[0, 0xffff]));
        s.poke_l(h0(0x9fa), 0x100 * g.below(6) as u32);
        rig.check("passage_step", case, 0x7772, &s, pass::passage_step);
    }
}

#[test]
fn bob_draw_matches_the_original() {
    use sf_core::level::objects;
    let mut rig = rig();
    let mut g = Gen::new(40);
    for case in 0..CASES / 20 {
        let mut s = Setup::default();
        // The camera near where W1-1 starts, so the bobs in memory are about the screen.
        let (x0, y0) = (rig.state().camera().x as i32, rig.state().camera().y as i32);
        let dx = g.below(129) as i32 - 64;
        let dy = g.below(129) as i32 - 64;
        s.poke_l(h0(0x16664), (x0 + dx).max(0x10) as u32);
        s.poke_l(h0(0x16660), (y0 + dy).max(0x20) as u32);
        s.poke_w(h0(0x18be4), g.pick(&[0, 0xffff]));
        s.poke_w(h0(0x9da), 1);
        rig.check("bob_draw", case, 0x19120, &s, objects::bob_draw);
    }
}

#[test]
fn bob_draw_list_and_hitboxes_match_the_original() {
    use sf_core::level::objects;
    let mut rig = rig();
    for (entry, name, port) in [
        (
            0x19b42,
            "build_bob_draw_list",
            objects::build_bob_draw_list as fn(&mut sf_core::level::LevelCtx),
        ),
        (0xd2f6, "obj_build_hitboxes", objects::obj_build_hitboxes),
    ] {
        rig.check(name, 0, entry, &Setup::default(), port);
    }
}
