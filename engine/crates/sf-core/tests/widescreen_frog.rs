//! real-widescreen T09 (DESIGN §2.5, §2.9): the frog shows in the side strips, whole and in the
//! right place. Its sprites are placed from their copper words; past HSTART's 9 bits (x `$180`
//! from the camera and on, shown up to `$190` while the camera catches up with the frog) the
//! engine carries the tenth bit in SPRxCTL's [`WIDE_HSTART_BIT`] and the wide renderer reads it.
//!
//! Each case compares the frog in a strip with the same game's frog moved to the centre: a clone
//! whose sprite records are put at a centre x before the step, so both draw the same animation
//! frame on the same camera.

use std::sync::Arc;

use sf_core::frog::anim::{WIDE_HSTART_BIT, spr};
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::level::camera::{FROG_SPRITE_LEFT, FROG_SPRITE_RIGHT, LEVEL_WIDTH};
use sf_core::render::{StripLayers, strip_layers};
use sf_core::{DisplayData, Game, LevelId, Start, View};

const N: i32 = 65;
const WIDE: View = View {
    extra_columns_each_side: N as u16,
};
/// The frog's two sprites side by side.
const FROG_WIDTH: i32 = 32;

fn display() -> Arc<DisplayData> {
    static D: std::sync::OnceLock<Arc<DisplayData>> = std::sync::OnceLock::new();
    D.get_or_init(|| Arc::new(DisplayData::new(sf_conform::game_data())))
        .clone()
}

fn level(id: &str, wide: bool) -> Game {
    let mut g = Game::with_display(display(), Start::Level(id.parse::<LevelId>().unwrap()));
    g.set_widescreen(wide);
    g
}

fn w(g: &Game, a: u32) -> u16 {
    let b = g.peek(a, 2);
    u16::from_be_bytes([b[0], b[1]])
}

fn set_w(g: &mut Game, a: u32, v: u16) {
    g.poke(a, &v.to_be_bytes());
}

fn steps(g: &mut Game, n: u32, j: Joy) {
    for _ in 0..n {
        g.step(FrameInput::joy(j));
    }
}

/// Sprite record `s`'s SPRxCTL low byte, as its copper words hold it.
fn ctl_low(g: &Game, s: u32) -> u8 {
    let b = g.peek(s + spr::COPPER, 4);
    g.peek(u32::from_be_bytes([b[0], b[1], b[2], b[3]]) + 0xf, 1)[0]
}

/// Where sprite record `s`'s copper words put its first pixel, in picture pixels of `l`: HSTART
/// (with widescreen's tenth bit) + 1 − the picture's left.
fn picture_x(g: &Game, s: u32, l: &StripLayers) -> i32 {
    let b = g.peek(s + spr::COPPER, 4);
    let cop = u32::from_be_bytes([b[0], b[1], b[2], b[3]]);
    let ctl = w(g, cop + 0xe);
    let hstart = (g.peek(cop + 0xb, 1)[0] as i32) << 1
        | (ctl & 1) as i32
        | if ctl as u8 & WIDE_HSTART_BIT != 0 { 0x200 } else { 0 };
    hstart + 1 - l.picture.left
}

fn layers(g: &Game) -> StripLayers {
    strip_layers(g.state(), &display(), WIDE)
}

fn sprite(l: &StripLayers, x: i32, y: usize) -> Option<u8> {
    let width = l.picture.width as i32;
    (0..width).contains(&x).then(|| l.sprites[y * width as usize + x as usize]).flatten()
}

/// `g` and a clone stepped once with the same input, the clone's frog sprites put at screen x
/// `at` and `at + 16` first: the same pose, elsewhere.
fn moved(g: &Game, at: u16) -> (Game, Game) {
    let (mut a, mut b) = (g.clone(), g.clone());
    set_w(&mut b, FROG_SPRITE_LEFT, at);
    set_w(&mut b, FROG_SPRITE_RIGHT, at.wrapping_add(0x10));
    a.step(FrameInput::NONE);
    b.step(FrameInput::NONE);
    (a, b)
}

/// The frog's sprite pixels in `a` from picture x `xa`, columns `cols`, equal `b`'s from `xb`,
/// on every row; returns how many of `a`'s are drawn.
fn same_frog(a: &StripLayers, xa: i32, b: &StripLayers, xb: i32, cols: std::ops::Range<i32>) -> usize {
    let mut drawn = 0;
    for y in 0..a.picture.height as usize {
        for k in cols.clone() {
            let (pa, pb) = (sprite(a, xa + k, y), sprite(b, xb + k, y));
            assert_eq!(pa, pb, "row {y}, frog column {k}: strip {} vs centre {}", xa + k, xb + k);
            drawn += pa.is_some() as usize;
        }
    }
    drawn
}

/// W1-1 in widescreen, the frog put near the end and walked into the right strip against the last
/// wall (as T04's `widescreen_camera` does), then left standing.
fn at_right_end() -> Game {
    let mut g = level("W1-1", true);
    steps(&mut g, 3, Joy::NONE);
    let width = w(&g, LEVEL_WIDTH);
    set_w(&mut g, FROG + f::X, width - 0x200);
    steps(&mut g, 700, Joy::RIGHT);
    steps(&mut g, 40, Joy::NONE);
    g
}

/// W1-1 in widescreen, walked into its left end: the left sprite at screen x −50.
fn at_left_end() -> Game {
    let mut g = level("W1-1", true);
    steps(&mut g, 200, Joy::LEFT);
    steps(&mut g, 40, Joy::NONE);
    g
}

#[test]
fn at_the_right_end_the_frog_in_the_right_strip_is_the_centres_shifted() {
    let (a, b) = moved(&at_right_end(), 0x90);
    let (la, lb) = (layers(&a), layers(&b));
    let (xa, xb) = (picture_x(&a, FROG_SPRITE_LEFT, &la), picture_x(&b, FROG_SPRITE_LEFT, &lb));
    let strip = la.picture.width as i32 - N;
    assert!(xa + FROG_WIDTH > strip && xa + FROG_WIDTH <= la.picture.width as i32, "frog at {xa}");
    assert_eq!(picture_x(&a, FROG_SPRITE_RIGHT, &la), xa + 16);
    assert!(same_frog(&la, xa, &lb, xb, 0..FROG_WIDTH) > 0);
    let in_strip = (strip - xa..FROG_WIDTH).filter(|&k| (0..la.picture.height as usize).any(|y| sprite(&la, xa + k, y).is_some()));
    assert!(in_strip.count() > 4, "the frog's pixels reach into the right strip");
}

#[test]
fn at_the_left_end_the_frog_in_the_left_strip_is_the_centres_shifted() {
    let g = at_left_end();
    let (a, b) = moved(&g, 0x90);
    let (la, lb) = (layers(&a), layers(&b));
    let (xa, xb) = (picture_x(&a, FROG_SPRITE_LEFT, &la), picture_x(&b, FROG_SPRITE_LEFT, &lb));
    assert!(xa >= 0 && xa < N, "frog at {xa}");
    assert!(same_frog(&la, xa, &lb, xb, 0..FROG_WIDTH) > 0);
    assert!(
        (0..la.picture.height as usize).any(|y| (xa..N).any(|x| sprite(&la, x, y).is_some())),
        "the frog's pixels in the left strip"
    );
}

/// Sprites at x `$180` and `$190` from the camera, the furthest shown (a restart near the end has
/// the frog there while the camera catches up): HSTART `$200` and `$210`, past 9 bits. The left
/// sprite's first 7 columns show at the right strip's last ones, as the centre frog's.
#[test]
fn past_hstarts_9_bits_the_frog_shows_at_the_right_edge() {
    let g = at_right_end();
    let (_, b) = moved(&g, 0x90);
    let mut c = g.clone();
    set_w(&mut c, FROG_SPRITE_LEFT, 0x180);
    set_w(&mut c, FROG_SPRITE_RIGHT, 0x190);
    c.step(FrameInput::NONE);
    assert_eq!(w(&c, FROG + f::SPRITE_DX), 0, "the pose has no offset");
    for s in [FROG_SPRITE_LEFT, FROG_SPRITE_RIGHT] {
        assert_ne!(ctl_low(&c, s) & WIDE_HSTART_BIT, 0, "HSTART bit 9 carried");
    }
    let (lc, lb) = (layers(&c), layers(&b));
    let xc = picture_x(&c, FROG_SPRITE_LEFT, &lc);
    let width = lc.picture.width as i32;
    assert_eq!(xc, width - 7, "the left sprite's first 7 columns are the picture's last");
    let xb = picture_x(&b, FROG_SPRITE_LEFT, &lb);
    assert!(same_frog(&lc, xc, &lb, xb, 0..7) > 0, "frog pixels at the right edge");
    // Nothing of it wraps to the left.
    for y in 0..lc.picture.height as usize {
        for x in 0..N {
            assert_eq!(sprite(&lc, x, y), sprite(&lb, x, y), "row {y} x {x}: a wrapped sprite");
        }
    }
}

/// The tenth bit is widescreen's alone: a 4:3 game's frog put at `$180` is parked at `$150` and
/// its copper words are the original's, the bit clear.
#[test]
fn a_4_3_game_never_sets_the_bit() {
    let mut g = level("W1-1", false);
    steps(&mut g, 40, Joy::NONE);
    set_w(&mut g, FROG_SPRITE_LEFT, 0x180);
    set_w(&mut g, FROG_SPRITE_RIGHT, 0x190);
    g.step(FrameInput::NONE);
    for s in [FROG_SPRITE_LEFT, FROG_SPRITE_RIGHT] {
        assert_eq!(ctl_low(&g, s) & WIDE_HSTART_BIT, 0);
    }
    let l = layers(&g);
    assert_eq!(picture_x(&g, FROG_SPRITE_LEFT, &l), 0x150 + 0x81 - l.picture.left);
}

/// A blinking frog is parked at `$150 + 65` (T04): HSTART `$211`, with the tenth bit past the
/// picture's right, nothing in either strip, at either end.
#[test]
fn a_blinking_frog_shows_nothing_in_either_strip() {
    for (end, mut g) in [("right", at_right_end()), ("left", at_left_end())] {
        set_w(&mut g, FROG + f::BLINK, 0x41);
        let mut parked = 0;
        for _ in 0..8 {
            g.step(FrameInput::NONE);
            if w(&g, FROG + f::BLINK) & 1 == 0 {
                continue;
            }
            parked += 1;
            let l = layers(&g);
            let width = l.picture.width as i32;
            assert!(picture_x(&g, FROG_SPRITE_LEFT, &l) >= width, "{end}: parked past the right");
            for y in 0..l.picture.height as usize {
                for x in (0..N).chain(width - N..width) {
                    assert_eq!(sprite(&l, x, y), None, "{end} end: row {y} x {x}");
                }
            }
        }
        assert!(parked > 0, "{end}: the frog blinked");
    }
}
