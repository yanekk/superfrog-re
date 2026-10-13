//! Scrolling (docs/12 "Scroll"): the vertical blank latches the camera into the level copper
//! list (`scroll_latch`, h0+$1700a).

use crate::layout::{h0, h2, h3};
use crate::level::beam::Cost;
use crate::level::{LevelCtx, n_clear};
use crate::mem::Bus;

/// `level_copper` (h2+$6bd0).
pub const LEVEL_COPPER: u32 = h2(0x6bd0);
/// `ring_rows` (h0+$174d6): the boot's pointers to each 16-line row of `playfield_buffer_0`'s ring.
const RING_ROWS: u32 = h0(0x174d6);
/// `row_lines` (h0+$17642): the byte offset of each of a row's 16 lines (`$30` apart).
const ROW_LINES: u32 = h0(0x17642);
/// `ring_row_lines` (h0+$176c4): the boot's line number of each ring row (0, `$10` … `$120`).
const RING_ROW_LINES: u32 = h0(0x176c4);
/// `playfield_buffer_0` (h3+$10600); buffer 1 is `$12930` on.
const BUFFER_0: u32 = h3(0x10600);
pub const BUFFER_BYTES: u32 = 0x12930;
/// `scroll_column` (h0+$172b0).
pub const SCROLL_COLUMN: u32 = h0(0x172b0);
/// `shown_buffer` (h0+$1665e): 1 while buffer 0 is shown, 2 while buffer 1 is.
pub const SHOWN_BUFFER: u32 = h0(0x1665e);
/// The level copper list's ring seam: the wait before the second pointer reload, and the moves
/// just past the list's first kilobyte (h2+$6fd0..$6fd8) that end it.
const SEAM_TAIL: u32 = h2(0x6fd0);
/// One plane of a display buffer.
const PLANE: u32 = 0x3b70;

/// Write a long into a copper move pair: high word at `at`, low word four bytes on.
fn pointer(b: &mut Bus, at: u32, v: u32) {
    b.set_w(at + 4, v as u16);
    b.set_w(at, (v >> 16) as u16);
}

/// `scroll_latch` (h0+$1700a): the camera, plus 7 across and `$17` down, into BPLCON1's delays,
/// the play field's bitplane pointers (the ring row and line under the camera's top, the column
/// at its left, in the shown buffer), the ring seam's wait and reload, and `scroll_column`.
pub fn scroll_latch(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let mut d0 = b.l(h0(0x16664)).wrapping_sub(8);
    let mut d1 = b.l(h0(0x16660)).wrapping_add(0x13);
    d0 = d0 & 0xffff_0000 | (d0 as u16).wrapping_add(0xf) as u32;
    d1 = d1.wrapping_add(4);
    let (d2, d3) = (d0, d1);
    let col = (d0 as u16 >> 4).wrapping_mul(2);
    d0 = d0 & 0xffff_0000 | col as u32;
    let row = (d1 as u16 >> 4).wrapping_mul(4);
    let line = (d3 as u16 & 0xf).wrapping_mul(4);
    let delay = !(d2 as u16) & 0xf;
    b.set_b(LEVEL_COPPER + 0x2ab, (delay | delay << 4) as u8);
    let mut d4 = b
        .l(RING_ROWS.wrapping_add(row as i16 as i32 as u32))
        .wrapping_add(b.l(ROW_LINES.wrapping_add(line as i16 as i32 as u32)))
        .wrapping_add(d0);
    let mut d5 = BUFFER_0.wrapping_add(d0);
    b.set_l(SCROLL_COLUMN, d5);
    // The seam: the line of the play field where the ring wraps.
    let ring_line = b.w(RING_ROW_LINES.wrapping_add((row >> 1) as i16 as i32 as u32));
    let mut d6 = 0x139u16.wrapping_sub(ring_line).wrapping_sub(line >> 2);
    if n_clear(d6.wrapping_sub(0x107)) {
        d6 = 0x107;
    }
    if n_clear(d6.wrapping_sub(0xd4)) {
        b.set_l(SEAM_TAIL + 4, 0x0101_ff00);
        b.set_w(SEAM_TAIL, 0xffdf);
        b.set_b(LEVEL_COPPER + 0x35c, d6.wrapping_sub(0xd4) as u8);
    } else {
        b.set_l(SEAM_TAIL + 4, 0xffdf_fffe);
        b.set_w(SEAM_TAIL, 1);
        b.set_b(LEVEL_COPPER + 0x35c, d6.wrapping_add(0x2c) as u8);
    }
    if b.l(LEVEL_COPPER + 0x35c) == 0xffd7_fffe {
        b.set_l(SEAM_TAIL + 4, 0x0101_ff00);
    }
    for k in 0..5u32 {
        b.set_w(LEVEL_COPPER + 0x360 + 8 * k, 0xe0 + 4 * k as u16);
        b.set_w(LEVEL_COPPER + 0x364 + 8 * k, 0xe2 + 4 * k as u16);
    }
    b.set_w(LEVEL_COPPER + 0x3d0, 0xffdf);
    if b.w(LEVEL_COPPER + 0x35c) == 0xffdf || b.w(SEAM_TAIL) == 0xffdf {
        b.set_w(LEVEL_COPPER + 0x3d0, b.w(LEVEL_COPPER + 0x35c));
        b.set_w(LEVEL_COPPER + 0x35c, 0xfedf);
        if b.w(LEVEL_COPPER + 0x3d0) == 0xffdf {
            b.set_w(LEVEL_COPPER + 0x3d0, 0x98);
        }
        for k in 0..5u32 {
            b.set_w(LEVEL_COPPER + 0x360 + 8 * k, 0x98);
            b.set_w(LEVEL_COPPER + 0x364 + 8 * k, 0x98);
        }
    }
    if b.w(SHOWN_BUFFER) != 1 {
        d4 = d4.wrapping_add(BUFFER_BYTES);
        d5 = d5.wrapping_add(BUFFER_BYTES);
        b.set_l(SCROLL_COLUMN, b.l(SCROLL_COLUMN).wrapping_add(BUFFER_BYTES));
    }
    for k in 0..5u32 {
        pointer(b, LEVEL_COPPER + 0x2ae + 8 * k, d4.wrapping_add(PLANE * k));
        pointer(b, LEVEL_COPPER + 0x362 + 8 * k, d5.wrapping_add(PLANE * k));
    }
    let col = b.l(SCROLL_COLUMN);
    for k in 0..5u32 {
        pointer(b, LEVEL_COPPER + 0x3d6 + 8 * k, col.wrapping_add(PLANE * k));
    }
}

/// `playfield_clean` (h3+$35860): the clean background, tiles only; display buffer 0 is
/// `$25260` below it, buffer 1 `$12930`.
pub const CLEAN: u32 = h3(0x35860);
const CLEAN_TO_BUFFER_0: u32 = 0x25260;
/// `tile_ptrs` (h1+$30fe0).
use crate::mem::TILE_PTRS;
/// `map_rows` (h0+$17334).
use crate::level::collision::MAP_ROWS;

/// The scroller's variables (h0+$16668..$1668a).
const ROW_SHIFT: u32 = h0(0x16668);
const COL_SHIFT_X: u32 = h0(0x1666c);
const DRAW_TOGGLE: u32 = h0(0x16674);
const TILES_PER_LINE: u32 = h0(0x16676);
const ROW_SHIFT_X: u32 = h0(0x1667a);
const COL_SHIFT_Y: u32 = h0(0x1667e);
const BLITS_PENDING: u32 = h0(0x16688);
const DRAWN: u32 = h0(0x1668a);
/// `rows_drawn` (h0+$16606): the 17 rows' level y the last row pass drew; `cols_drawn`
/// (h0+$1662c): the 23 columns' level x the last column pass drew.
const ROWS_DRAWN: u32 = h0(0x16606);
const COLS_DRAWN: u32 = h0(0x1662c);
/// `blit_queue` (h0+$1675a): 23 entries of 16 bytes (active, kind, size, wrap offset, source),
/// ended by a negative word; `blit_next` (h0+$16756) the next free one.
const BLIT_QUEUE: u32 = h0(0x1675a);
const BLIT_NEXT: u32 = h0(0x16756);
/// `shooter_column` (h0+$16752): the bonus level's next column, 1..19.
const SHOOTER_COLUMN: u32 = h0(0x16752);

/// The scroller (h0+$1668c), each vertical blank: on alternate frames the rows or the columns of
/// tiles that came into view since the last pass of that kind are drawn into the clean buffer
/// and queued for copying into both display buffers, which the copper interrupt does at once
/// ([`copper_irq_blits`]). In the bonus level a column scrolls in every frame instead.
pub fn scroller(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(crate::level::camera::SHOOTER_MODE) != 0 {
        return shooter_scroll(c);
    }
    b.set_w(DRAWN, 0);
    let t = !b.b(DRAW_TOGGLE);
    b.set_b(DRAW_TOGGLE, t);
    if t != 0 {
        draw_rows(b);
    } else {
        draw_cols(b);
    }
    let drawn = c.bus.w(DRAWN) as u32;
    let tiles = drawn * c.bus.l(TILES_PER_LINE);
    c.spend(Cost::Scroller);
    c.spend_n(Cost::ScrollQueue, drawn);
    c.spend_n(Cost::ScrollTile, tiles);
    if drawn == 0 {
        // h0+$12a14: nothing was drawn, and the original waits eight raster lines.
        c.wait_lines(8);
    }
    let b = &mut c.bus;
    b.set_l(BLIT_NEXT, BLIT_QUEUE);
    b.set_w(BLITS_PENDING, 1);
    // The copper interrupt, requested: it runs [`copper_irq_blits`] as the handler returns.
    c.custom(0x9c, 0x8010);
}

/// h0+$166d8: the bonus level's scroller: the column at the right edge, each frame.
fn shooter_scroll(c: &mut LevelCtx) {
    let b = &mut c.bus;
    b.set_w(DRAW_TOGGLE, 0);
    b.set_b(crate::level::camera::SCROLL_DIRS, b.b(crate::level::camera::SCROLL_DIRS) | 2);
    b.set_l(COL_SHIFT_X, 0x18);
    b.set_l(COL_SHIFT_Y, b.l(SHOOTER_COLUMN));
    b.set_l(TILES_PER_LINE, 1);
    let (x, y) = (b.l(h0(0x16664)), b.l(h0(0x16660)));
    draw_col(b, x, y);
    crate::level::spawn::scan_right_column(c);
    let b = &mut c.bus;
    let n = b.l(SHOOTER_COLUMN).wrapping_add(1);
    b.set_l(SHOOTER_COLUMN, if (n.wrapping_sub(0x14) as i32) < 0 { n } else { 1 });
    b.set_l(BLIT_NEXT, BLIT_QUEUE);
    b.set_w(BLITS_PENDING, 1);
    c.custom(0x9c, 0x8010);
}

/// Whether `v` is within one entry of `list[k]` (the entry before, itself or the one after).
fn seen(b: &Bus, at: u32, v: u16) -> bool {
    v == b.w(at.wrapping_sub(2)) || v == b.w(at) || v == b.w(at + 2)
}

/// h0+$16b8a: the 17 rows on screen not drawn by the last row pass.
fn draw_rows(b: &mut Bus) {
    b.set_l(ROW_SHIFT, 1);
    b.set_l(ROW_SHIFT_X, 0);
    b.set_l(TILES_PER_LINE, 0x18);
    let x = b.l(h0(0x16664));
    let cy = b.l(h0(0x16660));
    let mut y = cy & 0xffff_0000 | (cy as u16 & 0xfff0) as u32;
    for k in 0..0x11u32 {
        if !seen(b, ROWS_DRAWN + 2 * k, y as u16) {
            draw_row(b, x, y);
            b.set_w(DRAWN, b.w(DRAWN).wrapping_add(1));
        }
        y = y & 0xffff_0000 | (y as u16).wrapping_add(0x10) as u32;
    }
    let y0 = cy as u16 & 0xfff0;
    for k in 0..0x11u16 {
        b.set_w(ROWS_DRAWN + 2 * k as u32, y0.wrapping_add(0x10 * k));
    }
}

/// h0+$16dbe: the 23 columns on screen not drawn by the last column pass.
fn draw_cols(b: &mut Bus) {
    b.set_l(COL_SHIFT_X, 0);
    b.set_l(COL_SHIFT_Y, 0);
    b.set_l(TILES_PER_LINE, 0x14);
    let cx = b.l(h0(0x16664));
    let y = b.l(h0(0x16660));
    let x0 = (cx as u16).wrapping_add(5) & 0xfff0;
    let mut x = cx & 0xffff_0000 | x0 as u32;
    for k in 0..0x17u32 {
        if !seen(b, COLS_DRAWN + 2 * k, x as u16) {
            draw_col(b, x, y);
            b.set_w(DRAWN, b.w(DRAWN).wrapping_add(1));
        }
        x = x & 0xffff_0000 | (x as u16).wrapping_add(0x10) as u32;
    }
    for k in 0..0x17u16 {
        b.set_w(COLS_DRAWN + 2 * k as u32, x0.wrapping_add(0x10 * k));
    }
}

/// One tile's 16 rows of each of its 5 planes from `tile` into the buffer at `at`, lines `$30`
/// apart, planes `$3b70` apart. Returns where the fifth plane ended (its start).
fn draw_tile(b: &mut Bus, cell: u32, at: u32) -> u32 {
    let k = ((b.w(cell) & 0xffc0) >> 4) as u32;
    let mut src = b.l(TILE_PTRS + k);
    let mut a1 = at;
    for p in 0..5 {
        if p > 0 {
            a1 = a1.wrapping_add(PLANE);
        }
        for line in 0..16u32 {
            b.set_w(a1.wrapping_add(0x30 * line), b.w(src));
            src = src.wrapping_add(2);
        }
    }
    a1
}

/// Queue a copy of the area at `src` to both display buffers (h0+$168da, a row; h0+$168fc, a
/// column with its wrap offset), if the queue has room.
fn queue(b: &mut Bus, column: Option<u32>, size: u16, src: u32) {
    let e = b.l(BLIT_NEXT);
    if !n_clear(b.w(e)) {
        return;
    }
    match column {
        None => b.set_w(e + 2, 0),
        Some(wrap) => {
            b.set_w(e + 2, 1);
            b.set_l(e + 6, wrap);
        }
    }
    b.set_w(e, 1);
    b.set_w(e + 4, size);
    b.set_l(e + 0xa, src);
    b.set_l(BLIT_NEXT, e.wrapping_add(0x10));
}

/// The low word of `v` doubled (`add.w d0,d0`), the high word kept.
fn double_low(v: u32) -> u32 {
    v & 0xffff_0000 | (v as u16).wrapping_add(v as u16) as u32
}

/// h0+$16c0e: draw the map row at level y `y` (with `ROW_SHIFT` rows on) across the screen's
/// `TILES_PER_LINE` columns into the clean buffer, and queue its copy.
fn draw_row(b: &mut Bus, x: u32, y: u32) {
    let d0 = double_low((x >> 4).wrapping_add(b.l(ROW_SHIFT_X)));
    let mut d1 = y & 0xffff_0000 | (y as u16 >> 4) as u32;
    d1 = d1.wrapping_add(b.l(ROW_SHIFT));
    let d1w = double_low(double_low(d1)) as u16;
    let mut a1 = b
        .l(RING_ROWS.wrapping_add(d1w as i16 as i32 as u32))
        .wrapping_add(d0)
        .wrapping_add(CLEAN_TO_BUFFER_0);
    let mut a2 = b
        .l(MAP_ROWS.wrapping_add(d1w as i16 as i32 as u32))
        .wrapping_add(d0)
        .wrapping_sub(2);
    let n = b.l(TILES_PER_LINE);
    queue(b, None, n as u16, a1);
    let mut d3 = n as u8;
    loop {
        let end = draw_tile(b, a2, a1);
        a2 = a2.wrapping_add(2);
        a1 = end.wrapping_add(0xffff_1242);
        d3 = d3.wrapping_sub(1);
        if d3 == 0 {
            break;
        }
    }
}

/// h0+$16e4a: draw the map column at level x `x` (with `COL_SHIFT_X` columns on) down the
/// screen's `TILES_PER_LINE` rows into the clean buffer, and queue its copy.
fn draw_col(b: &mut Bus, x: u32, y: u32) {
    let mut d0 = x & 0xffff_0000 | (x as u16 >> 4) as u32;
    let mut d1 = y & 0xffff_0000 | (y as u16 >> 4) as u32;
    d1 = d1.wrapping_add(b.l(COL_SHIFT_Y));
    d0 = d0.wrapping_add(b.l(COL_SHIFT_X));
    d1 = d1.wrapping_sub(1);
    d0 = double_low(d0);
    d1 = double_low(double_low(d1));
    let mut a2 = b
        .l(MAP_ROWS.wrapping_add(d1 as u16 as i16 as i32 as u32))
        .wrapping_add(d0)
        .wrapping_sub(2);
    let mut a4 = RING_ROWS.wrapping_add(d1);
    let n = b.l(TILES_PER_LINE);
    let first = b.l(a4).wrapping_add(CLEAN_TO_BUFFER_0).wrapping_add(d0);
    queue(b, Some(d0), n as u16, first);
    let mut d3 = n as u8;
    loop {
        let a1 = b.l(a4).wrapping_add(CLEAN_TO_BUFFER_0).wrapping_add(d0);
        a4 = a4.wrapping_add(4);
        draw_tile(b, a2, a1);
        a2 = a2.wrapping_add(0x260);
        d3 = d3.wrapping_sub(1);
        if d3 == 0 {
            break;
        }
    }
}

/// A blitter copy (BLTCON0 `$09f0`: D = A, all masks open): `size` as BLTSIZE (height in bits
/// 15–6, 0 for 1024; width in words in bits 5–0, 0 for 64), both modulos `modulo`.
fn blit(b: &mut Bus, src: u32, dst: u32, size: u16, modulo: u16) {
    let h = match size >> 6 { 0 => 1024, h => h as u32 };
    let w = match size & 0x3f { 0 => 64, w => w as u32 };
    let (mut s, mut d) = (src & 0x7_fffe, dst & 0x7_fffe);
    for _ in 0..h {
        for _ in 0..w {
            b.set_w(d, b.w(s));
            s = s.wrapping_add(2);
            d = d.wrapping_add(2);
        }
        s = s.wrapping_add(modulo as i16 as i32 as u32);
        d = d.wrapping_add(modulo as i16 as i32 as u32);
    }
}

/// h0+$16a54: the five planes from the clean buffer at `a1` to both display buffers, with the
/// blitter given the bus (DMACON `$8400`, BLTPRI) while it runs.
fn blit_planes(c: &mut LevelCtx, a1: u32, size: u16, modulo: u16) {
    c.custom(0x96, 0x8400);
    let words = (match size >> 6 { 0 => 1024, h => h as u32 }) * (match size & 0x3f { 0 => 64, w => w as u32 });
    for p in 0..5u32 {
        let src = a1.wrapping_add(PLANE * p);
        c.spend(Cost::IrqPlane);
        blit(&mut c.bus, src, src.wrapping_sub(CLEAN_TO_BUFFER_0), size, modulo);
        c.blit_words(words, 0x09f0);
        blit(&mut c.bus, src, src.wrapping_sub(BUFFER_BYTES), size, modulo);
        c.blit_words(words, 0x09f0);
    }
    c.custom(0x96, 0x0400);
}

/// `copper_irq_blits` (h0+$1693e): the copper interrupt the scroller requests runs the queued
/// copies, rows 16 lines high and columns one word wide, a column split where it wraps round the
/// ring.
pub fn copper_irq_blits(c: &mut LevelCtx) {
    if c.bus.w(BLITS_PENDING) == 0 {
        return;
    }
    c.bus.set_w(BLITS_PENDING, 0);
    let mut e = BLIT_QUEUE;
    for _ in 0..0x17 {
        let b = &mut c.bus;
        if b.w(e) == 0 {
            return;
        }
        let a1 = b.l(e + 0xa);
        let n = b.w(e + 4);
        c.spend(Cost::IrqBlits);
        let b = &mut c.bus;
        if b.w(e + 2) == 0 {
            let modulo = 0x18u16.wrapping_sub(n).wrapping_mul(2);
            blit_planes(c, a1, n | 0x400, modulo);
        } else {
            let lines = n.wrapping_shl(4);
            let left = CLEAN.wrapping_add(PLANE).wrapping_sub(a1);
            let q = left / 0x30;
            let d2 = if q > 0xffff { left } else { left & 0xffff_0000 | q };
            let to_end = d2 as u16 & 0xfff0;
            let rest = to_end.wrapping_sub(lines);
            if n_clear(rest) {
                blit_planes(c, a1, lines.wrapping_shl(6) | 1, 0x2e);
            } else {
                blit_planes(c, a1, to_end.wrapping_shl(6) | 1, 0x2e);
                let wrap = CLEAN.wrapping_add(c.bus.l(e + 6));
                blit_planes(c, wrap, rest.wrapping_neg().wrapping_shl(6) | 1, 0x2e);
            }
        }
        c.bus.set_w(e, 0);
        e = e.wrapping_add(0x10);
    }
}
