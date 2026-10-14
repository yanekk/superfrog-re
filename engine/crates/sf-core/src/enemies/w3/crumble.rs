//! Template 150 (marker 36): `w3_crumble`, a platform that holds the frog while its animation
//! runs and gives way at its last step. Standing on it (falling onto it, its animation `$200`
//! steps in) restarts the animation; it plays sample 4 then. Passive: it never hurts.

use crate::enemies::common::follow_and_despawn;
use crate::frog::{FROG, f};
use crate::layout::h0;
use crate::level::objects::bob_anim;
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// The step its cells give way on (h0+$2e8ac).
const GIVEN_WAY: u32 = h0(0x2e8ac);

pub static CRUMBLE: ObjectKind = ObjectKind {
    name: "w3 crumble",
    routines: &[
        (h0(0x27466), init),
        (h0(0x2747c), update),
        (h0(0x23774), touch),
    ],
};

/// `w3_crumble_init` (h0+$27466).
fn init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x321d4));
}

/// `w3_crumble_update` (h0+$2747c): the two cells below its own are made solid (marker `$3e`,
/// their tiles kept), unless its bob's step is the last, when they are cleared (marker 0).
fn update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let at = b.l(slot + 0x40).wrapping_add(0x260);
    for a in [at, at + 2] {
        b.set_w(a, b.w(a) & 0xffc0 | 0x3e);
    }
    let bob = b.l(slot + 0x50);
    if b.l(b.l(bob + 0x16)) == GIVEN_WAY {
        for a in [at, at + 2] {
            b.set_w(a, b.w(a) & 0xffc0);
        }
    }
    follow_and_despawn(c, slot);
}

/// `touch_crumble` (h0+$23774): from its animation's `$200`th step timer on, the frog falling or
/// still: sample 4, the frog's fall stopped once it is 8 or faster, and on the ground the step
/// timer restarts.
fn touch(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &c.bus;
    let bob = b.l(slot + 0x50);
    if !n_clear(b.w(bob + 0x1a).wrapping_sub(0x200)) || !n_clear(b.w(FROG + f::VY)) {
        return;
    }
    c.sample(4);
    let b = &mut c.bus;
    if n_clear(b.w(FROG + f::VY).wrapping_sub(8)) {
        b.set_w(FROG + f::VY, 0);
    }
    if b.w(FROG + f::ON_GROUND) == 0 {
        return;
    }
    b.set_w(bob + 0x1a, 0);
}
