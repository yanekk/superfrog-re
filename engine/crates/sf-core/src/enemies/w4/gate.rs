//! Templates 146 (marker 32) and 147 (marker 33): a gate whose animation opens and closes it.
//! As each of its frames shows, the three cells of the row below its marker take that frame's
//! markers (`$3e`, solid, or 0) from a table, so the frog walks through it only while it is open.
//! 147's table is 146's mirror image: it fills the three cells right to left.

use crate::enemies::common::NOTHING;
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{
    bob_anim, bob_follow, nothing, obj_despawn_if_offscreen, obj_spawn_cell,
};
use crate::level::{LevelCtx, ObjectKind};

/// The tables (h0+$271e6, h0+$272f6): 16-byte entries, a frame record then three markers
/// (longs), ending at a negative long.
pub const TABLE_A: u32 = h0(0x271e6);
pub const TABLE_B: u32 = h0(0x272f6);

/// Template 146.
pub static GATE_A: ObjectKind = ObjectKind {
    name: "gate (marker 32)",
    routines: &[
        (h0(0x27166), init_a),
        (h0(0x2717c), update_a),
        (NOTHING, nothing),
    ],
};

/// Template 147.
pub static GATE_B: ObjectKind = ObjectKind {
    name: "gate (marker 33)",
    routines: &[
        (h0(0x27276), init_b),
        (h0(0x2728c), update_b),
        (NOTHING, nothing),
    ],
};

/// Marker 32 (h0+$187e6): `$1c` down its cell.
pub fn spawn_146(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(146), cell, 0, 0x1c);
}

/// Marker 33 (h0+$187f8): `$1c` down its cell.
pub fn spawn_147(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(147), cell, 0, 0x1c);
}

/// h0+$27166: its look (h0+$326cc).
fn init_a(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x326cc));
}

/// h0+$27276: its look (h0+$32658).
fn init_b(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x32658));
}

/// The gates' update: the entry of `table` for the frame showing (the first entry is always
/// looked at, then on until a negative long), its markers into the three cells under the marker
/// (`order` gives which of the entry's longs goes to each); then its bob where it is, gone once
/// off the screen.
fn update(c: &mut LevelCtx, slot: u32, table: u32, order: [u32; 3]) {
    let b = &mut c.bus;
    let bob = b.l(slot + 0x50);
    let frame = b.l(b.l(bob + 0x16));
    let mut at = table;
    let found = loop {
        if b.l(at) == frame {
            break true;
        }
        at = at.wrapping_add(0x10);
        if b.l(at) & 0x8000_0000 != 0 {
            break false;
        }
    };
    if found {
        let cells = b.l(slot + 0x40).wrapping_add(0x260);
        for (k, o) in order.into_iter().enumerate() {
            let cell = cells + 2 * k as u32;
            let m = b.w(at + o + 2);
            b.set_w(cell, (b.w(cell) & 0xffc0) | m);
        }
    }
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// h0+$2717c.
fn update_a(c: &mut LevelCtx, slot: u32, _d7: u16) {
    update(c, slot, TABLE_A, [4, 8, 0xc]);
}

/// h0+$2728c.
fn update_b(c: &mut LevelCtx, slot: u32, _d7: u16) {
    update(c, slot, TABLE_B, [0xc, 8, 4]);
}
