//! The play field (docs/12 "Scroll", "The play field").
//!
//! The original scrolls two display buffers, each a ring of 19 tile rows (304 lines) whose rows
//! are 48 bytes apart, and moves the bitplane pointers across them: a column of 16 pixels is two
//! bytes, so scrolling right adds 2 to every pointer, and the copper reloads the pointers where the
//! ring wraps. The buffers are state (owner, 2026-10-11): the map alone cannot say what they hold,
//! since objects and tile animations write into them and a bob over a changed tile is gone until
//! that buffer is drawn again. So the shown lines are fetched from the buffers like any other.
//!
//! Widescreen's extra columns are not in the buffers; they come from the map. A line fetched from
//! ring line `R` of a buffer, with `scroll_column` at column `c` (h0+$1700a latches both from the
//! camera), shows level row `R - 16` (mod 304), and fetch pixel `i` shows level x `16 c + i - 16`.
//! Where the map ends the level is colour 0.

use super::{Mem, Row, Under};
use crate::layout::{PLAYFIELD_BUFFER_BYTES, h3};
use crate::state::State;

/// The two display buffers (h3+$10600 and h3+$22f30).
pub const BUFFERS: [u32; 2] = [h3(0x10600), h3(0x22f30)];
/// Lines in a buffer's ring: 19 rows of 16.
const RING: i32 = 304;
/// A buffer line's level row is its ring line minus this; a buffer column's level x is
/// `16 × column − COLUMN_OFFSET`. Both follow from h0+$1700a scrolling to camera + $17 and + 7.
const RING_OFFSET: i32 = 16;
const COLUMN_OFFSET: i32 = 16;

/// The display buffer an address is in.
fn buffer_of(addr: u32) -> Option<usize> {
    BUFFERS
        .iter()
        .position(|&b| (b..b + PLAYFIELD_BUFFER_BYTES).contains(&addr))
}

/// Follows a frame's play-field lines: the level row each one fetches.
#[derive(Debug, Default)]
pub(super) struct Lines {
    prev: Option<i32>,
}

impl Lines {
    /// For a line fetched from `pt` with lines `stride` bytes apart: its level row and the level x
    /// of fetch pixel 0, when `pt` points into a display buffer on a whole line at the scrolled
    /// column.
    pub fn line(&mut self, state: &State, pt: u32, stride: i32) -> Option<(i32, i32)> {
        let buffer = buffer_of(pt)?;
        let column_bytes = state
            .scroll_column
            .checked_sub(BUFFERS[buffer_of(state.scroll_column)?])?;
        let rem = (pt - BUFFERS[buffer]).checked_sub(column_bytes)? as i32;
        if stride <= 0 || rem % stride != 0 {
            return None;
        }
        let ring = rem / stride;
        // The first line is nearest the camera (the latched scroll is at most a frame old); each
        // later line follows the one before.
        let near = self.prev.map_or(state.camera.y as i32 + 7, |p| p + 1);
        let mut y = near + (ring - RING_OFFSET - near).rem_euclid(RING);
        if y - near > RING / 2 {
            y -= RING;
        }
        self.prev = Some(y);
        Some((y, 8 * column_bytes as i32 - COLUMN_OFFSET))
    }
}

/// The level from the map, in level space, under the play-field lines: what widescreen's extra
/// columns show.
#[derive(Debug, Default)]
pub(super) struct Field {
    x: i32,
    y: i32,
    width: usize,
    pixels: Vec<u8>,
}

impl Field {
    /// The map's tiles under the lines `rows` show, `width` picture pixels from each line's
    /// level x; only where `extra` columns reach past the original's window.
    pub fn draw(state: &State, mem: &Mem, rows: &[Row], width: u16, extra: u16) -> Field {
        if extra == 0 {
            return Field::default();
        }
        let spans: Vec<(i32, i32)> = rows
            .iter()
            .filter_map(|r| match r.under {
                Under::Field { y, x0 } => Some((y, x0)),
                _ => None,
            })
            .collect();
        let (Some(x), Some(x_last)) = (
            spans.iter().map(|s| s.1).min(),
            spans.iter().map(|s| s.1).max(),
        ) else {
            return Field::default();
        };
        let x_end = x_last + width as i32;
        let y = spans.iter().map(|s| s.0).min().unwrap_or(0);
        let y_end = spans.iter().map(|s| s.0).max().unwrap_or(0) + 1;
        let mut f = Field {
            x,
            y,
            width: (x_end - x) as usize,
            pixels: vec![0; (x_end - x) as usize * (y_end - y) as usize],
        };
        let tiles = mem.world.map(|w| &w.tiles[..]).unwrap_or(&[]);
        for ly in y..y_end {
            for lx in x..x_end {
                let cell = state.cell(lx.div_euclid(16), ly.div_euclid(16));
                let px = cell
                    .and_then(|c| tiles.get((c >> 6) as usize))
                    .map_or(0, |t| {
                        t[(ly.rem_euclid(16) * 16 + lx.rem_euclid(16)) as usize]
                    });
                let p = (ly - y) as usize * f.width + (lx - x) as usize;
                f.pixels[p] = px;
            }
        }
        f
    }

    pub fn at(&self, x: i32, y: i32) -> u8 {
        let (dx, dy) = (x - self.x, y - self.y);
        if dx < 0 || dx as usize >= self.width || dy < 0 {
            return 0;
        }
        self.pixels
            .get(dy as usize * self.width + dx as usize)
            .copied()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_buffer_line_maps_to_its_level_row_through_the_ring() {
        let mut s = State::default();
        s.scroll_column = BUFFERS[0] + 2;
        s.camera.y = 561;
        let mut lines = Lines::default();
        // Ring line 280 at column 1: level row 264 + 304, x of fetch pixel 0 is 0.
        assert_eq!(
            lines.line(&s, BUFFERS[0] + 2 + 280 * 48, 48),
            Some((568, 0))
        );
        // The next line, after the copper's reload to ring line 0 (the ring wraps after 304).
        let mut y = 568;
        for r in 281..304 {
            y += 1;
            assert_eq!(
                lines.line(&s, BUFFERS[0] + 2 + r * 48, 48).map(|l| l.0),
                Some(y)
            );
        }
        assert_eq!(lines.line(&s, BUFFERS[0] + 2, 48), Some((592, 0)));
        // Not a whole line at the scrolled column, or not in a buffer: not the play field.
        assert_eq!(lines.line(&s, BUFFERS[0] + 3, 48), None);
        assert_eq!(lines.line(&s, 0x52b8c, 48), None);
    }

    #[test]
    fn past_the_map_s_edges_the_level_is_colour_0() {
        use super::super::{DisplayData, Mem, Row, Under, WorldDisplay};
        let mut s = State::default();
        s.world = 1;
        // Every cell tile 1, tile 1 all colour 7.
        s.map.fill(1 << 6);
        let data = DisplayData {
            worlds: vec![WorldDisplay {
                tiles: vec![[0; 256], [7; 256]],
            }],
            ..DisplayData::default()
        };
        let mem = Mem::new(&s, &data);
        let row = |y, x0| Row {
            under: Under::Field { y, x0 },
            priority: 0,
            sprites: [None; 8],
        };
        // A widescreen line reaching 40 pixels past the right edge (x 4800), one past the bottom.
        let rows = [row(1343, 4800 - 334), row(1344, 4800 - 334)];
        let f = Field::draw(&s, &mem, &rows, 374, 20);
        assert_eq!(f.at(4799, 1343), 7);
        assert_eq!(f.at(4800, 1343), 0);
        assert_eq!(f.at(4839, 1343), 0);
        assert_eq!(f.at(4700, 1344), 0);
        // And the left edge.
        let f = Field::draw(&s, &mem, &[row(0, -80)], 374, 80);
        assert_eq!((f.at(-1, 0), f.at(0, 0)), (0, 7));
    }
}
