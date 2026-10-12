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
    b.set_w(FROG + f::Y, b.w(FROG + f::Y).wrapping_add(vy.wrapping_neg()));
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
    (b.w(FROG + f::Y) & 0xfff0).wrapping_add(0x10).wrapping_sub(h)
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
