//! Templates 24, 25 and 26 (markers 13–15): scenery that only animates; passive, no hitbox.

use crate::enemies::w2::backdrop::still;
use crate::layout::h0;
use crate::level::ObjectKind;
use crate::level::objects::bob_anim;

const STILL: u32 = h0(0x24ef6);

pub static SCENERY_A: ObjectKind = ObjectKind {
    name: "scenery (marker 13)",
    routines: &[
        (h0(0x24e88), |c, s, _| bob_anim(&mut c.bus, s, h0(0x31af8))),
        (STILL, still),
    ],
};

pub static SCENERY_B: ObjectKind = ObjectKind {
    name: "scenery (marker 14)",
    routines: &[
        (h0(0x24e9e), |c, s, _| bob_anim(&mut c.bus, s, h0(0x31b0c))),
        (STILL, still),
    ],
};

pub static SCENERY_C: ObjectKind = ObjectKind {
    name: "scenery (marker 15)",
    routines: &[
        (h0(0x24eb4), |c, s, _| bob_anim(&mut c.bus, s, h0(0x31b38))),
        (STILL, still),
    ],
};
