//! The `camera` routines of `re/translate.toml` T36 keeps (plans/forest-dumb-level-1 T36, DESIGN
//! §4.3): `camera_update`, its four edge-flag steps `h0_4e9a`..`h0_4ec2`, the one-pixel scrolls they
//! loop on (`h0_51fc`, `h0_526a`, `h0_52d8`, `h0_5346`) and the clamp `h0_513c`. Named as in
//! `re/translate.toml`. None touches hardware, so the generators lay no base.
//!
//! What each reads, from the listing (2026-11-05): the character's `char_x`/`char_y` (longs), the
//! camera's `camera_x`/`camera_y` (words), the four edge flags `h0+$51f8..$51fb` (cleared by
//! `camera_update` before it tests them), and one latch word per direction at `h0+$5196`, `$5198`,
//! `$519a`, `$519c` (right, left, up, down). A scroll step moves the camera one pixel; on crossing a
//! 16-pixel boundary it sets its edge flag, its latch and `frame_flag`, unless the latch is set
//! already, when it moves nothing and returns `d0 = -1`. The flag steps run a scroll step up to the
//! distance past the dead zone (`$94`, `$78` in x; `$5a`, `$46` in y), stopping when one returns
//! non-zero. Every generator writes only those bytes; registers are [`Setup::random`] with the
//! entry register (`d0` or `d1`) of a flag step set.
//!
//! The states are biased to where the branches turn: a camera on a 16-pixel boundary, at its clamp
//! (`$17cf`, `$53f`, 0), negative (and the `$8000..$853e` band where `cmpi.w #$53f` overflows and
//! the clamp sets `$53f`, not 0), and a character just either side of each dead-zone edge.

use super::{Gen, Generator, Setup, h};

pub const ALL: &[(&str, Generator)] = &[
    ("camera_update", g(camera_update)),
    ("h0_4e9a", g(step_right)),
    ("h0_4ea6", g(step_left)),
    ("h0_4eb4", g(step_up)),
    ("h0_4ec2", g(step_down)),
    ("h0_513c", g(clamp)),
    ("h0_51fc", g(scroll_right)),
    ("h0_526a", g(scroll_left)),
    ("h0_52d8", g(scroll_up)),
    ("h0_5346", g(scroll_down)),
];

const fn g(case: fn(&mut Gen) -> Setup) -> Generator {
    Generator { base: None, case }
}

pub const CHAR_X: u32 = 0x1e5a;
pub const CHAR_Y: u32 = 0x1e5e;
pub const CAMERA_X: u32 = 0x1e62;
pub const CAMERA_Y: u32 = 0x1e64;
/// The latches, right, left, up, down: a word each.
pub const LATCHES: [u32; 4] = [0x5196, 0x5198, 0x519a, 0x519c];
/// The edge flags, right, left, up, down: a byte each.
pub const FLAGS: u32 = 0x51f8;
/// The camera's largest x and y (`h0_513c`, the scroll steps).
const MAX_X: u16 = 0x17cf;
const MAX_Y: u16 = 0x53f;

/// A camera coordinate whose limit is `max`, biased to where the scroll steps and the clamp branch.
fn camera(g: &mut Gen, max: u16) -> u16 {
    let cell = g.below(max as u64 / 16 + 1) as u16 * 16;
    match g.below(12) {
        // On a boundary: low nibble $f crosses going up, 0 going down.
        0..=1 => cell | 0xf,
        2..=3 => cell,
        // At and around the clamps.
        4 => max.wrapping_sub(2).wrapping_add(g.below(20) as u16),
        5 => g.below(18) as u16,
        // Negative, the band where `cmpi.w #max` overflows to plus included.
        6 => 0x8000 + g.below(max as u64 + 0x40) as u16,
        7 => g.pick(&[0xffff, 0xfff0, 0xfff1, 0x8000, 0x7fff, 0x7ff0]),
        8 => g.next() as u16,
        _ => g.below(max as u64 + 1) as u16,
    }
}

/// A latch: clear about half the time, else set as the game sets it or to any non-zero word.
fn latch(g: &mut Gen) -> u16 {
    match g.below(4) {
        0 | 1 => 0,
        2 => 0xffff,
        _ => g.next() as u16 | 1,
    }
}

/// A distance from the camera to the character, either side of a dead-zone edge `edge`: mostly
/// within `$30` of it (so a flag step loops past a boundary or runs out first), sometimes anything.
fn distance(g: &mut Gen, edge: u16) -> u16 {
    match g.below(8) {
        0 => g.next() as u16,
        1 => edge.wrapping_add(g.below(0x80) as u16),
        _ => edge.wrapping_sub(0x18).wrapping_add(g.below(0x30) as u16),
    }
}

/// A long whose low word is `w`: the high word mostly 0, as a position in the level is, sometimes any.
fn long_with(g: &mut Gen, w: u16) -> u32 {
    let high = if g.one_in(6) { g.next() as u32 } else { 0 };
    high << 16 | w as u32
}

/// `camera_update`: the character and camera, every latch and flag. The flags are poked non-zero
/// sometimes so a port that skips the `clr.l` differs.
fn camera_update(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let (cx, cy) = (camera(g, MAX_X), camera(g, MAX_Y));
    s.w(h(0, CAMERA_X), cx);
    s.w(h(0, CAMERA_Y), cy);
    // x and y each on one side: right or left edge, down or up edge.
    let ex = if g.one_in(2) { 0x94 } else { 0x78 };
    let dx = distance(g, ex);
    let ey = if g.one_in(2) { 0x5a } else { 0x46 };
    let dy = distance(g, ey);
    let x = long_with(g, cx.wrapping_add(dx));
    let y = long_with(g, cy.wrapping_add(dy));
    s.l(h(0, CHAR_X), x);
    s.l(h(0, CHAR_Y), y);
    for a in LATCHES {
        s.w(h(0, a), latch(g));
    }
    let flags = if g.one_in(3) { g.next() as u32 } else { 0 };
    s.l(h(0, FLAGS), flags);
    s
}

/// A flag step: its entry register `reg` (d0 for x, d1 for y) a distance from `edge`, its camera
/// coordinate and its latch.
fn step(g: &mut Gen, reg: usize, edge: u16, coord: u32, max: u16, latch_at: u32) -> Setup {
    let mut s = Setup::random(g);
    let d = distance(g, edge);
    s.d[reg] = long_with(g, d);
    if g.one_in(8) {
        s.d[reg] = g.next() as u32;
    }
    s.w(h(0, coord), camera(g, max));
    s.w(h(0, latch_at), latch(g));
    s
}

fn step_right(g: &mut Gen) -> Setup {
    step(g, 0, 0x94, CAMERA_X, MAX_X, LATCHES[0])
}

fn step_left(g: &mut Gen) -> Setup {
    step(g, 0, 0x78, CAMERA_X, MAX_X, LATCHES[1])
}

fn step_up(g: &mut Gen) -> Setup {
    step(g, 1, 0x46, CAMERA_Y, MAX_Y, LATCHES[2])
}

fn step_down(g: &mut Gen) -> Setup {
    step(g, 1, 0x5a, CAMERA_Y, MAX_Y, LATCHES[3])
}

/// A scroll step: its camera coordinate and its latch.
fn scroll(g: &mut Gen, coord: u32, max: u16, latch_at: u32) -> Setup {
    let mut s = Setup::random(g);
    s.w(h(0, coord), camera(g, max));
    s.w(h(0, latch_at), latch(g));
    s
}

fn scroll_right(g: &mut Gen) -> Setup {
    scroll(g, CAMERA_X, MAX_X, LATCHES[0])
}

fn scroll_left(g: &mut Gen) -> Setup {
    scroll(g, CAMERA_X, MAX_X, LATCHES[1])
}

fn scroll_up(g: &mut Gen) -> Setup {
    scroll(g, CAMERA_Y, MAX_Y, LATCHES[2])
}

fn scroll_down(g: &mut Gen) -> Setup {
    scroll(g, CAMERA_Y, MAX_Y, LATCHES[3])
}

/// `h0_513c`: the camera alone.
fn clamp(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    s.w(h(0, CAMERA_X), camera(g, MAX_X));
    s.w(h(0, CAMERA_Y), camera(g, MAX_Y));
    s
}
