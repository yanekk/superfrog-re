//! Pickups drawn in the map's tiles (marker 55, h0+$ed8a): each world has a table of tiles and what
//! touching one does (h0+$f382, one per world): fruit for points, a coin, a checkpoint, the spud,
//! wings, speed, a slow-down, an extra life, a shield, a bottle that fills the hearts and adds two
//! minutes. A taken pickup's tile steps to the next one (the empty one) through the tile animation
//! queue (h0+$fa40), which also clears its marker so it is taken once.

use crate::frog::{FROG, f};
use crate::layout::h0;
use crate::level::pass::TILE_ANIM_QUEUE;
use crate::level::{Dispatch, LevelCtx, n_clear};

use super::SCORE;

/// h0+$f382: per world (`obj_world_offset / $100`), its table: (tile, handler) longs, ending
/// with a negative long.
const PICKUP_TABLES: u32 = h0(0xf382);
/// `obj_world_offset` (h0+$9fa).
const OBJ_WORLD_OFFSET: u32 = h0(0x9fa);
/// `level_start_x`, `level_start_y` (h0+$a1a, $a1e): where the level restarts after a death.
const START_X: u32 = h0(0xa1a);
const START_Y: u32 = h0(0xa1e);
/// `level_clock` (h0+$876): four digits, minutes first.
const LEVEL_CLOCK: u32 = h0(0x876);
/// `extra_life_shown` (h0+$dd4).
const EXTRA_LIFE_SHOWN: u32 = h0(0xdd4);
/// `opt_game_mode` (h0+$9e6): 0 the easy game.
const OPT_GAME_MODE: u32 = h0(0x9e6);
/// The frog's coins taken from the map's tiles (`+$7e`), hearts (`+$5a`), and its walking speed
/// with no speed-up (`+$b0`).
const TILE_COINS: u32 = 0x7e;
const HEARTS: u32 = 0x5a;
const BASE_SPEED: u32 = 0xb0;

pub(super) fn register(d: &mut Dispatch) {
    d.tile_pickup = Some(tile_pickup);
}

/// h0+$ed8a: the pickup the frog touched at `cell`, by its tile (h0+$f34e); a tile the world's
/// table does not list does nothing.
pub fn tile_pickup(c: &mut LevelCtx, cell: u32) {
    let b = &c.bus;
    let k = ((b.l(OBJ_WORLD_OFFSET) >> 8).wrapping_mul(4)) as u16;
    let mut at = b.l(PICKUP_TABLES.wrapping_add(k as i16 as i32 as u32));
    let tile = (b.w(cell) >> 6) as u32;
    let handler = loop {
        if b.l(at) == tile {
            break b.l(at + 4);
        }
        at += 8;
        if (b.l(at) as i32) < 0 {
            return;
        }
    };
    match handler.wrapping_sub(h0(0)) {
        0xed92 => checkpoint(c, cell),
        0xede0 => coin(c, cell),
        0xee08 => spud(c, cell),
        0xee38 => wings(c, cell),
        0xee6e => bottle(c, cell),
        0xeec6 => speed_up(c, cell),
        0xef12 => slow_down(c, cell),
        0xef40 => extra_life(c, cell),
        0xef78 => shield(c, cell),
        0xefc2 => fruit(c, cell, 1000),
        0xefd0 => fruit(c, cell, 800),
        0xefde => fruit(c, cell, 600),
        0xefec => fruit(c, cell, 400),
        0xeffa => fruit(c, cell, 50_000),
        0xf008 => fruit(c, cell, 20_000),
        0xf016 => fruit(c, cell, 2_500),
        0xf024 => fruit(c, cell, 15_000),
        0xf032 => fruit(c, cell, 2_000),
        0xf040 => fruit(c, cell, 10_000),
        0xf04e => fruit(c, cell, 3_500),
        0x7d4 => {}
        other => debug_assert!(false, "tile pickup handler ${other:x} is not ported"),
    }
}

/// h0+$fa40: `cell` is queued to show its next tile (the pickup gone) and its marker cleared;
/// false when the queue's four places are taken (nothing happens then).
pub fn take_tile(c: &mut LevelCtx, cell: u32) -> bool {
    let b = &mut c.bus;
    let mut at = TILE_ANIM_QUEUE;
    loop {
        let v = b.l(at);
        at += 4;
        if (v as i32) < 0 {
            return false;
        }
        if v == 0 {
            b.set_l(at - 4, cell);
            b.set_w(cell, b.w(cell) & 0xffc0);
            return true;
        }
    }
}

fn add_score(c: &mut LevelCtx, points: u32) {
    let b = &mut c.bus;
    b.set_l(SCORE, b.l(SCORE).wrapping_add(points));
}

/// h0+$ed92: a checkpoint: the level restarts here after a death (the start point is the
/// camera's place with the frog at it, at least 0, 0); 500 points.
fn checkpoint(c: &mut LevelCtx, cell: u32) {
    if !take_tile(c, cell) {
        return;
    }
    let b = &mut c.bus;
    let x = b.w(FROG + f::X).wrapping_sub(0x90);
    let y = b.w(FROG + f::Y).wrapping_sub(0x70);
    b.set_w(START_X + 2, if n_clear(x) { x } else { 0 });
    b.set_w(START_Y + 2, if n_clear(y) { y } else { 0 });
    add_score(c, 500);
    c.sfx(0x19, 2);
}

/// h0+$ede0: a coin drawn in the tiles: 50 points, counted in the frog's `+$7e`.
fn coin(c: &mut LevelCtx, cell: u32) {
    if !take_tile(c, cell) {
        return;
    }
    let b = &mut c.bus;
    b.set_w(FROG + TILE_COINS, b.w(FROG + TILE_COINS).wrapping_add(1));
    add_score(c, 50);
    c.sfx(5, 3);
}

/// h0+$ee08: the spud: fire throws it from now on; 1 000 points.
fn spud(c: &mut LevelCtx, cell: u32) {
    take_tile(c, cell);
    c.bus.set_w(FROG + f::SPUD_ON, 1);
    // (h0+$87c2: the panel's spud icon, an `rts` in this version.)
    c.sample(6);
    add_score(c, 1000);
}

/// h0+$ee38: wings: fire in the air starts a flight; 350 points.
fn wings(c: &mut LevelCtx, cell: u32) {
    take_tile(c, cell);
    c.bus.set_w(FROG + f::FLIGHT, 1);
    // (h0+$8850: the panel's wings icon, an `rts` in this version.)
    c.sfx(0x19, 2);
    add_score(c, 350);
}

/// h0+$ee6e: a bottle (two tiles high, both taken): the hearts full and two more minutes (the
/// minutes digit carries at ten into one more ten and one, as the original's `subi #9` has it);
/// 500 points.
fn bottle(c: &mut LevelCtx, cell: u32) {
    take_tile(c, cell);
    take_tile(c, cell.wrapping_sub(0x260));
    add_score(c, 500);
    let b = &mut c.bus;
    b.set_w(FROG + HEARTS, 4);
    let m = b.b(LEVEL_CLOCK + 1).wrapping_add(2);
    b.set_b(LEVEL_CLOCK + 1, m);
    if !((m.wrapping_sub(0xa) as i8) < 0) {
        b.set_b(LEVEL_CLOCK + 1, m.wrapping_sub(9));
        b.set_b(LEVEL_CLOCK, b.b(LEVEL_CLOCK).wrapping_add(1));
    }
    c.sample(5);
}

/// h0+$eec6: speed: `$1f4` frames of a walking speed 3 higher (at most `$c`); 500 points.
fn speed_up(c: &mut LevelCtx, cell: u32) {
    take_tile(c, cell);
    add_score(c, 500);
    let b = &mut c.bus;
    b.set_w(FROG + f::SPEED_UP, 0x1f4);
    b.set_w(
        FROG + f::MAX_SPEED,
        b.w(FROG + f::MAX_SPEED).wrapping_add(3),
    );
    c.sfx(0xc, 2);
    let b = &mut c.bus;
    if n_clear(b.w(FROG + f::MAX_SPEED).wrapping_sub(0xc)) {
        b.set_w(FROG + f::MAX_SPEED, 0xc);
    }
}

/// h0+$ef12: a slow-down: the frog stops, its speed back to its base.
fn slow_down(c: &mut LevelCtx, cell: u32) {
    take_tile(c, cell);
    let b = &mut c.bus;
    b.set_w(FROG + f::VX, 0);
    b.set_w(FROG + f::MAX_SPEED, b.w(FROG + BASE_SPEED));
    c.sample(3);
}

/// h0+$ef40: an extra life; 3 000 points.
fn extra_life(c: &mut LevelCtx, cell: u32) {
    take_tile(c, cell);
    let b = &mut c.bus;
    b.set_w(FROG + f::LIVES, b.w(FROG + f::LIVES).wrapping_add(1));
    add_score(c, 3000);
    c.bus.set_w(EXTRA_LIFE_SHOWN, 1);
    c.sfx(0x14, 1);
}

/// h0+$ef78: a shield: `$190` frames (twice that in the easy game) of no harm, blinking at the
/// end; 500 points.
fn shield(c: &mut LevelCtx, cell: u32) {
    take_tile(c, cell);
    add_score(c, 500);
    let b = &mut c.bus;
    b.set_w(FROG + f::SHIELD, 0x190);
    b.set_w(FROG + f::BLINK, 0x19);
    c.sample(7);
    let b = &mut c.bus;
    if b.w(OPT_GAME_MODE) == 0 {
        b.set_w(FROG + f::SHIELD, b.w(FROG + f::SHIELD).wrapping_shl(1));
        b.set_w(FROG + f::BLINK, b.w(FROG + f::BLINK).wrapping_shl(1));
    }
}

/// h0+$efc2..$f05c: fruit, for its points; the tile is taken after the sound.
fn fruit(c: &mut LevelCtx, cell: u32, points: u32) {
    add_score(c, points);
    c.sfx(5, 3);
    take_tile(c, cell);
}
