//! Templates 57 and 58 (markers 18 and 19): a dart that waits hidden at its spawn point, then
//! flies left (57) or right (58) at 8 pixels a pass until it reaches solid ground, and waits again.
//! Only the darts' inits are W6's; their update is W3's (`enemies::w3::dart`), shared with
//! templates 53–62.

use crate::enemies::w3::dart::{FLY, WAIT, fly, wait};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind};

/// Template 57: flies left.
pub static DART_LEFT: ObjectKind = ObjectKind {
    name: "dart (left)",
    routines: &[
        (h0(0x25e9a), dart_left_init),
        (WAIT, wait),
        (FLY, fly),
    ],
};

/// Template 58: flies right.
pub static DART_RIGHT: ObjectKind = ObjectKind {
    name: "dart (right)",
    routines: &[
        (h0(0x25eb6), dart_right_init),
        (WAIT, wait),
        (FLY, fly),
    ],
};

/// Marker 18 (h0+$1895a): a left dart, `$15` pixels down its cell.
pub fn spawn_57(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(57), cell, 0, 0x15);
}

/// Marker 19 (h0+$1896c): a right dart, 8 right and `$11` down.
pub fn spawn_58(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(58), cell, 8, 0x11);
}

/// h0+$25e9a: its look (h0+$32b18); `$20` passes before it flies.
fn dart_left_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32b18));
    c.bus.set_w(slot + 0x64, 0x20);
}

/// h0+$25eb6: its look (h0+$32b24); `$20` passes before it flies.
fn dart_right_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32b24));
    c.bus.set_w(slot + 0x64, 0x20);
}
