//! Templates 77 (marker 17) and 78 (marker 16): `w1_patroller`, which paces between the two
//! cells marked 16 and 17 on its row, 4 pixels a pass. 78 starts at the left end, 77 at the right
//! (`$20` left of its cell). It hurts on any touch and nothing kills it (`touch_kill_1hit` on
//! `$7d00` hit points).

use crate::enemies::common::{TOUCH_KILL_1HIT, follow_and_despawn, row_base, touch_kill_1hit};
use crate::layout::h0;
use crate::level::objects::bob_anim;
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// Its animations, left (h0+$3135c) and right (h0+$31398).
const LEFT: u32 = h0(0x3135c);
const RIGHT: u32 = h0(0x31398);

pub static PATROLLER_LEFT: ObjectKind = ObjectKind {
    name: "w1 patroller from the right end",
    routines: &[
        (h0(0x269be), init_77),
        (h0(0x26a2c), update),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

pub static PATROLLER_RIGHT: ObjectKind = ObjectKind {
    name: "w1 patroller from the left end",
    routines: &[
        (h0(0x269f8), init_78),
        (h0(0x26a2c), update),
        (TOUCH_KILL_1HIT, touch_kill_1hit),
    ],
};

/// The two inits' common part: its spawn position in `+$68`, its row's base in `+$64`.
fn init(c: &mut LevelCtx, slot: u32, anim: u32) {
    let b = &mut c.bus;
    bob_anim(b, slot, anim);
    b.set_l(slot + 0x68, b.l(slot + 8));
    b.set_l(slot + 0x64, 0);
    b.set_l(slot + 0x64, row_base(b, slot, 0));
}

/// `w1_patroller_77_init` (h0+$269be): going left, `$20` left of its cell.
fn init_77(c: &mut LevelCtx, slot: u32, _d7: u16) {
    init(c, slot, LEFT);
    c.bus.set_w(slot + 8, c.bus.w(slot + 8).wrapping_sub(0x20));
}

/// `w1_patroller_78_init` (h0+$269f8): going right.
fn init_78(c: &mut LevelCtx, slot: u32, _d7: u16) {
    init(c, slot, RIGHT);
}

/// `w1_patroller_update` (h0+$26a2c): x moves by `+$0c`; at a cell marked 16 or 17 two cells on
/// from the one under x − 16 it turns, and going right steps on twice its speed.
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    b.set_w(slot + 8, b.w(slot + 8).wrapping_add(b.w(slot + 0xc)));
    let x = b.w(slot + 8).wrapping_sub(0x10);
    let at = ((x >> 4).wrapping_add(x >> 4) as u32).wrapping_add(b.l(slot + 0x64)).wrapping_add(4);
    if matches!(b.w(at) & 0x3f, 0x10 | 0x11) {
        let vx = b.w(slot + 0xc).wrapping_neg();
        b.set_w(slot + 0xc, vx);
        bob_anim(b, slot, LEFT);
        if n_clear(vx) {
            bob_anim(b, slot, RIGHT);
            b.set_w(slot + 8, b.w(slot + 8).wrapping_add(vx.wrapping_add(vx)));
        }
    }
    follow_and_despawn(c, slot);
}
