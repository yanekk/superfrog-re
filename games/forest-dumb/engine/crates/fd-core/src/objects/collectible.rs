//! Taking what lies under the character, and creatures written back as they leave the view
//! (plans/forest-dumb-level-1 T22; docs/10-object-system.md, "Taking and writing back, ported").
//!
//! Each pass `char_object_cells` reads the object map at two cells in the character's column, its
//! feet (`char_y + $19`) then its body (`char_y + 9`), and `h0_26ec` acts on what each holds: the exit
//! sets `level_exit_touched`; an item (`$20` to `$27`) or a bonus (`$40` to `$5f`) is cleared from the
//! map, its tile redrawn (`h0_29ca`) and scored. Before that, in the object block of the pass, `h0_2dd6`
//! writes each creature that has left the view back into the map as its value, near where it was.
//!
//! Ported from `char_object_cells` (`h0+$2698`), `h0_26ec`, `h0_29ca`, `score_add_routine`
//! (`h0+$2a6e`), `countdown_plus_100` (`h0+$6a04`), `items_left_minus_one` (`h0+$6a4c`), `h0_2dd6` and
//! `h0_2e0e`; each is proven on the routine rig (`fd-conform/tests/collect_routines.rs`). The arithmetic
//! is the original's, at its widths: a wild coordinate reaches where the original's would.

use super::{Io, MapObjects, Sound};
use crate::level::redraw::{Blit, OBJECT_MAP, SCREEN_LINE, TILE_MAP};

/// Bytes an object map column: 96 cells.
pub const MAP_COLUMN: u32 = 0x60;
/// The tile table (`h0+$3b988`): a long a tile, its offset from `h3+$13540` (`n * $a0`).
pub const TILE_TABLE: u32 = 0x3_b988;
/// The column `char_object_cells` reads: `char_x` plus this.
pub const PROBE_X: u32 = 0xe;
/// The two rows it reads, below `char_y`, in its order: the feet, then the body.
pub const PROBE_ROWS: [u32; 2] = [0x19, 9];
/// The exit's value.
pub const EXIT: u8 = 0xff;
/// Item values, `$20` to `$27` (the coins); what an item scores, BCD.
pub const ITEM_FIRST: u8 = 0x20;
pub const ITEM_END: u8 = 0x28;
pub const ITEM_SCORE: u32 = 0x100;
/// Bonus values, `$40` to `$5f`; `$51` up are taken and score nothing.
pub const BONUS_FIRST: u8 = 0x40;
pub const BONUS_END: u8 = 0x60;
/// The bonus that adds 100 to the countdown instead of scoring.
pub const TIME_BONUS: u8 = 0x4c;
/// What bonuses `$40` to `$50` score, BCD, by value less [`BONUS_FIRST`]; the [`TIME_BONUS`]'s entry
/// is unused.
pub const BONUS_SCORE: [u32; 17] = [
    0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0x100, 0x1000, 0x1500, 0x2000, 0, 0x2000,
    0x3000, 0x4000, 0x5000,
];
/// `countdown_plus_100` stops at this many seconds, BCD: a countdown of `$899` or more becomes it.
pub const COUNTDOWN_MAX: u16 = 0x999;
/// A creature leaving the view: the window it stays in, from the camera, in level pixels. Out past
/// the left edge at `camera_x - $10` or less, the right at more than `camera_x + $130`, the top at
/// `camera_y - $20` or less, the bottom at more than `camera_y + $c5`.
pub const VIEW_LEFT: u16 = 0x10;
pub const VIEW_RIGHT: u16 = 0x130;
pub const VIEW_TOP: u16 = 0x20;
pub const VIEW_BOTTOM: u16 = 0xc5;
/// Creature slots.
pub const SLOTS: u32 = 5;

/// ABCD: `dst + src + x` on two valid BCD digits, as the 68000 adds them; the byte and the carry.
pub fn abcd(dst: u8, src: u8, x: bool) -> (u8, bool) {
    let mut r = (dst & 0xf) as u16 + (src & 0xf) as u16 + x as u16;
    if r > 9 {
        r += 6;
    }
    r += (dst & 0xf0) as u16 + (src & 0xf0) as u16;
    let carry = r > 0x99;
    if carry {
        r = r.wrapping_sub(0xa0);
    }
    (r as u8, carry)
}

/// SBCD: `dst - src - x` on two valid BCD digits, as the 68000 subtracts them; the byte and the borrow.
pub fn sbcd(dst: u8, src: u8, x: bool) -> (u8, bool) {
    let mut r = ((dst & 0xf) as u16)
        .wrapping_sub((src & 0xf) as u16)
        .wrapping_sub(x as u16);
    if r > 9 {
        r = r.wrapping_sub(6);
    }
    r = r
        .wrapping_add((dst & 0xf0) as u16)
        .wrapping_sub((src & 0xf0) as u16);
    let borrow = r > 0x99;
    if borrow {
        r = r.wrapping_add(0xa0);
    }
    (r as u8, borrow)
}

/// `a + b` as four BCD bytes added from the last, the carry starting clear (the original's run of
/// `abcd -(a3), -(a2)`); a carry out of the first byte is lost.
pub fn bcd_add(a: u32, b: u32) -> u32 {
    let (a, b) = (a.to_be_bytes(), b.to_be_bytes());
    let mut out = [0u8; 4];
    let mut x = false;
    for i in (0..4).rev() {
        (out[i], x) = abcd(a[i], b[i], x);
    }
    u32::from_be_bytes(out)
}

/// A coordinate's 8-pixel cell as a byte offset into a word map: the low word `>> 3` with bit 0
/// cleared (`lsr.w #3; andi.b #$fe`), the high word kept.
fn half_cell(v: u32) -> u32 {
    (v & 0xffff_0000) | ((v as u16 >> 3) & 0xfffe) as u32
}

impl MapObjects<'_> {
    /// `score_add_routine` (`h0+$2a6e`): `score_add` added to the score in BCD, then cleared.
    pub fn add_score(&mut self) {
        let s = &mut *self.state;
        s.score = bcd_add(s.score, s.collect.score_add);
        s.collect.score_add = 0;
    }

    /// `h0_29ca` (`h0+$29ca`): the cell at pixel (`x`, `y`) redrawn on the play screen from the tile
    /// map: its tile word, the tile table's offset for it, one tile's blit. The screen cell is the
    /// row `y & $f0` and the column `x`'s 8-pixel byte, its high word kept as the original adds it.
    /// The original returns `object_taken` in `d0`, which the caller compares.
    pub fn redraw_cell(&mut self, x: u32, y: u32) {
        let (col, row) = (half_cell(x), half_cell(y));
        let dst = col.wrapping_add((y & 0xf0) * SCREEN_LINE);
        let tile = self.io.rw(TILE_MAP
            .wrapping_add((col as u16 as u32) * MAP_COLUMN)
            .wrapping_add(row));
        let src = self.io.rl(TILE_TABLE.wrapping_add(tile as u32 * 4));
        self.io.blit(Blit::Cell { src, dst });
    }

    /// `char_object_cells` (`h0+$2698`): `level_exit_touched` cleared, then the cells at the
    /// character's column and each of [`PROBE_ROWS`] taken ([`MapObjects::take`]), `collect.row` set
    /// to the row first.
    pub fn object_cells(&mut self) {
        todo!()
    }

    /// `h0_26ec` (`h0+$26ec`): what the object map holds at pixel (`x`, `y`) is taken. The exit sets
    /// `level_exit_touched`. An item or a bonus is cleared from the map, `collect.taken` set, and its
    /// cell redrawn ([`MapObjects::redraw_cell`]) at the character's column and `collect.row`; an item
    /// then counts down `items_left` and scores [`ITEM_SCORE`] with [`Sound::Item`]; a bonus scores
    /// [`BONUS_SCORE`] with [`Sound::Item`], or the [`TIME_BONUS`] adds time with [`Sound::Time`].
    /// The value's compares are signed bytes (`bmi`, `bpl`).
    pub fn take(&mut self, x: u32, y: u32) {
        todo!()
    }

    /// `countdown_plus_100` (`h0+$6a04`): 100 added to the countdown's three BCD bytes, or the
    /// countdown's word set to [`COUNTDOWN_MAX`] when it is `$899` or more (a signed compare).
    pub fn add_time(&mut self) {
        todo!()
    }

    /// `items_left_minus_one` (`h0+$6a4c`): `items_left` counted down one in BCD, unless it is 0.
    pub fn count_item(&mut self) {
        todo!()
    }

    /// `h0_2dd6` (`h0+$2dd6`): each active creature slot, by `creatures.index` from 0 to 4, that has
    /// left the view written back ([`MapObjects::write_back_slot`]); the index is left at [`SLOTS`].
    pub fn write_back(&mut self) {
        todo!()
    }

    /// `h0_2e0e` (`h0+$2e0e`): creature slot `i`, if it is out of the view the camera leaves (left,
    /// right, top, bottom, tested in that order as signed word differences), is written into the
    /// object map as its kind plus one, in its cell or the first empty one of two neighbours that
    /// side's tries name, and the active flag of slot `creatures.index` cleared.
    pub fn write_back_slot(&mut self, i: usize) {
        todo!()
    }
}

/// The object map cell for pixel (`x`, `y`): column `x >> 4` (its low word times [`MAP_COLUMN`]),
/// row `y >> 4`, as a hunk 0 offset.
pub fn map_cell(x: u32, y: u32) -> u32 {
    let col = ((x >> 4) & 0xffff) * MAP_COLUMN;
    OBJECT_MAP.wrapping_add(col).wrapping_add(y >> 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bcd_adds_and_subtracts_with_carry() {
        assert_eq!(abcd(0x99, 0x01, false), (0x00, true));
        assert_eq!(abcd(0x45, 0x45, true), (0x91, false));
        assert_eq!(sbcd(0x00, 0x01, false), (0x99, true));
        assert_eq!(sbcd(0x10, 0x01, false), (0x09, false));
        assert_eq!(bcd_add(0x0000_9990, 0x0000_0020), 0x0001_0010);
        assert_eq!(bcd_add(0x9999_9999, 0x1), 0);
    }

    #[test]
    fn the_cell_is_the_columns_96_rows_on() {
        assert_eq!(map_cell(0x10, 0x20), OBJECT_MAP + MAP_COLUMN + 2);
        assert_eq!(half_cell(0x0001_0019), 0x0001_0002);
    }
}
