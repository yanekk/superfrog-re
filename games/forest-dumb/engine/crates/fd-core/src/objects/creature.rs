//! The creatures: level 1's enemies moving in their five slots, and their sort into drawing order
//! (plans/forest-dumb-level-1 T24; docs/10-object-system.md, "Creatures, ported").
//!
//! A creature is not a [`crate::Object`]: the edge redraws (T55) start one from its object map value
//! (1 to 6) into a free slot of `CreatureSlots` as kind `value - 1`, and from then on it lives in the
//! slot. Each pass `h0_2fae` moves every active slot by its kind (`h0_2fea`): kind 0 flies at the
//! character on a 32-step path, kinds 1 to 5 walk, turning where the collision map's probes find a
//! wall or a ledge, kind 7 is a stomped creature counting out its last 32 passes. Kind 6 and any
//! other do nothing. After the write-back (T22) `h0_3698` sorts the slots by y for drawing (T52). The
//! stomp itself is `char_hit_check`'s ([`crate::character::Play::stomp`]).
//!
//! The step tables are the program's own (`h0+$30fe` and on), read through [`Ground`] as the original
//! reads them; the probes read `collision_map` at the original's offsets with no wrap at a row's end.

use crate::level::redraw::CreatureSlot;
use crate::State;

/// The bytes a creature can read besides its slot: `collision_map` and the program's step tables.
pub trait Ground {
    /// The byte at `off` from `collision_map`'s first (`h3+$fa50`), however far past it.
    fn collision(&self, off: u32) -> u8;
    /// The byte at hunk 0 offset `a`: the step tables.
    fn program(&self, a: u32) -> u8;
}

/// What moving the creatures writes that nothing reads back.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Moving {
    /// `h0+$2fe6`, a long: the slot `h0_2fea` was last called with.
    pub slot: u32,
    /// `h0+$306e`, a word: set to 1 by any pass that moves a stomped creature, never cleared.
    pub stomped: u16,
}

/// Bytes a `collision_map` row: one bit a pixel, 368 pixels.
pub const ROW: u32 = 0x2e;
/// The kind a stomped creature becomes (`char_hit_check`), counting out [`STOMPED_PASSES`].
pub const STOMPED: u16 = 7;
/// The passes a stomped creature stays: its flag is cleared once its phase reaches this.
pub const STOMPED_PASSES: u8 = 0x20;
/// A step table's index is the phase's low five bits: 32 steps.
pub const PHASE_MASK: u8 = 0x1f;
/// The `facing` byte: negative moves left, else right. The probes set these.
pub const LEFT: u8 = 0xff;
pub const RIGHT_FROM_PROBE: u8 = 0;
/// Kind 0's facing right, which it sets itself.
pub const FLY_RIGHT: u8 = 1;
/// Kind 0's flying path: signed x and y steps, hunk 0 offsets.
pub const FLY_X: u32 = 0x30fe;
pub const FLY_Y: u32 = 0x313e;
/// Kind 2's walk: unsigned x steps left and right.
pub const WALK2_LEFT: u32 = 0x323a;
pub const WALK2_RIGHT: u32 = 0x328a;
/// Kind 3's hop: unsigned x steps, signed y steps, both ways.
pub const HOP_X: u32 = 0x334c;
pub const HOP_Y: u32 = 0x336c;
/// Kind 4's walk.
pub const WALK4_LEFT: u32 = 0x33f6;
pub const WALK4_RIGHT: u32 = 0x3416;
/// Kind 5's walk.
pub const WALK5_LEFT: u32 = 0x348c;
pub const WALK5_RIGHT: u32 = 0x34dc;
/// Kind 0 flies right while the character's x is past its own plus this.
pub const FLY_AIM: u16 = 0x10;
/// The probes' rows below a creature's y: its body, then under its feet.
pub const BODY: u32 = 0x18;
pub const FEET: u32 = 0x21;
/// The probes' columns from its x: its right edge, the body, then past its feet.
pub const RIGHT_EDGE: u16 = 0x20;
pub const PAST_RIGHT: u16 = 0x21;
/// Creature slots.
pub const SLOTS: usize = 5;

/// The creature ports over the level's state and what they read.
pub struct Creatures<'a> {
    pub state: &'a mut State,
    pub ground: &'a dyn Ground,
}

impl Creatures<'_> {
    /// Slot `i`.
    fn slot(&mut self, i: usize) -> &mut CreatureSlot {
        &mut self.state.edges.creatures.slots[i]
    }

    /// The step table at `table` for slot `i`'s phase.
    fn step(&self, table: u32, i: usize) -> u8 {
        let phase = self.state.edges.creatures.slots[i].phase & PHASE_MASK;
        self.ground.program(table + phase as u32)
    }

    /// The `collision_map` byte under slot `i`'s point (`x + dx`, `y + dy`): row `(y + dy) & $ff`,
    /// column `(x + dx) >> 3` as a word, so a column past the row's end reads into later rows.
    fn ground_at(&self, i: usize, dx: u16, dy: u32) -> u8 {
        let s = self.state.edges.creatures.slots[i];
        let row = (s.y as u32).wrapping_add(dy) & 0xff;
        let col = (s.x.wrapping_add(dx) >> 3) as u32;
        self.ground.collision(row * ROW + col)
    }

    /// `h0_320a` (`h0+$320a`): kind 2's step left, slot `i`'s x less its phase's entry of
    /// [`WALK2_LEFT`].
    pub fn step_back(&mut self, i: usize) {
        let d = self.step(WALK2_LEFT, i) as u16;
        let s = self.slot(i);
        s.x = s.x.wrapping_sub(d);
    }

    /// `h0_3530` (`h0+$3530`): moving left, slot `i` turns right (facing 0, phase 0) when its body's
    /// row is solid at its x.
    pub fn wall_left(&mut self, i: usize) {
        if self.ground_at(i, 0, BODY) != 0 {
            let s = self.slot(i);
            (s.facing, s.phase) = (RIGHT_FROM_PROBE, 0);
        }
    }

    /// `h0_3222` (`h0+$3222`): kind 2's step right, slot `i`'s x plus its phase's entry of
    /// [`WALK2_RIGHT`].
    pub fn step_on(&mut self, i: usize) {
        let d = self.step(WALK2_RIGHT, i) as u16;
        let s = self.slot(i);
        s.x = s.x.wrapping_add(d);
    }

    /// `h0_32f8` (`h0+$32f8`): kind 3's hop left: x less its phase's [`HOP_X`] entry (unsigned), y
    /// plus its [`HOP_Y`] entry (signed).
    pub fn hop_back(&mut self, i: usize) {
        let dx = self.step(HOP_X, i) as u16;
        let dy = self.step(HOP_Y, i) as i8;
        let s = self.slot(i);
        s.x = s.x.wrapping_sub(dx);
        s.y = s.y.wrapping_add(dy as u16);
    }

    /// `h0_3322` (`h0+$3322`): kind 3's hop right: x plus its phase's [`HOP_X`] entry (unsigned), y
    /// plus its [`HOP_Y`] entry (signed).
    pub fn hop_on(&mut self, i: usize) {
        let dx = self.step(HOP_X, i) as u16;
        let dy = self.step(HOP_Y, i) as i8;
        let s = self.slot(i);
        s.x = s.x.wrapping_add(dx);
        s.y = s.y.wrapping_add(dy as u16);
    }

    /// `h0_3566` (`h0+$3566`): moving left, slot `i` turns right (facing 0, phase 0) when the
    /// ground under its feet one pixel left of its x is clear: a ledge.
    pub fn edge_left(&mut self, i: usize) {
        if self.ground_at(i, u16::MAX, FEET) == 0 {
            let s = self.slot(i);
            (s.facing, s.phase) = (RIGHT_FROM_PROBE, 0);
        }
    }

    /// `h0_35a0` (`h0+$35a0`): moving right, slot `i` turns left (facing [`LEFT`], phase 0) when its
    /// body's row is solid at its right edge.
    pub fn wall_right(&mut self, i: usize) {
        if self.ground_at(i, RIGHT_EDGE, BODY) != 0 {
            let s = self.slot(i);
            (s.facing, s.phase) = (LEFT, 0);
        }
    }

    /// `h0_35dc` (`h0+$35dc`): moving right, slot `i` turns left (facing [`LEFT`], phase 0) when the
    /// ground under its feet past its right edge is clear: a ledge.
    pub fn edge_right(&mut self, i: usize) {
        if self.ground_at(i, PAST_RIGHT, FEET) == 0 {
            let s = self.slot(i);
            (s.facing, s.phase) = (LEFT, 0);
        }
    }

    /// `h0_2fea` (`h0+$2fea`): one pass of slot `i` by its kind, `moving.slot` set to `i` first.
    /// Every kind but 6 counts its phase on. Kind 0 faces the character and flies a
    /// [`FLY_X`]/[`FLY_Y`] step; kinds 1 to 5 walk or hop their way, the probes turning them (kind 5
    /// probes before it steps); kind 7 counts its phase to [`STOMPED_PASSES`], then clears the flag
    /// of slot `creatures.index` (not `i`'s).
    pub fn move_slot(&mut self, i: usize) {
        self.state.moving.slot = i as u32;
        let kind = self.slot(i).kind;
        if kind == 0 {
            self.aim(i);
        }
        if !(0..=5).contains(&kind) && kind != STOMPED {
            return;
        }
        if kind == STOMPED {
            self.state.moving.stomped = 1;
        }
        let s = self.slot(i);
        s.phase = s.phase.wrapping_add(1);
        let left = s.facing & 0x80 != 0;
        match kind {
            0 => self.fly(i, left),
            1 => {
                let s = self.slot(i);
                s.x = if left {
                    s.x.wrapping_sub(1)
                } else {
                    s.x.wrapping_add(1)
                };
                self.turn(i, left, true);
            }
            2 => {
                if left {
                    self.step_back(i);
                } else {
                    self.step_on(i);
                }
                self.turn(i, left, true);
            }
            3 => {
                if left {
                    self.hop_back(i);
                } else {
                    self.hop_on(i);
                }
                self.turn(i, left, false);
            }
            4 => {
                self.walk(i, left, WALK4_LEFT, WALK4_RIGHT);
                self.turn(i, left, true);
            }
            5 => {
                self.turn(i, left, true);
                self.walk(i, left, WALK5_LEFT, WALK5_RIGHT);
            }
            _ => self.count_out(i),
        }
    }

    /// Part of `h0_2fea`: kind 0 faces left while its x plus [`FLY_AIM`] is not below the
    /// character's (`char_x`'s low word, the sign of the word difference), else right.
    fn aim(&mut self, i: usize) {
        let here = self.state.char_x as u16;
        let s = self.slot(i);
        let behind = (s.x.wrapping_add(FLY_AIM).wrapping_sub(here) as i16) >= 0;
        s.facing = if behind { LEFT } else { FLY_RIGHT };
    }

    /// Part of `h0_2fea`: kind 0's signed flight step, x either way, y down by [`FLY_Y`].
    fn fly(&mut self, i: usize, left: bool) {
        let dx = self.step(FLY_X, i) as i8 as u16;
        let dy = self.step(FLY_Y, i) as i8 as u16;
        let s = self.slot(i);
        s.x = if left {
            s.x.wrapping_sub(dx)
        } else {
            s.x.wrapping_add(dx)
        };
        s.y = s.y.wrapping_add(dy);
    }

    /// Part of `h0_2fea`: kinds 4's and 5's unsigned walk step from the table for their way.
    fn walk(&mut self, i: usize, left: bool, left_table: u32, right_table: u32) {
        let d = self.step(if left { left_table } else { right_table }, i) as u16;
        let s = self.slot(i);
        s.x = if left {
            s.x.wrapping_sub(d)
        } else {
            s.x.wrapping_add(d)
        };
    }

    /// Part of `h0_2fea`: the probes for the way slot `i` moves, the ledge first when `ledge`
    /// (all but kind 3), then the wall.
    fn turn(&mut self, i: usize, left: bool, ledge: bool) {
        match (left, ledge) {
            (true, true) => self.edge_left(i),
            (false, true) => self.edge_right(i),
            _ => {}
        }
        if left {
            self.wall_left(i);
        } else {
            self.wall_right(i);
        }
    }

    /// Part of `h0_2fea`: kind 7, its phase counted, clears the flag of slot `creatures.index` once
    /// the phase less [`STOMPED_PASSES`] is not negative as a byte. `h0_2fae` keeps the index on a
    /// slot; a wild one past the five writes nothing here.
    fn count_out(&mut self, i: usize) {
        let phase = self.slot(i).phase;
        if (phase.wrapping_sub(STOMPED_PASSES) as i8) < 0 {
            return;
        }
        let c = &mut self.state.edges.creatures;
        if let Some(f) = c.active.get_mut(c.index as usize) {
            *f = 0;
        }
    }

    /// `h0_2fae` (`h0+$2fae`): each active slot moved ([`Creatures::move_slot`]), by
    /// `creatures.index` from 0 to 4, the index left at [`SLOTS`].
    pub fn move_all(&mut self) {
        for i in 0..SLOTS {
            self.state.edges.creatures.index = i as u32;
            if self.state.edges.creatures.active[i] != 0 {
                self.move_slot(i);
            }
        }
        self.state.edges.creatures.index = SLOTS as u32;
    }

    /// `h0_3698` (`h0+$3698`): the slots and their flags sorted by y, five rounds of four
    /// compare-and-swap steps of neighbours, each swapping when the later's y less the earlier's is
    /// negative: the outer two pairs compare the words, the middle two the low bytes.
    pub fn sort(&mut self) {
        let c = &mut self.state.edges.creatures;
        let mut ys = c.slots.map(|s| s.y);
        let mut order = [0, 1, 2, 3, 4];
        for _ in 0..SLOTS {
            for j in 0..SLOTS - 1 {
                let later_lower = if j == 0 || j == SLOTS - 2 {
                    (ys[j + 1].wrapping_sub(ys[j]) as i16) < 0
                } else {
                    ((ys[j + 1] as u8).wrapping_sub(ys[j] as u8) as i8) < 0
                };
                if later_lower {
                    ys.swap(j, j + 1);
                    order.swap(j, j + 1);
                }
            }
        }
        let (active, slots) = (c.active, c.slots);
        c.active = order.map(|k| active[k]);
        c.slots = order.map(|k| slots[k]);
    }
}

/// `h0_3dd8` (`h0+$3dd8`): whether the point (`px`, `py`) lies inside slot `s`'s box, `$30` wide and
/// `$20` high from its x and y, open at the top-left: each edge is the sign of a word difference.
pub fn overlaps(s: &CreatureSlot, px: u16, py: u16) -> bool {
    let below = |a: u16, b: u16| (a.wrapping_sub(b) as i16) < 0;
    below(s.x, px)
        && !below(s.x.wrapping_add(BOX_W), px)
        && below(s.y, py)
        && !below(s.y.wrapping_add(BOX_H), py)
}

/// The stomp box's size from a slot's x and y.
pub const BOX_W: u16 = 0x30;
pub const BOX_H: u16 = 0x20;
