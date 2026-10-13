//! The objects' side of the 25 Hz pass (docs/09): the slots and their spawners, the slots'
//! updates, hitboxes and collisions, and the bobs. The kinds themselves (items T15, enemies and
//! bosses T16–T18) register their routines into the level's dispatch; what is here is what the
//! original does whatever the objects: spawning into a free slot, freeing, the hitbox list,
//! touches, the bob draw list and the bob drawer, which erases and draws every bob into the back
//! display buffer.

use crate::frog::{FROG, f};
use crate::layout::{h0, h3};
use crate::level::camera::{CAMERA_X, CAMERA_Y, SCROLL_DIRS, SHOOTER_MODE, SPRITES_HIDDEN};
use crate::level::beam::Cost;
use crate::level::{LevelCtx, n_clear};
use crate::mem::Bus;

/// `obj_slots` (h0+$1ba6c): 20 slots of `$7c` bytes.
pub const OBJ_SLOTS: u32 = h0(0x1ba6c);
const SLOT_SIZE: u32 = 0x7c;
const SLOTS: u32 = 20;
/// `hitboxes` (h0+$d350): 12-byte entries (x1, x2, y1, y2, slot), ending with a negative word.
pub const HITBOXES: u32 = h0(0xd350);
/// `spud_bob` (h0+$297b6): the spud's bob record; its level position is the long before it.
pub const SPUD_BOB: u32 = h0(0x297b6);
/// `bob_draw_list` (h0+$2970e): the bobs to draw, ending with 0; the long before it is 0 too.
pub const BOB_DRAW_LIST: u32 = h0(0x2970e);
/// `bob_free_list` (h0+$1ba16).
pub const BOB_FREE_LIST: u32 = h0(0x1ba16);

/// `obj_run_all` (h0+$1b324): each live slot's update routine (`+$04`), in slot order.
pub fn obj_run_all(c: &mut LevelCtx) {
    for k in 0..SLOTS {
        let slot = OBJ_SLOTS + k * SLOT_SIZE;
        c.spend(Cost::ObjSlot);
        if c.bus.l(slot + 0x40) != 0 {
            c.spend(Cost::ObjUpdate);
            let update = c.bus.l(slot + 4);
            crate::level::call(c, update, slot, 0);
        }
        c.step_done();
    }
}

/// `obj_count` (h0+$1ba6a).
pub const OBJ_COUNT: u32 = h0(0x1ba6a);
/// `obj_templates` (h0+$1c6e2): 160 templates of `$ae` bytes.
pub const OBJ_TEMPLATES: u32 = h0(0x1c6e2);
/// The slot's first `$34` bytes come from its template.
const TEMPLATE_HEAD: u32 = 0x34;
/// `map_rows` (h0+$17334): a pointer to each map row, from one row above the first.
const MAP_ROWS: u32 = h0(0x17334);

/// What a spawner leaves in the registers its callers read: `d0` 0 when a slot was found (some
/// paths leave it so even when none was: the original's), `d1`, and `a0` the slot (0 for none).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spawned {
    pub d0: u32,
    pub d1: u32,
    pub a0: u32,
}

/// The slot the spawner settles on (`a1`) after its search, or `None`: no slot.
fn find_free(c: &mut LevelCtx) -> Option<u32> {
    if let Some(s) = (0..SLOTS)
        .map(|k| OBJ_SLOTS + k * SLOT_SIZE)
        .find(|&s| c.bus.l(s + 0x40) == 0)
    {
        return Some(s);
    }
    // Every slot full: an evictable one off the screen is reclaimed.
    for k in 0..SLOTS {
        let s = OBJ_SLOTS + k * SLOT_SIZE;
        if c.bus.w(s + 0x2a) != 0 {
            obj_despawn_if_offscreen(c, s, 0);
            if c.bus.l(s + 0x40) == 0 {
                return Some(s);
            }
        }
    }
    None
}

/// A free bob record for `slot` from `bob_free_list` (its entry taken), or false: none.
fn take_bob(b: &mut Bus, slot: u32) -> bool {
    for k in 0..SLOTS {
        let entry = BOB_FREE_LIST + 4 * k;
        if b.l(entry) != 0 {
            b.set_l(slot + 0x50, b.l(entry));
            b.set_l(slot + 0x4c, entry);
            b.set_l(entry, 0);
            return true;
        }
    }
    false
}

/// The pixel position of map cell `cell`, offset by `dx`, `dy` (words): `(cell − map_rows) /
/// $260` gives the row and, as the remainder, the column's byte offset.
fn cell_position(b: &Bus, cell: u32, dx: u16, dy: u16) -> (u16, u16) {
    let d = cell.wrapping_sub(b.l(MAP_ROWS));
    let (row, rem) = if d / 0x260 > 0xffff {
        // divu overflow: the register keeps the dividend.
        (d as u16, (d >> 16) as u16)
    } else {
        ((d / 0x260) as u16, (d % 0x260) as u16)
    };
    (
        dx.wrapping_add(rem.wrapping_shl(3)),
        dy.wrapping_add(row.wrapping_shl(4)),
    )
}

/// The map cell under pixel `x`, `y` less one cell (h0+$1b79c): what a pixel spawn records as its
/// cell.
fn pixel_cell(b: &Bus, x: u16, y: u16) -> u32 {
    let col = (x >> 4).wrapping_add(x >> 4);
    let row = (y >> 4).wrapping_mul(4);
    b.l(MAP_ROWS.wrapping_add(row as i16 as i32 as u32))
        .wrapping_add(col as u32)
        .wrapping_sub(2)
}

/// The end every spawner shares (h0+$1b47a…): the slot takes the template's position, its cell,
/// the template's first `$34` bytes, its bob's look from `+$26`; then the template's init routine
/// runs on it and `obj_count` grows.
fn fill_slot(c: &mut LevelCtx, slot: u32, template: u32, cell: u32) {
    let b = &mut c.bus;
    b.set_l(slot + 0x44, b.l(template + 8));
    b.set_l(slot + 0x40, cell);
    for k in 0..TEMPLATE_HEAD / 4 {
        b.set_l(slot + 4 * k, b.l(template + 4 * k));
    }
    let bob = b.l(slot + 0x50);
    let look = b.l(slot + 0x26);
    for o in [4, 0x4a, 0x90, 0xd6] {
        b.set_l(bob + o, look);
    }
    let init = b.l(template);
    c.spend(Cost::Spawn);
    crate::level::call(c, init, slot, 0);
    let b = &mut c.bus;
    b.set_w(OBJ_COUNT, b.w(OBJ_COUNT).wrapping_add(1));
}

/// `obj_spawn_cell` (h0+$1b3d8): `template` at map cell `cell`, `dx`, `dy` pixels into it,
/// unless the cell already has a live slot (that slot is returned, `d1` -1).
pub fn obj_spawn_cell(c: &mut LevelCtx, template: u32, cell: u32, dx: u16, dy: u16) -> Spawned {
    if let Some(s) = (0..SLOTS)
        .map(|k| OBJ_SLOTS + k * SLOT_SIZE)
        .find(|&s| c.bus.l(s + 0x40) == cell)
    {
        return Spawned {
            d0: 0,
            d1: 0xffff_ffff,
            a0: s,
        };
    }
    // With no slot the original sets d0 to -1, then clears it.
    spawn_at_cell(c, template, cell, dx, dy).map_or(
        Spawned {
            d0: 0,
            d1: dy as u32,
            a0: 0,
        },
        |(s, _)| Spawned {
            d0: 0,
            d1: 0,
            a0: s,
        },
    )
}

/// `obj_spawn_cell_alt` (h0+$1b4e6): [`obj_spawn_cell`] without looking for a live slot there.
pub fn obj_spawn_cell_alt(c: &mut LevelCtx, template: u32, cell: u32, dx: u16, dy: u16) -> Spawned {
    spawn_at_cell(c, template, cell, dx, dy).map_or(
        Spawned {
            d0: 0xffff_ffff,
            d1: dy as u32,
            a0: 0,
        },
        |(s, y)| Spawned {
            d0: 0,
            d1: y as u32,
            a0: s,
        },
    )
}

/// The cell spawners' search and fill: the slot and the y it took, or `None` (nothing changed
/// but evictions).
fn spawn_at_cell(
    c: &mut LevelCtx,
    template: u32,
    cell: u32,
    dx: u16,
    dy: u16,
) -> Option<(u32, u16)> {
    let slot = find_free(c)?;
    if !take_bob(&mut c.bus, slot) {
        return None;
    }
    let (x, y) = cell_position(&c.bus, cell, dx, dy);
    c.bus.set_w(template + 8, x);
    c.bus.set_w(template + 0xa, y);
    fill_slot(c, slot, template, cell);
    Some((slot, y))
}

/// `obj_spawn_coin_forced` (h0+$1b5d0): a coin at map cell `cell`. With every slot full and none
/// evictable off the screen, the first slot that is neither evictable nor a coin is freed for it.
pub fn obj_spawn_coin_forced(
    c: &mut LevelCtx,
    template: u32,
    cell: u32,
    dx: u16,
    dy: u16,
) -> Spawned {
    let slots = || (0..SLOTS).map(|k| OBJ_SLOTS + k * SLOT_SIZE);
    if let Some(s) = slots().find(|&s| c.bus.l(s + 0x40) == cell) {
        return Spawned {
            d0: 0,
            d1: dy as u32,
            a0: s,
        };
    }
    let none = Spawned {
        d0: 0xffff_ffff,
        d1: dy as u32,
        a0: template,
    };
    let slot = match find_free(c) {
        Some(s) => s,
        None => {
            let coin_init = h0(0x23f60);
            let Some(s) = slots().find(|&s| c.bus.w(s + 0x2a) == 0 && c.bus.l(s) != coin_init)
            else {
                return none;
            };
            obj_free(c, s, 0);
            s
        }
    };
    if !take_bob(&mut c.bus, slot) {
        return none;
    }
    let (x, y) = cell_position(&c.bus, cell, dx, dy);
    c.bus.set_w(template + 8, x);
    c.bus.set_w(template + 0xa, y);
    fill_slot(c, slot, template, cell);
    Spawned {
        d0: 0,
        d1: y as u32,
        a0: slot,
    }
}

/// `obj_spawn_at_xy` (h0+$1b706): `template` at level pixel `x`, `y`, unless a live slot was
/// spawned at that very position (that slot is returned, `d1` -1).
pub fn obj_spawn_at_xy(c: &mut LevelCtx, template: u32, x: u16, y: u16) -> Spawned {
    let at = (x as u32) << 16 | y as u32;
    if let Some(s) = (0..SLOTS)
        .map(|k| OBJ_SLOTS + k * SLOT_SIZE)
        .find(|&s| c.bus.l(s + 0x44) == at)
    {
        return Spawned {
            d0: 0,
            d1: 0xffff_ffff,
            a0: s,
        };
    }
    spawn_at_pixel(c, template, x, y).map_or(
        Spawned {
            d0: 0xffff_ffff,
            d1: 0,
            a0: 0,
        },
        |s| Spawned {
            d0: 0,
            d1: 0,
            a0: s,
        },
    )
}

/// `obj_spawn_at_xy_nodedup` (h0+$1b822): [`obj_spawn_at_xy`] without looking for a slot there.
pub fn obj_spawn_at_xy_nodedup(c: &mut LevelCtx, template: u32, x: u16, y: u16) -> Spawned {
    spawn_at_pixel(c, template, x, y).map_or(
        Spawned {
            d0: 0xffff_ffff,
            d1: y as u32,
            a0: 0,
        },
        |s| Spawned {
            d0: 0,
            d1: y as u32,
            a0: s,
        },
    )
}

/// The pixel spawners' search and fill.
fn spawn_at_pixel(c: &mut LevelCtx, template: u32, x: u16, y: u16) -> Option<u32> {
    let slot = find_free(c)?;
    if !take_bob(&mut c.bus, slot) {
        return None;
    }
    c.bus.set_w(template + 8, x);
    c.bus.set_w(template + 0xa, y);
    let cell = pixel_cell(&c.bus, x, y);
    fill_slot(c, slot, template, cell);
    Some(slot)
}

/// `obj_free` (h0+$1b90e): the slot is emptied and its bob parked (`$7d00`, `$7d00`) and given
/// back to `bob_free_list`.
pub fn obj_free(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    if b.l(slot + 0x40) == 0 {
        return;
    }
    let entry = b.l(slot + 0x4c);
    let bob = b.l(slot + 0x50);
    b.set_w(bob + 0x1a, 0);
    b.set_l(bob.wrapping_sub(4), 0x7d00_7d00);
    b.set_l(entry, bob);
    b.set_l(slot + 0x50, 0);
    b.set_l(slot + 0x40, 0);
    b.set_l(slot + 0x44, 0);
    b.set_w(OBJ_COUNT, b.w(OBJ_COUNT).wrapping_sub(1));
}

/// h0+$1b9ee: every slot freed (a level's set-up).
pub fn obj_free_all(c: &mut LevelCtx) {
    for k in 0..SLOTS {
        obj_free(c, OBJ_SLOTS + k * SLOT_SIZE, 0);
    }
}

/// The frame record a slot's bob shows: its animation step's frame (`(bob+$16)` → `(…)`).
fn shown_frame(b: &Bus, slot: u32) -> u32 {
    let bob = b.l(slot + 0x50);
    b.l(b.l(bob + 0x16))
}

/// h0+$1b940: freed once its right edge is left of the camera.
pub fn obj_free_if_left_behind(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &c.bus;
    let frame = shown_frame(b, slot);
    let d0 = b.w(CAMERA_X + 2).wrapping_sub(b.w(frame + 4));
    if n_clear(d0.wrapping_sub(b.w(slot + 8))) {
        obj_free(c, slot, 0);
    }
}

/// `obj_despawn_if_offscreen` (h0+$1b95e): freed once it is out of the screen on a side the
/// camera last moved away from (`scroll_dirs`; every side in the bonus level): `$170` right of
/// the camera, `$30` past the left edge by its width, `$150` below, `$30` above by its height.
pub fn obj_despawn_if_offscreen(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &c.bus;
    if b.l(slot + 0x40) == 0 {
        return;
    }
    let dirs = if b.w(SHOOTER_MODE) != 0 {
        0xff
    } else {
        b.b(SCROLL_DIRS)
    };
    let frame = shown_frame(b, slot);
    let (x, y) = (b.w(slot + 8), b.w(slot + 0xa));
    let (cx, cy) = (b.w(CAMERA_X + 2), b.w(CAMERA_Y + 2));
    let gone = (dirs & 4 != 0 && n_clear(x.wrapping_sub(0x170).wrapping_sub(cx)))
        || (dirs & 2 != 0
            && !n_clear(
                x.wrapping_add(b.w(frame + 4))
                    .wrapping_add(0x30)
                    .wrapping_sub(cx),
            ))
        || (dirs & 0x10 != 0 && n_clear(y.wrapping_sub(0x150).wrapping_sub(cy)))
        || (dirs & 8 != 0
            && !n_clear(
                y.wrapping_add(b.w(frame + 6))
                    .wrapping_add(0x30)
                    .wrapping_sub(cy),
            ));
    if gone {
        obj_free(c, slot, 0);
    }
}

/// A slot routine that does nothing (`rts`: h0+$7d4, h0+$29708).
pub fn nothing(_c: &mut LevelCtx, _slot: u32, _d7: u16) {}

/// A slot's bob takes its position (`bob − 4` = `+$08` x, y).
pub fn bob_follow(b: &mut Bus, slot: u32) {
    let bob = b.l(slot + 0x50);
    b.set_l(bob.wrapping_sub(4), b.l(slot + 8));
}

/// Start a slot's bob on animation `anim` (`+$28` and `+$42`: the drawer starts it next pass).
pub fn bob_anim(b: &mut Bus, slot: u32, anim: u32) {
    let bob = b.l(slot + 0x50);
    b.set_l(bob + 0x28, anim);
    b.set_l(bob + 0x42, anim);
}

/// `obj_build_hitboxes` (h0+$d2f6): every live slot's hitboxes, offset by its position.
pub fn obj_build_hitboxes(c: &mut LevelCtx) {
    let mut out = HITBOXES;
    for k in 0..SLOTS {
        let slot = OBJ_SLOTS + k * SLOT_SIZE;
        c.spend(Cost::HitboxSlot);
        let b = &mut c.bus;
        let mut hb = if b.l(slot + 0x40) == 0 { 0 } else { b.l(slot + 0x14) };
        let mut boxes = 0;
        while hb != 0 {
            boxes += 1;
            let (dx, dy) = (b.w(hb + 4), b.w(hb + 6));
            let x1 = b.w(slot + 8).wrapping_add(dx);
            b.set_w(out, x1);
            b.set_w(out + 2, x1.wrapping_add(b.w(hb)));
            let y1 = b.w(slot + 0xa).wrapping_add(dy);
            b.set_w(out + 4, y1);
            b.set_w(out + 6, y1.wrapping_add(b.w(hb + 2)));
            b.set_l(out + 8, slot);
            out += 12;
            hb = hb.wrapping_add(8);
            if !n_clear(b.w(hb)) {
                break;
            }
        }
        c.spend_n(Cost::Hitbox, boxes);
        c.step_done();
    }
    c.bus.set_w(out, 0xffff);
}

/// The hitbox entries overlapping x `x1..x2`, y `y1..y2`, walked as the original walks the list
/// (an entry is passed over when it ends left of or starts right of the box, and likewise down):
/// each entry's slot, in order.
fn overlapping(b: &Bus, x1: u16, x2: u16, y1: u16, y2: u16) -> Vec<u32> {
    let mut hits = Vec::new();
    let mut at = HITBOXES;
    while n_clear(b.w(at)) {
        let (ex1, ex2, ey1, ey2) = (b.w(at), b.w(at + 2), b.w(at + 4), b.w(at + 6));
        let slot = b.l(at + 8);
        at += 12;
        if !n_clear(ex2.wrapping_sub(x1))
            || n_clear(ex1.wrapping_sub(x2))
            || !n_clear(ey2.wrapping_sub(y1))
            || n_clear(ey1.wrapping_sub(y2))
        {
            continue;
        }
        hits.push(slot);
    }
    hits
}

/// `touch_spud_damage` (h0+$2360e): the touch handler the spud passes over.
const TOUCH_SPUD_DAMAGE: u32 = h0(0x2360e);
/// The hidden coin spring's update (h0+$2408a): the spud sets it off although it is immune.
const SPRING_UPDATE: u32 = h0(0x2408a);
/// `opt_game_mode` (h0+$9e6): 0 the easy game.
const OPT_GAME_MODE: u32 = h0(0x9e6);

/// `collide_all` (h0+$d0cc): the spud against the objects, then the frog (which is off any
/// platform until an object says otherwise). The hitbox list is the one this pass built; a
/// handler that frees a slot does not change it.
pub fn collide_all(c: &mut LevelCtx) {
    let mut entries = 0;
    let mut at = HITBOXES;
    while n_clear(c.bus.w(at)) {
        entries += 1;
        at += 12;
    }
    c.spend(Cost::Collide);
    c.spend_n(Cost::CollideEntry, 2 * entries);
    spud_touches(c);
    c.step_done();
    frog_touches(c);
    debug_assert_eq!(
        c.bus.w(SHOOTER_MODE),
        0,
        "the bonus level's collisions are T19's"
    );
}

/// h0+$d284: the spud's 16-pixel box against every object's: each hit runs the object's touch
/// handler with d7 = 1, unless the object takes no spud damage (`touch_spud_damage`), has no
/// contact (`+$1c` negative) or is immune (`$7d00`; the hidden coin spring is hit anyway).
fn spud_touches(c: &mut LevelCtx) {
    let b = &c.bus;
    let (sx, sy) = (b.w(FROG + 0x74), b.w(FROG + 0x76));
    for slot in overlapping(b, sx, sx.wrapping_add(0x10), sy, sy.wrapping_add(0x10)) {
        let b = &c.bus;
        let touch = b.l(slot + 0x10);
        if touch == TOUCH_SPUD_DAMAGE || !n_clear(b.w(slot + 0x1c)) {
            continue;
        }
        if b.l(slot + 4) != SPRING_UPDATE && b.w(slot + 0x1c) == 0x7d00 {
            continue;
        }
        crate::level::call(c, touch, slot, 1);
    }
}

/// h0+$d0e0: the frog's box against every object's. Landing on a stompable object (falling,
/// its feet above the object's top) bounces the frog and runs the touch handler; a passive one's
/// handler runs on any contact; any other with contact (`+$1c` not negative) hurts the frog
/// (`frog_hurt`) unless it is hurt already.
fn frog_touches(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(SPRITES_HIDDEN) != 0 {
        return;
    }
    b.set_w(FROG + f::ON_PLATFORM, 0);
    let x1 = b.w(FROG + f::X).wrapping_add(4).wrapping_add(0xe);
    let mut y1 = b.w(FROG + f::Y).wrapping_add(4).wrapping_add(0x16);
    let (x2, y2) = (x1.wrapping_add(0xa), y1.wrapping_add(0x20));
    if b.w(FROG + f::CROUCH) != 0 {
        y1 = y1.wrapping_add(0xb);
    }
    for slot in overlapping(b, x1, x2, y1, y2) {
        let b = &mut c.bus;
        let hurt = b.w(FROG + f::HURT) != 0;
        if b.w(slot + 0x2e) != 0
            && !hurt
            && n_clear(b.w(FROG + f::VY))
            && !n_clear(
                b.w(FROG + f::Y)
                    .wrapping_add(0x29)
                    .wrapping_sub(b.w(slot + 0xa)),
            )
        {
            let vy = if b.w(OPT_GAME_MODE) == 0 {
                0xffff
            } else {
                0xfffe
            };
            b.set_w(FROG + f::VY, vy);
            let touch = b.l(slot + 0x10);
            crate::level::call(c, touch, slot, 0);
            continue;
        }
        if b.w(slot + 0x2e) == 0 && b.w(slot + 0x30) != 0 {
            let touch = b.l(slot + 0x10);
            crate::level::call(c, touch, slot, 0);
            continue;
        }
        if b.w(slot + 0x30) != 0 || !n_clear(b.w(slot + 0x1c)) || hurt {
            continue;
        }
        crate::level::death::frog_hurt(c, slot);
    }
}

/// `build_bob_draw_list` (h0+$19b42): the spud, the slots' bobs (those in front first), then
/// the free bobs that still have something on screen to erase.
pub fn build_bob_draw_list(c: &mut LevelCtx) {
    c.spend(Cost::DrawList);
    let mut out = BOB_DRAW_LIST;
    c.bus.set_l(out, SPUD_BOB);
    out += 4;
    for front in [true, false] {
        for k in 0..SLOTS {
            c.spend(Cost::DrawListSlot);
            let b = &mut c.bus;
            let slot = OBJ_SLOTS + k * SLOT_SIZE;
            let bob = b.l(slot + 0x50);
            if bob != 0 && ((b.l(slot + 0x26) as i32) >= 0) == front {
                b.set_l(out, bob);
                out += 4;
            }
            c.step_done();
        }
    }
    for k in 0..SLOTS {
        c.spend(Cost::DrawListSlot);
        let b = &mut c.bus;
        let bob = b.l(BOB_FREE_LIST + 4 * k);
        if bob != 0 && [0x10, 0x56, 0x9c, 0xe2].iter().any(|&o| b.l(bob + o) != 0) {
            b.set_l(out, bob);
            out += 4;
        }
        c.step_done();
    }
    c.bus.set_l(out, 0);
}

/// The bob drawer's variables (h0+$18bca..$18be4).
const BD_CAMERA_X: u32 = h0(0x18bca);
const BD_CAMERA_Y: u32 = h0(0x18bce);
/// The camera's low words, as the drawer reads them.
const BD_CX: u32 = h0(0x18bcc);
const BD_CY: u32 = h0(0x18bd0);
/// The play field's lines the current part covers: top and bottom (screen lines; the ring seam
/// splits the drawing in two), and the same for drawing into the clean buffer.
const BD_TOP: u32 = h0(0x18bd2);
const BD_BOTTOM: u32 = h0(0x18bd4);
const BD_CLEAN_TOP: u32 = h0(0x18bd6);
const BD_CLEAN_BOTTOM: u32 = h0(0x18bd8);
/// The column of the camera, as a byte offset.
const BD_COLUMN: u32 = h0(0x18bda);
/// Set while the seam's part is drawn.
const BD_SEAM: u32 = h0(0x18bde);
/// `bob_buffer`: toggles each pass, the buffer the bobs go into.
pub const BOB_BUFFER: u32 = h0(0x18be4);
/// `line_offsets` (h0+$18be8).
const LINE_OFFSETS: u32 = h0(0x18be8);
/// `ring_row_lines` (h0+$176c4).
const RING_ROW_LINES: u32 = h0(0x176c4);
const BUFFER_0: u32 = h3(0x10600);
const BUFFER_1: u32 = h3(0x22f30);
const BUFFER_BYTES: u32 = 0x12930;
const PLANE: u32 = 0x3b70;
const CLEAN: u32 = h3(0x35860);
/// A sub-record of a bob record (4 per bob, `$46` apart: the two buffers' main part and seam
/// part).
const SUB: u32 = 0x46;

/// One blit of the drawer: BLTCON0/1, masks, the four pointers and modulos, BLTSIZE.
#[derive(Clone, Copy)]
pub(crate) struct Blit {
    pub(crate) con0: u16,
    pub(crate) con1: u16,
    pub(crate) afwm: u16,
    pub(crate) alwm: u16,
    pub(crate) a: u32,
    pub(crate) b: u32,
    pub(crate) c: u32,
    pub(crate) d: u32,
    pub(crate) amod: u16,
    pub(crate) bmod: u16,
    pub(crate) cmod: u16,
    pub(crate) dmod: u16,
    pub(crate) size: u16,
}

/// The words a blit of BLTSIZE `size` moves.
fn blit_words(size: u16) -> u32 {
    (match size >> 6 { 0 => 1024, h => h as u32 }) * (match size & 0x3f { 0 => 64, w => w as u32 })
}

/// The blitter in area mode, ascending (what the drawer uses): A shifted and masked, B shifted,
/// C, minterm, into D.
pub(crate) fn blit(b: &mut Bus, x: Blit) {
    let h = match x.size >> 6 {
        0 => 1024,
        h => h as u32,
    };
    let w = match x.size & 0x3f {
        0 => 64,
        w => w as u32,
    };
    let (use_a, use_b, use_c, use_d) = (
        x.con0 & 0x800 != 0,
        x.con0 & 0x400 != 0,
        x.con0 & 0x200 != 0,
        x.con0 & 0x100 != 0,
    );
    let (ash, bsh) = ((x.con0 >> 12) as u32, (x.con1 >> 12) as u32);
    let mt = x.con0 as u8;
    let (mut pa, mut pb, mut pc, mut pd) = (
        x.a & 0x7_fffe,
        x.b & 0x7_fffe,
        x.c & 0x7_fffe,
        x.d & 0x7_fffe,
    );
    let (mut aold, mut bold) = (0u16, 0u16);
    for _ in 0..h {
        for col in 0..w {
            let mut a = if use_a { b.w(pa) } else { 0 };
            if use_a {
                pa = pa.wrapping_add(2);
            }
            if col == 0 {
                a &= x.afwm;
            }
            if col == w - 1 {
                a &= x.alwm;
            }
            let bv = if use_b { b.w(pb) } else { 0 };
            if use_b {
                pb = pb.wrapping_add(2);
            }
            let cv = if use_c { b.w(pc) } else { 0 };
            if use_c {
                pc = pc.wrapping_add(2);
            }
            let ash_v = ((aold as u32) << 16 | a as u32) >> ash;
            let bsh_v = ((bold as u32) << 16 | bv as u32) >> bsh;
            let (sa, sb) = (ash_v as u16, bsh_v as u16);
            aold = a;
            bold = bv;
            let mut d = 0u16;
            for (bit, term) in [
                (0x80, sa & sb & cv),
                (0x40, sa & sb & !cv),
                (0x20, sa & !sb & cv),
                (0x10, sa & !sb & !cv),
                (0x08, !sa & sb & cv),
                (0x04, !sa & sb & !cv),
                (0x02, !sa & !sb & cv),
                (0x01, !sa & !sb & !cv),
            ] {
                if mt & bit != 0 {
                    d |= term;
                }
            }
            if use_d {
                b.set_w(pd, d);
                pd = pd.wrapping_add(2);
            }
        }
        if use_a {
            pa = pa.wrapping_add(x.amod as i16 as i32 as u32) & 0x7_fffe;
        }
        if use_b {
            pb = pb.wrapping_add(x.bmod as i16 as i32 as u32) & 0x7_fffe;
        }
        if use_c {
            pc = pc.wrapping_add(x.cmod as i16 as i32 as u32) & 0x7_fffe;
        }
        if use_d {
            pd = pd.wrapping_add(x.dmod as i16 as i32 as u32) & 0x7_fffe;
        }
    }
}

/// `bob_draw` (h0+$19120): the bobs of the draw list, erased from the back buffer (copied back
/// from the clean buffer where they were last drawn) and drawn again (cookie-cut, minterm `$ca`,
/// their mask over the background), in two parts split at the display ring's seam; then the
/// buffers swap (`bob_buffer`).
pub fn bob_draw(c: &mut LevelCtx) {
    let b = &mut c.bus;
    b.set_l(BD_CAMERA_X, b.l(h0(0x16664)));
    b.set_l(BD_CAMERA_Y, b.l(h0(0x16660)));
    // (The camera is latched first: an interrupt in the drawer's set-up comes after it.)
    let mut bobs = 0;
    while c.bus.l(BOB_DRAW_LIST + 4 * bobs) != 0 {
        bobs += 1;
    }
    c.spend(Cost::BobDraw);
    c.spend_n(Cost::BobPosition, bobs);
    let b = &mut c.bus;
    // The seam: the screen line where the ring wraps.
    let mut d1 = (b.w(BD_CY) as u32).wrapping_add(0xffff_fff4);
    let d3 = d1 & 0xffff_0000 | (d1 as u16 & 0xf) as u32;
    let row = ((d1 as u16 >> 4).wrapping_mul(4)) >> 1;
    d1 = d1 & 0xffff_0000 | row as u32;
    let d6 = 0x130u16
        .wrapping_sub(b.w(RING_ROW_LINES.wrapping_add(d1 as u16 as i16 as i32 as u32)))
        .wrapping_sub(d3 as u16);
    let seam = 0x130u16.wrapping_sub(d6);
    b.set_w(BD_TOP, seam);
    b.set_w(BD_BOTTOM, 0x130);
    let pushed = 0x130u16.wrapping_sub(0x130u16.wrapping_sub(seam)) & 0xfff0;
    b.set_w(BD_CLEAN_TOP, (b.w(BD_TOP) & 0xfff0).wrapping_add(0x10));
    b.set_w(BD_CLEAN_BOTTOM, b.w(BD_BOTTOM));
    // (h0+$1918e: the drawer waits for the vertical blank's flag, which a pass on time finds set.)
    screen_positions(b);
    b.set_w(BD_SEAM, 0);
    // h0+$19216: the column, and which buffer is the back one.
    let col = ((b.w(BD_CX).wrapping_add(0xf) >> 4).wrapping_mul(2)) as u32;
    b.set_l(BD_COLUMN, col);
    let (mut back, mut other) = (BUFFER_0 + col, BUFFER_1 + col);
    let mut sub = 0u32;
    let t = !b.w(BOB_BUFFER);
    b.set_w(BOB_BUFFER, t);
    if t != 0 {
        std::mem::swap(&mut back, &mut other);
        sub = SUB;
    }
    draw_part(c, back, sub);
    // The part below the seam.
    let b = &mut c.bus;
    b.set_w(BD_TOP, 0);
    b.set_w(BD_BOTTOM, if n_clear(pushed) { pushed } else { 0 });
    b.set_w(BD_CLEAN_TOP, b.w(BD_TOP));
    b.set_w(BD_CLEAN_BOTTOM, b.w(BD_BOTTOM));
    let mut at = BOB_DRAW_LIST;
    loop {
        let bob = b.l(at).wrapping_add(sub);
        at += 4;
        b.set_l(bob + 0x8c, b.l(bob));
        b.set_w(bob + 0x8e, b.w(bob + 0x8e).wrapping_sub(0x130));
        if b.l(at) == 0 {
            break;
        }
    }
    b.set_w(BD_SEAM, 0xffff);
    draw_part(c, back, sub + 0x8c);
}

/// h0+$1909c: each listed bob's screen position from its level position (the long before the
/// record) and the camera, the ring's line under the camera's top added.
fn screen_positions(b: &mut Bus) {
    let d1 = (b.w(BD_CY) as u32).wrapping_add(0xffff_fff4);
    let line = d1 as u16 & 0xf;
    let row = (((d1 as u16 & 0xfff0) >> 4).wrapping_mul(4)) >> 1;
    let d6 = b
        .w(RING_ROW_LINES.wrapping_add(row as i16 as i32 as u32))
        .wrapping_add(line);
    let d5 = !(b.w(BD_CX).wrapping_add(0xf)) & 0xf;
    let d7 = 0xfu16.wrapping_sub(d5);
    let (cx, cy) = (b.w(BD_CX), b.w(BD_CY));
    let mut at = BOB_DRAW_LIST;
    loop {
        let bob = b.l(at);
        at += 4;
        let x = b.w(bob.wrapping_sub(4)).wrapping_sub(cx).wrapping_add(3);
        b.set_w(bob, x);
        let y = b.w(bob.wrapping_sub(2)).wrapping_sub(cy);
        b.set_w(bob + 2, y.wrapping_add(d6));
        b.set_w(bob, b.w(bob).wrapping_add(d7));
        if b.l(at) == 0 {
            break;
        }
    }
}

/// h0+$19242: one part of the drawing into the back buffer at `back` (with the camera's column),
/// the bobs' sub-records `sub` on: first (unless this is the seam's part) each bob's last
/// drawing is erased, going down the list; then each is drawn, going back up it.
fn draw_part(c: &mut LevelCtx, back: u32, sub: u32) {
    c.spend(Cost::BobPart);
    c.custom(0x96, 0x8400);
    let mut at = BOB_DRAW_LIST;
    if c.bus.w(BD_SEAM) != 0 {
        while c.bus.l(at) != 0 {
            at += 4;
        }
    } else {
        loop {
            let rec = c.bus.l(at).wrapping_add(sub);
            at += 4;
            for part in [rec, rec + 0x8c] {
                c.spend(Cost::BobEraseLook);
                let dest = c.bus.l(part + 0x10);
                if dest != 0 {
                    erase(c, part, dest);
                }
                c.bus.set_l(part + 0x10, 0);
            }
            if c.bus.l(at) == 0 {
                break;
            }
        }
    }
    // h0+$1940e: draw, going back up the list.
    loop {
        at -= 4;
        let bob = c.bus.l(at);
        draw_bob(c, bob, back, sub);
        if c.bus.l(at - 4) == 0 {
            break;
        }
    }
    c.custom(0x96, 0x0400);
}

/// h0+$1928c: copy the clean background back over a bob's last drawing (five planes).
fn erase(c: &mut LevelCtx, part: u32, dest: u32) {
    let b = &mut c.bus;
    let size = b.w(part + 0x1e);
    let modulo = b.w(part + 0x26);
    let mut src = dest.wrapping_add(BUFFER_BYTES);
    if !n_clear_l(b.l(part + 0x2c).wrapping_sub(BUFFER_1)) {
        src = src.wrapping_add(BUFFER_BYTES);
    }
    c.spend(Cost::BobErase);
    c.custom(0x9a, 0x0010);
    for p in 0..5 {
        c.spend(Cost::BobPlane);
        c.blit_words(blit_words(size), 0x09f0);
        blit(
            &mut c.bus,
            Blit {
                con0: 0x09f0,
                con1: 0,
                afwm: 0xffff,
                alwm: 0xffff,
                a: src.wrapping_add(PLANE * p),
                b: 0,
                c: 0,
                d: dest.wrapping_add(PLANE * p),
                amod: modulo,
                bmod: 0,
                cmod: 0,
                dmod: modulo,
                size,
            },
        );
    }
    c.custom(0x9a, 0x8010);
}

fn n_clear_l(v: u32) -> bool {
    (v as i32) >= 0
}

/// h0+$19410: one bob's sub-record `sub`: its animation steps (in the main part), then it is
/// clipped to the part of the screen being drawn and cookie-cut into the back buffer, or, for a
/// bob drawn into the background (`+4` negative, h0+$19708), copied into it.
fn draw_bob(c: &mut LevelCtx, bob: u32, back: u32, sub: u32) {
    c.spend(Cost::Bob);
    let b = &mut c.bus;
    b.set_l(bob + SUB, b.l(bob));
    let rec;
    let mut frame_at;
    if b.w(BD_SEAM) != 0 {
        rec = bob.wrapping_add(sub);
        frame_at = b.l(rec.wrapping_sub(0x76));
    } else {
        if b.l(bob + 0x28) != 0 {
            let start = b.l(bob + 0x28);
            b.set_l(bob + 0x28, 0);
            b.set_l(bob + 8, b.l(bob + 0x42));
            b.set_l(bob + 0x16, start.wrapping_sub(8));
            b.set_w(bob + 0x1a, 0);
        }
        let left = b.w(bob + 0x1a).wrapping_sub(1);
        b.set_w(bob + 0x1a, left);
        if !n_clear(left) {
            b.set_l(bob + 0x16, b.l(bob + 0x16).wrapping_add(8));
            let mut step = b.l(bob + 0x16);
            if (b.l(step) as i32) < 0 {
                step = b.l(bob + 8);
                b.set_l(bob + 0x16, step);
            }
            b.set_w(bob + 0x1a, b.w(step.wrapping_add(6)));
        }
        b.set_l(bob + 0x5c, b.l(bob + 0x16));
        rec = bob.wrapping_add(sub);
        frame_at = b.l(rec + 0x16);
    }
    if b.w(BD_TOP) == b.w(BD_BOTTOM) {
        return;
    }
    if (b.l(rec + 4) as i32) < 0 {
        return draw_background(c, rec, frame_at, back);
    }
    let fr = b.l(frame_at);
    frame_at = fr;
    let a3 = frame_at;
    b.set_l(rec + 0x2c, back);
    let mut a1 = back;
    b.set_l(rec + 0x1e, b.l(a3 + 0x1a));
    b.set_l(rec + 0x22, b.l(a3 + 0x1e));
    b.set_w(rec + 0x26, b.w(a3 + 0x22));
    let mut d7 = b.w(rec).wrapping_add(b.w(a3 + 8));
    let mut a4 = b.l(a3 + 0x24);
    b.set_l(rec + 0x10, 0);
    let width = b.w(a3 + 4);
    let d6 = d7.wrapping_add(width).wrapping_sub(0x180);
    if n_clear(d6) {
        if n_clear(d7.wrapping_sub(0x170)) {
            return;
        }
        let d5 = width.wrapping_sub(d6) >> 4;
        let d6 = ((width >> 4).wrapping_sub(d5)).wrapping_mul(2);
        b.set_w(rec + 0x26, b.w(rec + 0x26).wrapping_add(d6));
        b.set_w(rec + 0x24, b.w(rec + 0x24).wrapping_add(d6));
        b.set_w(rec + 0x1e, (b.w(a3 + 0x1a) & 0xffc0).wrapping_add(d5));
    }
    let mut d5 = 0u16;
    if !n_clear(d7) {
        if !n_clear(d7.wrapping_add(width).wrapping_sub(0x10)) {
            return;
        }
        let neg = d7.wrapping_neg();
        let words = neg >> 4;
        b.set_w(rec + 0x1e, b.w(rec + 0x1e).wrapping_sub(words));
        let bytes = words.wrapping_mul(2);
        let ext = bytes as i16 as i32 as u32;
        b.set_l(rec + 0x20, b.l(rec + 0x20).wrapping_add(ext));
        a4 = a4.wrapping_add(ext);
        b.set_w(rec + 0x26, b.w(rec + 0x26).wrapping_add(bytes));
        b.set_w(rec + 0x24, b.w(rec + 0x24).wrapping_add(bytes));
        let d6 = 0xfu16.wrapping_sub(neg & 0xf);
        d7 = 0x170u16.wrapping_add(d6).wrapping_add(1);
        d5 = 2;
    }
    a1 = a1.wrapping_add((d7 >> 3) as u32);
    let mut y = b.w(rec + 2).wrapping_add(b.w(a3 + 0xa));
    let height = b.w(a3 + 6);
    let below = y.wrapping_add(height).wrapping_sub(b.w(BD_BOTTOM));
    if n_clear(below) {
        if n_clear(below.wrapping_sub(height)) {
            return;
        }
        let s = b.w(rec + 0x1e);
        b.set_w(rec + 0x1e, s & 0x3f);
        let d1 = ((s >> 6).wrapping_sub(below)).wrapping_shl(6);
        b.set_w(rec + 0x1e, b.w(rec + 0x1e) | d1);
    }
    y = y.wrapping_sub(b.w(BD_TOP));
    let offset_index;
    if !n_clear(y) {
        let cut = y.wrapping_neg();
        if n_clear(cut.wrapping_sub(height)) {
            return;
        }
        let s = b.w(rec + 0x1e);
        b.set_w(rec + 0x1e, s & 0x3f);
        let d1 = ((s >> 6).wrapping_sub(cut)).wrapping_shl(6);
        b.set_w(rec + 0x1e, b.w(rec + 0x1e) | d1);
        let skip = (b.w(a3 + 0x14) as u32 * cut as u32) as u16 as i16 as i32 as u32;
        b.set_l(rec + 0x20, b.l(rec + 0x20).wrapping_add(skip));
        a4 = a4.wrapping_add(skip);
        offset_index = b.w(BD_TOP).wrapping_mul(2).wrapping_sub(d5);
    } else {
        offset_index = y.wrapping_add(b.w(BD_TOP)).wrapping_mul(2).wrapping_sub(d5);
    }
    a1 = a1.wrapping_add(line_offset_long(b, offset_index));
    if (b.w(rec) as i16) < 0 && !n_clear(b.w(rec + 2).wrapping_sub(1)) {
        a1 = a1.wrapping_sub(0x30);
    }
    let size = b.w(rec + 0x1e);
    if size & 0x3f == 0 || size & 0xffc0 == 0 {
        return;
    }
    b.set_l(rec + 0x10, a1);
    let shift = (b.w(rec).wrapping_add(b.w(a3 + 8)) & 0xf).wrapping_shl(12);
    let (cmod, bmod) = (b.w(rec + 0x26), b.w(rec + 0x24));
    let mut src = b.l(rec + 0x20);
    let step = b.l(a3 + 0x16);
    c.spend(Cost::BobShown);
    c.custom(0x9a, 0x0010);
    for p in 0..5u32 {
        c.spend(Cost::BobPlane);
        c.blit_words(blit_words(size), 0x0fca);
        blit(
            &mut c.bus,
            Blit {
                con0: shift | 0x0fca,
                con1: shift,
                afwm: 0xffff,
                alwm: 0x0000,
                a: a4,
                b: src,
                c: a1.wrapping_add(PLANE * p),
                d: a1.wrapping_add(PLANE * p),
                amod: bmod,
                bmod,
                cmod,
                dmod: cmod,
                size,
            },
        );
        src = src.wrapping_add(step);
    }
    c.custom(0x9a, 0x8010);
}

/// The long the drawer adds for a line: `move.w (a5,d7.w),d7` after `ext.l d7` of a line that is
/// never negative there: the table's word.
fn line_offset_long(b: &Bus, index: u16) -> u32 {
    let word = b.w(LINE_OFFSETS.wrapping_add(index as i16 as i32 as u32)) as u32;
    word
}

/// h0+$19708: a bob drawn into the background: copied (no mask) into the back buffer, or into
/// the clean buffer when its frame's y offset is `$fc00`; nothing erases it.
fn draw_background(c: &mut LevelCtx, rec: u32, frame_at: u32, back: u32) {
    let b = &mut c.bus;
    let mut a1 = back;
    let a3 = b.l(frame_at);
    let to_clean = b.w(a3 + 0xa) == 0xfc00;
    if to_clean {
        a1 = CLEAN.wrapping_add(b.l(BD_COLUMN));
    }
    b.set_w(rec, b.w(rec) & 0xfff0);
    b.set_l(rec + 0x1e, b.l(a3 + 0x1a));
    b.set_l(rec + 0x22, b.l(a3 + 0x1e));
    b.set_w(rec + 0x26, b.w(a3 + 0x22));
    let mut d7 = b.w(rec);
    let width = b.w(a3 + 4);
    let d6 = d7.wrapping_add(width).wrapping_sub(0x170);
    if n_clear(d6) {
        if n_clear(d7.wrapping_sub(0x170)) {
            return;
        }
        let d5 = width.wrapping_sub(d6) >> 4;
        let d6 = ((width >> 4).wrapping_sub(d5)).wrapping_mul(2);
        b.set_w(rec + 0x26, b.w(rec + 0x26).wrapping_add(d6));
        b.set_w(rec + 0x24, b.w(rec + 0x24).wrapping_add(d6));
        b.set_w(rec + 0x1e, (b.w(a3 + 0x1a) & 0xffc0).wrapping_add(d5));
    }
    let mut d5 = 0u16;
    if !n_clear(d7) {
        if !n_clear(d7.wrapping_add(width)) {
            return;
        }
        let neg = d7.wrapping_neg();
        let words = neg >> 4;
        b.set_w(rec + 0x1e, b.w(rec + 0x1e).wrapping_sub(words));
        let bytes = words.wrapping_mul(2);
        b.set_l(
            rec + 0x20,
            b.l(rec + 0x20).wrapping_add(bytes as i16 as i32 as u32),
        );
        b.set_w(rec + 0x26, b.w(rec + 0x26).wrapping_add(bytes));
        b.set_w(rec + 0x24, b.w(rec + 0x24).wrapping_add(bytes));
        let d6 = 0xfu16.wrapping_sub(neg & 0xf);
        d7 = 0x170u16.wrapping_add(d6).wrapping_add(1);
        d5 = 2;
    }
    a1 = a1.wrapping_add((d7 >> 3) as u32);
    let mut y = b.w(rec + 2);
    if !to_clean {
        y = y.wrapping_add(b.w(a3 + 0xa));
    }
    let height = b.w(a3 + 6);
    let below = y.wrapping_add(height).wrapping_sub(b.w(BD_CLEAN_BOTTOM));
    if n_clear(below) {
        if n_clear(below.wrapping_sub(height)) {
            return;
        }
        let s = b.w(rec + 0x1e);
        b.set_w(rec + 0x1e, s & 0x3f);
        let d1 = ((s >> 6).wrapping_sub(below)).wrapping_shl(6);
        b.set_w(rec + 0x1e, b.w(rec + 0x1e) | d1);
    }
    y = y.wrapping_sub(b.w(BD_CLEAN_TOP));
    let index;
    if !n_clear(y) {
        let cut = y.wrapping_neg();
        if n_clear(cut.wrapping_sub(height)) {
            return;
        }
        let s = b.w(rec + 0x1e);
        b.set_w(rec + 0x1e, s & 0x3f);
        let d1 = ((s >> 6).wrapping_sub(cut)).wrapping_shl(6);
        b.set_w(rec + 0x1e, b.w(rec + 0x1e) | d1);
        let skip = (b.w(a3 + 0x14) as u32 * cut as u32) as u16 as i16 as i32 as u32;
        b.set_l(rec + 0x20, b.l(rec + 0x20).wrapping_add(skip));
        index = b.w(BD_CLEAN_TOP).wrapping_mul(2).wrapping_sub(d5);
    } else {
        index = y
            .wrapping_add(b.w(BD_CLEAN_TOP))
            .wrapping_mul(2)
            .wrapping_sub(d5);
    }
    a1 = a1.wrapping_add(line_offset_long(b, index));
    if (b.w(rec) as i16) < 0 && !n_clear(b.w(rec + 2).wrapping_sub(1)) {
        a1 = a1.wrapping_sub(0x30);
    }
    b.set_l(rec + 0x10, 0);
    let size = b.w(rec + 0x1e);
    if size & 0x3f == 0 || size & 0xffc0 == 0 {
        return;
    }
    let (dmod, amod) = (b.w(rec + 0x26), b.w(rec + 0x24));
    let mut src = b.l(rec + 0x20);
    let step = b.l(a3 + 0x16);
    let shift = (b.w(rec) & 0xf).wrapping_shl(12);
    c.spend(Cost::BobShown);
    c.custom(0x9a, 0x0010);
    for p in 0..5u32 {
        c.spend(Cost::BobPlane);
        c.blit_words(blit_words(size), 0x09f0);
        blit(
            &mut c.bus,
            Blit {
                con0: shift | 0x09f0,
                con1: shift,
                afwm: 0xffff,
                alwm: 0xffff,
                a: src,
                b: 0,
                c: 0,
                d: a1.wrapping_add(PLANE * p),
                amod,
                bmod: 0,
                cmod: 0,
                dmod,
                size,
            },
        );
        src = src.wrapping_add(step);
    }
    c.custom(0x9a, 0x8010);
}
