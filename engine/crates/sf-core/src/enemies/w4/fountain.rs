//! Template 151 (marker 26): a fountain. Each time its animation reaches h0+$2e36c it throws four
//! drops (templates 152–159) from 4 right and 6 below its position, by turns the four straight
//! ways (152, 154, 156, 158: up, right, down, left) and the four slanting ones (153, 155, 157,
//! 159). A drop flies at its template's speed until it meets solid ground. Drops are out of the
//! spud's reach (`$7d00`); contact hurts.

use crate::enemies::common::{NOTHING, TOUCH_SPUD_DAMAGE, map_cell, solid, touch_spud_damage};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{
    bob_anim, bob_follow, nothing, obj_despawn_if_offscreen, obj_free, obj_spawn_at_xy_nodedup,
    obj_spawn_cell,
};
use crate::level::{LevelCtx, ObjectKind};

/// The frame that throws.
const THROW: u32 = h0(0x2e36c);
/// The drops' update (h0+$276aa).
const DROP_UPDATE: u32 = h0(0x276aa);

/// Template 151.
pub static FOUNTAIN: ObjectKind = ObjectKind {
    name: "fountain",
    routines: &[
        (h0(0x274d6), fountain_init),
        (h0(0x274f0), fountain_update),
        (NOTHING, nothing),
    ],
};

/// Templates 152–159.
pub static SPRAY: ObjectKind = ObjectKind {
    name: "fountain drop",
    routines: &[
        (h0(0x275fa), |c, s, _| drop_init(c, s, h0(0x325ac))),
        (h0(0x27610), |c, s, _| drop_init(c, s, h0(0x325b8))),
        (h0(0x27626), |c, s, _| drop_init(c, s, h0(0x325c4))),
        (h0(0x2763c), |c, s, _| drop_init(c, s, h0(0x325d0))),
        (h0(0x27652), |c, s, _| drop_init(c, s, h0(0x325dc))),
        (h0(0x27668), |c, s, _| drop_init(c, s, h0(0x325e8))),
        (h0(0x2767e), |c, s, _| drop_init(c, s, h0(0x325f4))),
        (h0(0x27694), |c, s, _| drop_init(c, s, h0(0x32600))),
        (DROP_UPDATE, drop_update),
        (TOUCH_SPUD_DAMAGE, touch_spud_damage),
    ],
};

/// Marker 26 (h0+$187c2): `$c` down its cell.
pub fn spawn_151(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(151), cell, 0, 0xc);
}

/// h0+$274d6: its look (h0+$32588); the straight drops first.
fn fountain_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32588));
    c.bus.set_w(slot + 0x5c, 0);
}

/// h0+$274f0: at the throwing frame `+$5c` flips; set, the straight drops, clear, the slanting
/// ones; then its bob where it is, gone once off the screen.
fn fountain_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let bob = c.bus.l(slot + 0x50);
    if c.bus.l(c.bus.l(bob + 0x16)) == THROW {
        let t = !c.bus.w(slot + 0x5c);
        c.bus.set_w(slot + 0x5c, t);
        let first = if t != 0 { 152 } else { 153 };
        for k in 0..4 {
            let x = c.bus.w(slot + 8).wrapping_add(4);
            let y = c.bus.w(slot + 0xa).wrapping_add(6);
            obj_spawn_at_xy_nodedup(c, template(first + 2 * k), x, y);
        }
    }
    bob_follow(&mut c.bus, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// The drops' inits (h0+$275fa … h0+$27694, `$16` apart): a look each.
fn drop_init(c: &mut LevelCtx, slot: u32, look: u32) {
    bob_anim(&mut c.bus, slot, look);
}

/// h0+$276aa: it moves by `+$0c`, `+$0e`; gone at solid ground under (x, y − `$c`), else its bob
/// where it is, gone once off the screen.
fn drop_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    b.set_w(slot + 0xa, b.w(slot + 0xa).wrapping_add(b.w(slot + 0xe)));
    let x = b.w(slot + 8);
    let y = b.w(slot + 0xa).wrapping_add(4).wrapping_sub(0x10);
    let cell = map_cell(b, (x >> 4).wrapping_mul(2), (y >> 4).wrapping_mul(4));
    if solid(b, cell) {
        return obj_free(c, slot, 0);
    }
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}
