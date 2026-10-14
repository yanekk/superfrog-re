//! What the enemies of worlds 1–6 share (T39–T44): the two touch handlers most of them name, what
//! a killed or stomped enemy turns into, the map tests their updates make, and the direction from
//! one point to another (h0+$da62) that the watchers face by.

use crate::bonus::death::become_reward;
use crate::items::SCORE;
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, obj_despawn_if_offscreen, obj_free};
use crate::level::{LevelCtx, SlotFn, n_clear};
use crate::mem::Bus;

/// `map_rows` (h0+$17334): a pointer to each map row, from one row above the first.
const MAP_ROWS: u32 = h0(0x17334);
/// A slot routine that does nothing (h0+$7d4).
pub const NOTHING: u32 = h0(0x7d4);

/// `touch_kill_1hit` (h0+$23582).
pub const TOUCH_KILL_1HIT: u32 = h0(0x23582);
/// h0+$23b34: the update of an enemy [`touch_kill_1hit`] killed.
const KILL_FALL: u32 = h0(0x23b34);
/// `touch_spud_damage` (h0+$2360e).
pub const TOUCH_SPUD_DAMAGE: u32 = h0(0x2360e);
/// h0+$23bee and h0+$23662: the update and touch of an enemy [`touch_spud_damage`] stomped.
const STOMPED: u32 = h0(0x23bee);
const TOUCH_STOMPED: u32 = h0(0x23662);

/// The splat a falling enemy shows on the ground (h0+$2cef4), and the step its animation ends on
/// (h0+$2af0a).
pub const SPLAT: u32 = h0(0x2cef4);
pub const ANIM_END: u32 = h0(0x2af0a);

/// The routines here, by address, for the dispatch.
pub const ROUTINES: &[(u32, SlotFn)] = &[
    (TOUCH_KILL_1HIT, touch_kill_1hit),
    (KILL_FALL, kill_fall),
    (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    (STOMPED, stomped),
    (TOUCH_STOMPED, touch_stomped),
    (NOTHING, crate::level::objects::nothing),
];

/// The marker of the map cell `cols` bytes into the row the word `row4` indexes in `map_rows`
/// (`movea.l (a3,d1.w),a2; adda.l d0,a2`), plus `extra` bytes: what `(a2)` or `2(a2)` reads.
pub fn marker_at(b: &Bus, cols: u32, row4: u16, extra: u32) -> u16 {
    let row = b.l(MAP_ROWS.wrapping_add(row4 as i16 as i32 as u32));
    b.w(row.wrapping_add(cols).wrapping_add(extra)) & 0x3f
}

/// The common map test (h0+$24854 and others): the marker of the cell right of the one under
/// pixel `x`, `y` (`(x >> 4) * 2` bytes in, row `y >> 4`).
pub fn marker_right_of(b: &Bus, x: u16, y: u16) -> u16 {
    let cols = ((x >> 4).wrapping_add(x >> 4)) as u32;
    let row4 = (y >> 4).wrapping_mul(4);
    marker_at(b, cols, row4, 2)
}

/// Whether a marker is a solid cell's (`$3a` and above).
pub fn solid(marker: u16) -> bool {
    n_clear(marker.wrapping_sub(0x3a))
}

/// The base a walker keeps in `+$64` to find the cells of its row from its x: its cell, less its
/// column's bytes, plus `extra` (`$260`: a row down). The column comes from `ext.l`'d x, so its
/// high word is x's sign (h0+$2494e).
pub fn row_base(b: &Bus, slot: u32, extra: u32) -> u32 {
    let x = b.w(slot + 8);
    let d1 = (x as i16 as i32 as u32 & 0xffff_0000) | ((x >> 4).wrapping_add(x >> 4)) as u32;
    b.l(slot + 0x40).wrapping_sub(d1).wrapping_add(extra)
}

/// A walker's step (h0+$2496a, $248e2, $247d0): x moves by `+$0c`; at a cell marked `$25` on its
/// row it turns, taking `left` going left and `right` going right.
pub fn walk_and_turn(b: &mut Bus, slot: u32, left: u32, right: u32) {
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    let x = b.w(slot + 8);
    let at = ((x >> 4).wrapping_add(x >> 4) as u32).wrapping_add(b.l(slot + 0x64));
    if b.w(at.wrapping_sub(0x25e)) & 0x3f == 0x25 {
        let vx = b.w(slot + 0xc).wrapping_neg();
        b.set_w(slot + 0xc, vx);
        bob_anim(b, slot, if n_clear(vx) { right } else { left });
    }
}

/// The end most updates share: the bob takes the slot's position, then the slot is freed if the
/// camera has left it (`move.l 8(a0),-4(a1); jmp obj_despawn_if_offscreen`).
pub fn follow_and_despawn(c: &mut LevelCtx, slot: u32) {
    bob_follow(&mut c.bus, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// `touch_kill_1hit` (h0+$23582): a hit takes `$20` hit points; below none the enemy dies: it
/// falls ([`kill_fall`]), touches nothing more, its bob's step timer runs out (`$7d00`), its
/// marker is cleared so it stays dead, and sfx 7 or 8 (the RNG's) plays on channel 3.
pub fn touch_kill_1hit(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let hp = b.w(slot + 0x1c).wrapping_sub(0x20);
    b.set_w(slot + 0x1c, hp);
    if n_clear(hp) {
        return;
    }
    b.set_l(slot + 4, KILL_FALL);
    b.set_l(slot + 0x10, NOTHING);
    let bob = b.l(slot + 0x50);
    b.set_w(bob + 0x1a, 0x7d00);
    let cell = b.l(slot + 0x40);
    b.set_w(cell, b.w(cell) & 0xffc0);
    let effect = c.rng_range(1).wrapping_add(7);
    c.sfx(effect as u8, 3);
}

/// h0+$23b34: a killed enemy falls, faster each pass up to 8 pixels; passive and evictable. Once
/// the cell under its middle is solid it splats; when the splat's animation is over it pays 200
/// points and is freed, until then it sits (`+$0e` -1, which the next pass makes 0).
pub fn kill_fall(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let mut vy = b.w(slot + 0xe).wrapping_add(1);
    if n_clear(vy.wrapping_sub(8)) {
        vy = 8;
    }
    b.set_w(slot + 0xe, vy);
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(vy));
    bob_follow(b, slot);
    b.set_w(slot + 0x2a, 1);
    b.set_w(slot + 0x2c, 0);
    b.set_w(slot + 0x2e, 0);
    b.set_w(slot + 0x30, 1);
    let (x, y) = (b.w(slot + 8).wrapping_add(8), b.w(slot + 0xa).wrapping_add(8));
    if !solid(marker_right_of(b, x, y)) {
        return;
    }
    let bob = b.l(slot + 0x50);
    if b.l(bob + 8) != SPLAT {
        bob_anim(b, slot, SPLAT);
    }
    if b.l(b.l(bob + 0x16)) == ANIM_END {
        b.set_l(SCORE, b.l(SCORE).wrapping_add(0xc8));
        obj_free(c, slot, 0);
    } else {
        b.set_w(slot + 0xe, 0xffff);
    }
}

/// `touch_spud_damage` (h0+$2360e): despite its name the spud passes over an enemy with it
/// (`collide_all`); a stomp takes `$20` hit points. Below none the enemy is flattened: passive,
/// its death animation (`+$18`), [`stomped`] for 4 passes then [`touch_stomped`], its marker
/// cleared; sfx 6 on channel 2.
pub fn touch_spud_damage(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let hp = b.w(slot + 0x1c).wrapping_sub(0x20);
    b.set_w(slot + 0x1c, hp);
    if n_clear(hp) {
        return;
    }
    b.set_w(slot + 0x5c, 4);
    b.set_w(slot + 0x30, 1);
    b.set_l(slot + 4, STOMPED);
    b.set_l(slot + 0x10, TOUCH_STOMPED);
    let death = b.l(slot + 0x18);
    bob_anim(b, slot, death);
    let cell = b.l(slot + 0x40);
    b.set_w(cell, b.w(cell) & 0xffc0);
    c.sfx(6, 2);
}

/// h0+$23bee: a stomped enemy counts its 4 passes down (`+$5c`, to -1) and is stompable again.
pub fn stomped(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    if n_clear(b.w(slot + 0x5c)) {
        b.set_w(slot + 0x5c, b.w(slot + 0x5c).wrapping_sub(1));
    }
    b.set_w(slot + 0x2c, 0);
    b.set_w(slot + 0x2e, 1);
    follow_and_despawn(c, slot);
}

/// h0+$23662: touched once its count is over, a stomped enemy plays sfx 6 on channel 2, now and
/// then (the RNG under 30 of 101) sample 2, and becomes a reward 8 pixels right.
pub fn touch_stomped(c: &mut LevelCtx, slot: u32, _d7: u16) {
    if n_clear(c.bus.w(slot + 0x5c)) {
        return;
    }
    c.sfx(6, 2);
    if !n_clear(c.rng_range(0x64).wrapping_sub(0x1e)) {
        c.sample(2);
    }
    become_reward(c, slot, 8);
}

/// h0+$da62: the direction from (`x0`, `y0`) to (`x1`, `y1`), screen coordinates, as one of 16
/// (0 right, 4 up, 8 left, 12 down; the original's `d1`). Each coordinate is clamped to the
/// screen first. The steepness is `|dy| * $c0 / |dx|` against four thresholds, compared signed,
/// so a quotient of `$8000` and over counts as flattest (the original's).
pub fn direction(x0: u16, y0: u16, x1: u16, y1: u16) -> u8 {
    let clamp = |v: u16, top: i32| (v as i16 as i32).clamp(0, top - 1) as u16;
    let (x0, x1) = (clamp(x0, 0x154), clamp(x1, 0x154));
    let (y0, y1) = (clamp(y0, 0xf0), clamp(y1, 0xf0));
    let mut dy = 0xf0u16.wrapping_sub(y1).wrapping_sub(0xf0u16.wrapping_sub(y0));
    let mut dx = x1.wrapping_sub(x0);
    if dx == 0 {
        return if n_clear(dy) { 0 } else { 8 };
    }
    let quadrant = match (n_clear(dy), n_clear(dx)) {
        (true, true) => 1,
        (true, false) => {
            dx = dx.wrapping_neg();
            2
        }
        (false, false) => {
            dy = dy.wrapping_neg();
            dx = dx.wrapping_neg();
            3
        }
        (false, true) => {
            dy = dy.wrapping_neg();
            4
        }
    };
    let q = (dy as u32 * 0xc0 / dx as u32) as u16 as i16;
    let steps = [0x3c5, 0x11f, 0x80, 0x26]
        .iter()
        .take_while(|&&t| q < t)
        .count() as u8;
    match quadrant {
        1 => steps,
        2 if steps == 0 => 0,
        2 => 0x10 - steps,
        3 => steps + 8,
        _ => 8 - steps,
    }
}
