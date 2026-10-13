//! The status panel's contents (docs/12 "The panel"): the pass redraws the lives icon, one digit
//! of the score, one digit of the clock and the lives count into `panel_bitmap`, from the
//! graphics in hunk 2 (h0+$8620, h0+$d9e).

use crate::frog::{FROG, f};
use crate::layout::{h0, h2, h3};
use crate::level::beam::Cost;
use crate::level::{LevelCtx, n_clear};
use crate::mem::Bus;

/// `panel_bitmap` (h3+$4b390): five planes of `$2a0`, lines `$30` apart.
const PANEL: u32 = h3(0x4b390);
const PANEL_PLANE: u32 = 0x2a0;
/// The panel graphics' planes in hunk 2, `$b40` apart, lines `$3c` apart.
const GFX_PLANE: u32 = 0xb40;
/// `lives_icon_shown` (h0+$87c0): the frog's `+$5a` the icon was last drawn for.
const LIVES_ICON_SHOWN: u32 = h0(0x87c0);
/// `score_digit` (h0+$8a74): which score digit is drawn next (6 or more: the score is made into
/// digits again first); `score_text` (h0+$8ab6): the score's digits, right-aligned.
const SCORE_DIGIT: u32 = h0(0x8a74);
const SCORE_TEXT: u32 = h0(0x8ab6);
/// `score_digit_gfx` (h0+$8b02): each score digit place's graphics.
const SCORE_DIGIT_GFX: u32 = h0(0x8b02);
/// `clock_digit` (h0+$8be0), `clock_shown` (h0+$8be2): the clock digit drawn next and the four
/// digits last drawn; `clock_digit_gfx` (h0+$8be6) each place's graphics.
const CLOCK_DIGIT: u32 = h0(0x8be0);
const CLOCK_SHOWN: u32 = h0(0x8be2);
const CLOCK_DIGIT_GFX: u32 = h0(0x8be6);
/// `number_text` (h0+$161b2), `number_started` (h0+$161aa): `number_to_text`'s output and its
/// leading-zero flag; `powers_of_ten` (h0+$16182).
const NUMBER_TEXT: u32 = h0(0x161b2);
const NUMBER_STARTED: u32 = h0(0x161aa);
const POWERS_OF_TEN: u32 = h0(0x16182);
/// `extra_life_shown` (h0+$dd4).
const EXTRA_LIFE_SHOWN: u32 = h0(0xdd4);
/// The frog's score (`+$5e`) and the score for the next extra life (`+$aa`).
const SCORE: u32 = 0x5e;
const NEXT_LIFE: u32 = 0xaa;

/// A panel blit: five planes of `size` from hunk 2 at `src` to the panel at `dst`.
fn blit_panel(c: &mut LevelCtx, mut src: u32, mut dst: u32, amod: u16, dmod: u16, size: u16) {
    c.spend(Cost::PanelBlit);
    c.custom(0x9a, 0x0010);
    c.custom(0x96, 0x8400);
    let (h, w) = ((size >> 6) as u32, (size & 0x3f) as u32);
    for _ in 0..5 {
        c.blit_words(h * w, 0x09f0);
        let (mut s, mut d) = (src, dst);
        for _ in 0..h {
            for _ in 0..w {
                c.bus.set_w(d, c.bus.w(s));
                s += 2;
                d += 2;
            }
            s = s.wrapping_add(amod as u32);
            d = d.wrapping_add(dmod as u32);
        }
        src += GFX_PLANE;
        dst += PANEL_PLANE;
    }
    c.custom(0x9a, 0x8010);
}

/// Copy one 8-pixel digit: 14 lines of each plane, a byte each.
fn copy_digit(b: &mut Bus, mut src: u32, mut dst: u32) {
    for _ in 0..5 {
        for k in 0..14u32 {
            b.set_b(dst + 0x30 * k, b.b(src + 0x3c * k));
        }
        src = src.wrapping_add(GFX_PLANE);
        dst = dst.wrapping_add(PANEL_PLANE);
    }
}

/// `panel_update` (h0+$8620): the lives icon (if it changed), a score digit and a clock digit.
pub fn panel_update(c: &mut LevelCtx) {
    lives_icon(c);
    score_digit(c);
    clock_digit(c);
}

/// h0+$8716: the icon for the frog's `+$5a`, when it is not the one shown.
fn lives_icon(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let v = b.w(FROG + 0x5a);
    if v == b.w(LIVES_ICON_SHOWN) {
        return;
    }
    lives_icon_draw(c);
}

/// h0+$8728: the icon for the frog's `+$5a`, drawn and kept as the one shown.
pub(crate) fn lives_icon_draw(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let v = b.w(FROG + 0x5a);
    b.set_w(LIVES_ICON_SHOWN, v);
    let k = if n_clear(v) { v } else { 0 };
    let src = h2(0x2580).wrapping_add((k.wrapping_mul(4)) as i16 as i32 as u32);
    blit_panel(c, src, PANEL + 0x20, 0x38, 0x2c, 0x382);
}

/// h0+$8982: every seventh pass the score is made into digits; the six passes between draw one
/// digit each.
fn score_digit(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let k = b.w(SCORE_DIGIT).wrapping_add(1);
    b.set_w(SCORE_DIGIT, k);
    if n_clear(k.wrapping_sub(6)) {
        number_to_text(b, b.l(FROG + SCORE));
        b.set_l(SCORE_TEXT, 0);
        b.set_l(SCORE_TEXT + 4, 0);
        b.set_l(NUMBER_TEXT + 8, 0);
        // The digits, right-aligned against `score_text + 8`.
        let mut n = 0u32;
        let mut a0 = NUMBER_TEXT;
        loop {
            n += 1;
            if n == 0xa {
                break;
            }
            let v = b.b(a0);
            a0 += 1;
            if v == 0 {
                break;
            }
        }
        let mut a1 = SCORE_TEXT + 8;
        for _ in 0..n {
            a0 -= 1;
            a1 -= 1;
            b.set_b(a1, b.b(a0));
        }
        b.set_w(SCORE_DIGIT, 0xffff);
        return;
    }
    let k = k as i16 as i32 as u32;
    let gfx = b.l(SCORE_DIGIT_GFX.wrapping_add((k as u16).wrapping_mul(4) as i16 as i32 as u32));
    let d = b.b(SCORE_TEXT.wrapping_add(k)).wrapping_sub(0x30);
    let d = if (d as i8) < 0 { 0 } else { d as u32 };
    copy_digit(b, gfx.wrapping_add(d), PANEL + 9 + k);
}

/// h0+$8b1a: one clock digit each pass, redrawn if it changed (a gap before the seconds).
fn clock_digit(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let mut k = b.w(CLOCK_DIGIT).wrapping_add(1);
    b.set_w(CLOCK_DIGIT, k);
    if n_clear(k.wrapping_sub(4)) {
        k = 0;
        b.set_w(CLOCK_DIGIT, 0);
    }
    let k = k as i16 as i32 as u32;
    let d = b.b(FROG + 0xa0 + k);
    if d == b.b(CLOCK_SHOWN + k) {
        return;
    }
    b.set_b(CLOCK_SHOWN + k, d);
    let gfx = b.l(CLOCK_DIGIT_GFX + 4 * k);
    let mut dst = PANEL + 0x25 + k;
    if n_clear((k as u16).wrapping_sub(2)) {
        dst += 1;
    }
    copy_digit(b, gfx.wrapping_add(d as u32), dst);
}

/// `number_to_text` (h0+$160dc): `n` in decimal at `number_text`, no leading zeros (nothing at
/// all for 0); over 999 999 999, nine nines.
fn number_to_text(b: &mut Bus, mut n: u32) {
    for k in 0..3 {
        b.set_l(NUMBER_TEXT + 4 * k, 0);
    }
    let mut out = NUMBER_TEXT;
    if n > 0x3b9a_c9ff {
        b.set_l(NUMBER_TEXT, 0x3939_3939);
        b.set_l(NUMBER_TEXT + 4, 0x3939_3939);
        b.set_b(NUMBER_TEXT + 8, 0x39);
        out = NUMBER_TEXT + 9;
    } else {
        b.set_l(NUMBER_STARTED, 0);
        let mut p = POWERS_OF_TEN;
        let mut digit = 0x30u8;
        while b.l(p) != 0 {
            let power = b.l(p);
            n = n.wrapping_sub(power);
            digit = digit.wrapping_add(1);
            if n != 0 && (n as i32) > 0 {
                continue;
            }
            if n != 0 {
                n = n.wrapping_add(power);
                digit = digit.wrapping_sub(1);
            }
            p += 4;
            if b.b(NUMBER_STARTED) != 0 || digit != 0x30 {
                b.set_b(out, digit);
                out += 1;
                b.set_b(NUMBER_STARTED, 1);
            }
            digit = 0x30;
        }
    }
    while b.b(out) != 0 {
        b.set_b(out, 0);
        out += 1;
    }
}

/// h0+$d9e: an extra life for every 200 000 points (looked at until the score is below the next
/// one), then the lives count is drawn (h0+$88de).
pub fn extra_life(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(EXTRA_LIFE_SHOWN) == 0 {
        loop {
            let score = b.l(FROG + SCORE);
            if !crate::level::collision::n_clear_l(score.wrapping_sub(b.l(FROG + NEXT_LIFE))) {
                break;
            }
            b.set_l(FROG + NEXT_LIFE, b.l(FROG + NEXT_LIFE).wrapping_add(0x30d40));
            b.set_w(FROG + f::LIVES, b.w(FROG + f::LIVES).wrapping_add(1));
        }
    }
    b.set_w(EXTRA_LIFE_SHOWN, 0);
    lives_count(c);
}

/// h0+$88de: the lives digit (0 to 5).
pub(crate) fn lives_count(c: &mut LevelCtx) {
    let mut n = c.bus.w(FROG + f::LIVES);
    if !n_clear(n.wrapping_sub(5)) {
    } else {
        n = 5;
    }
    if !n_clear(n) {
        n = 0;
    }
    let src = h2(0x21c0).wrapping_add((n as u32) * 0xa);
    blit_panel(c, src, PANEL + 0x12, 0x32, 0x26, 0x385);
}

/// What `level_start` (h0+$6952) draws of the panel, in its order: the two sprite digits
/// (h0+$862e), the panel sliding in (h0+$8510), the lives count (h0+$88de) and icon (h0+$8728),
/// the clock (h0+$11dfe) and the score (h0+$8982, seven times from `score_digit = $a`).
pub fn draw_at_start(c: &mut LevelCtx) {
    sprite_digits(&mut c.bus);
    slide_begin(&mut c.bus);
    for k in 0..SLIDE_STEPS {
        slide_step(&mut c.bus, k);
    }
    after_slide(c);
}

/// What `level_start` draws once the panel is in: the lives count and icon, the clock and the
/// score.
pub(crate) fn after_slide(c: &mut LevelCtx) {
    lives_count(c);
    lives_icon_draw(c);
    clock_at_start(c);
    c.bus.set_w(SCORE_DIGIT, 0xa);
    for _ in 0..7 {
        score_digit(c);
    }
}

/// h0+$862e: the frog's `+$5c` (0..99) as two sprite digits, by pointing the copper MOVEs of the
/// two digit sprites (their records' `+8`, at h0+$1b130 and $1b14c) at the tens' and the ones'
/// frames (tables at h0+$86c2 and $86ee; the entry before each is blank, for 0).
pub(crate) fn sprite_digits(b: &mut Bus) {
    let n = b.w(FROG + 0x5c);
    let (tens, ones) = if n == 0 {
        (-1i32, -1i32)
    } else {
        ((n / 10) as i32, (n % 10) as i32)
    };
    for (k, table, rec) in [(tens, h0(0x86c2), h0(0x1b130)), (ones, h0(0x86ee), h0(0x1b14c))] {
        let frame = b.l(table.wrapping_add((4 * k) as u32));
        let at = b.l(rec);
        for (o, v) in [(0, frame), (0x10, frame.wrapping_add(0x78))] {
            b.set_w(at + o + 6, v as u16);
            b.set_w(at + o + 2, (v >> 16) as u16);
        }
    }
}

/// The steps of the panel's slide (h0+$8510): `$18`, a frame each.
pub const SLIDE_STEPS: u32 = 0x18;

/// h0+$8510's start: the display window opened to line `$27`. (Its INTENA and blitter set-up
/// are the caller's.)
pub(crate) fn slide_begin(b: &mut Bus) {
    b.set_w(h2(0x6bd0) + 6, 0x2779);
}

/// Step `k` of h0+$8510's slide: the panel's picture (hunk 2 from h2+$14a0) slides in from the
/// middle, each step copying its seven stripes of two lines, `k + 1` words wide on each side; what
/// is left is the last step's.
pub fn slide_step(b: &mut Bus, k: u32) {
    let width = k + 1;
    let (a1, a4) = (PANEL + 0x2e - 2 * k, h2(0x14fe) - 2 * k);
    let (a0, a5) = (h2(0x14a0), PANEL + 0x30);
    for row in 0..7 {
        for plane in 0..5 {
            let o = row * 0x60 + plane * PANEL_PLANE;
            for w in 0..width {
                let v = b.w(a0 + o + 2 * w);
                b.set_w(a1 + o + 2 * w, v);
                let v = b.w(a4 + o + 2 * w);
                b.set_w(a5 + o + 2 * w, v);
            }
        }
    }
}

/// `clock_ticks` (h0+$11dfc).
const CLOCK_TICKS: u32 = h0(0x11dfc);

/// h0+$11dfe: in the easy game the minutes doubled (at most 9); the clock's colours into the
/// copper list (`level_copper+$1f2`, `$1f6`), its ticks reset, and its four digits drawn.
fn clock_at_start(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(h0(0x9e6)) == 0 {
        let m = b.b(h0(0x877)).wrapping_add(b.b(h0(0x877)));
        b.set_b(h0(0x877), if (m.wrapping_sub(0xa) as i8) < 0 { m } else { 9 });
    }
    b.set_w(h2(0x6bd0) + 0x1f2, 0x149);
    b.set_w(h2(0x6bd0) + 0x1f6, 0xbf);
    b.set_w(CLOCK_TICKS, 0);
    b.set_w(CLOCK_DIGIT, 0);
    b.set_l(CLOCK_SHOWN, 0xffff_ffff);
    for _ in 0..4 {
        clock_digit(c);
    }
}
