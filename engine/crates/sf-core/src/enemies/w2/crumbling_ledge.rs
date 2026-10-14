//! Template 149 (marker 36): a ledge the frog can stand on (it writes solid markers `$3e` into the
//! two cells below its own); landing on it sets it crumbling, and once its animation reaches the
//! broken frame the cells are open again. Passive.

use crate::enemies::common::{follow_and_despawn, frog_not_rising};
use crate::frog::{FROG, f};
use crate::layout::h0;
use crate::level::objects::bob_anim;
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// The frame it shows when broken (h0+$2ebac).
const BROKEN: u32 = h0(0x2ebac);

pub static LEDGE: ObjectKind = ObjectKind {
    name: "crumbling ledge",
    routines: &[
        (h0(0x273f6), |c, s, _| bob_anim(&mut c.bus, s, h0(0x31bec))),
        (h0(0x2740c), ledge_update),
        (h0(0x23774), touch_ledge),
    ],
};

/// h0+$2740c: the two cells below its own solid (marker `$3e`), or not while it shows broken.
fn ledge_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let below = b.l(slot + 0x40).wrapping_add(0x260);
    let broken = b.l(b.l(b.l(slot + 0x50) + 0x16)) == BROKEN;
    for at in [below, below.wrapping_add(2)] {
        b.set_w(at, b.w(at) & 0xffc0 | 0x3e);
        if broken {
            b.set_w(at, b.w(at) & 0xffc0);
        }
    }
    follow_and_despawn(c, slot);
}

/// h0+$23774: the frog falling or standing on it while it is whole (its step's count `$200` or
/// more): a creak (sample 4), a fall of 8 or more stopped, and, with the frog on the ground, the
/// step's count cleared so the crumbling starts.
fn touch_ledge(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let bob = c.bus.l(slot + 0x50);
    if !n_clear(c.bus.w(bob + 0x1a).wrapping_sub(0x200)) || !frog_not_rising(&c.bus) {
        return;
    }
    c.sample(4);
    let b = &mut c.bus;
    if n_clear(b.w(FROG + f::VY).wrapping_sub(8)) {
        b.set_w(FROG + f::VY, 0);
    }
    if b.w(FROG + f::ON_GROUND) != 0 {
        b.set_w(bob + 0x1a, 0);
    }
}
