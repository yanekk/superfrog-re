//! The frog against the map (docs/07, docs/09 "Placement"): the ground under it, slopes, and the
//! reactions to the markers in the cells it touches (`frog_tiles`, h0+$e2e2).
//!
//! The map is read the way the original reads it: a cell's address from the boot's row table
//! (`map_rows`, h0+$17334, one long per row from one row above the map), the marker in its low six
//! bits. Markers `$3a` and up are ground; `$3e` and `$3f` the solid ground the probe climbs onto,
//! `$3d` a surface it may step up through.

use crate::frog::{FROG, f};
use crate::layout::h0;
use crate::level::{LevelCtx, n_clear};
use crate::mem::{Bus, TILE_HEIGHTS};

/// `map_rows` (h0+$17334): the row pointers (built by the boot), from the row above the map.
pub const MAP_ROWS: u32 = h0(0x17334);

/// The marker of the cell at `addr`.
pub fn marker(b: &Bus, addr: u32) -> u16 {
    b.w(addr) & 0x3f
}

/// The address the original computes for the cell under pixel (`x`, `y`), as
/// `(map_rows)[y >> 4] + (x >> 4) * 2`, with the 68000's word arithmetic.
pub fn cell_at(b: &Bus, x: u16, y: u16) -> u32 {
    let col = (x >> 4).wrapping_add(x >> 4);
    let row = (y >> 4).wrapping_mul(4);
    b.l(MAP_ROWS.wrapping_add(row as i16 as i32 as u32))
        .wrapping_add(col as u32)
}

/// The height the tile in the cell at `cell` stands at pixel column `x & 15` (`tile_heights`).
fn height(b: &Bus, cell: u32, x: u16) -> u16 {
    let tile = ((b.w(cell) & 0xffc0) >> 6) as u32;
    b.b(TILE_HEIGHTS + tile * 16 + (x & 15) as u32) as i8 as i16 as u16
}

/// h0+$dfb0: the ground probe. A frog moving down looks for ground a frame's fall ahead.
pub fn ground_probe(c: &mut LevelCtx) {
    let vy = c.bus.w(FROG + f::VY);
    if !n_clear(vy) {
        return ground(c);
    }
    let b = &mut c.bus;
    b.set_w(FROG + f::Y, b.w(FROG + f::Y).wrapping_add(vy));
    ground(c);
    let b = &mut c.bus;
    let vy = b.w(FROG + f::VY);
    b.set_w(
        FROG + f::Y,
        b.w(FROG + f::Y).wrapping_add(vy.wrapping_neg()),
    );
}

/// h0+$dfd4: lands the frog on the ground under it; a frog standing still beside a drop tries
/// the ground 7 pixels to one side (the slope probe).
fn ground(c: &mut LevelCtx) {
    let b = &mut c.bus;
    b.set_w(FROG + f::ON_GROUND, 0);
    land(c, true, false);
    let b = &mut c.bus;
    if b.w(FROG + f::ON_GROUND) != 0 || b.w(FROG + f::ON_PLATFORM) != 0 {
        b.set_w(FROG + f::SLOPE_SIDE, 0);
        return;
    }
    if b.w(FROG + f::AIR) != 0 {
        return;
    }
    let vy = b.w(FROG + f::VY);
    if n_clear(vy.wrapping_sub(2)) || !n_clear(vy) {
        return;
    }
    let side = b.w(FROG + f::SLOPE_SIDE);
    let left = side == 1 || (side != 2 && b.w(FROG + f::FACING) == 0);
    let step: u16 = if left { 7u16.wrapping_neg() } else { 7 };
    b.set_w(FROG + f::X, b.w(FROG + f::X).wrapping_add(step));
    land(c, false, true);
    let b = &mut c.bus;
    b.set_w(FROG + f::X, b.w(FROG + f::X).wrapping_sub(step));
    b.set_w(FROG + f::SLOPE_SIDE, if left { 1 } else { 2 });
}

/// h0+$e05e (`count`: the air counter goes up) and h0+$e046 (it does not): a frog four or more
/// frames into the air lands only on ground it falls onto ([`land_from_air`]); otherwise it is
/// put on the ground under it ([`settle`]). `side` is the slope probe's d7.
fn land(c: &mut LevelCtx, count: bool, side: bool) {
    let b = &mut c.bus;
    let floor = b.w(FROG + f::Y);
    let air = b.w(FROG + f::AIR);
    if air != 0 {
        let air = if count { air.wrapping_add(1) } else { air };
        if count {
            b.set_w(FROG + f::AIR, air);
        }
        if n_clear(air.wrapping_sub(4)) {
            return land_from_air(c, floor, side);
        }
    }
    settle(c, side);
}

/// The cell two rows below the one under the frog's middle, as the probes start (`+2+$4c0`).
fn probe_cell(b: &Bus) -> u32 {
    let x = b.w(FROG + f::X).wrapping_add(8);
    let y = b.w(FROG + f::Y).wrapping_sub(0x10);
    cell_at(b, x, y).wrapping_add(2).wrapping_add(0x4c0)
}

/// Where the tile in `cell` puts the frog's y: the bottom of its row, less the tile's height
/// under the frog's middle.
fn surface(b: &Bus, cell: u32) -> u16 {
    let h = height(b, cell, b.w(FROG + f::X).wrapping_add(8));
    (b.w(FROG + f::Y) & 0xfff0)
        .wrapping_add(0x10)
        .wrapping_sub(h)
}

/// h0+$e076: put the frog on the ground under its middle.
fn settle(c: &mut LevelCtx, side: bool) {
    loop {
        let b = &mut c.bus;
        let floor = b.w(FROG + f::Y);
        let mut a2 = probe_cell(b);
        if b.w(FROG + f::AIR) != 0 && marker(b, a2) >= 0x3e {
            b.set_w(FROG + f::Y, b.w(FROG + f::Y).wrapping_sub(0x10));
        }
        a2 = a2.wrapping_add(0x260);
        if marker(b, a2) >= 0x3e {
            b.set_w(FROG + f::Y, b.w(FROG + f::Y).wrapping_sub(0x10));
        } else {
            a2 = a2.wrapping_add(0x260);
            let m = marker(b, a2);
            if m < 0x3e {
                if step_up(b, a2, m) {
                    continue;
                }
                return;
            }
        }
        // h0+$e0f0: on the tile in a2.
        b.set_l(FROG + f::GROUND_CELL, a2);
        let y = surface(b, a2);
        if y > floor {
            return;
        }
        b.set_w(FROG + f::Y, y);
        if b.w(FROG + f::AIR) != 0 {
            return;
        }
        if !side {
            let (mut ahead, mut behind) = (a2.wrapping_add(2), a2.wrapping_sub(4));
            if c.hw.joy1dat & 0x303 == 0x300 || b.w(FROG + f::FACING) != 0 {
                ahead = ahead.wrapping_sub(4);
                behind = behind.wrapping_add(6);
            }
            if marker(b, ahead.wrapping_add(0x260)) >= 0x3a || marker(b, ahead) >= 0x3a {
                // h0+$e18c: at a step.
                if !n_clear(b.w(FROG + f::VY)) {
                    return;
                }
                b.set_w(FROG + f::ON_GROUND, 1);
                if marker(b, behind) >= 0x3a || marker(b, behind.wrapping_add(0x260)) >= 0x3b {
                    b.set_w(FROG + f::VY, 9);
                    return;
                }
            }
        }
        b.set_w(FROG + f::VY, 0);
        b.set_w(FROG + f::ON_GROUND, 1);
        return;
    }
}

/// h0+$e1be: under the frog is no ground (`m`, the marker found, is below `$3e`). If it is a `$3d`
/// surface between two others, the frog rises a row and the probe runs again (true).
fn step_up(b: &mut Bus, a2: u32, m: u16) -> bool {
    if marker(b, a2.wrapping_sub(2)) < 0x3d || marker(b, a2.wrapping_add(2)) < 0x3d || m != 0x3d {
        return false;
    }
    b.set_w(FROG + f::Y, b.w(FROG + f::Y).wrapping_sub(0x10));
    b.set_w(FROG + f::VY, 0);
    true
}

/// h0+$e1fc: a frog four frames into the air lands on ground it reaches this frame. `floor` is
/// its y when the probe began. On a `$3f` top it is lifted 19 pixels and probed again.
fn land_from_air(c: &mut LevelCtx, mut floor: u16, side: bool) {
    let b = &mut c.bus;
    let mut a2 = probe_cell(b);
    let lift = marker(b, a2) == 0x3f || {
        a2 = a2.wrapping_add(0x260);
        marker(b, a2) == 0x3f
    };
    if lift {
        b.set_w(FROG + f::Y, b.w(FROG + f::Y).wrapping_sub(0x13));
        return ground_probe(c);
    }
    if marker(b, a2) >= 0x3e {
        floor = floor.wrapping_add(0x10);
    } else {
        a2 = a2.wrapping_add(0x260);
        let m = marker(b, a2);
        if m < 0x3e {
            if step_up(b, a2, m) {
                settle(c, side);
            }
            return;
        }
    }
    // h0+$e274.
    b.set_l(FROG + f::GROUND_CELL, a2);
    let at = surface(b, a2);
    let vy = b.w(FROG + f::VY);
    let mut y = at.wrapping_add(vy);
    if !n_clear(vy) {
        let vx = b.w(FROG + f::VX);
        let speed = if n_clear(vx) { vx } else { vx.wrapping_neg() };
        if n_clear(speed.wrapping_sub(4)) {
            y = at;
        }
    }
    if y > floor {
        b.set_w(FROG + f::AIR, 0);
        b.set_w(FROG + f::ON_GROUND, 1);
    }
}

/// h0+$e4fc: a frog moving up stops under a ceiling: the cell above its head and, past ground
/// cells, the ones beside it in the way it faces.
pub fn head_probe(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if n_clear(b.w(FROG + f::VY)) {
        return;
    }
    let (x, y) = (b.w(FROG + f::X), b.w(FROG + f::Y));
    let mut a2 = cell_at(b, x, y.wrapping_sub(0x10))
        .wrapping_add(2)
        .wrapping_add(0x4c0);
    let step = |b: &Bus, a2: u32| {
        if b.w(FROG + f::FACING) != 0 {
            a2.wrapping_add(2)
        } else {
            a2.wrapping_sub(2)
        }
    };
    loop {
        let m = marker(b, a2.wrapping_add(0x260));
        if m >= 0x3e {
            a2 = step(b, a2);
            continue;
        }
        if m != 0x3d {
            let m = marker(b, a2);
            if m == 0x3f || m == 0x3d {
                break;
            }
        }
        // h0+$e56e: the next cell on.
        let m = marker(b, a2.wrapping_add(0x262));
        if m >= 0x3e {
            a2 = step(b, a2);
            continue;
        }
        if m == 0x3d {
            return;
        }
        let m = marker(b, a2.wrapping_add(2));
        if m != 0x3f && m != 0x3d {
            return;
        }
        break;
    }
    b.set_w(FROG + f::Y, (b.w(FROG + f::Y) & 0xfff0).wrapping_add(0x10));
    b.set_w(FROG + f::VY, 0);
    b.set_w(FROG + f::HEAD_BUMP, 1);
    b.set_w(FROG + f::AIR, 8);
}

/// `slope_anims_right`, `_left` (h0+$dee8, h0+$df6c): walking animations by the slope under the
/// frog, a long each, indexed by the slope (negative indices reach the table before).
const SLOPE_ANIMS_RIGHT: u32 = h0(0xdee8);
const SLOPE_ANIMS_LEFT: u32 = h0(0xdf6c);
/// Animations below this address face right (the frog's tables are in two halves).
pub const LEFT_ANIMS: u32 = h0(0x10b7e);
const SLOPE_FRAME_RIGHT: u32 = h0(0x1050e);
const SLOPE_FRAME_LEFT: u32 = h0(0x10ce6);

/// h0+$de2a: the slope of the ground cell's tile (its height 13 pixels in less 2 in) picks the
/// walking animation, and the frames of a step in both walking animations follow it.
pub fn slope_anim(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let tile = ((b.w(b.l(FROG + f::GROUND_CELL)) & 0xffc0) >> 6) as u32;
    let heights = TILE_HEIGHTS + tile * 16;
    let slope = b.b(heights + 0xd).wrapping_sub(b.b(heights + 2)) as i8 as i16;
    b.set_w(FROG + f::SLOPE, slope as u16);
    let mut index = (slope as i32 * 4) as u32;
    let table = if b.w(FROG + f::FACING) != 0 {
        SLOPE_ANIMS_LEFT
    } else {
        SLOPE_ANIMS_RIGHT
    };
    if b.w(FROG + f::ON_PLATFORM) != 0 {
        index = 0;
    }
    let anim = b.l(table.wrapping_add(index));
    b.set_l(FROG + f::SLOPE_ANIM, anim);
    let frame = b.l(anim);
    let other = frame.wrapping_add(0xc);
    if !n_clear_l(anim.wrapping_sub(LEFT_ANIMS)) {
        b.set_l(SLOPE_FRAME_RIGHT, frame);
        b.set_l(SLOPE_FRAME_LEFT, other);
    } else {
        b.set_l(SLOPE_FRAME_LEFT, frame);
        b.set_l(SLOPE_FRAME_RIGHT, other);
    }
}

/// [`n_clear`] for a long result.
pub fn n_clear_l(result: u32) -> bool {
    (result as i32) >= 0
}

/// `frog_tiles`'s probe lists (h0+$f07a facing left, h0+$f086 facing right): three longs, the
/// steps from the cell before the frog's feet to the cells beside it it tests for walls.
const PROBE_LEFT: u32 = h0(0xf07a);
const PROBE_RIGHT: u32 = h0(0xf086);
/// `level_state` (h0+$a06): the exit sets 1.
pub const LEVEL_STATE: u32 = h0(0xa06);
/// `passage_cell` (h0+$776c): the fake-wall cell the frog entered (marker 38).
pub const PASSAGE_CELL: u32 = h0(0x776c);
/// `ice_tick`, `push_tick` (h0+$eb28, h0+$ebf0): frames on a slippery or a pushing surface.
const ICE_TICK: u32 = h0(0xeb28);
const PUSH_TICK: u32 = h0(0xebf0);
/// `no_death` (h0+$14): non-zero, hazards do nothing (a cheat, inferred).
pub const NO_DEATH: u32 = h0(0x14);
/// `obj_world_offset` (h0+$9fa): the world's row of the object tables (`$100` per world).
pub const OBJ_WORLD_OFFSET: u32 = h0(0x9fa);
/// `obj_templates` (h0+$1c6e2): the object templates, `$ae` bytes each.
pub const OBJ_TEMPLATES: u32 = h0(0x1c6e2);

/// What a marker handler does to the cell; which table the marker indexes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Table {
    /// h0+$f092 / h0+$f192: walls at the frog's side (markers 61 and 63).
    WallRight,
    WallLeft,
    /// h0+$e5ba, h0+$e6ba, h0+$e7ba: the cells at the frog's head, middle and feet.
    Head,
    Middle,
    Feet,
}

/// `frog_tiles` (h0+$e2e2): gravity, the ground and ceiling probes, walls beside the frog, and the
/// markers in the three cells down its middle.
pub fn frog_tiles(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(crate::level::camera::SHOOTER_MODE) != 0 {
        return;
    }
    b.set_w(FROG + f::HEAD_BUMP, 0);
    // Gravity: vy grows by one every third frame on the ground, every `GRAVITY_DELAY + 1`th in
    // the air, up to 8.
    let tick = b.w(FROG + f::GRAVITY_TICK).wrapping_add(1);
    b.set_w(FROG + f::GRAVITY_TICK, tick);
    if n_clear(tick.wrapping_sub(3))
        && (b.w(FROG + f::ON_GROUND) != 0
            || !n_clear(b.w(FROG + f::GRAVITY_DELAY).wrapping_sub(tick)))
    {
        b.set_w(FROG + f::GRAVITY_TICK, 0);
        if !n_clear(b.w(FROG + f::VY).wrapping_sub(8)) {
            b.set_w(FROG + f::VY, b.w(FROG + f::VY).wrapping_add(1));
            crate::frog::anim::frog_jump_float(c);
        }
    }
    let b = &mut c.bus;
    if b.w(FROG + f::SPRING) == 0 {
        head_probe(c);
        ground_probe(c);
        slope_anim(c);
    }
    let b = &mut c.bus;
    if b.w(FROG + f::SLIDING) == 0 {
        b.set_w(FROG + f::SLIDE_TICK, 0);
    }
    b.set_w(FROG + f::SLIDING, 0);
    b.set_w(FROG + f::PUSHED, 0);
    let air = b.w(FROG + f::AIR);
    if b.w(FROG + f::ON_GROUND) == 0 && b.w(FROG + f::ON_PLATFORM) == 0 {
        b.set_w(FROG + f::AIR, 0x10);
    }
    if b.w(FROG + f::SPRING) == 0 {
        walls(c);
    }
    let b = &mut c.bus;
    if b.w(FROG + f::DYING) == 0 {
        b.set_w(FROG + f::SPRING, 0);
    }
    let x = b.w(FROG + f::X).wrapping_add(8);
    let mut y = b.w(FROG + f::Y);
    let vy = b.w(FROG + f::VY);
    if n_clear(vy) {
        y = y.wrapping_add(vy);
    }
    let a2 = cell_at(b, x, y).wrapping_add(0x262);
    for (k, table) in [Table::Head, Table::Middle, Table::Feet]
        .into_iter()
        .enumerate()
    {
        let at = a2.wrapping_add(0x260 * k as u32);
        let m = marker(&c.bus, at);
        handle(c, table, m, at, false);
    }
    c.bus.set_w(FROG + f::AIR, air);
}

/// h0+$e36a..$e466: the cells beside the frog's feet, by the probe list for the way it faces.
fn walls(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let (x, mut y) = (b.w(FROG + f::X), b.w(FROG + f::Y));
    let vy = b.w(FROG + f::VY);
    if n_clear(vy) {
        y = y.wrapping_add(vy);
    }
    let mut a2 = cell_at(b, x, y.wrapping_sub(0x10))
        .wrapping_add(2)
        .wrapping_add(0x260);
    let facing = b.w(FROG + f::FACING) != 0;
    if !facing || marker(b, a2.wrapping_add(0x4c2)) == 0x3d {
        if facing {
            b.set_w(FROG + f::X, b.w(FROG + f::X) & 0xfff0);
            b.set_w(FROG + f::WALL_RIGHT, 1);
            b.set_w(FROG + f::VX, 0);
        }
        if marker(b, a2.wrapping_add(0x4c0)) == 0x3d {
            b.set_w(FROG + f::X, (b.w(FROG + f::X) & 0xfff0).wrapping_add(0xf));
            b.set_w(FROG + f::WALL_LEFT, 1);
            b.set_w(FROG + f::VX, 0);
        }
    }
    // h0+$e3f8: the side the frog faces is tested unless a wall already stopped it there, then the
    // other. (With both wall flags set the original spins here for ever; the side it faces is
    // taken instead, DESIGN §2.11.)
    let (stopped_r, stopped_l) = (
        b.w(FROG + f::WALL_RIGHT) != 0,
        b.w(FROG + f::WALL_LEFT) != 0,
    );
    let left = if facing {
        !stopped_r || stopped_l
    } else {
        stopped_l && !stopped_r
    };
    let (list, table) = if left {
        (PROBE_LEFT, Table::WallLeft)
    } else {
        (PROBE_RIGHT, Table::WallRight)
    };
    b.set_l(FROG + f::WALL_RIGHT, 0);
    a2 = a2.wrapping_add(b.l(list));
    let m = marker(b, a2);
    handle(c, table, m, a2, false);
    let b = &mut c.bus;
    if b.w(FROG + f::ON_PLATFORM) != 0 || b.w(FROG + f::AIR) == 0 {
        return;
    }
    a2 = a2.wrapping_add(b.l(list + 4));
    let m = marker(b, a2);
    handle(c, table, m, a2, false);
    let b = &mut c.bus;
    a2 = a2.wrapping_add(b.l(list + 8));
    let m = marker(b, a2);
    handle(c, table, m, a2, true);
}

/// One marker's handler from `table` (the original's `jsr (a3,d0.w)`), with the cell's address
/// (`a2`) and d7.
fn handle(c: &mut LevelCtx, table: Table, m: u16, a2: u32, d7: bool) {
    use Table::*;
    match (table, m) {
        (WallRight, 61 | 63) => wall(c, a2, d7, false),
        (WallLeft, 61 | 63) => wall(c, a2, d7, true),
        (WallRight | WallLeft, _) => {}
        (Middle, 44) => {
            let b = &mut c.bus;
            if b.w(FROG + f::HURT) == 0 && b.w(FROG + f::AIR) == 0 {
                b.set_w(FROG + f::Y, b.w(FROG + f::Y).wrapping_sub(0x10));
            }
        }
        (Head, 7) => launcher(c, a2),
        (Feet, 4) => bounce_pad(c, a2, 4),
        (Feet, 5) => bounce_pad(c, a2, 5),
        (Middle, 38) => passage(c, a2),
        (_, 56) => ice(c),
        (_, 46) => conveyor(c, -4),
        (_, 47) => conveyor(c, 4),
        (_, 57) => push(c),
        (_, 48..=51) => {
            // h0+$f6be: a switch: its gate opens (`crate::items::fixtures::switch`).
            if let Some(bump) = c.dispatch.bump_block {
                bump(c, a2);
            }
        }
        (Middle, 54) => {
            let b = &mut c.bus;
            if b.w(FROG + f::COINS_LEFT) == 0 {
                b.set_l(LEVEL_STATE, 1);
                c.sfx(0x17, 3);
            }
        }
        (_, 6) => {
            let b = &mut c.bus;
            if b.w(FROG + f::DYING) == 0 {
                b.set_w(FROG + f::HURT, 0);
                b.set_w(FROG + f::BLINK, 0);
                b.set_w(FROG + f::SHIELD, 0);
                frog_kill(c);
            }
        }
        (_, 39..=43) => spring(c, m),
        (_, 55) => {
            // h0+$ed8a: a pickup drawn in the tiles, by tile (`crate::items::pickups`).
            if let Some(pickup) = c.dispatch.tile_pickup {
                pickup(c, a2);
            }
        }
        _ => {}
    }
}

/// h0+$f294 / h0+$f2ee: a wall at the frog's side stops it at the cell's edge; at the foot probe
/// (d7) only when no ground stands in the three cells beyond.
fn wall(c: &mut LevelCtx, a2: u32, d7: bool, left: bool) {
    let b = &mut c.bus;
    if d7 {
        let beyond = if left {
            a2.wrapping_add(2)
        } else {
            a2.wrapping_sub(2)
        };
        for k in 0..3u32 {
            let at = beyond.wrapping_sub(0x260).wrapping_add(0x260 * k);
            if marker(b, at) >= 0x3e {
                return;
            }
        }
    }
    let x = b.w(FROG + f::X) & 0xfff0;
    if left {
        b.set_w(FROG + f::X, x.wrapping_add(0xf));
        b.set_w(FROG + f::WALL_LEFT, 1);
    } else {
        b.set_w(FROG + f::X, x);
        b.set_w(FROG + f::WALL_RIGHT, 1);
    }
    b.set_w(FROG + f::VX, 0);
}

/// h0+$ecc8..$ed62: springs (markers 39–43) throw the frog up, right, down or left at 9; all but
/// the first carry it (`SPRING`), its sprites lowered out of view.
fn spring(c: &mut LevelCtx, m: u16) {
    let b = &mut c.bus;
    if b.w(FROG + f::HURT) != 0 {
        return;
    }
    b.set_w(FROG + f::LAUNCHED, 0x18);
    if m != 39 {
        b.set_w(FROG + f::SPRING, 1);
    }
    let (vx, vy, snap_x): (u16, u16, bool) = match m {
        39 | 40 => (0, 0xfff7, true),
        41 => (9, 0, false),
        42 => (0, 9, true),
        _ => (0xfff7, 0, false),
    };
    b.set_w(FROG + f::VX, vx);
    b.set_w(FROG + f::VY, vy);
    if snap_x {
        b.set_w(FROG + f::X, b.w(FROG + f::X) & 0xfff8);
    } else {
        b.set_w(FROG + f::Y, b.w(FROG + f::Y) & 0xfff8);
    }
}

/// h0+$e8ce: a launcher (marker 7) throws the frog away from the wall beside it, 20 a frame (4
/// less from a standing start on `$3f` ground), and spawns its spring object.
fn launcher(c: &mut LevelCtx, mut a2: u32) {
    let b = &mut c.bus;
    if b.w(FROG + f::HURT) != 0 {
        return;
    }
    let vx = b.w(FROG + f::VX);
    let slow = marker(b, a2.wrapping_add(0x4c0)) == 0x3f
        && !n_clear(vx.wrapping_sub(6))
        && n_clear(vx.wrapping_add(5));
    let d7: u16 = if slow { 4 } else { 0 };
    b.set_w(FROG + f::LAUNCHED, 0x1e);
    b.set_w(FROG + f::LAUNCH_HOLD, 0x1e);
    b.set_w(FROG + f::AIR, 0x30);
    b.set_w(FROG + f::GRAVITY_TICK, 1);
    b.set_w(FROG + f::X, b.w(FROG + f::X) & 0xfff0);
    if marker(b, a2.wrapping_sub(0x260)) == 7 {
        a2 = a2.wrapping_sub(0x260);
    }
    let left = marker(b, a2.wrapping_sub(4)) < 0x3a;
    if left {
        b.set_w(FROG + f::VX, 0xffecu16.wrapping_add(d7));
    } else {
        b.set_w(FROG + f::X, b.w(FROG + f::X).wrapping_add(0x10));
        b.set_w(FROG + f::VX, 0x14u16.wrapping_add(d7.wrapping_neg()));
    }
    b.set_w(FROG + f::VY, 0xfffb);
    if let Some(spring) = c.dispatch.launcher {
        spring(c, a2, left);
    }
    crate::frog::anim::place_frog_sprites(c);
}

/// h0+$e9cc / h0+$ea22: a bounce pad (markers 4 and 5) under a falling frog bounces it
/// (h0+$11626) and spawns the pad's object, which throws it up the next pass.
fn bounce_pad(c: &mut LevelCtx, a2: u32, m: u16) {
    let b = &mut c.bus;
    if !n_clear(b.w(FROG + f::VY).wrapping_sub(3)) || b.w(FROG + f::HURT) != 0 {
        return;
    }
    bounce(c);
    if c.bus.w(FROG + f::AIR) == 0 {
        return;
    }
    if let Some(pad) = c.dispatch.bounce_pad {
        pad(c, a2, m);
    }
    crate::frog::anim::place_frog_sprites(c);
}

/// h0+$11626: the frog bounces off: up at its bounce speed.
pub fn bounce(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(FROG + f::AIR) != 0 || b.w(FROG + f::HEAD_BUMP) != 0 {
        return;
    }
    if b.w(FROG + f::ON_PLATFORM) == 0 && b.w(FROG + f::ON_GROUND) == 0 {
        return;
    }
    b.set_w(FROG + f::AIR, 1);
    b.set_w(FROG + f::GRAVITY_TICK, 3);
    b.set_w(FROG + f::VY, b.w(FROG + f::BOUNCE_VY));
    b.set_w(FROG + f::HEAD_BUMP, 1);
}

/// h0+$ea78: a fake wall (marker 38, docs/07): the frog stops and the passage sequence starts
/// (the pass runs it, h0+$7772).
fn passage(c: &mut LevelCtx, a2: u32) {
    let b = &mut c.bus;
    if b.w(FROG + f::HURT) != 0 || b.w(FROG + f::IN_PASSAGE) != 0 {
        return;
    }
    b.set_w(FROG + f::FROZEN, 1);
    b.set_w(FROG + f::IN_PASSAGE, 1);
    b.set_l(PASSAGE_CELL, a2);
    c.sfx(0x15, 2);
}

/// h0+$eaae: ice (marker 56): every third frame on it the slope speeds the frog downhill, up to 7.
fn ice(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(FROG + f::ON_GROUND) == 0 {
        return;
    }
    if !n_clear(b.w(ICE_TICK).wrapping_sub(4)) {
        b.set_w(ICE_TICK, b.w(ICE_TICK).wrapping_add(1));
    }
    let slope = b.w(FROG + f::SLOPE);
    if n_clear(b.w(ICE_TICK).wrapping_sub(3)) && !n_clear(slope.wrapping_add(1)) {
        b.set_w(ICE_TICK, 0);
        if !n_clear(b.w(FROG + f::VX).wrapping_sub(7)) {
            b.set_w(FROG + f::VX, b.w(FROG + f::VX).wrapping_add(1));
        }
    }
    if n_clear(b.w(ICE_TICK).wrapping_sub(3)) && n_clear(b.w(FROG + f::SLOPE).wrapping_sub(1)) {
        b.set_w(ICE_TICK, 0);
        if n_clear(b.w(FROG + f::VX).wrapping_add(6)) {
            b.set_w(FROG + f::VX, b.w(FROG + f::VX).wrapping_sub(1));
        }
    }
    b.set_w(FROG + f::SLIDING, 1);
    let t = b.w(FROG + f::SLIDE_TICK).wrapping_add(1);
    b.set_w(FROG + f::SLIDE_TICK, t);
    if n_clear(t.wrapping_sub(4)) {
        b.set_w(FROG + f::SLIDE_TICK, 0);
    }
}

/// h0+$eb2a (left, marker 46) / h0+$eb48 (right, 47): a conveyor sets the frog's speed to at
/// least 4 its way, unless it stands against a wall that way; then it slides as on a push surface.
fn conveyor(c: &mut LevelCtx, speed: i16) {
    let b = &mut c.bus;
    if b.w(FROG + f::ON_GROUND) == 0 {
        return;
    }
    let speed = speed as u16;
    let (wall, slower) = if (speed as i16) < 0 {
        (
            f::WALL_LEFT,
            !n_clear(b.w(FROG + f::VX).wrapping_sub(speed)),
        )
    } else {
        (
            f::WALL_RIGHT,
            !n_clear(b.w(FROG + f::VX).wrapping_sub(speed)),
        )
    };
    if b.w(FROG + wall) == 0 && slower {
        b.set_w(FROG + f::VX, speed);
    }
    push(c);
}

/// h0+$eb64: a push surface (marker 57): every third frame the slope speeds the frog by 2 up to
/// 8 either way.
fn push(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(FROG + f::ON_GROUND) == 0 {
        return;
    }
    if !n_clear(b.w(PUSH_TICK).wrapping_sub(8)) {
        b.set_w(PUSH_TICK, b.w(PUSH_TICK).wrapping_add(1));
    }
    if n_clear(b.w(PUSH_TICK).wrapping_sub(3)) && !n_clear(b.w(FROG + f::SLOPE).wrapping_add(1)) {
        b.set_w(PUSH_TICK, 0);
        let vx = b.w(FROG + f::VX).wrapping_add(2);
        b.set_w(FROG + f::VX, vx);
        if n_clear(vx.wrapping_sub(8)) {
            b.set_w(FROG + f::VX, 8);
        }
    }
    if n_clear(b.w(PUSH_TICK).wrapping_sub(3)) && n_clear(b.w(FROG + f::SLOPE).wrapping_sub(1)) {
        b.set_w(PUSH_TICK, 0);
        let vx = b.w(FROG + f::VX).wrapping_sub(2);
        b.set_w(FROG + f::VX, vx);
        if !n_clear(vx.wrapping_add(8)) {
            b.set_w(FROG + f::VX, 0xfff8);
        }
    }
    b.set_w(FROG + f::PUSHED, 1);
    b.set_w(FROG + f::SLIDING, 1);
    let t = b.w(FROG + f::SLIDE_TICK).wrapping_add(1);
    b.set_w(FROG + f::SLIDE_TICK, t);
    if n_clear(t.wrapping_sub(2)) {
        b.set_w(FROG + f::SLIDE_TICK, 0);
    }
}

/// `frog_kill` (h0+$1c482): a hazard (marker 6, or any hit in the bonus level) costs a life: the
/// frog starts dying, the death sample plays, and a frog standing on the ground at the edge it
/// faces is stopped and lifted.
pub fn frog_kill(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.l(crate::level::ATTRACT_MODE) != 0 || b.w(NO_DEATH) != 0 {
        return;
    }
    if b.w(FROG + f::BLINK) != 0 || b.w(FROG + f::SHIELD) != 0 {
        return;
    }
    if b.w(crate::level::camera::SHOOTER_MODE) != 0 {
        b.set_w(FROG + f::HURT, 0);
    }
    if b.w(FROG + f::DYING) != 0 || b.w(FROG + f::HURT) != 0 {
        return;
    }
    b.set_w(FROG + f::DYING, 1);
    b.set_w(FROG + f::HURT, 1);
    b.set_w(FROG + f::LIVES, b.w(FROG + f::LIVES).wrapping_sub(1));
    crate::frog::frog_life_reset(c);
    c.sample(0);
    let b = &mut c.bus;
    b.set_w(FROG + f::ON_GROUND, 0);
    let right = !n_clear_l(b.l(FROG + f::ANIM).wrapping_sub(LEFT_ANIMS));
    if b.l(FROG + f::WALL_RIGHT) == 0 {
        let cell = b.l(FROG + f::GROUND_CELL);
        let (near, far) = if right {
            (0x4be, 0x4bc)
        } else {
            (0x4c2, 0x4c4)
        };
        if marker(b, cell.wrapping_sub(near)) >= 0x3a || marker(b, cell.wrapping_sub(far)) >= 0x3a {
            return;
        }
        b.set_w(FROG + f::VX, 0);
        b.set_w(FROG + f::VY, 0);
        if b.l(crate::layout::h0(0x40da)) != 7 {
            b.set_w(FROG + f::VY, 0xfffc);
        }
    }
    b.set_w(FROG + f::AIR, 1);
}
