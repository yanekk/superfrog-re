//! The animated cells: items, water, vanishing blocks and flames (plans/forest-dumb-level-1 T25;
//! docs/10-object-system.md, "Items", "Water", "Vanishing blocks", "Mechanics, ported").
//!
//! As the view crosses a cell holding `$20` to `$3f`, the edge redraws add it to one of four lists of
//! twelve (`h0_6882`, `level::redraw::AnimatedCells`). Each pass `h0_62b6` steps a phase through the
//! four lists, and the tick every fourth pass, and runs the phase's list: a cell whose map value is
//! gone or that left the view is dropped; the rest are drawn by value. Items and water cut an
//! `_ELEMENTY` frame over their tile. A vanishing block copies the `_ZNIKAJKA` frame its 64-step cycle
//! gives and makes the top half of its cell solid in `collision_map`, or clear at its empty frame: the
//! one game state this changes. A flame copies a `_Plomien` frame over two cells.
//!
//! Ported from `h0_62b6` by hand: at 224 instructions it is over the translation service's size
//! (docs/11-translation.md). Proven on the routine rig (`fd-conform/tests/mechanic_routines.rs`).

use super::MapObjects;
use super::collectible::{MAP_COLUMN, TILE_TABLE, map_cell};
use crate::level::redraw::{AnimCell, Blit, SCREEN_LINE, TILE_MAP};

/// Cells a list holds.
pub const LIST: usize = 12;
/// The lowest animated value: items `$20` to `$27` (frame `value - $20 + tick`, of eight).
pub const ITEM_FIRST: u8 = 0x20;
/// `value - ITEM_FIRST` of water, of the faster two-frame cell, of the first vanishing block, past
/// the last, and of the flame.
pub const WATER: u8 = 8;
pub const WATER_2: u8 = 0xa;
pub const VANISH_FIRST: u8 = 0x10;
pub const VANISH_END: u8 = 0x14;
pub const FLAME: u8 = 0x14;
/// The view a cell stays in, from the camera, in level pixels: dropped at `camera_x - $10` or less,
/// more than `camera_x + $130` (right), `camera_y - $20` or less, more than `camera_y + $c0`.
pub const VIEW_LEFT: u16 = 0x10;
pub const VIEW_RIGHT: u16 = 0x130;
pub const VIEW_TOP: u16 = 0x20;
pub const VIEW_BOTTOM: u16 = 0xc0;
/// The vanishing blocks' frame table (`h0+$63ec`): 64 longs, each a byte offset into
/// [`VANISH_PICTURE`]'s row of six frames. Read from the program, as the original does.
pub const VANISH_FRAMES: u32 = 0x63ec;
/// The offset of the empty frame: the block is clear in `collision_map` while it shows.
pub const VANISH_EMPTY: u8 = 0x14;
/// `_ZNIKAJKA1` in hunk 4: six 32-pixel frames side by side, so a frame's rows are `$14` bytes apart
/// past its own four.
pub const VANISH_PICTURE: u32 = 0x9b18;
pub const VANISH_MODULO: u16 = 0x14;
/// A solid block's `collision_map` long: 28 pixels from its left.
pub const SOLID: u32 = 0xffff_fff0;
/// The block's lines in `collision_map`: the top eight of its cell.
pub const SOLID_LINES: u32 = 8;
/// `collision_map`'s bytes a line.
pub const COLLISION_LINE: u32 = 0x2e;
/// `_Plomien` in hunk 4: two 32-pixel frames side by side, the lower cell's half [`FLAME_HALF`] on.
pub const FLAME_PICTURE: u32 = 0xa298;
pub const FLAME_MODULO: u16 = 4;
pub const FLAME_HALF: u32 = 0x280;

/// What `h0_62b6` keeps from pass to pass, and the water's speed the level sets.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Animation {
    /// `anim_tick` (`h0+$62b2`): +1 each time the phase wraps.
    pub tick: u8,
    /// `anim_tick + 1` (`h0+$62b3`): the list this pass runs, 0 to 3.
    pub phase: u8,
    /// `water_cycle` (`h0+$6342`): set, water runs four frames at half speed; clear, two.
    pub water_cycle: u16,
}

impl Animation {
    /// Both bytes of `anim_tick`, as the original stores them.
    pub fn word(&self) -> u16 {
        u16::from_be_bytes([self.tick, self.phase])
    }

    pub fn set_word(&mut self, w: u16) {
        [self.tick, self.phase] = w.to_be_bytes();
    }
}

/// Whether `h0_62b6` goes on to the list's next cell or returns for the pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Next {
    Cell,
    Return,
}

/// The cell's screen column byte (`x >> 3`, even) and its band's first line (`y & $f0`): 16-pixel cells
/// on the 256-line circular screen.
fn screen_cell(x: u16, y: u16) -> (u32, u32) {
    (((x >> 3) & 0x1ffe) as u32, (y & 0xf0) as u32)
}

impl MapObjects<'_> {
    /// `h0_62b6` (`h0+$62b6`): the phase stepped (the tick with it as it wraps), then the phase's list
    /// of [`LIST`] cells in order. A used cell whose object map value is now 0, or which is outside
    /// the view, is dropped; the rest are drawn ([`MapObjects::animate_cell`]), which can end the pass's
    /// work early.
    pub fn animate(&mut self) {
        let a = &mut self.state.animation;
        a.phase = a.phase.wrapping_add(1) & 3;
        if a.phase == 0 {
            a.tick = a.tick.wrapping_add(1);
        }
        let list = a.phase as usize;
        for i in 0..LIST {
            if self.state.edges.cells.used[list][i] == 0 {
                continue;
            }
            let c = self.state.edges.cells.cells[list][i];
            if self.io.rb(map_cell(c.x as u32, c.y as u32)) == 0 || !self.in_view(c.x, c.y) {
                self.state.edges.cells.used[list][i] = 0;
                continue;
            }
            if self.animate_cell(c) == Next::Return {
                return;
            }
        }
    }

    /// Part of `h0_62b6`: (`x`, `y`) is inside the view, by the original's signed word differences.
    fn in_view(&self, x: u16, y: u16) -> bool {
        let below = |a: u16, b: u16| a.wrapping_sub(b) & 0x8000 != 0;
        let c = &self.state.camera;
        below(c.x.wrapping_sub(VIEW_LEFT), x)
            && !below(c.x.wrapping_add(VIEW_RIGHT), x)
            && below(c.y.wrapping_sub(VIEW_TOP), y)
            && !below(c.y.wrapping_add(VIEW_BOTTOM), y)
    }

    /// Part of `h0_62b6`: one cell drawn by its value less [`ITEM_FIRST`], a byte the original compares
    /// by the sign of the difference alone (`cmpi.b; bpl`), so `$a0` to `$a7` are not items. Items, and
    /// values from `$a8` or under `$20`: the item frame; [`WATER`] and [`WATER_2`] their frames; the
    /// vanishing blocks and the flame their pictures. Any other value draws nothing and ends the pass's
    /// list.
    fn animate_cell(&mut self, c: AnimCell) -> Next {
        let a = self.state.animation;
        let v = c.code.wrapping_sub(ITEM_FIRST);
        let below = |imm: u8| v.wrapping_sub(imm) & 0x80 != 0;
        let frame = if below(WATER) {
            v.wrapping_add(a.tick) & 7
        } else if v == WATER {
            let step = if a.water_cycle != 0 {
                (a.tick >> 1) & 3
            } else {
                a.tick & 1
            };
            v + step
        } else if v == WATER_2 {
            v + (a.tick & 1)
        } else if below(VANISH_FIRST) {
            return Next::Return;
        } else if below(VANISH_END) {
            self.vanish(c, v - VANISH_FIRST);
            return Next::Cell;
        } else if v == FLAME {
            self.flame(c);
            return Next::Cell;
        } else {
            return Next::Return;
        };
        self.element(c, frame as u32);
        Next::Cell
    }

    /// Part of `h0_62b6`: `_ELEMENTY` frame `frame` cut over the cell's tile, which the tile map gives.
    fn element(&mut self, c: AnimCell, frame: u32) {
        let (col, line) = screen_cell(c.x, c.y);
        let row = ((c.y >> 3) & 0x1ffe) as u32;
        let tile = self.io.rw(TILE_MAP + col * MAP_COLUMN + row);
        let tile = self.io.rl(TILE_TABLE + tile as u32 * 4);
        self.io.blit(Blit::Element {
            frame,
            tile,
            dst: col + line * SCREEN_LINE,
        });
    }

    /// Part of `h0_62b6`: vanishing block `k` (0 to 3, a quarter cycle apart) shows the frame the
    /// table gives for `k * 16 + tick`, and its cell's top [`SOLID_LINES`] lines in `collision_map`
    /// are made solid, or clear on the empty frame.
    fn vanish(&mut self, c: AnimCell, k: u8) {
        let step = ((k as u32) << 4).wrapping_add(self.state.animation.tick as u32) & 0x3f;
        let offset = self.io.rl(VANISH_FRAMES + step * 4);
        let (col, line) = screen_cell(c.x, c.y);
        self.io.blit(Blit::Picture {
            src: VANISH_PICTURE.wrapping_add(offset),
            modulo: VANISH_MODULO,
            dst: col + line * SCREEN_LINE,
        });
        let v = if offset as u8 == VANISH_EMPTY {
            0
        } else {
            SOLID
        };
        for k in 0..SOLID_LINES {
            self.io.collision(col + (line + k) * COLLISION_LINE, v);
        }
    }

    /// Part of `h0_62b6`: the flame's frame for the tick over its cell and the cell below.
    fn flame(&mut self, c: AnimCell) {
        let src = FLAME_PICTURE + (self.state.animation.tick as u32 & 1) * 4;
        let (col, line) = screen_cell(c.x, c.y);
        for (k, line) in [line, (line + 0x10) & 0xf0].into_iter().enumerate() {
            self.io.blit(Blit::Picture {
                src: src + k as u32 * FLAME_HALF,
                modulo: FLAME_MODULO,
                dst: col + line * SCREEN_LINE,
            });
        }
    }
}
