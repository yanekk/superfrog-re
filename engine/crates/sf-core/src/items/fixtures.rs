//! The level's fixtures the frog sets off from the map's markers (docs/09): the bounce pads
//! (markers 4 and 5), whose object throws the frog up a pass after it lands on one; the
//! launchers (marker 7), whose object shows the spring; and the switches (markers 48–51), which
//! open their gate in the map (`marker_cells`) and turn into their pulled lever.

use crate::frog::{FROG, f};
use crate::layout::h0;
use crate::level::collision::marker;
use crate::level::objects::{
    bob_anim, bob_follow, nothing, obj_despawn_if_offscreen, obj_free, obj_spawn_cell,
};
use crate::level::{Dispatch, LevelCtx, ObjectKind, register};

use super::template;

/// `obj_world_offset` (h0+$9fa).
const OBJ_WORLD_OFFSET: u32 = h0(0x9fa);
/// `marker_cells` (h0+$162e4): the cell of each switch's gate (markers 48–51), found at load.
const MARKER_CELLS: u32 = h0(0x162e4);
/// `opt_game_mode` (h0+$9e6): 0 the easy game.
const OPT_GAME_MODE: u32 = h0(0x9e6);
/// The parked frame: nothing shows.
const PARKED: u32 = h0(0x2aefe);

/// Templates 6 and 7: a bounce pad pressed (markers 5 and 4).
pub static PAD: ObjectKind = ObjectKind {
    name: "bounce pad",
    routines: &[
        (h0(0x24106), pad5_init),
        (h0(0x2414a), pad4_init),
        (h0(0x241b2), pad_update),
        (h0(0x7d4), nothing),
    ],
};

/// Templates 9 and 10: a launcher's spring, pointing left or right.
pub static LAUNCHER: ObjectKind = ObjectKind {
    name: "launcher",
    routines: &[
        (h0(0x240ca), launcher_left_init),
        (h0(0x240aa), launcher_right_init),
        (h0(0x240ea), fixture_fade),
        (h0(0x7d4), nothing),
    ],
};

/// Templates 42–51: a switch's lever, pulled (two per world: on the switch's left or right side).
pub static LEVER: ObjectKind = ObjectKind {
    name: "lever",
    routines: &[
        (h0(0x255ca), |c, s, _| lever_init(c, s, h0(0x318b8))),
        (h0(0x255e0), |c, s, _| lever_init(c, s, h0(0x318d4))),
        (h0(0x25606), |c, s, _| lever_init(c, s, h0(0x31f0c))),
        (h0(0x2561c), |c, s, _| lever_init(c, s, h0(0x31f28))),
        (h0(0x25632), |c, s, _| lever_init(c, s, h0(0x32dd8))),
        (h0(0x25648), |c, s, _| lever_init(c, s, h0(0x32df4))),
        (h0(0x2565e), |c, s, _| lever_init(c, s, h0(0x3243c))),
        (h0(0x25674), |c, s, _| lever_init(c, s, h0(0x32458))),
        (h0(0x2568a), |c, s, _| lever_init(c, s, h0(0x32740))),
        (h0(0x256a0), |c, s, _| lever_init(c, s, h0(0x3275c))),
        (h0(0x255f6), lever_update),
        (h0(0x7d4), nothing),
    ],
};

pub(super) fn register_all(d: &mut Dispatch) {
    register(d, 6, &PAD);
    register(d, 7, &PAD);
    register(d, 9, &LAUNCHER);
    register(d, 10, &LAUNCHER);
    for t in 42..=51 {
        register(d, t, &LEVER);
    }
    d.bump_block = Some(switch);
    d.bounce_pad = Some(spawn_pad);
    d.launcher = Some(spawn_launcher);
}

/// h0+$2418e: a random one of the pad and launcher sounds (1–4).
fn twang(c: &mut LevelCtx) {
    let e = c.rng_range(3).wrapping_add(1);
    c.sfx(e as u8, 3);
}

/// h0+$24106 / h0+$2414a: the pad pressed down, for six passes; the frog's fall stops on it. In
/// world 3 the pad does not show.
fn pad_init(c: &mut LevelCtx, slot: u32, anim: u32) {
    let b = &mut c.bus;
    bob_anim(b, slot, anim);
    b.set_w(slot + 0x5c, 6);
    b.set_w(FROG + f::VY, 0);
    twang(c);
    if c.bus.l(OBJ_WORLD_OFFSET) == 0x200 {
        bob_anim(&mut c.bus, slot, PARKED);
    }
}

fn pad5_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    pad_init(c, slot, h0(0x2cecc));
}

fn pad4_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    pad_init(c, slot, h0(0x2cee0));
}

/// h0+$241b2: the pass after it is pressed the pad throws the frog up (the frog's jump, h0+$11614,
/// then up at 11, 12 outside the easy game); while it is still pressed the frog stands on it. It
/// is gone after six passes.
fn pad_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let t = b.w(slot + 0x5c).wrapping_sub(1);
    b.set_w(slot + 0x5c, t);
    if t == 0 {
        return obj_free(c, slot, 0);
    }
    if t == 5 {
        crate::frog::physics::up(c);
        let b = &mut c.bus;
        b.set_w(FROG + f::GRAVITY_TICK, 0);
        let vy = if b.w(OPT_GAME_MODE) == 0 {
            0xfff5
        } else {
            0xfff4
        };
        b.set_w(FROG + f::VY, vy);
    } else if (t as i16) > 5 {
        b.set_w(FROG + f::Y, b.w(slot + 0xa).wrapping_sub(0x2f));
    }
    bob_follow(&mut c.bus, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// h0+$240ca / h0+$240aa: the launcher's spring, for six passes.
fn launcher_init(c: &mut LevelCtx, slot: u32, anim: u32) {
    let b = &mut c.bus;
    bob_anim(b, slot, anim);
    b.set_w(slot + 0x5c, 6);
    twang(c);
}

fn launcher_left_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    launcher_init(c, slot, h0(0x31b54));
}

fn launcher_right_init(c: &mut LevelCtx, slot: u32, _d7: u16) {
    launcher_init(c, slot, h0(0x31b68));
}

/// h0+$240ea: gone after its passes (`+$5c`).
fn fixture_fade(c: &mut LevelCtx, slot: u32, _d7: u16) {
    let b = &mut c.bus;
    let t = b.w(slot + 0x5c).wrapping_sub(1);
    b.set_w(slot + 0x5c, t);
    if t == 0 {
        return obj_free(c, slot, 0);
    }
    bob_follow(b, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

fn lever_init(c: &mut LevelCtx, slot: u32, anim: u32) {
    bob_anim(&mut c.bus, slot, anim);
}

/// h0+$255f6: the lever stays.
fn lever_update(c: &mut LevelCtx, slot: u32, _d7: u16) {
    bob_follow(&mut c.bus, slot);
    obj_despawn_if_offscreen(c, slot, 0);
}

/// h0+$e9cc / h0+$ea22: the pad object under the frog bouncing off marker `m` (4 or 5) at `cell`
/// (one cell left when the pad's left half is there too), four pixels up its cell.
pub fn spawn_pad(c: &mut LevelCtx, cell: u32, m: u16) {
    let t = if m == 4 { template(7) } else { template(6) };
    let at = if marker(&c.bus, cell.wrapping_sub(2)) == m {
        cell.wrapping_sub(2)
    } else {
        cell
    };
    obj_spawn_cell(c, t, at, 0, 0xfffc);
}

/// h0+$e95e / h0+$e9aa: the launcher's spring at `cell` (one cell left when the launcher's left
/// half is there), `left` for the one throwing the frog left (template 9; template 10 sits a cell
/// further left).
pub fn spawn_launcher(c: &mut LevelCtx, cell: u32, left: bool) {
    let mut at = cell;
    if marker(&c.bus, at.wrapping_sub(2)) == 7 {
        at = at.wrapping_sub(2);
    }
    if left {
        obj_spawn_cell(c, template(9), at, 0, 0xc);
    } else {
        obj_spawn_cell(c, template(10), at.wrapping_sub(2), 0, 0xc);
    }
}

/// `switch` (h0+$f6be): the frog touched a switch (markers 48–51) at `cell`, whose other half is
/// above or below it: its gate (`marker_cells`) opens, drawn in the map world by world, the switch
/// turns into its pulled lever (its cells' tiles changed) and the lever's object spawns.
pub fn switch(c: &mut LevelCtx, cell: u32) {
    let b = &c.bus;
    let m = marker(b, cell);
    let a2 = if marker(b, cell.wrapping_add(0x260)) == m {
        cell
    } else if marker(b, cell.wrapping_sub(0x260)) == m {
        cell.wrapping_sub(0x260)
    } else {
        return;
    };
    let k = (m.wrapping_sub(0x30) as i16 as i32 as u32).wrapping_mul(4);
    let gate = b.l(MARKER_CELLS.wrapping_add(k));
    if gate == 0 {
        return;
    }
    let a4 = gate.wrapping_add(2);
    c.sfx(0x12, 3);
    let b = &mut c.bus;
    let rows = |b: &mut crate::mem::Bus, mut at: u32, cells: &[(u16, u16)]| {
        for &(l, r) in cells {
            b.set_w(at, l);
            b.set_w(at + 2, r);
            at = at.wrapping_add(0x260);
        }
        at
    };
    // The gate opened in the map; then the lever: its template and its two cells, for a switch
    // with a wall on its left, and for one without.
    let (lever, open_lever): ((u32, u16, u16), (u32, u16, u16)) = match b.l(OBJ_WORLD_OFFSET) {
        0 | 0x100 => {
            let at = rows(
                b,
                a4.wrapping_sub(0x260),
                &[
                    (0x7800, 0x7840),
                    (0x7d00, 0x7d40),
                    (0x8200, 0x8240),
                    (0x8700, 0x8740),
                    (0x8c00, 0x8c40),
                ],
            );
            for o in [0, 2] {
                b.set_w(at + o, b.w(at + o) & 0xffc0 | 0x3f);
            }
            ((43, 0xe80, 0x1380), (42, 0xec0, 0x13c0))
        }
        0x200 => {
            let at = a4.wrapping_sub(2);
            for (o, m) in [
                (0, 0x29),
                (2, 0x28),
                (4, 0x2b),
                (0x262, 0x22),
                (0x4c2, 0x22),
            ] {
                b.set_w(at + o, b.w(at + o) & 0xffc0 | m);
            }
            ((45, 0x3c00, 0x4100), (44, 0x3c40, 0x4140))
        }
        0x300 => {
            rows(
                b,
                a4,
                &[
                    (0x8200, 0x8240),
                    (0x8700, 0x8740),
                    (0x8c00, 0x8c40),
                    (0x9100, 0x9140),
                    (0x9600, 0x9640),
                ],
            );
            ((49, 0x6a00, 0x6f00), (48, 0x6a40, 0x6f40))
        }
        0x400 => {
            rows(
                b,
                a4.wrapping_sub(0x260),
                &[
                    (0x7940, 0x7980),
                    (0x7e40, 0x7e80),
                    (0x8340, 0x8380),
                    (0x8840, 0x8880),
                    (0x8d40, 0x8d80),
                ],
            );
            ((51, 0x6fc0, 0x74c0), (50, 0x7000, 0x7500))
        }
        0x500 => {
            rows(b, a4, &[(0x340, 0x340), (0, 0), (0, 0), (0, 0)]);
            ((47, 0x5b40, 0x6040), (46, 0x5b40, 0x6040))
        }
        _ => return,
    };
    let (t, top, bottom) = if marker(b, a2.wrapping_sub(2)) < 0x3a {
        open_lever
    } else {
        lever
    };
    b.set_w(a2, top);
    b.set_w(a2 + 0x260, bottom);
    obj_spawn_cell(c, template(t), a2, 0, 0xc);
}
