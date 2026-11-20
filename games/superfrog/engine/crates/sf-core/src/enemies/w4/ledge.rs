//! Template 148 (marker 27): a crumbling ledge. The two cells under its marker are solid (`$3e`)
//! except while its last frame (h0+$2dfdc) shows. Its animation counter runs on; once it has
//! reached `$200`, the frog landing on it (not rising) makes a sound, loses a fall of 8 or more,
//! and, standing, sets the counter back to 0.

use crate::frog::{FROG, f};
use crate::items::template;
use crate::layout::h0;
use crate::level::objects::{bob_anim, bob_follow, obj_despawn_if_offscreen, obj_spawn_cell};
use crate::level::{LevelCtx, ObjectKind, n_clear};

/// Its touch handler (h0+$23774).
pub const TOUCH_LEDGE: u32 = h0(0x23774);
/// The frame that leaves it open.
const OPEN: u32 = h0(0x2dfdc);
/// `stream_table`'s entry it plays (`stream_table+$40`).
const SAMPLE: usize = 4;

/// Template 148.
pub static LEDGE: ObjectKind = ObjectKind {
    name: "crumbling ledge",
    routines: &[
        (h0(0x27386), ledge_init),
        (h0(0x2739c), ledge_update),
        (TOUCH_LEDGE, touch_ledge),
    ],
};

/// Marker 27 (h0+$187d4): `$1c` down its cell.
pub fn spawn_148(c: &mut LevelCtx, cell: u32) {
    obj_spawn_cell(c, template(148), cell, 0, 0x1c);
}

/// h0+$27386: its look (h0+$3260c).
fn ledge_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_anim(&mut c.bus, slot, h0(0x3260c));
}

/// h0+$2739c: the two cells under its marker solid, unless the open frame shows; its bob where it
/// is; gone once off the screen.
fn ledge_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let cells = b.l(slot + 0x40).wrapping_add(0x260);
    for cell in [cells, cells + 2] {
        b.set_w(cell, (b.w(cell) & 0xffc0) | 0x3e);
    }
    let bob = b.l(slot + 0x50);
    if b.l(b.l(bob + 0x16)) == OPEN {
        for cell in [cells, cells + 2] {
            b.set_w(cell, b.w(cell) & 0xffc0);
        }
    }
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// h0+$23774: see the module's note.
pub fn touch_ledge(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let bob = c.bus.l(slot + 0x50);
    if !n_clear(c.bus.w(bob + 0x1a).wrapping_sub(0x200)) || !n_clear(c.bus.w(FROG + f::VY)) {
        return;
    }
    c.sample(SAMPLE);
    let b = &mut c.bus;
    if n_clear(b.w(FROG + f::VY).wrapping_sub(8)) {
        b.set_w(FROG + f::VY, 0);
    }
    if b.w(FROG + f::ON_GROUND) == 0 {
        return;
    }
    b.set_w(bob + 0x1a, 0);
}
