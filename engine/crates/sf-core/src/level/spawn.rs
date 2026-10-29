//! Objects come into play from the map's markers (docs/09 "Placement"): the cells scrolling into
//! view are scanned each vertical blank (`map_spawn_scan_edges`, h0+$17810), and each marker
//! `1..$39` runs its world's spawner from `obj_flag_jumptab` (h0+$178ec). The spawners are the
//! object kinds' (T15–T18): [`crate::level::Dispatch`] holds those registered, and an
//! unregistered marker spawns nothing.

use crate::layout::h0;
use crate::level::collision::{MAP_ROWS, OBJ_WORLD_OFFSET};
use crate::level::beam::Cost;
use crate::level::{LevelCtx, n_clear};
use crate::level::screen::Screen;
use crate::mem::Bus;
use std::ops::Range;

const CAMERA_X: u32 = h0(0x16664);
const CAMERA_Y: u32 = h0(0x16660);

/// h0+$177da: the cell 32 pixels left of and above the camera (0 at the level's edge); in
/// widescreen 5 cells further left ([`Screen::scan_left`], DESIGN §2.2).
fn scan_origin(b: &Bus, screen: Screen) -> u32 {
    let clamp = |v: u32| if (v as i32) < 0 { 0 } else { v };
    let x = clamp(b.l(CAMERA_X).wrapping_sub(screen.scan_left() as u32));
    let y = clamp(b.l(CAMERA_Y).wrapping_sub(0x20));
    let col = x & 0xffff_0000 | ((x as u16 >> 4).wrapping_mul(2)) as u32;
    let row = (y as u16 >> 4).wrapping_mul(4);
    b.l(MAP_ROWS.wrapping_add(row as i16 as i32 as u32))
        .wrapping_add(col)
}

/// The cells of a row scan from the origin that are the original's (`n` of them from where its
/// own origin is): the rest are widescreen's and cost no beam time (DESIGN §2.4, D11). The two
/// origins differ by fewer than 5 cells at the level's left edge, where both clamp.
fn originals(c: &LevelCtx, n: u32) -> Range<u32> {
    let skip = scan_origin(&c.bus, Screen::NARROW).wrapping_sub(scan_origin(&c.bus, c.screen)) / 2;
    skip..skip + n
}

/// Run the spawner of each marker `1..$39` in `n` cells from `at`, `step` bytes apart; the cells
/// outside `paid` are widescreen's and cost nothing.
fn scan(c: &mut LevelCtx, mut at: u32, n: u32, step: u32, paid: Range<u32>) {
    c.spend(Cost::Scan);
    for k in 0..n {
        if paid.contains(&k) {
            scan_cell(c, at);
        } else {
            c.unpaid(|c| scan_cell(c, at));
        }
        at = at.wrapping_add(step);
    }
}

fn scan_cell(c: &mut LevelCtx, at: u32) {
    c.spend(Cost::ScanCell);
    let m = c.bus.w(at) & 0x3f;
    if m != 0 && !n_clear(m.wrapping_sub(0x3a)) {
        c.spend(Cost::Spawner);
        let world = (c.bus.l(OBJ_WORLD_OFFSET) >> 8) as u8;
        if let Some(spawn) = c.dispatch.marker(world, m as u8) {
            spawn(c, at);
        }
    }
}

/// `map_spawn_scan_edges` (h0+$17810): the column just outside the screen on the side the
/// camera last moved to (21 cells), then the row just outside it above or below (25 cells; 35
/// in widescreen, whose column is 5 cells further out, DESIGN §2.2).
pub fn scan_edges(c: &mut LevelCtx) {
    let b = &c.bus;
    if b.w(crate::level::camera::SHOOTER_MODE) != 0 {
        return;
    }
    let dirs = b.b(crate::level::camera::SCROLL_DIRS);
    let origin = scan_origin(b, c.screen);
    let col = if dirs & 4 != 0 { origin } else { origin.wrapping_add(c.screen.scan_right_column()) };
    scan(c, col, 0x15, 0x260, 0..0x15);
    let b = &c.bus;
    let dirs = b.b(crate::level::camera::SCROLL_DIRS);
    let origin = scan_origin(b, c.screen);
    let row = if dirs & 0x10 != 0 { origin } else { origin.wrapping_add(0x2d20) };
    let paid = originals(c, Screen::NARROW.scan_row_cells());
    scan(c, row, c.screen.scan_row_cells(), 2, paid);
}

/// h0+$1788a: the bonus level's right-hand column, 16 cells (5 cells further out in widescreen,
/// so its markers 11 and 55 act sooner, DESIGN §2.6, D12).
pub fn scan_right_column(c: &mut LevelCtx) {
    let at = scan_origin(&c.bus, c.screen).wrapping_add(0x720).wrapping_add(c.screen.scan_right_column());
    scan(c, at, 0x10, 0x260, 0..0x10);
}

/// `map_spawn_scan_screen` (h0+$1777c, from the level's start point): every marker's spawner in
/// the 18 rows of 24 cells (34 in widescreen) from 32 pixels left of and above the camera (the
/// scan stops at the map's end).
pub fn scan_screen(c: &mut LevelCtx) {
    let start = scan_origin(&c.bus, c.screen).wrapping_add(2);
    let end = crate::mem::TILE_PTRS;
    let world = (c.bus.l(OBJ_WORLD_OFFSET) >> 8) as u8;
    let (cells, paid) = (c.screen.scan_screen_cells(), originals(c, Screen::NARROW.scan_screen_cells()));
    for row in 0..0x12u32 {
        let mut at = start.wrapping_add(row * 0x260);
        for k in 0..cells {
            let cell = |c: &mut LevelCtx| {
                c.spend(Cost::ScreenScanCell);
                let m = c.bus.w(at) & 0x3f;
                if let Some(spawn) = c.dispatch.marker(world, m as u8) {
                    c.spend(Cost::ScreenSpawner);
                    spawn(c, at);
                }
            };
            if paid.contains(&k) {
                cell(c);
            } else {
                c.unpaid(cell);
            }
            at = at.wrapping_add(2);
            if (at.wrapping_sub(end) as i32) >= 0 {
                return;
            }
        }
    }
}
