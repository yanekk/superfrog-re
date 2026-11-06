//! What a pass does to the object map besides the edge redraws (plans/forest-dumb-level-1 T22;
//! games/forest-dumb/docs/10-object-system.md): the character takes the item, bonus or exit under it,
//! which moves the score, `items_left` and the countdown, and creatures that left the view are written
//! back into the map.
//!
//! Forest Dumb has no object list and no activation scan of its own. The edge redraws (T55,
//! `level::redraw`) are the scan: as the view crosses a cell they start creatures from the object map,
//! add items to the animated cells and blit bonuses. So collectibles are not [`crate::Object`]s with a
//! per-pass kind: they stay map cells until `char_object_cells` takes them, and the one map write a
//! pass makes besides is `h0_2dd6`'s write-back (`h0+$2e54` and its three twins).
//!
//! The ports read and write through [`Io`] at the original's hunk 0 offsets, so a cell a coordinate
//! off the level reaches is the byte the original's would be (`level::redraw` does the same).

pub mod collectible;

use crate::State;
use crate::level::LevelCtx;
use crate::level::redraw::{Blit, IMAGE_END, LevelImage, OBJECT_MAP, SHAPES};
use crate::program::Program;

/// A sound a port starts. The plan is silent, so play drops them; the routine rig compares the Paula
/// writes each one makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sound {
    /// `item_sound` (`h0+$2648`): the item sample on audio channel 2, for an item or a bonus.
    Item,
    /// `h0_26ec`'s own copy for the `$4c` bonus: the clock sample on channel 3.
    Time,
}

/// What the object ports read and write outside their typed state.
pub trait Io {
    /// The byte at hunk 0 offset `a`: the object map (`h0+$cb38`), the tile map (`h0+$16318`), the
    /// tile table (`h0+$3b988`), or whatever else a wild coordinate reaches.
    fn rb(&self, a: u32) -> u8;
    /// The byte at hunk 0 offset `a` written: an object map cell taken or a creature written back.
    fn wb(&mut self, a: u32, v: u8);
    /// A blit started.
    fn blit(&mut self, b: Blit);
    /// A sound started.
    fn sound(&mut self, s: Sound);

    fn rw(&self, a: u32) -> u16 {
        u16::from_be_bytes([self.rb(a), self.rb(a.wrapping_add(1))])
    }

    fn rl(&self, a: u32) -> u32 {
        (self.rw(a) as u32) << 16 | self.rw(a.wrapping_add(2)) as u32
    }
}

/// What the object ports keep beyond `State`'s counters: cells the original writes before it reads
/// them, kept so the routine rig sees every byte it writes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Collect {
    /// `score_add` (`h0+$2a6a`): the BCD amount `score_add_routine` adds, 0 once added.
    pub score_add: u32,
    /// `object_row` (`h0+$2692`): the row offset below `char_y` of the cell `char_object_cells` is
    /// reading, `$19` then 9.
    pub row: u32,
    /// `object_taken` (`h0+$29c6`): `h0_26ec`'s `d0` when it took a cell, the cell's value in the low
    /// byte; `h0_29ca` returns it.
    pub taken: u32,
}

/// The object ports over the level's state and its memory.
pub struct MapObjects<'a> {
    pub state: &'a mut State,
    pub io: &'a mut dyn Io,
}

/// The object ports' [`Io`] in play: the object map as play has it, the level's buffers from their
/// image, the program's tables (the tile table) from hunk 0, and the blits kept for the picture. A
/// write off the object map, which only a coordinate off the level makes, is dropped.
pub struct PlayIo<'a> {
    pub map: &'a mut [u8],
    pub image: &'a LevelImage,
    pub program: &'a Program,
    pub blits: Vec<Blit>,
}

impl PlayIo<'_> {
    fn cell(&self, a: u32) -> Option<usize> {
        let i = a.wrapping_sub(OBJECT_MAP) as usize;
        (i < self.map.len()).then_some(i)
    }
}

impl Io for PlayIo<'_> {
    fn rb(&self, a: u32) -> u8 {
        match self.cell(a) {
            Some(i) => self.map[i],
            None if (SHAPES..IMAGE_END).contains(&a) => self.image.byte(a),
            None => self.program.byte(a),
        }
    }
    fn wb(&mut self, a: u32, v: u8) {
        if let Some(i) = self.cell(a) {
            self.map[i] = v;
        }
    }
    fn blit(&mut self, b: Blit) {
        self.blits.push(b);
    }
    /// The plan is silent (DESIGN §2.1).
    fn sound(&mut self, _: Sound) {}
}

/// Run `f` on the object ports in play, then draw the blits it started into the play screen.
pub fn in_play(l: &mut LevelCtx, f: impl FnOnce(&mut MapObjects)) {
    let mut map = std::mem::take(&mut l.state.object_map);
    let mut io = PlayIo {
        map: &mut map,
        image: l.image,
        program: l.program,
        blits: Vec::new(),
    };
    f(&mut MapObjects {
        state: l.state,
        io: &mut io,
    });
    let blits = std::mem::take(&mut io.blits);
    l.state.object_map = map;
    for b in &blits {
        l.display.screen.apply(b, &|a| l.display_data.byte(a));
    }
}
