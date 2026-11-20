//! Templates 53 (marker 18) and 54 (marker 19): a dart that waits hidden, flies out from its spawn
//! point until it meets a solid cell, and hides again; it hurts (`touch_hurt`) while flying, and
//! on contact with its spawn point while it waits there passive. Only the inits are world 2's:
//! the wait and the flight are [`crate::enemies::w3::dart`]'s (h0+$25fa8, h0+$25f42).

use crate::enemies::w3::dart::{FLY, WAIT, fly, wait};
use crate::layout::h0;
use crate::level::objects::bob_anim;
use crate::level::{LevelCtx, ObjectKind};

pub static DART: ObjectKind = ObjectKind {
    name: "dart (marker 18)",
    routines: &[
        (h0(0x25e2a), |c, s, _| dart_init(c, s, h0(0x316f0))),
        (WAIT, wait),
        (FLY, fly),
    ],
};

pub static DART_B: ObjectKind = ObjectKind {
    name: "dart (marker 19)",
    routines: &[
        (h0(0x25e46), |c, s, _| dart_init(c, s, h0(0x316fc))),
        (WAIT, wait),
        (FLY, fly),
    ],
};

/// h0+$25e2a / h0+$25e46: `$20` passes before it flies.
fn dart_init(c: &mut LevelCtx, slot: u32, look: u32) {
    bob_anim(&mut c.bus, slot, look);
    c.bus.set_w(slot + 0x64, 0x20);
}
