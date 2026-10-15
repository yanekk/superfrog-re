//! Template 134 (the final level's marker 1): the witch. She rises into the arena from below the
//! screen, then flies two patterns in turn, both read from the code as paths relative to the
//! camera: a bob up and down on the spot ([`witch_bob`], `+$68` passes, at least 100) and a loop
//! round the arena ([`witch_loop`]). Touching her hurts the frog; she takes 128 spud hits
//! ([`touch_boss_hit`]), then flashes the palette white and ends the level (`level_state` `$64`).

use crate::enemies::common::{NOTHING, obj_path_y};
use crate::layout::h0;
use crate::level::camera::{CAMERA_X, CAMERA_Y};
use crate::level::collision::LEVEL_STATE;
use crate::level::fade::{FADES, stepped_start};
use crate::level::objects::{bob_follow, nothing, obj_despawn_if_offscreen, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind, n_clear};
use crate::render::LEVEL_COPPER;

/// `final_boss_init` (h0+$259ae), `final_boss_update` (h0+$25a16).
pub const WITCH_INIT: u32 = h0(0x259ae);
pub const WITCH_UPDATE: u32 = h0(0x25a16);
/// `witch_rise_wait` (h0+$259fc): `+$6c` passes before she rises.
pub const WITCH_WAIT: u32 = h0(0x259fc);
/// `witch_bob` (h0+$25a7c) and `witch_loop` (h0+$25b40): her two patterns.
pub const WITCH_BOB: u32 = h0(0x25a7c);
pub const WITCH_LOOP: u32 = h0(0x25b40);
/// `witch_beaten` (h0+$259ec): beaten, she stays where she is.
pub const WITCH_BEATEN: u32 = h0(0x259ec);
/// `touch_boss_hit` (h0+$236b4).
pub const TOUCH_BOSS_HIT: u32 = h0(0x236b4);
/// `witch_bob_path` (h0+$25ade): words, every other one a y below the camera's top plus `$20`,
/// ending at `$ffff`. `witch_loop_path` (h0+$25bba): (x, y) word pairs from the camera's top
/// left (y plus `$20`), ending at a `$ffffffff` long.
pub const WITCH_BOB_PATH: u32 = h0(0x25ade);
pub const WITCH_LOOP_PATH: u32 = h0(0x25bba);
/// `witch_flash` (h0+$23734): the 32 colours (all white) the palette flashes from as she dies.
pub const WITCH_FLASH: u32 = h0(0x23734);
/// `level_palette_ptr` (h0+$9fe): the level's palette.
const LEVEL_PALETTE_PTR: u32 = h0(0x9fe);

/// Her looks: flying (h0+$32e4c) and beaten (h0+$32e60).
const LOOK: u32 = h0(0x32e4c);
const LOOK_BEATEN: u32 = h0(0x32e60);

/// Template 134: the witch.
pub static WITCH: ObjectKind = ObjectKind {
    name: "witch",
    routines: &[
        (WITCH_INIT, witch_init),
        (WITCH_UPDATE, witch_update),
        (WITCH_WAIT, witch_wait),
        (WITCH_BOB, witch_bob),
        (WITCH_LOOP, witch_loop),
        (WITCH_BEATEN, witch_beaten),
        (TOUCH_BOSS_HIT, touch_boss_hit),
        (NOTHING, nothing),
    ],
};

/// Marker 1 (h0+$18aa8): the witch, keyed to the cell 8 rows above her marker's.
pub fn spawn_134(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, crate::items::template(134), cell.wrapping_sub(0x1300), 0, 0);
}

fn camera_y(c: &LevelCtx) -> u16 {
    c.bus.w(CAMERA_Y + 2)
}

/// `final_boss_init` (h0+$259ae): her look; `$160` below the camera's top, rising 21 a pass,
/// `$18` left of her cell; `$32` passes' wait ([`witch_wait`]).
fn witch_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let y = camera_y(c).wrapping_add(0x160);
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    b.set_l(bob + 0x28, LOOK);
    b.set_l(bob + 0x42, LOOK);
    b.set_w(slot + 0xa, y);
    b.set_w(slot + 0xe, 0xffeb);
    b.set_w(slot + 8, b.w(slot + 8).wrapping_sub(0x18));
    b.set_w(slot + 0x6c, 0x32);
    b.set_l(slot + 4, WITCH_WAIT);
}

/// h0+$259fc: once `+$6c` has counted below 0 she rises ([`witch_update`]). Never despawned.
fn witch_wait(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let t = b.w(slot + 0x6c).wrapping_sub(1);
    b.set_w(slot + 0x6c, t);
    if !n_clear(t) {
        b.set_l(slot + 4, WITCH_UPDATE);
    }
    bob_follow(b, slot);
}

/// The end of each of her updates: the bob follows, and she goes if the camera has left her.
fn follow_and_despawn(c: &mut LevelCtx, slot: u32) {
    bob_follow(&mut c.bus, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// Her bob pattern from its start, for `$64` plus the RNG's 0..=`$64` passes.
fn start_bob(c: &mut LevelCtx, slot: u32) {
    let b = &mut c.bus;
    b.set_l(slot + 0x58, WITCH_BOB_PATH);
    b.set_l(slot + 0x54, WITCH_BOB_PATH);
    b.set_l(slot + 4, WITCH_BOB);
    let n = c.rng_range(0x64).wrapping_add(0x64);
    c.bus.set_w(slot + 0x68, n);
}

/// `final_boss_update` (h0+$25a16): she rises, slowing to 2 a pass; once no lower than `$70`
/// below the camera's top she starts bobbing.
fn witch_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let vy = b.w(slot + 0xe);
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(vy));
    if !n_clear(vy.wrapping_sub(0xfffe)) {
        b.set_w(slot + 0xe, vy.wrapping_add(1));
    }
    let top = camera_y(c).wrapping_add(0x70);
    if n_clear(top.wrapping_sub(c.bus.w(slot + 0xa))) {
        start_bob(c, slot);
    }
    follow_and_despawn(c, slot);
}

/// h0+$25a7c: she bobs along her path; once it is at its end with `+$68` run out, she loops.
fn witch_bob(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let top = camera_y(c);
    let b = &mut c.bus;
    let left = b.w(slot + 0x68).wrapping_sub(1);
    b.set_w(slot + 0x68, left);
    if b.w(b.l(slot + 0x54)) == 0xffff && !n_clear(left) {
        b.set_l(slot + 0x58, WITCH_LOOP_PATH);
        b.set_l(slot + 0x54, WITCH_LOOP_PATH);
        b.set_l(slot + 4, WITCH_LOOP);
    } else {
        obj_path_y(b, slot, top.wrapping_add(0x20));
        b.set_l(slot + 0x54, b.l(slot + 0x54).wrapping_add(2));
    }
    follow_and_despawn(c, slot);
}

/// h0+$25b40: she flies her loop; at its end she bobs again, starting this pass.
fn witch_loop(c: &mut LevelCtx, slot: u32, d7: u16) {
    let (x0, y0) = (c.bus.w(CAMERA_X + 2), camera_y(c).wrapping_add(0x20));
    let b = &mut c.bus;
    let at = b.l(slot + 0x54);
    if b.l(at) == 0xffff_ffff {
        b.set_l(slot + 0x54, b.l(slot + 0x58));
        start_bob(c, slot);
        return witch_bob(c, slot, d7);
    }
    b.set_l(slot + 8, b.l(at));
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(x0));
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(y0));
    b.set_l(slot + 0x54, at.wrapping_add(4));
    follow_and_despawn(c, slot);
}

/// h0+$259ec: beaten, she only follows her bob and may be despawned.
fn witch_beaten(c: &mut LevelCtx, slot: u32, _d7: u16) {
    follow_and_despawn(c, slot);
}

/// `touch_boss_hit` (h0+$236b4): every touch takes one hit point and restarts her bob's step
/// timer. At none, the palette is set white and steps back to the level's one by one a frame
/// (fade record 0, mode 1), she touches nothing more, shows her beaten look and stays
/// ([`witch_beaten`]), and the level ends `$64` passes on (`level_state`).
pub fn touch_boss_hit(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let hp = b.w(slot + 0x1c).wrapping_sub(1);
    b.set_w(slot + 0x1c, hp);
    if hp != 0 {
        let bob = b.l(slot + 0x50);
        b.set_w(bob + 0x1a, 0);
        return;
    }
    b.set_w(FADES[0] + 2, 1);
    let palette = b.l(LEVEL_PALETTE_PTR);
    stepped_start(c, FADES[0], WITCH_FLASH, palette, LEVEL_COPPER + 0x2d4, 0x20);
    let b = &mut c.bus;
    b.set_w(slot + 0x30, 1);
    b.set_l(slot + 0x10, NOTHING);
    let bob = b.l(slot + 0x50);
    b.set_l(bob + 0x28, LOOK_BEATEN);
    b.set_l(bob + 0x42, LOOK_BEATEN);
    b.set_l(slot + 4, WITCH_BEATEN);
    b.set_l(LEVEL_STATE, 0x64);
}
