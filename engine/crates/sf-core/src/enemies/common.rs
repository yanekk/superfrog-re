//! What the enemies of every world share: the touch handlers their templates name (docs/09 "Touch
//! handlers"), what a killed or stunned enemy turns into, and the map look-ups their updates make.

use crate::layout::h0;
use crate::level::objects::{OBJ_SLOTS, bob_anim, bob_follow, obj_despawn_if_offscreen, obj_free};
use crate::level::{LevelCtx, SlotFn, n_clear};
use crate::mem::Bus;

/// `map_rows` (h0+$17334): a pointer to each map row, from one row above the first.
pub const MAP_ROWS: u32 = h0(0x17334);
/// `score` (h0+$834).
const SCORE: u32 = h0(0x834);

/// The touch handlers' addresses, as templates name them.
pub const TOUCH_KILL_1HIT: u32 = h0(0x23582);
pub const TOUCH_SPUD_DAMAGE: u32 = h0(0x2360e);
pub const TOUCH_HURT: u32 = h0(0x236ac);
/// What a killed enemy runs: it falls (h0+$23b34).
pub const KILLED_FALL: u32 = h0(0x23b34);
/// What a stunned enemy runs (h0+$23bee), and its touch (h0+$23662).
pub const STUNNED: u32 = h0(0x23bee);
pub const TOUCH_STUNNED: u32 = h0(0x23662);
/// The slot routine that does nothing (`rts`).
pub const NOTHING: u32 = h0(0x7d4);

/// The look of a killed enemy that has landed (h0+$2cef4), and the frame it ends on (h0+$2af0a).
const LANDED: u32 = h0(0x2cef4);
const LANDED_LAST: u32 = h0(0x2af0a);

/// The shared routines, by address, for the dispatch.
pub const ROUTINES: &[(u32, SlotFn)] = &[
    (TOUCH_KILL_1HIT, touch_kill_1hit),
    (KILLED_FALL, killed_fall),
    (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    (STUNNED, stunned),
    (TOUCH_STUNNED, touch_stunned),
    (TOUCH_HURT, touch_hurt),
];

/// The map cell `col_bytes` (a column times two) into the row whose `map_rows` entry is
/// `row_index` bytes in (a row times four): the row's index a signed word, the column unsigned,
/// as `movea.l (a3, d1.w), a2` then `adda.l d0, a2` with a cleared `d0` take them.
pub fn map_cell(b: &Bus, col_bytes: u16, row_index: u16) -> u32 {
    b.l(MAP_ROWS.wrapping_add(row_index as i16 as i32 as u32))
        .wrapping_add(col_bytes as u32)
}

/// The cell under pixel `x`, `y`, one column left: `(x >> 4) * 2` into row `(y >> 4)`, then 2
/// bytes on (the pointers start one cell early).
pub fn cell_at(b: &Bus, x: u16, y: u16) -> u32 {
    map_cell(b, (x >> 4).wrapping_mul(2), (y >> 4).wrapping_mul(4)).wrapping_add(2)
}

/// Whether a cell's marker is solid ground (`$3a` and above, as `cmpi.w #$3a; bmi` tests it).
pub fn solid(b: &Bus, cell: u32) -> bool {
    n_clear((b.w(cell) & 0x3f).wrapping_sub(0x3a))
}

/// The row base an init keeps at `+$64` for a walker: its cell less its column (`ext.l` then
/// `lsr.w`: a negative x leaves the high word set), `rows` bytes on.
pub fn row_base(b: &Bus, slot: u32, rows: u32) -> u32 {
    let x = b.w(slot + 8);
    let high = if x & 0x8000 != 0 { 0xffff_0000 } else { 0 };
    let col = high | (x >> 4).wrapping_mul(2) as u32;
    b.l(slot + 0x40).wrapping_sub(col).wrapping_add(rows)
}

/// `touch_kill_1hit` (h0+$23582): a hit costs `$20` points; below none the enemy is killed: it
/// falls ([`killed_fall`]), touches nothing, its bob's counter is `$7d00`, its marker is cleared
/// so it stays dead, and sfx 7 or 8 (the RNG's) plays on channel 3.
pub fn touch_kill_1hit(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let hp = b.w(slot + 0x1c).wrapping_sub(0x20);
    b.set_w(slot + 0x1c, hp);
    if n_clear(hp) {
        return;
    }
    b.set_l(slot + 4, KILLED_FALL);
    b.set_l(slot + 0x10, NOTHING);
    let bob = b.l(slot + 0x50);
    b.set_w(bob + 0x1a, 0x7d00);
    let cell = b.l(slot + 0x40);
    b.set_w(cell, b.w(cell) & 0xffc0);
    let effect = c.rng_range(1).wrapping_add(7);
    c.sfx(effect as u8, 3);
}

/// h0+$23b34: a killed enemy falls (speed up to 8), harmless and evictable, until its centre
/// reaches solid ground; there it shows [`LANDED`], and once that has played out it is gone with
/// `$c8` points. Until then it keeps falling at -1.
pub fn killed_fall(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let mut vy = b.w(slot + 0xe).wrapping_add(1);
    b.set_w(slot + 0xe, vy);
    if n_clear(vy.wrapping_sub(8)) {
        vy = 8;
        b.set_w(slot + 0xe, vy);
    }
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(vy));
    bob_follow(b, slot);
    b.set_w(slot + 0x2a, 1);
    b.set_w(slot + 0x2c, 0);
    b.set_w(slot + 0x2e, 0);
    b.set_w(slot + 0x30, 1);
    let cell = cell_at(
        b,
        b.w(slot + 8).wrapping_add(8),
        b.w(slot + 0xa).wrapping_add(8),
    );
    if !solid(b, cell) {
        return;
    }
    let bob = b.l(slot + 0x50);
    if b.l(bob + 8) != LANDED {
        bob_anim(b, slot, LANDED);
    }
    if b.l(b.l(bob + 0x16)) != LANDED_LAST {
        b.set_w(slot + 0xe, 0xffff);
        return;
    }
    b.set_l(SCORE, b.l(SCORE).wrapping_add(0xc8));
    obj_free(c, slot, 0);
}

/// `touch_spud_damage` (h0+$2360e): a hit costs `$20` points; below none the enemy is stunned for
/// 4 passes: harmless, showing its hit animation (`+$18`), its marker cleared, sfx 6 on channel 2.
pub fn touch_spud_damage(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let hp = b.w(slot + 0x1c).wrapping_sub(0x20);
    b.set_w(slot + 0x1c, hp);
    if n_clear(hp) {
        return;
    }
    b.set_w(slot + 0x5c, 4);
    b.set_w(slot + 0x30, 1);
    b.set_l(slot + 4, STUNNED);
    b.set_l(slot + 0x10, TOUCH_STUNNED);
    let anim = b.l(slot + 0x18);
    bob_anim(b, slot, anim);
    let cell = b.l(slot + 0x40);
    b.set_w(cell, b.w(cell) & 0xffc0);
    c.sfx(6, 2);
}

/// h0+$23bee: a stunned enemy counts its passes down (`+$5c`) and can be stood on.
pub fn stunned(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    if n_clear(b.w(slot + 0x5c)) {
        b.set_w(slot + 0x5c, b.w(slot + 0x5c).wrapping_sub(1));
    }
    b.set_w(slot + 0x2c, 0);
    b.set_w(slot + 0x2e, 1);
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// h0+$23662: touched once its count is out, a stunned enemy gives sfx 6 on channel 2, now and
/// then (the RNG's 0..=`$64` below `$1e`) sample 2, and becomes a reward 8 pixels right.
pub fn touch_stunned(c: &mut LevelCtx, slot: u32, _d7: u16) {
    if n_clear(c.bus.w(slot + 0x5c)) {
        return;
    }
    c.sfx(6, 2);
    if !n_clear(c.rng_range(0x64).wrapping_sub(0x1e)) {
        c.sample(2);
    }
    crate::bonus::death::become_reward(c, slot, 8);
}

/// `touch_hurt` (h0+$236ac): the frog is hurt.
pub fn touch_hurt(c: &mut LevelCtx, slot: u32, _d7: u16) {
    crate::level::death::frog_hurt(c, slot);
}

/// h0+$1c462: a slot's y follows a path of words at `+$54` (starting over from `+$58` at a `$ffff`
/// word): the next one plus `dy`.
pub fn obj_path_y(b: &mut Bus, slot: u32, dy: u16) {
    let mut at = b.l(slot + 0x54);
    if b.w(at) == 0xffff {
        at = b.l(slot + 0x58);
        b.set_l(slot + 0x54, at);
    }
    b.set_w(slot + 0xa, b.w(at).wrapping_add(dy));
    b.set_l(slot + 0x54, b.l(slot + 0x54).wrapping_add(2));
}

/// How far a pair's marker looks for its partner: `$98` cells.
const PARTNER_REACH: u32 = 0x98;

/// Whether a slot is live on map cell `cell`.
fn cell_live(b: &Bus, cell: u32) -> bool {
    (0..20).any(|k| b.l(OBJ_SLOTS + k * 0x7c + 0x40) == cell)
}

/// h0+$1b350: whether the first cell right of `cell` (within `$98`) holding `marker` has no live
/// slot; false too when there is none. A pair's left end spawns only then.
pub fn partner_right_free(b: &Bus, cell: u32, marker: u16) -> bool {
    (1..=PARTNER_REACH)
        .map(|k| cell.wrapping_add(2 * k))
        .find(|&a| b.w(a) & 0x3f == marker)
        .is_some_and(|a| !cell_live(b, a))
}

/// h0+$1b396: [`partner_right_free`] looking left.
pub fn partner_left_free(b: &Bus, cell: u32, marker: u16) -> bool {
    (1..=PARTNER_REACH)
        .map(|k| cell.wrapping_sub(2 * k))
        .find(|&a| b.w(a) & 0x3f == marker)
        .is_some_and(|a| !cell_live(b, a))
}
