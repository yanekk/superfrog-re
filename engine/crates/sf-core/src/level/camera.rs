//! The camera (h0+$db92): follows the frog with a lag, keeps it inside the level, and places the
//! frog's two hardware sprites on screen. The vertical blank then scrolls to the camera
//! (`scroll_latch`, docs/12 "Scroll").

use crate::frog::{FROG, f};
use crate::layout::h0;
use crate::level::{LevelCtx, n_clear};
use crate::state::State;

/// `camera_x`, `camera_y`: longs; the camera works on their low words.
pub const CAMERA_X: u32 = h0(0x16664);
pub const CAMERA_Y: u32 = h0(0x16660);
/// `level_width`, `level_height` (h0+$a22, h0+$a24): the level's size in pixels.
pub const LEVEL_WIDTH: u32 = h0(0xa22);
pub const LEVEL_HEIGHT: u32 = h0(0xa24);
/// `scroll_dirs` (h0+$16682): which way the camera last moved, for the edge spawner and the
/// scroller: bit 1 right, bit 2 left, bit 3 down, bit 4 up.
pub const SCROLL_DIRS: u32 = h0(0x16682);
/// `shooter_mode` (h0+$9dc): the bonus level's auto-scrolling mode.
pub const SHOOTER_MODE: u32 = h0(0x9dc);
/// `final_level_flag` (h0+$1e): the final level, whose camera only keeps inside the level.
pub const FINAL_LEVEL_FLAG: u32 = h0(0x1e);
/// `sprites_hidden` (h0+$9e2): non-zero leaves the frog's sprites where they are.
pub const SPRITES_HIDDEN: u32 = h0(0x9e2);
/// `shooter_half_step` (h0+$de28): toggles every frame in shooter mode; the camera and the frog
/// creep right on every second one.
pub const SHOOTER_HALF_STEP: u32 = h0(0xde28);
/// The frog's two hardware sprites (`frog_sprite_left`, `_right`, h0+$1aff4 and h0+$1b010): screen
/// x, y first.
pub const FROG_SPRITE_LEFT: u32 = h0(0x1aff4);
pub const FROG_SPRITE_RIGHT: u32 = h0(0x1b010);

/// One axis of the camera: `target` is where the frog would have it; it moves there at once
/// unless that is more than 8 pixels from the camera's even position, in which case it closes by
/// all but 8.
fn lag(target: u16, camera: u16) -> u16 {
    let d = target.wrapping_sub(camera & 0xfffe);
    if n_clear(d.wrapping_sub(9)) {
        target.wrapping_add(d.wrapping_sub(8).wrapping_neg())
    } else if !n_clear(d.wrapping_add(8)) {
        target.wrapping_add(d.wrapping_add(8).wrapping_neg())
    } else {
        target
    }
}

/// h0+$db92.
pub fn camera_follow(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(SHOOTER_MODE) != 0 {
        return shooter(c);
    }
    if b.w(FINAL_LEVEL_FLAG) == 0 {
        // Across: the frog stays 64 pixels short of the level's right edge.
        let edge = b.w(LEVEL_WIDTH).wrapping_sub(0x40);
        if !n_clear(edge.wrapping_sub(b.w(FROG + f::X))) {
            b.set_w(FROG + f::X, edge);
            b.set_w(FROG + f::VX, 0);
        }
        let old = b.w(CAMERA_X + 2);
        let target = lag(b.w(FROG + f::X).wrapping_sub(0x90), old);
        if b.w(FROG + f::CAMERA_HELD) == 0 {
            b.set_w(CAMERA_X + 2, target);
            let moved = old.wrapping_sub(target);
            let left = if moved == 0 {
                (b.w(FROG + f::FACING) as i16) < 0
            } else {
                (moved as i16) > 0
            };
            let dirs = b.b(SCROLL_DIRS) & 0xf9 | if left { 4 } else { 2 };
            b.set_b(SCROLL_DIRS, dirs);
        }
        // Down: the frog stays 32 pixels above the level's bottom.
        let edge = b.w(LEVEL_HEIGHT).wrapping_sub(0x20);
        if !n_clear(edge.wrapping_sub(b.w(FROG + f::Y))) {
            b.set_w(FROG + f::Y, edge);
            b.set_w(FROG + f::VY, 0);
        }
        let old = b.w(CAMERA_Y + 2);
        let target = lag(b.w(FROG + f::Y).wrapping_sub(0x70), old);
        if b.w(FROG + f::CAMERA_HELD) == 0 {
            b.set_w(CAMERA_Y + 2, target);
            let moved = old.wrapping_sub(target);
            let up = if moved == 0 {
                (b.w(FROG + f::CLIMB) as i16) >= 0
            } else {
                (moved as i16) > 0
            };
            let dirs = b.b(SCROLL_DIRS) & 0xe7 | if up { 0x10 } else { 8 };
            b.set_b(SCROLL_DIRS, dirs);
        }
    }
    keep_inside(c);
    let b = &mut c.bus;
    if b.w(SPRITES_HIDDEN) == 0 {
        place_sprites(c);
    }
}

/// The camera stays inside the level: 360 pixels from its right edge, 240 from its bottom, and
/// at least 16 and 32 from its left and top (those two write the whole long).
fn keep_inside(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let right = b.w(LEVEL_WIDTH).wrapping_sub(0x168);
    if !n_clear(right.wrapping_sub(b.w(CAMERA_X + 2))) {
        b.set_w(CAMERA_X + 2, right);
    }
    if !n_clear(b.w(CAMERA_X + 2).wrapping_sub(0x10)) {
        b.set_l(CAMERA_X, 0x10);
    }
    let bottom = b.w(LEVEL_HEIGHT).wrapping_sub(0xf0);
    if !n_clear(bottom.wrapping_sub(b.w(CAMERA_Y + 2))) {
        b.set_w(CAMERA_Y + 2, bottom);
    }
    if !n_clear(b.w(CAMERA_Y + 2).wrapping_sub(0x20)) {
        b.set_l(CAMERA_Y, 0x20);
    }
}

/// The frog's sprites at its screen position: the right one 16 pixels on.
fn place_sprites(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let x = b.w(FROG + f::X).wrapping_sub(b.w(CAMERA_X + 2));
    b.set_w(FROG_SPRITE_LEFT, x);
    b.set_w(FROG_SPRITE_RIGHT, x.wrapping_add(0x10));
    let y = b.w(FROG + f::Y).wrapping_sub(b.w(CAMERA_Y + 2));
    b.set_w(FROG_SPRITE_LEFT + 2, y);
    b.set_w(FROG_SPRITE_RIGHT + 2, y);
}

/// The bonus level's camera (h0+$dd40): it creeps right a pixel every second frame, carrying the
/// frog, and the frog is kept on screen.
fn shooter(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let half = !b.w(SHOOTER_HALF_STEP);
    b.set_w(SHOOTER_HALF_STEP, half);
    if half != 0 {
        b.set_l(CAMERA_X, b.l(CAMERA_X).wrapping_add(1));
        b.set_w(FROG + f::X, b.w(FROG + f::X).wrapping_add(1));
    }
    keep_inside(c);
    let b = &mut c.bus;
    if b.w(SPRITES_HIDDEN) != 0 {
        return;
    }
    let (cx, cy) = (b.w(CAMERA_X + 2), b.w(CAMERA_Y + 2));
    let top = cy.wrapping_add(0xa);
    if n_clear(top.wrapping_sub(b.w(FROG + f::Y))) {
        b.set_w(FROG + f::Y, top);
    }
    let bottom = cy.wrapping_add(0xd8);
    if !n_clear(bottom.wrapping_sub(b.w(FROG + f::Y))) {
        b.set_w(FROG + f::Y, bottom);
    }
    let left = cx.wrapping_add(0xfff8);
    if n_clear(left.wrapping_sub(b.w(FROG + f::X))) {
        b.set_w(FROG + f::X, left);
    }
    let right = cx.wrapping_add(0x12a);
    if !n_clear(right.wrapping_sub(b.w(FROG + f::X))) {
        b.set_w(FROG + f::X, right);
    }
    place_sprites(c);
}

/// The camera's position.
pub fn camera(st: &State) -> (u32, u32) {
    let c = st.camera();
    (c.x, c.y)
}
