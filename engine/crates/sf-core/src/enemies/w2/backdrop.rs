//! Template 52 (marker 10): scenery drawn into the background (its look's `+4` is negative), with
//! no hitbox.

use crate::enemies::common::follow_and_despawn;
use crate::layout::h0;
use crate::level::objects::bob_anim;
use crate::level::{LevelCtx, ObjectKind};

pub static BACKDROP: ObjectKind = ObjectKind {
    name: "backdrop",
    routines: &[(h0(0x25fe8), backdrop_init), (h0(0x25ffe), still)],
};

/// h0+$25fe8.
fn backdrop_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x31578));
}

/// h0+$25ffe (and h0+$24ef6, h0+$26692): it stays where it is until it is off the screen.
pub fn still(c: &mut LevelCtx, slot: u32, _d7: u16) {
    follow_and_despawn(c, slot);
}
