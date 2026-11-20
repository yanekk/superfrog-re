//! The status bar's generators (plans/forest-dumb-level-1 T10): `main_loop`'s `h0_6a96` and the
//! routines it calls, and the start-up's two copper list pointer routines (`h0_9a34`, `h0_988a`,
//! which read nothing), which are not in `re/translate.toml` (they are outside `game_frame`'s call
//! tree) and which T10 ports by hand (`fd_core::render::status`). Each case sets only what its routine
//! reads ([`READS`], checked by `tests/drawing_routines.rs`) and spreads it so every branch goes both
//! ways: each number equal to and different from what was last drawn, every digit value 0 to 15,
//! every remainder of the lives by 3.
//!
//! The font, the icons and the icon table are the program's own data and are not poked. A digit's
//! destination (`a1`, `d1`) is kept inside the bar's buffer, which is where the callers put it.

use std::ops::Range;

use super::{Gen, Generator, Setup, h};

pub const ALL: &[(&str, Generator)] = &[
    entry("h0_6a96", update),
    entry("h0_6ae8", score),
    entry("h0_6b70", score_digit),
    entry("h0_6b78", digit),
    entry("h0_6bc0", countdown),
    entry("h0_6c20", items),
    entry("h0_6c80", lives),
    entry("h0_9a34", Setup::random),
    entry("h0_988a", Setup::random),
];

const fn entry(name: &'static str, case: fn(&mut Gen) -> Setup) -> (&'static str, Generator) {
    (name, Generator { base: None, case })
}

/// `countdown`'s first word, `lives`, `items_left`, `score`; what `h0_6a96` last drew of each.
const COUNTDOWN: u32 = h(0, 0x6a80);
const LIVES: u32 = h(0, 0x6a84);
const ITEMS: u32 = h(0, 0x6a88);
const SCORE: u32 = h(0, 0x2a66);
const SHOWN: Range<u32> = h(0, 0x6a8c)..h(0, 0x6a96);

/// What each routine reads that a case may set (registers aside).
#[allow(clippy::single_range_in_vec_init)]
pub const READS: &[(&str, &[Range<u32>])] = &[
    (
        "h0_6a96",
        &[
            COUNTDOWN..COUNTDOWN + 2,
            LIVES..LIVES + 2,
            ITEMS..ITEMS + 2,
            SCORE..SCORE + 4,
            SHOWN,
        ],
    ),
    ("h0_6ae8", &[SCORE..SCORE + 4]),
    ("h0_6b70", &[]),
    ("h0_6b78", &[]),
    ("h0_6bc0", &[COUNTDOWN..COUNTDOWN + 2]),
    ("h0_6c20", &[ITEMS..ITEMS + 2]),
    ("h0_6c80", &[LIVES..LIVES + 2]),
    ("h0_9a34", &[]),
    ("h0_988a", &[]),
];

/// A word of four digits 0 to 15, mostly decimal ones.
fn bcd(g: &mut Gen) -> u16 {
    (0..4).fold(0, |v, _| {
        let d = if g.one_in(4) {
            g.below(16)
        } else {
            g.below(10)
        };
        v << 4 | d as u16
    })
}

fn update(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let (c, l, i) = (bcd(g), bcd(g) & 0xff, bcd(g));
    let sc = (bcd(g) as u32) << 16 | bcd(g) as u32;
    s.w(COUNTDOWN, c);
    s.w(LIVES, l);
    s.w(ITEMS, i);
    s.l(SCORE, sc);
    // Each last drawn value equal half the time; $ffff, as loaded, some of the rest.
    let last = |g: &mut Gen, v: u32, wide: bool| match g.below(4) {
        0 | 1 => v,
        2 if wide => 0xffff_ffff,
        2 => 0xffff,
        _ if wide => g.next() as u32,
        _ => g.next() as u16 as u32,
    };
    let (lc, ll, li, ls) = (
        last(g, c as u32, false),
        last(g, l as u32, false),
        last(g, i as u32, false),
        last(g, sc, true),
    );
    s.w(h(0, 0x6a8c), lc as u16);
    s.w(h(0, 0x6a8e), ll as u16);
    s.w(h(0, 0x6a90), li as u16);
    s.l(h(0, 0x6a92), ls);
    s
}

fn score(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let sc = (bcd(g) as u32) << 16 | bcd(g) as u32;
    s.l(SCORE, sc);
    s
}

/// `d0` a digit, `d1` a place of the score (0 to 7, as `h0_6ae8` passes 2 to 7).
fn score_digit(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    s.d[0] = g.below(16) as u32;
    s.d[1] = g.below(8) as u32;
    s
}

/// `d0` a digit, `a1` anywhere in the bar's top block (the callers' places are all there).
fn digit(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    s.d[0] = g.below(16) as u32;
    s.a[1] = h(4, 0x6400) + g.below(0x2bc0 - 0x640) as u32;
    s
}

fn countdown(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let v = bcd(g);
    s.w(COUNTDOWN, v);
    s
}

fn items(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let v = bcd(g);
    s.w(ITEMS, v);
    s
}

/// Lives as BCD bytes, every value of both digits, so every remainder by 3 and both digits of the
/// count of threes occur; now and then a word with a high byte set.
fn lives(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let v = if g.one_in(8) {
        g.next() as u16
    } else {
        g.below(256) as u16
    };
    s.w(LIVES, v);
    s
}
