//! Template 39 (marker 12): walks a step on one frame of its walk, turning at a marker `$25` in
//! the row above; the spud stuns it (`touch_spud_damage`), then a stomp makes it a reward.

use crate::enemies::common::{follow_and_despawn, row_base};
use crate::enemies::w2::marker;
use crate::layout::h0;
use crate::level::objects::bob_anim;
use crate::level::{LevelCtx, ObjectKind, n_clear};

const WALK_LEFT: u32 = h0(0x318f0);
const WALK_RIGHT: u32 = h0(0x319f4);
/// The death animations it takes facing left and right.
const DIE_LEFT: u32 = h0(0x32080);
const DIE_RIGHT: u32 = h0(0x32074);

pub static WALKER: ObjectKind = ObjectKind {
    name: "walker",
    routines: &[(h0(0x256b6), walker_init), (h0(0x256ea), walker_update)],
};

/// h0+$256b6: `+$64` the start of the map row below its cell.
fn walker_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    bob_anim(b, slot, WALK_LEFT);
    let row = row_base(b, slot, 0x260);
    b.set_l(slot + 0x64, row);
    b.set_w(slot + 0x5c, 0);
}

/// h0+$256ea: `vx` along while the walk shows its step frame (`$f8` into the left walk, `$f0`
/// into the right); at a marker `$25` above and right of its column it turns round.
fn walker_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    let vx = b.w(slot + 0xc);
    let step = b
        .l(bob + 8)
        .wrapping_add(if n_clear(vx) { 0xf0 } else { 0xf8 });
    let dx = if step == b.l(bob + 0x16) { vx } else { 0 };
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(dx));
    let at = ((b.w(slot + 8).wrapping_add(3) >> 4).wrapping_mul(2) as u32)
        .wrapping_add(b.l(slot + 0x64))
        .wrapping_sub(0x25e);
    if marker(b, at) == 0x25 {
        let vx = vx.wrapping_neg();
        b.set_w(slot + 0xc, vx);
        b.set_w(slot + 8, b.w(slot + 8).wrapping_add(vx));
        let (walk, die) = if n_clear(vx) {
            (WALK_RIGHT, DIE_RIGHT)
        } else {
            (WALK_LEFT, DIE_LEFT)
        };
        b.set_l(slot + 0x18, die);
        bob_anim(b, slot, walk);
    }
    follow_and_despawn(c, slot);
}
