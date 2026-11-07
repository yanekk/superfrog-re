//! Springs (plans/forest-dumb-level-1 T25; docs/10-object-system.md, "Springs" and "Mechanics,
//! ported").
//!
//! A spring is two object map cells, a left half (`$10`, or `$12`) and a right half (`$11`, `$13`),
//! that stay in the map. Each pass, while the character falls, `h0_24ce` reads the cell under its feet;
//! on a spring it throws the character up (`char_rising`, `char_jump_41b6`), keeps where (`spring_x`,
//! `spring_y`) and which picture (`spring_kind`), and starts `spring_timer` at 1. `h0_21c0`, earlier in
//! the next pass, draws the pressed spring with a sound at 1, counts to 6, then redraws the cell's four
//! tiles and stops.
//!
//! Ported from `h0_24ce` and `h0_21c0`; each is proven on the routine rig
//! (`fd-conform/tests/mechanic_routines.rs`). The arithmetic is the original's, at its widths.

use super::collectible::{MAP_COLUMN, TILE_TABLE, map_cell};
use super::{MapObjects, Sound};
use crate::level::redraw::{Blit, SCREEN_LINE, TILE_MAP};

/// Spring values: `$10` and `$11` the two halves of one picture, `$12` and `$13` of the other.
pub const SPRING_FIRST: u8 = 0x10;
pub const SPRING_LAST: u8 = 0x13;
/// Where `h0_24ce` reads the cell under the feet: `char_x` and `char_y` plus these.
pub const FEET_X: u32 = 0xe;
pub const FEET_Y: u32 = 0x23;
/// A right half keeps `char_x` less this as `spring_x`, so both halves redraw the same two columns.
pub const RIGHT_HALF_X: u32 = 2;
/// `spring_timer` as a spring fires: the pressed picture and the sound are next.
pub const PRESSED: u16 = 1;
/// `spring_timer` from which the spring's tiles are redrawn and the timer stopped (a signed compare).
pub const RELEASED: u16 = 6;
/// The pressed spring's pictures in hunk 4, by `spring_kind`: zero is `_WYBIJACZKA` (`h4+$9618`), any
/// other value the second picture's slot (`h4+$9898`).
pub const PICTURES: [u32; 2] = [0x9618, 0x9898];
/// A picture's lower 16 lines: 16 lines of five planes of two words.
pub const PICTURE_HALF: u32 = 0x140;
/// A spring picture's rows are back to back: no modulo on A.
pub const PICTURE_MODULO: u16 = 0;
/// A tile map column of words: a column of 16-pixel cells is two of these.
pub const TILE_COLUMN: u32 = 0xc0;
/// The play screen's 256 lines; a tile redraw that moves past them is moved back once.
pub const SCREEN_BYTES: u32 = 0xe600;
/// A cell's pixels, and so the lines of a screen band.
pub const CELL: u32 = 16;
/// One 16-line screen band.
pub const SCREEN_BAND: u32 = CELL * SCREEN_LINE;

/// The spring that fired last (`h0+$21b4` to `$21bf`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Spring {
    /// `spring_x`: the redraw's x in level pixels, `char_x + $e` (left half) or `char_x - 2`.
    pub x: u32,
    /// `spring_y`: `char_y + $23`.
    pub y: u32,
    /// `spring_timer`: 0 idle, [`PRESSED`] as it fires, counted to [`RELEASED`] by `h0_21c0`.
    pub timer: u16,
    /// `spring_kind`: 0 for `$10` and `$11`, 1 for `$12` and `$13`.
    pub kind: u16,
}

impl MapObjects<'_> {
    /// `h0_24ce` (`h0+$24ce`): while the character falls (`char_falling`) and is not dying, a spring
    /// in the cell under its feet throws it up (`char_rising` and `char_jump_41b6` set) and fires:
    /// [`Spring`] takes where and which, its timer [`PRESSED`]. The cell stays.
    pub fn spring_trigger(&mut self) {
        let m = &self.state.motion;
        if m.falling == 0 || m.dying != 0 {
            return;
        }
        let (char_x, char_y) = (self.state.char_x, self.state.char_y);
        let (x, y) = (char_x.wrapping_add(FEET_X), char_y.wrapping_add(FEET_Y));
        let value = self.io.rb(map_cell(x, y));
        if !(SPRING_FIRST..=SPRING_LAST).contains(&value) {
            return;
        }
        let right_half = value & 1 != 0;
        self.state.spring = Spring {
            x: if right_half {
                char_x.wrapping_sub(RIGHT_HALF_X)
            } else {
                x
            },
            y,
            timer: PRESSED,
            kind: ((value - SPRING_FIRST) >> 1) as u16,
        };
        self.state.motion.rising = 1;
        self.state.motion.jump_41b6 = 1;
    }

    /// `h0_21c0` (`h0+$21c0`): one pass of the spring that fired. Idle at `spring_timer` 0. At
    /// [`PRESSED`] the spring sound ([`Sound::Spring`]) and the pressed picture of `spring_kind`
    /// ([`PICTURES`]) over the cell's two screen bands, the second [`PICTURE_HALF`] on; at [`RELEASED`]
    /// or more (signed) the four tiles of the tile map at `spring_x`, `spring_y` redrawn, two columns by
    /// two rows, the lower row moved back [`SCREEN_BYTES`] if it passes the screen's end, and the timer
    /// stopped; otherwise the timer counts one.
    pub fn spring_redraw(&mut self) {
        let timer = self.state.spring.timer;
        if timer == 0 {
            return;
        }
        if timer.wrapping_sub(RELEASED) & 0x8000 != 0 {
            if timer != PRESSED {
                self.state.spring.timer = timer.wrapping_add(1);
                return;
            }
            self.spring_pressed();
            return;
        }
        self.spring_released();
    }

    /// Part of `h0_21c0` (`h0+$21c0`): the pressed spring's picture over its two rows,
    /// with [`Sound::Spring`], the timer counted on.
    fn spring_pressed(&mut self) {
        self.io.sound(Sound::Spring);
        let spring = &self.state.spring;
        let (sx, sy) = (spring.x, spring.y);
        let base = PICTURES[usize::from(spring.kind != 0)];
        let column = sx.wrapping_shr(4).wrapping_shl(1);
        let first = (sy & 0xF0).wrapping_mul(SCREEN_LINE);
        let second = (sy.wrapping_add(CELL) & 0xF0).wrapping_mul(SCREEN_LINE);
        for (i, row) in [first, second].into_iter().enumerate() {
            self.io.blit(Blit::Picture {
                src: base.wrapping_add((i as u32).wrapping_mul(PICTURE_HALF)),
                modulo: PICTURE_MODULO,
                dst: column.wrapping_add(row),
            });
        }
        self.state.spring.timer += 1;
    }

    /// Part of `h0_21c0` (`h0+$21c0`): the spring's 2 x 2 tile map cells redrawn where
    /// the picture was, the timer stopped.
    fn spring_released(&mut self) {
        let (sx, sy) = (self.state.spring.x, self.state.spring.y);
        let col = ((sx & 0xFFFF) >> 3) & !1;
        let row = ((sy & 0xFFFF) >> 3) & !1;
        let screen_col = (sx & 0xFFFF_0000) | col;
        let screen_row = (sy & 0xF0).wrapping_mul(SCREEN_LINE);
        let dst_top = screen_col.wrapping_add(screen_row);
        let mut dst_bottom = dst_top.wrapping_add(SCREEN_BAND);
        if dst_bottom.wrapping_sub(SCREEN_BYTES) & 0x8000_0000 == 0 {
            dst_bottom = dst_bottom.wrapping_sub(SCREEN_BYTES);
        }
        let map_col = col.wrapping_mul(MAP_COLUMN);
        let map_row = (sy & 0xFFFF_0000) | row;
        let tile_top = TILE_MAP.wrapping_add(map_col).wrapping_add(map_row);
        let tiles = [
            tile_top,
            tile_top.wrapping_add(TILE_COLUMN),
            tile_top.wrapping_add(2),
            tile_top.wrapping_add(TILE_COLUMN).wrapping_add(2),
        ];
        let dsts = [
            dst_top,
            dst_top.wrapping_add(2),
            dst_bottom,
            dst_bottom.wrapping_add(2),
        ];
        for (&tile, &dst) in tiles.iter().zip(dsts.iter()) {
            self.spring_redraw_cell(tile, dst);
        }
        self.state.spring.timer = 0;
    }

    /// Part of `h0_21c0` (`h0+$21c0`): the tile map cell at hunk 0 offset `tile`
    /// redrawn on the play screen at `dst`, through the tile table.
    fn spring_redraw_cell(&mut self, tile: u32, dst: u32) {
        let index = self.io.rw(tile);
        let src = self
            .io
            .rl(TILE_TABLE.wrapping_add((index as u32).wrapping_mul(4)));
        self.io.blit(Blit::Cell { src, dst });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::State;
    use crate::level::redraw::OBJECT_MAP;

    struct Map(Vec<u8>);

    impl super::super::Io for Map {
        fn rb(&self, a: u32) -> u8 {
            self.0
                .get(a.wrapping_sub(OBJECT_MAP) as usize)
                .copied()
                .unwrap_or(0)
        }
        fn wb(&mut self, _: u32, _: u8) {}
        fn blit(&mut self, _: Blit) {}
        fn sound(&mut self, _: Sound) {}
        fn collision(&mut self, _: u32, _: u32) {}
    }

    #[test]
    fn a_right_half_fires_two_pixels_left_of_the_character_with_the_second_picture() {
        let mut s = State::level_1();
        s.motion.falling = 1;
        let (x, y) = (0x100, 0x80);
        s.char_x = x;
        s.char_y = y;
        let mut map = Map(vec![0; 0x60 * 0x20]);
        let cell = map_cell(x + FEET_X, y + FEET_Y) - OBJECT_MAP;
        map.0[cell as usize] = 0x13;
        MapObjects {
            state: &mut s,
            io: &mut map,
        }
        .spring_trigger();
        assert_eq!(
            s.spring,
            Spring {
                x: x - 2,
                y: y + FEET_Y,
                timer: PRESSED,
                kind: 1
            }
        );
        assert_eq!((s.motion.rising, s.motion.jump_41b6), (1, 1));
        // Dying, nothing fires.
        let mut d = State::level_1();
        d.motion.falling = 1;
        d.motion.dying = 1;
        d.char_x = x;
        d.char_y = y;
        let before = d.clone();
        MapObjects {
            state: &mut d,
            io: &mut map,
        }
        .spring_trigger();
        assert_eq!(d, before);
    }
}
