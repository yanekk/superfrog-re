//! The `objects` routines of `re/translate.toml` T38 keeps (plans/forest-dumb-level-1 T38, DESIGN
//! §4.3): the map codes under the character (`h0_24ce`), the object cells it walks over
//! (`char_object_cells` and its body `h0_26ec`), the BCD counters they bump (`score_add_routine`,
//! `countdown_plus_100`, `items_left_minus_one`), the slots leaving the screen written back into the
//! map (`h0_2dd6`, its body `h0_2e0e`), and creature contact (`char_hit_check`, its box test
//! `h0_3dd8`). Named as in `re/translate.toml`.
//!
//! What each reads, from the flow listing (2026-11-05):
//! - the byte map at `h0+$cb38`, `$60` cells to a column, cell `(x >> 4) * $60 + (y >> 4)`;
//! - `char_x`/`char_y` (longs), `camera_x`/`camera_y` (words);
//! - the five slots: a flag byte each at `h0+$67da`, eight bytes each at `h0+$67ec` (x, y, kind
//!   words, then two bytes), and the slot index long `h0+$2faa` (`h0_2e0e` only: `h0_2dd6` clears it);
//! - the BCD counters: score `h0+$2a66` (long), `score_add` `h0+$2a6a` (long), `countdown`
//!   `h0+$6a80` (three bytes), `items_left` `h0+$6a88`, `lives` `h0+$6a84` (words);
//! - the item blit of `h0_29ca`: the tile word at `h0+$16318`, the tile table long at `h0+$3b988`,
//!   and the screens in hunk 3, filled with noise by the base so a blit's copy is never all zero;
//! - flags `h0+$4ed0` (byte), `h0+$4b62` (word), `h0+$9b04` (byte), `hits_taken`, `cheat_flag`, the
//!   row offset long `h0+$2692`, and CLXDAT (`$dff00e`) as a hardware read.
//!
//! The counters are valid BCD on every state. The game only ever writes BCD there, and the 68000's
//! ABCD/SBCD on non-BCD digits is undocumented, so a port judged on them would be judged on how one
//! emulator does it. Map cells and tile words are poked only where they fall in the map
//! (`h0+$cb38..$16318`) or the tile map (`h0+$16318..$1c318`); a coordinate off the level reads
//! whatever the program left there.

use super::{Gen, Generator, Setup, h};

pub const ALL: &[(&str, Generator)] = &[
    ("h0_24ce", g(map_code)),
    ("char_object_cells", gb(object_cells)),
    ("h0_26ec", gb(object_cell)),
    ("score_add_routine", g(score_add)),
    ("h0_2dd6", g(slots_off_screen)),
    ("h0_2e0e", g(slot_off_screen)),
    ("char_hit_check", g(hit_check)),
    ("h0_3dd8", g(box_test)),
    ("countdown_plus_100", g(countdown)),
    ("items_left_minus_one", g(items_left)),
];

const fn g(case: fn(&mut Gen) -> Setup) -> Generator {
    Generator { base: None, case }
}

const fn gb(case: fn(&mut Gen) -> Setup) -> Generator {
    Generator {
        base: Some(screens),
        case,
    }
}

pub const CHAR_X: u32 = 0x1e5a;
pub const CHAR_Y: u32 = 0x1e5e;
pub const CAMERA_X: u32 = 0x1e62;
pub const CAMERA_Y: u32 = 0x1e64;
/// The byte map, `$60` cells to a column, and where the tile map after it starts.
pub const MAP: u32 = 0xcb38;
pub const MAP_END: u32 = 0x1_6318;
/// The tile map: a word per 8x8 cell, `$60` bytes to a column.
pub const TILES: u32 = 0x1_6318;
pub const TILES_END: u32 = 0x1_c318;
/// The tile table: a long per tile, an offset from `h3+$13540`.
pub const TILE_TABLE: u32 = 0x3_b988;
/// How many tile numbers the generators use, so the table entries they poke stay in
/// `TILE_TABLE..TILE_TABLE + 4 * TILE_COUNT`.
pub const TILE_COUNT: u32 = 0x400;
pub const SLOT_FLAGS: u32 = 0x67da;
pub const SLOTS: u32 = 0x67ec;
pub const SLOT_INDEX: u32 = 0x2faa;
pub const SCORE: u32 = 0x2a66;
pub const SCORE_ADD: u32 = 0x2a6a;
pub const COUNTDOWN: u32 = 0x6a80;
pub const LIVES: u32 = 0x6a84;
pub const ITEMS_LEFT: u32 = 0x6a88;
pub const HITS_TAKEN: u32 = 0x3b26;
pub const CHEAT_FLAG: u32 = 0x780c;
pub const ROW_OFFSET: u32 = 0x2692;
/// `tst.b` in `h0_24ce` and `char_hit_check`: zero skips the work.
pub const FLAG_4ED0: u32 = 0x4ed0;
/// `tst.w` in `h0_24ce` and `char_hit_check`: non-zero skips the work.
pub const FLAG_4B62: u32 = 0x4b62;
/// `tst.b` in `char_hit_check`: non-zero spares the character a hit.
pub const FLAG_9B04: u32 = 0x9b04;
/// CLXDAT, read as a word.
pub const CLXDAT: u32 = 0xdf_f00e;

/// The map cell `h0_24ce` and `h0_26ec` read for the pixel `x`, `y` (longs): `lsr.l #4`, `mulu.w
/// #$60` on the column's low word, plus the row as a long.
pub fn map_cell(x: u32, y: u32) -> u32 {
    let col = ((x >> 4) & 0xffff) * 0x60;
    h(0, MAP).wrapping_add(col).wrapping_add(y >> 4) & 0xff_ffff
}

/// The tile word `h0_29ca` reads for the pixel `x`, `y` (longs): `lsr.w #3`, `andi.b #$fe` on each
/// low word, the column times `$60`, the row added as the whole long.
pub fn tile_word(x: u32, y: u32) -> u32 {
    let col = (((x as u16) >> 3) & 0xfffe) as u32 * 0x60;
    let row = (y & 0xffff_0000) | (((y as u16) >> 3) & 0xfffe) as u32;
    h(0, TILES).wrapping_add(col).wrapping_add(row) & 0xff_ffff
}

fn in_map(a: u32) -> bool {
    (h(0, MAP)..h(0, MAP_END)).contains(&a)
}

fn in_tiles(a: u32) -> bool {
    (h(0, TILES)..h(0, TILES_END) - 1).contains(&a)
}

/// A BCD byte, biased to the digits where a carry or borrow turns.
fn bcd(g: &mut Gen) -> u8 {
    match g.below(6) {
        0 => g.pick(&[0x00, 0x99, 0x09, 0x90, 0x01, 0x10]),
        _ => (g.below(10) << 4 | g.below(10)) as u8,
    }
}

/// A BCD word, zero about one time in `zero`.
fn bcd_word(g: &mut Gen, zero: u64) -> u16 {
    if g.one_in(zero) {
        return 0;
    }
    (bcd(g) as u16) << 8 | bcd(g) as u16
}

/// A character coordinate: mostly in the level, sometimes with a high word.
fn char_coord(g: &mut Gen, max: u64) -> u32 {
    match g.below(16) {
        0 => g.next() as u32,
        1 => (g.next() as u32) << 16 | g.below(max) as u32,
        _ => g.below(max) as u32,
    }
}

const LEVEL_W: u64 = 0x1900;
const LEVEL_H: u64 = 0x600;

/// A map code under the character for `h0_24ce`: the four marks, their neighbours, nothing or any.
fn mark_code(g: &mut Gen) -> u8 {
    match g.below(8) {
        0..=3 => 0x10 + g.below(4) as u8,
        4 => g.pick(&[0x0f, 0x14, 0x90, 0x93]),
        5 => 0,
        _ => g.next() as u8,
    }
}

/// `h0_24ce`: the two gates, the character and the cell it stands on.
fn map_code(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let v = if g.one_in(5) { 0 } else { g.next() as u8 | 1 };
    s.b(h(0, FLAG_4ED0), v);
    let v = if g.one_in(5) { g.next() as u16 | 1 } else { 0 };
    s.w(h(0, FLAG_4B62), v);
    let (x, y) = (char_coord(g, LEVEL_W), char_coord(g, LEVEL_H));
    s.l(h(0, CHAR_X), x);
    s.l(h(0, CHAR_Y), y);
    let c = map_cell(x.wrapping_add(0xe), y.wrapping_add(0x23));
    if in_map(c) {
        s.b(c, mark_code(g));
    }
    s
}

/// An object-map code for `h0_26ec`, by where its compares turn (N only, so `$a0..$ff` read as
/// below `$20`): the exit `$ff`, items `$20..$27`, `$28..$3f`, the scored codes `$40..$50` (`$4c`
/// the countdown), `$51..$5f` (blitted, nothing scored), `$60` up, below `$20`.
fn object_code(g: &mut Gen) -> u8 {
    match g.below(16) {
        0 => 0xff,
        1..=3 => 0x20 + g.below(8) as u8,
        4..=9 => 0x40 + g.below(0x11) as u8,
        10 => 0x4c,
        11 => 0x51 + g.below(0xf) as u8,
        12 => g.pick(&[0x1f, 0x28, 0x3f, 0x60, 0x9f, 0xa0, 0xa7, 0xc0, 0xdf, 0xe0]),
        13 => 0,
        _ => g.next() as u8,
    }
}

/// The counters `h0_26ec` reads on its way: items left, the score, the countdown.
fn counters(g: &mut Gen, s: &mut Setup) {
    s.w(h(0, ITEMS_LEFT), bcd_word(g, 5));
    for i in 0..4 {
        s.b(h(0, SCORE) + i, bcd(g));
    }
    countdown_bytes(g, s);
}

/// The item blit's inputs for a character at `x`, `y` with row offset `off`: the tile word, and
/// the table entry it picks, an even offset into the screens' noise.
fn blit_inputs(g: &mut Gen, s: &mut Setup, x: u32, y: u32, off: u32) {
    let t = tile_word(x.wrapping_add(0xe), y.wrapping_add(off));
    if !in_tiles(t) {
        return;
    }
    let tile = g.below(TILE_COUNT as u64) as u32;
    s.w(t, tile as u16);
    s.l(h(0, TILE_TABLE) + tile * 4, g.below(0x2_0000) as u32 & !1);
}

/// `char_object_cells`: the character, the two cells it reads (rows `+$19` and `+9`), and what an
/// item there reaches.
fn object_cells(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let (x, y) = (char_coord(g, LEVEL_W), char_coord(g, LEVEL_H));
    s.l(h(0, CHAR_X), x);
    s.l(h(0, CHAR_Y), y);
    for off in [0x19, 9] {
        let c = map_cell(x.wrapping_add(0xe), y.wrapping_add(off));
        if in_map(c) {
            // The lower cell's code is mostly nothing, so the upper one is reached as often.
            let v = if off == 0x19 && g.one_in(2) {
                0
            } else {
                object_code(g)
            };
            s.b(c, v);
        }
        blit_inputs(g, &mut s, x, y, off);
    }
    counters(g, &mut s);
    s
}

/// `h0_26ec` alone: its entry pixel in `d0`/`d1`, mostly the one `char_object_cells` passes, and
/// the row offset the item branch adds to `char_y`.
fn object_cell(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let (x, y) = (char_coord(g, LEVEL_W), char_coord(g, LEVEL_H));
    s.l(h(0, CHAR_X), x);
    s.l(h(0, CHAR_Y), y);
    let off = match g.below(4) {
        0 => 9,
        1 => 0x19,
        2 => g.below(0x40) as u32,
        _ => g.pick(&[9, 0x19]),
    };
    s.l(h(0, ROW_OFFSET), off);
    s.d[0] = if g.one_in(6) {
        char_coord(g, LEVEL_W)
    } else {
        x.wrapping_add(0xe)
    };
    s.d[1] = if g.one_in(6) {
        char_coord(g, LEVEL_H)
    } else {
        y.wrapping_add(off)
    };
    let c = map_cell(s.d[0], s.d[1]);
    if in_map(c) {
        s.b(c, object_code(g));
    }
    blit_inputs(g, &mut s, x, y, off);
    counters(g, &mut s);
    s
}

/// `score_add_routine`: the score and what is added to it.
fn score_add(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    for i in 0..8 {
        s.b(h(0, SCORE) + i, bcd(g));
    }
    s
}

/// The countdown's three BCD bytes, biased to the `$899` compare's edge, and sometimes a word the
/// compare reads as negative.
fn countdown_bytes(g: &mut Gen, s: &mut Setup) {
    let w: u16 = match g.below(8) {
        0 => g.pick(&[0x0898, 0x0899, 0x0900, 0x0999, 0x0799, 0x0000]),
        1 => g.pick(&[0x8899, 0x9000, 0x9999, 0x8898]),
        _ => bcd_word(g, 8),
    };
    s.w(h(0, COUNTDOWN), w);
    s.b(h(0, COUNTDOWN) + 2, bcd(g));
}

fn countdown(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    countdown_bytes(g, &mut s);
    s
}

fn items_left(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    s.w(h(0, ITEMS_LEFT), bcd_word(g, 4));
    s
}

/// A camera coordinate up to `max`, sometimes near zero where `cx - $10` and `cy - $20` wrap.
fn camera(g: &mut Gen, max: u64) -> u16 {
    match g.below(10) {
        0 => g.below(0x30) as u16,
        1 => g.next() as u16,
        _ => g.below(max) as u16,
    }
}

/// A slot coordinate against a window whose low edge is `lo` and high edge `hi` (both inclusive
/// inside, as `h0_2e0e` compares): on each edge, just outside, outside, inside or anything.
fn around(g: &mut Gen, lo: u16, hi: u16) -> u16 {
    match g.below(12) {
        0 => lo,
        1 => lo.wrapping_add(1),
        2 => hi,
        3 => hi.wrapping_add(1),
        4 => lo.wrapping_sub(g.below(0x60) as u16),
        5 => hi.wrapping_add(1 + g.below(0x60) as u16),
        6 => g.next() as u16,
        _ => lo.wrapping_add(1 + g.below(hi.wrapping_sub(lo) as u64) as u16),
    }
}

/// A map byte around a slot's cell: empty half the time, so each of the three tries can land.
fn cell_byte(g: &mut Gen) -> u8 {
    if g.one_in(2) { 0 } else { g.next() as u8 | 1 }
}

/// Slot `i` placed around the camera window, its kind, and the map cells its write-back tries.
fn slot_off_screen_state(g: &mut Gen, s: &mut Setup, i: u32, cx: u16, cy: u16) {
    let o = h(0, SLOTS) + i * 8;
    // `h0_2e0e` keeps a slot with x in `cx-$f..=cx+$130` and y in `cy-$1f..=cy+$c5`.
    let (x, y) = if g.one_in(2) {
        let x = around(g, cx.wrapping_sub(0x10), cx.wrapping_add(0x130));
        let y = cy.wrapping_sub(0x1f).wrapping_add(g.below(0xe5) as u16);
        (x, y)
    } else {
        let x = cx.wrapping_sub(0xf).wrapping_add(g.below(0x140) as u16);
        let y = around(g, cy.wrapping_sub(0x20), cy.wrapping_add(0xc5));
        (x, y)
    };
    s.w(o, x);
    s.w(o + 2, y);
    let kind = if g.one_in(10) {
        g.next() as u16
    } else {
        g.below(9) as u16
    };
    s.w(o + 4, kind);
    let col = (x as u32 >> 4) * 0x60;
    for row in [y as u32 >> 4, y.wrapping_add(1) as u32 >> 4] {
        for d in [-0x60i32, 0, 0x60, 0xc0] {
            let a = h(0, MAP)
                .wrapping_add(col)
                .wrapping_add(row)
                .wrapping_add(d as u32)
                & 0xff_ffff;
            if in_map(a) {
                s.b(a, cell_byte(g));
            }
        }
    }
}

fn camera_window(g: &mut Gen, s: &mut Setup) -> (u16, u16) {
    let (cx, cy) = (camera(g, 0x17d0), camera(g, 0x540));
    s.w(h(0, CAMERA_X), cx);
    s.w(h(0, CAMERA_Y), cy);
    (cx, cy)
}

/// `h0_2dd6`: the camera, the five slots and their flags.
fn slots_off_screen(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let (cx, cy) = camera_window(g, &mut s);
    for i in 0..5 {
        let flag = if g.one_in(3) {
            0
        } else {
            let any = g.next() as u8 | 1;
            g.pick(&[1, 0xff, any])
        };
        s.b(h(0, SLOT_FLAGS) + i, flag);
        slot_off_screen_state(g, &mut s, i, cx, cy);
    }
    s
}

/// `h0_2e0e` alone: `d0` the slot, the index long whose flag it clears (mostly the same slot), and
/// that slot. The flag itself is only written, so it is not poked.
fn slot_off_screen(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let (cx, cy) = camera_window(g, &mut s);
    let i = g.below(5) as u32;
    s.d[0] = i;
    let index = if g.one_in(4) { g.below(5) as u32 } else { i };
    s.l(h(0, SLOT_INDEX), index);
    slot_off_screen_state(g, &mut s, i, cx, cy);
    s
}

/// A box corner against the probe point `p`: `h0_3dd8` hits when `corner < p <= corner + size`.
fn box_corner(g: &mut Gen, p: u16, size: u16) -> u16 {
    match g.below(10) {
        0 => p.wrapping_sub(size),
        1 => p.wrapping_sub(size + 1),
        2 => p.wrapping_sub(1),
        3 => p,
        4 => g.next() as u16,
        5 => p.wrapping_add(g.below(0x80) as u16),
        6 => p.wrapping_sub(size + 1 + g.below(0x80) as u16),
        _ => p
            .wrapping_sub(size)
            .wrapping_add(g.below(size as u64) as u16),
    }
}

/// `char_hit_check`: CLXDAT, the gates, the character and the five slots around it, the hit
/// count and what a third hit reads.
fn hit_check(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let clx = if g.one_in(4) {
        g.next() as u16 & !6
    } else {
        g.next() as u16 | (2 << g.below(2))
    };
    s.hw_reads.push((CLXDAT, (clx >> 8) as u8));
    s.hw_reads.push((CLXDAT + 1, clx as u8));
    let v = if g.one_in(6) { g.next() as u16 | 1 } else { 0 };
    s.w(h(0, FLAG_4B62), v);
    let v = if g.one_in(6) { 0 } else { g.next() as u8 | 1 };
    s.b(h(0, FLAG_4ED0), v);
    let (x, y) = (char_coord(g, LEVEL_W), char_coord(g, LEVEL_H));
    s.l(h(0, CHAR_X), x);
    s.l(h(0, CHAR_Y), y);
    let (px, py) = (x.wrapping_add(0x10) as u16, y.wrapping_add(0x24) as u16);
    for i in 0..5 {
        let flag = if g.one_in(3) { 0 } else { g.next() as u8 | 1 };
        s.b(h(0, SLOT_FLAGS) + i, flag);
        let o = h(0, SLOTS) + i * 8;
        s.w(o, box_corner(g, px, 0x30));
        s.w(o + 2, box_corner(g, py, 0x20));
        let kind = match g.below(4) {
            0 => 7,
            1 => g.next() as u16,
            _ => g.below(9) as u16,
        };
        s.w(o + 4, kind);
    }
    let hits: u16 = match g.below(8) {
        0 => 0xffff,
        1 => g.pick(&[0x7fff, 0x8000, 0x7ffe]),
        2 => g.next() as u16,
        _ => g.below(5) as u16,
    };
    s.w(h(0, HITS_TAKEN), hits);
    let v = if g.one_in(5) { g.next() as u8 | 1 } else { 0 };
    s.b(h(0, FLAG_9B04), v);
    let v = if g.one_in(5) { g.next() as u16 | 1 } else { 0 };
    s.w(h(0, CHEAT_FLAG), v);
    s.w(h(0, LIVES), bcd_word(g, 4));
    for i in 0..4 {
        s.b(h(0, SCORE) + i, bcd(g));
    }
    s
}

/// `h0_3dd8`: `a0` a slot, its corner, and the probe point in `d0`/`d1` (upper words random).
fn box_test(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let o = h(0, SLOTS) + g.below(5) as u32 * 8;
    s.a[0] = o;
    let (px, py) = (g.below(LEVEL_W) as u16, g.below(LEVEL_H) as u16);
    s.d[0] = (s.d[0] & 0xffff_0000) | px as u32;
    s.d[1] = (s.d[1] & 0xffff_0000) | py as u32;
    s.w(o, box_corner(g, px, 0x30));
    s.w(o + 2, box_corner(g, py, 0x20));
    s
}

/// Hunk 3 (chip BSS: the screens and tiles) filled with seeded noise, so the item blit's source is
/// never all zero and a port that skips the blit differs.
fn screens(mem: &mut [u8]) {
    let mut g = Gen::new(0x5EED_0B1E);
    for b in &mut mem[h(3, 0) as usize..h(3, 0x3_aa28) as usize] {
        *b = g.next() as u8;
    }
}
