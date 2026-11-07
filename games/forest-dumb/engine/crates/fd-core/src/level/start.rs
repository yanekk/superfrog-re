//! `level_init`'s first draw of the view (plans/forest-dumb-level-1 T65; games/forest-dumb/docs/
//! 12-level-start-draw.md). After `load_world`, `level_init` draws the 23 columns by 16 rows of cells
//! from the map's top-left: their tiles onto the play screen (`h0+$62e`, each cell by `h0+$672`) and
//! their collision shapes into `collision_map` (`h0+$6c4`, each by `h0+$708`). From then on only the
//! edge redraws (T55, `level/redraw.rs`) add to either.
//!
//! The arithmetic is the original's at its widths: the row (`d6`) is multiplied as a word
//! (`mulu.w`), the column (`d7`) shifted as a long, so a cell past the view lands where the
//! original's would. Callers only pass rows 0 to 15 and columns 0 to 22.
//!
//! The tiles are copied by the CPU, not the blitter, five words a line over 16 lines `$e6` bytes
//! apart: the same bytes the blit `h0_774` writes for the same tile and destination (A to D,
//! `BLTSIZE=$1401`, D modulo `$2c`). So the engine lists each as a [`Blit::Tile`], and the picture (T10)
//! draws it as it draws an edge redraw's: [`crate::Game::new`] leaves the 368 of them in
//! [`crate::Game::blits`] until the first step.

use super::collision::{BEFORE, CollisionPlane, MAP};
use super::redraw::{Blit, Io, SHAPE_MAP, TILE_MAP, shape_copy};

/// Where `load_list_title` reads the title's music (`song`, `_BALLA`, `load_file_raw`): hunk 3 offset.
pub const TITLE_SONG_AT: u32 = 0xf000;

/// What `collision_map` and the slack around it hold before `level_init` draws: the title's music,
/// which runs from `h3+$f000` across them, untouched since the title read it (measured at the sync
/// point, T65: every kept byte outside the map equals `_BALLA`'s). The first view then overwrites the
/// map; the slack keeps the music until the edge redraws reach it. Bytes past the song's end are 0.
pub fn lay_title_song(plane: &mut CollisionPlane, song: &[u8]) {
    let from = (MAP - BEFORE - TITLE_SONG_AT) as usize;
    for (i, b) in plane.kept_mut().iter_mut().enumerate() {
        *b = song.get(from + i).copied().unwrap_or(0);
    }
}

/// Columns and rows the first view draws: `cmpi.l #$17, d7`, `cmpi.l #$10, d6`.
pub const COLUMNS: u32 = 0x17;
pub const ROWS: u32 = 0x10;

/// `h0+$672`: the tile of the cell at row `d6`, column `d7` of the tile map copied onto the play
/// screen at `d6 * $e60 + d7 * 2`.
pub fn tile_cell(io: &mut impl Io, d6: u32, d7: u32) {
    let dst = (d6 & 0xffff)
        .wrapping_mul(0xe60)
        .wrapping_add(d7.wrapping_shl(1));
    let src = TILE_MAP.wrapping_add(
        (d7 & 0xffff)
            .wrapping_mul(0xc0)
            .wrapping_add(d6.wrapping_shl(1)),
    );
    let tile = io.rw(src);
    io.blit(Blit::Tile { tile, dst });
}

/// `h0+$62e`: the first view's tiles, column by column, each column top to bottom.
pub fn draw_tiles(io: &mut impl Io) {
    for d7 in 0..COLUMNS {
        for d6 in 0..ROWS {
            tile_cell(io, d6, d7);
        }
    }
}

/// `h0+$708`: the shape of the cell at row `d6`, column `d7` of the shape map copied into
/// `collision_map` at `d6 * $2e0 + d7 * 2`, 16 words `$2e` apart, as `h0_7cc` copies one.
pub fn shape_cell(io: &mut impl Io, d6: u32, d7: u32) {
    let at = (d6 & 0xffff)
        .wrapping_mul(0x2e0)
        .wrapping_add(d7.wrapping_shl(1));
    let src = SHAPE_MAP.wrapping_add((d7 & 0xffff).wrapping_mul(0x60).wrapping_add(d6));
    let n = io.rb(src) as u32;
    shape_copy(io, n, at);
}

/// `h0+$6c4`: the first view's shapes, in the same order as its tiles.
pub fn draw_shapes(io: &mut impl Io) {
    for d7 in 0..COLUMNS {
        for d6 in 0..ROWS {
            shape_cell(io, d6, d7);
        }
    }
}

/// `level_init`'s two calls, in its order: the tiles, then the shapes.
pub fn first_view(io: &mut impl Io) {
    draw_tiles(io);
    draw_shapes(io);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::level::redraw::SHAPES;

    /// A level of distinct cells: tile word `col << 8 | row`, shape byte `col + row`.
    struct Fake {
        blits: Vec<Blit>,
        words: Vec<(u32, u16)>,
    }

    impl Io for Fake {
        fn rb(&self, a: u32) -> u8 {
            if (SHAPES..SHAPES + 0x1000).contains(&a) {
                // Shape `n`'s word `k` holds `n << 8 | k`, so a wrong shape shows.
                let o = a - SHAPES;
                return if o & 1 == 0 {
                    (o >> 5) as u8
                } else {
                    (o >> 1 & 15) as u8
                };
            }
            let o = a.wrapping_sub(SHAPE_MAP);
            if o < 0x60 * 0x40 {
                return ((o / 0x60) + (o % 0x60)) as u8;
            }
            let o = a.wrapping_sub(TILE_MAP);
            if o < 0xc0 * 0x40 {
                let (col, row) = (o / 0xc0, (o % 0xc0) / 2);
                return if o & 1 == 0 { col as u8 } else { row as u8 };
            }
            0
        }
        fn clear_object(&mut self, _: u32) {
            unreachable!("the first view clears no object")
        }
        fn blit(&mut self, b: Blit) {
            self.blits.push(b);
        }
        fn collision_word(&mut self, at: u32, v: u16) {
            self.words.push((at, v));
        }
    }

    #[test]
    fn every_cell_of_the_view_lands_in_its_row_and_column() {
        let mut f = Fake {
            blits: Vec::new(),
            words: Vec::new(),
        };
        first_view(&mut f);
        assert_eq!(f.blits.len(), 23 * 16);
        assert_eq!(f.words.len(), 23 * 16 * 16);
        for (i, b) in f.blits.iter().enumerate() {
            let (col, row) = ((i / 16) as u32, (i % 16) as u32);
            assert_eq!(
                *b,
                Blit::Tile {
                    tile: (col << 8 | row) as u16,
                    dst: row * 0xe60 + col * 2,
                },
                "cell {col},{row}"
            );
        }
        for (i, &(at, v)) in f.words.iter().enumerate() {
            let (cell, k) = (i / 16, (i % 16) as u32);
            let (col, row) = ((cell / 16) as u32, (cell % 16) as u32);
            assert_eq!(at, row * 0x2e0 + col * 2 + k * 0x2e);
            assert_eq!(
                v,
                ((col + row) << 8 | k) as u16,
                "cell {col},{row} word {k}"
            );
        }
    }
}
