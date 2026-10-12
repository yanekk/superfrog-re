//! Scrolling (docs/12 "Scroll"): the vertical blank latches the camera into the level copper
//! list (`scroll_latch`, h0+$1700a).

use crate::layout::{h0, h2, h3};
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
