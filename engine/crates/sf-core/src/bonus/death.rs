//! A bonus enemy a shot kills, and the reward it leaves (h0+$238e2…$23f38). A shot takes one hit
//! point and half the shot power from the enemy; below none it falls away off the screen
//! ([`death_fall`]) and turns into a reward ([`become_reward`]), which floats left along a wave in
//! shooter mode and pays out when the frog takes it ([`touch_reward`]). [`become_reward`] is
//! shared with `touch_spud_damage` (h0+$2360e): a spud-killed enemy in worlds 1–6 leaves one too.

use super::shots::{SHOT_POWER, shots_faster};
use crate::frog::FROG;
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, nothing, obj_despawn_if_offscreen, obj_free};
use crate::level::{LevelCtx, SlotFn, n_clear};
use crate::mem::TILE_HEIGHTS;

/// `shooter_mode` (h0+$9dc).
const SHOOTER_MODE: u32 = h0(0x9dc);
/// `frog_lives` (h0+$82e).
const FROG_LIVES: u32 = h0(0x82e);
/// `map_rows` (h0+$17334).
const MAP_ROWS: u32 = h0(0x17334);
/// `reward_life_count` (h0+$23de4): shooter-mode rewards since the last life; every `$28`th is one.
pub const REWARD_LIFE_COUNT: u32 = h0(0x23de4);
/// `reward_power_count` (h0+$23e20): the other rewards since the last power-up; every tenth is
/// one. The executable starts it at 9.
pub const REWARD_POWER_COUNT: u32 = h0(0x23e20);

/// The routines' addresses, as slots name them.
pub const TOUCH_BONUS_ENEMY: u32 = h0(0x238e2);
const DEATH_FALL: u32 = h0(0x23956);
/// The reward's update in shooter mode (h0+$23e22); shots pass through a slot running it.
pub const REWARD_FLOAT: u32 = h0(0x23e22);
const REWARD_ARC: u32 = h0(0x23e6c);
const TOUCH_REWARD: u32 = h0(0x23eac);
const NOTHING: u32 = h0(0x7d4);

/// The bob of a shot enemy, falling away (h0+$32eb0).
const FALLING: u32 = h0(0x32eb0);
/// The reward's hitbox list (h0+$233f2).
const REWARD_HITBOXES: u32 = h0(0x233f2);
/// The reward's three looks, by kind: points (h0+$2cf58), a life (h0+$2cf64), the shot power
/// (h0+$2cf70). The life's in capitals: the purity test reads a literal ending `f64` as a float.
const REWARD_LOOKS: [u32; 3] = [h0(0x2cf58), h0(0x2CF64), h0(0x2cf70)];
/// The shooter-mode reward's wave (h0+$23de6: y offsets, ended by `$80`).
const WAVE: u32 = h0(0x23de6);
/// The reward's arc as it appears (h0+$23e90: y offsets, ended by `$7d00`), and an arc already
/// over (h0+$23e68).
const ARC: u32 = h0(0x23e90);
const ARC_OVER: u32 = h0(0x23e68);
const ARC_END: u16 = 0x7d00;

/// The routines here, by address, for the dispatch.
pub const ROUTINES: &[(u32, SlotFn)] = &[
    (TOUCH_BONUS_ENEMY, touch_bonus_enemy),
    (DEATH_FALL, death_fall),
    (REWARD_FLOAT, reward_float),
    (REWARD_ARC, reward_arc),
    (TOUCH_REWARD, touch_reward),
    (NOTHING, nothing),
];

/// `touch_bonus_enemy` (h0+$238e2): a hit costs `+$1c` one point and half the shot power. Below
/// none the enemy falls away: no hitboxes, no touch, sfx 1 on channel 3.
pub fn touch_bonus_enemy(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let hp = b
        .w(slot + 0x1c)
        .wrapping_sub(1)
        .wrapping_sub(b.w(SHOT_POWER) >> 1);
    b.set_w(slot + 0x1c, hp);
    if n_clear(hp) {
        return;
    }
    bob_anim(b, slot, FALLING);
    b.set_l(slot + 4, DEATH_FALL);
    b.set_l(slot + 0x14, 0);
    b.set_l(slot + 0x10, NOTHING);
    c.sfx(1, 3);
}

/// h0+$23956: the bob follows the slot; once the bob's `+$1a` reaches `$7530` (its fall is
/// over), the enemy becomes a reward 8 pixels right.
pub fn death_fall(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_follow(b, slot);
    let bob = b.l(slot + 0x50);
    if n_clear(b.w(bob + 0x1a).wrapping_sub(0x7530)) {
        become_reward(c, slot, 8);
    }
}

/// h0+$23c14: a killed enemy becomes a reward, moved `dx` right and set on the ground under it:
/// up out of solid cells (markers `$3a` and above, at most two), then down onto the cell below
/// unless its own is a slope (markers `$3e`, `$3f`), and onto that tile's height at its column,
/// 4 pixels up. In shooter mode it floats along the wave from `$18` higher for `$20` passes, and
/// its kind comes from the two counters; otherwise it rises on its arc and its kind is the RNG's
/// (`rng_range(2)`, drawn in both modes).
pub fn become_reward(c: &mut LevelCtx, slot: u32, dx: u16) {
    let b = &mut c.bus;
    b.set_l(slot + 0xc, 0);
    let x = b.w(slot + 8).wrapping_add(dx);
    b.set_w(slot + 8, x);
    b.set_w(slot + 0xa, (b.w(slot + 0xa) & 0xfff0).wrapping_add(0x10));
    let col = (x.wrapping_sub(8) >> 4).wrapping_mul(2);
    let row = (b.w(slot + 0xa) >> 4).wrapping_mul(4);
    let mut cell = b
        .l(MAP_ROWS.wrapping_add(row as i16 as u32))
        .wrapping_add(col as u32)
        .wrapping_add(2);
    for _ in 0..2 {
        if n_clear((b.w(cell.wrapping_sub(0x260)) & 0x3f).wrapping_sub(0x3a)) {
            cell = cell.wrapping_sub(0x260);
            b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_sub(0x10));
        }
    }
    let v = b.w(cell);
    if !matches!(v & 0x3f, 0x3e | 0x3f) {
        b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(0x10));
    }
    let heights = TILE_HEIGHTS
        .wrapping_add(((v & 0xffc0) >> 6) as u32 * 16)
        .wrapping_add((b.w(slot + 8).wrapping_sub(8) & 0xf) as u32);
    let h = b.b(heights) as i8 as i16 as u16;
    let ground = (b.w(slot + 0xa) & 0xfff0)
        .wrapping_add(0x10)
        .wrapping_sub(h);
    b.set_w(slot + 0xa, ground.wrapping_sub(4));
    b.set_l(slot + 4, REWARD_ARC);
    let shooter = b.w(SHOOTER_MODE) != 0;
    if shooter {
        b.set_l(slot + 4, REWARD_FLOAT);
        b.set_w(slot + 0xe, 0xfff8);
        b.set_w(slot + 0x5c, 0x20);
        b.set_l(slot + 0x68, 0);
        let y = b.w(slot + 0xa).wrapping_sub(0x18);
        b.set_w(slot + 0xa, y);
        b.set_w(slot + 0x60, y);
        b.set_l(slot + 0x6c, WAVE);
        b.set_l(slot + 0x70, WAVE);
    }
    b.set_l(slot + 0x14, REWARD_HITBOXES);
    b.set_l(slot + 0x10, TOUCH_REWARD);
    b.set_w(slot + 0x2a, 1);
    b.set_w(slot + 0x2c, 0);
    b.set_w(slot + 0x2e, 0);
    b.set_w(slot + 0x30, 1);
    let mut kind = c.rng_range(2);
    if shooter {
        let b = &mut c.bus;
        kind = 0;
        let lives = b.w(REWARD_LIFE_COUNT).wrapping_add(1);
        b.set_w(REWARD_LIFE_COUNT, lives);
        if n_clear(lives.wrapping_sub(0x28)) {
            b.set_w(REWARD_LIFE_COUNT, 0);
            kind = 1;
        } else {
            let power = b.w(REWARD_POWER_COUNT).wrapping_add(1);
            b.set_w(REWARD_POWER_COUNT, power);
            if n_clear(power.wrapping_sub(10)) {
                b.set_w(REWARD_POWER_COUNT, 0);
                kind = 2;
            }
        }
    }
    let b = &mut c.bus;
    bob_anim(b, slot, REWARD_LOOKS[(kind as usize).min(2)]);
    b.set_l(slot + 0x64, b.l(slot + 8));
    b.set_l(slot + 0x54, ARC);
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// h0+$23e22, the reward's update in shooter mode: one pixel left a pass, its y the wave's next
/// offset from `+$60` (the wave starting over at its `$80`), its arc over at once; freed after
/// `$20` passes (`+$5c`).
pub fn reward_float(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let mut at = b.l(slot + 0x6c);
    if b.w(at) == 0x80 {
        at = b.l(slot + 0x70);
        b.set_l(slot + 0x6c, at);
    }
    b.set_l(slot + 0x6c, b.l(slot + 0x6c).wrapping_add(2));
    b.set_w(slot + 0xa, b.w(slot + 0x60).wrapping_add(b.w(at)));
    b.set_w(slot + 8, b.w(slot + 8).wrapping_sub(1));
    bob_follow(b, slot);
    b.set_l(slot + 0x54, ARC_OVER);
    let t = b.w(slot + 0x5c).wrapping_sub(1);
    b.set_w(slot + 0x5c, t);
    if t == 0 {
        obj_free(c, slot, 0);
    }
}

/// h0+$23e6c, the reward's update outside shooter mode: its y the arc's next offset from where it
/// became one (`+$66`), until the arc's `$7d00`.
pub fn reward_arc(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let at = b.l(slot + 0x54);
    let dy = b.w(at);
    if dy != ARC_END {
        b.set_l(slot + 0x54, at.wrapping_add(2));
        b.set_w(slot + 0xa, b.w(slot + 0x66).wrapping_add(dy));
    }
    bob_follow(b, slot);
}

/// h0+$23eac, the reward's touch: once its arc is over the frog takes it. In shooter mode sfx 2
/// on channel 3, and by its look: a life, or the shot power raised (every shot one faster first
/// when it was already on). Then sfx 5 on channel 3, 200 points, the frog's `+$7e` one up, and the
/// slot freed.
pub fn touch_reward(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &c.bus;
    if b.w(b.l(slot + 0x54)) != ARC_END {
        return;
    }
    if b.w(SHOOTER_MODE) != 0 {
        c.sfx(2, 3);
        let b = &mut c.bus;
        let look = b.l(b.l(slot + 0x50) + 8);
        if look == REWARD_LOOKS[1] {
            b.set_w(FROG_LIVES, b.w(FROG_LIVES).wrapping_add(1));
        } else if look == REWARD_LOOKS[2] {
            if b.w(SHOT_POWER) != 0 {
                shots_faster(c);
            }
            let b = &mut c.bus;
            b.set_w(SHOT_POWER, b.w(SHOT_POWER).wrapping_add(1));
        }
    }
    c.sfx(5, 3);
    let b = &mut c.bus;
    b.set_l(FROG + 0x5e, b.l(FROG + 0x5e).wrapping_add(0xc8));
    b.set_w(FROG + 0x7e, b.w(FROG + 0x7e).wrapping_add(1));
    obj_free(c, slot, 0);
}
