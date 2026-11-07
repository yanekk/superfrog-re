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

use super::{MapObjects, Sound};
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
/// Where `h0_26ec` keeps what each scored bonus is worth: the immediate of its branch's
/// `move.l #amount, score_add` (BCD), `$1c` bytes a branch, in two runs either side of the
/// [`TIME_BONUS`]'s longer branch: (first value, last value, the first's immediate, hunk 0 offset).
/// Read from the program, as the original does, so no table of the game's is copied here.
pub const BONUS_SCORES: [(u8, u8, u32); 2] = [(0x40, 0x4b, 0x27b8), (0x4d, 0x50, 0x295e)];
/// Bytes from one bonus's branch to the next.
pub const BONUS_BRANCH: u32 = 0x1c;
/// `countdown_plus_100` stops at this many seconds, BCD.
pub const COUNTDOWN_MAX: u16 = 0x999;
/// The countdown at or above which `countdown_plus_100` sets [`COUNTDOWN_MAX`], BCD.
pub const TIME_LIMIT: u16 = 0x899;
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

    /// `char_object_cells` (`h0+$2698`): `level_exit_touched` cleared, then the object cells at the
    /// character's column taken ([`MapObjects::take`]), feet then body, `collect.row` set to each row
    /// first.
    pub fn object_cells(&mut self) {
        self.state.level_exit_touched = 0;
        let x = self.state.char_x.wrapping_add(PROBE_X);
        for row in PROBE_ROWS {
            self.state.collect.row = row;
            self.take(x, self.state.char_y.wrapping_add(row));
        }
    }

    /// `h0_26ec` (`h0+$26ec`): what the object map holds at pixel (`x`, `y`) is taken. The exit sets
    /// `level_exit_touched`. An item counts down `items_left` and scores [`ITEM_SCORE`]; a bonus scores
    /// what its branch adds ([`BONUS_SCORES`]), or the [`TIME_BONUS`] adds time with [`Sound::Time`]; `$51` to `$5f` are taken
    /// for nothing. The original's signed byte compares (`bmi`, `bpl`) read `$a0` to `$fe` as below
    /// [`ITEM_FIRST`], which the unsigned ones here reach as past [`BONUS_END`]: the same values do
    /// nothing either way.
    pub fn take(&mut self, x: u32, y: u32) {
        let cell = map_cell(x, y);
        let value = self.io.rb(cell);
        if value == EXIT {
            self.state.level_exit_touched = 1;
            return;
        }
        if value < ITEM_FIRST {
            return;
        }
        if value < ITEM_END {
            self.take_cell(cell, x, value);
            self.count_item();
            self.award(ITEM_SCORE);
            return;
        }
        if !(BONUS_FIRST..BONUS_END).contains(&value) {
            return;
        }
        self.take_cell(cell, x, value);
        if value == TIME_BONUS {
            self.io.sound(Sound::Time);
            self.add_time();
            return;
        }
        if let Some(amount) = self.bonus_score(value) {
            self.award(amount);
        }
    }

    /// What bonus `value` scores, read from its branch of `h0_26ec` ([`BONUS_SCORES`]); `None` for
    /// one whose compare no branch matches.
    fn bonus_score(&self, value: u8) -> Option<u32> {
        BONUS_SCORES
            .iter()
            .find(|&&(first, last, _)| (first..=last).contains(&value))
            .map(|&(first, _, at)| {
                self.io
                    .rl(at.wrapping_add((value - first) as u32 * BONUS_BRANCH))
            })
    }

    /// Part of `h0_26ec`: the taken cell cleared, `object_taken` kept as the original's `d0` (the
    /// column offset with the value in its low byte), and the character's cell at `collect.row`
    /// redrawn, whatever pixel was taken.
    fn take_cell(&mut self, cell: u32, x: u32, value: u8) {
        self.io.wb(cell, 0);
        let column = ((x >> 4) & 0xffff).wrapping_mul(MAP_COLUMN);
        self.state.collect.taken = (column & 0xffff_ff00) | value as u32;
        let px = self.state.char_x.wrapping_add(PROBE_X);
        let py = self.state.char_y.wrapping_add(self.state.collect.row);
        self.redraw_cell(px, py);
    }

    /// Part of `h0_26ec`: `amount` scored through `score_add`, then the item sound.
    fn award(&mut self, amount: u32) {
        self.state.collect.score_add = amount;
        self.add_score();
        self.io.sound(Sound::Item);
    }

    /// `countdown_plus_100` (`h0+$6a04`): 100 added to the countdown's three BCD bytes, or its word set
    /// to [`COUNTDOWN_MAX`] when it is [`TIME_LIMIT`] or more (a signed compare); the long's last byte
    /// is untouched. The adds start with the carry clear, as the one caller leaves it.
    pub fn add_time(&mut self) {
        let s = &mut *self.state;
        let word = (s.countdown >> 16) as u16;
        if word.wrapping_sub(TIME_LIMIT) & 0x8000 == 0 {
            s.countdown = (COUNTDOWN_MAX as u32) << 16 | (s.countdown & 0xffff);
            return;
        }
        let [hi, mid, lo, last] = s.countdown.to_be_bytes();
        let (lo, c) = abcd(lo, 0, false);
        let (mid, c) = abcd(mid, 0, c);
        let (hi, _) = abcd(hi, 1, c);
        s.countdown = u32::from_be_bytes([hi, mid, lo, last]);
    }

    /// `items_left_minus_one` (`h0+$6a4c`): `items_left` counted down one in BCD, unless it is 0.
    pub fn count_item(&mut self) {
        if self.state.items_left == 0 {
            return;
        }
        let [hi, lo] = self.state.items_left.to_be_bytes();
        let (lo, borrow) = sbcd(lo, 1, false);
        let (hi, _) = sbcd(hi, 0, borrow);
        self.state.items_left = u16::from_be_bytes([hi, lo]);
    }

    /// `h0_2dd6` (`h0+$2dd6`): each active creature slot, by `creatures.index` from 0 to 4, that has
    /// left the view written back ([`MapObjects::write_back_slot`]); the index is left at [`SLOTS`].
    pub fn write_back(&mut self) {
        for i in 0..SLOTS {
            self.state.edges.creatures.index = i;
            if self.state.edges.creatures.active[i as usize] != 0 {
                self.write_back_slot(i as usize);
            }
        }
        self.state.edges.creatures.index = SLOTS;
    }

    /// `h0_2e0e` (`h0+$2e0e`): creature slot `i`, if it is out of the view the camera leaves (left,
    /// right, top, bottom, tested in that order as signed word differences), is written into the
    /// object map as its kind plus one, in its cell or the first empty one of the two neighbours that
    /// side tries (the last tried when none is), and the active flag of slot `creatures.index`
    /// cleared. Top and bottom take the cell a pixel lower.
    pub fn write_back_slot(&mut self, i: usize) {
        let slot = self.state.edges.creatures.slots[i];
        let value = slot.kind.wrapping_add(1) as u8;
        let (cx, cy) = (self.state.camera.x, self.state.camera.y);
        let past = |edge: u16, v: u16| edge.wrapping_sub(v) & 0x8000 == 0;
        let at = map_cell(slot.x as u32, slot.y as u32);
        let below = map_cell(slot.x as u32, slot.y.wrapping_add(1) as u32);
        let (left, right) = (
            |c: u32| c.wrapping_sub(MAP_COLUMN),
            |c: u32| c.wrapping_add(MAP_COLUMN),
        );
        let tries = if past(cx.wrapping_sub(VIEW_LEFT), slot.x) {
            [at, left(at), right(at)]
        } else if !past(cx.wrapping_add(VIEW_RIGHT), slot.x) {
            [right(at), right(right(at)), at]
        } else if past(cy.wrapping_sub(VIEW_TOP), slot.y) {
            [below, right(below), below]
        } else if !past(cy.wrapping_add(VIEW_BOTTOM), slot.y) {
            [below, right(below), left(below)]
        } else {
            return;
        };
        let cell = tries[..2]
            .iter()
            .copied()
            .find(|&c| self.io.rb(c) == 0)
            .unwrap_or(tries[2]);
        self.io.wb(cell, value);
        let c = &mut self.state.edges.creatures;
        c.active[c.index as usize] = 0;
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
