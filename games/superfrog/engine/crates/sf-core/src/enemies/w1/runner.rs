//! Templates 19 (marker 14) and 20 (marker 13): `w1_runner`, which runs right (19) or left (20)
//! at 4 pixels a pass until the cell ahead of it is solid or it leaves the map's left edge, and is
//! freed there. It hurts on any touch; a hit or stomp cannot kill it (`touch_kill_1hit` on
//! `$7d00` hit points), and it is not stompable.

use crate::enemies::common::{
    TOUCH_KILL_1HIT, follow_and_despawn, marker_right_of, solid, touch_kill_1hit,
};
use crate::layout::h0;
use crate::level::objects::{bob_anim, obj_free};
use crate::level::{LevelCtx, ObjectKind, n_clear};

pub static RUNNER_RIGHT: ObjectKind = ObjectKind {
    name: "w1 runner right",
    routines: &[
        (h0(0x2483e), init_right),
        (h0(0x24854), update),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

pub static RUNNER_LEFT: ObjectKind = ObjectKind {
    name: "w1 runner left",
    routines: &[
        (h0(0x24828), init_left),
        (h0(0x24854), update),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// `w1_runner_left_init` (h0+$24828): its animation (h0+$314c0).
fn init_left(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x314c0));
}

/// `w1_runner_right_init` (h0+$2483e): its animation (h0+$3147c).
fn init_right(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x3147c));
}

/// `w1_runner_update` (h0+$24854): freed when the cell right of the one 8 pixels above its
/// position is solid; otherwise x moves by `+$0c`, and it is freed once x is negative.
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let (x, y) = (b.w(slot + 8), b.w(slot + 0xa));
    if solid(marker_right_of(b, x, y.wrapping_sub(8))) {
        return obj_free(c, slot, 0);
    }
    let x = x.wrapping_add(b.w(slot + 0xc));
    b.set_w(slot + 8, x);
    if !n_clear(x) {
        return obj_free(c, slot, 0);
    }
    follow_and_despawn(c, slot);
}
