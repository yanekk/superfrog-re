//! The `objects` routines of `re/translate.toml` T54 keeps (plans/forest-dumb-level-1 T54, DESIGN
//! §4.3): the five creature slots' movement (`h0_2fae`, its per-slot body `h0_2fea` and the
//! helpers it calls: the step tables `h0_320a`, `h0_3222`, `h0_32f8`, `h0_3322` and the
//! `collision_map` probes `h0_3530`, `h0_3566`, `h0_35a0`, `h0_35dc`), and the slots' sort by y
//! (`h0_3698`). Named as in `re/translate.toml`.
//!
//! What each reads, from the listing (`tools/re_listing.py`, 2026-11-05), callees included:
//! - the five slots: a flag byte each at `h0+$67da`, eight bytes each at `h0+$67ec`: x, y, kind
//!   words, a direction byte (`+6`, its sign tested) and a step counter byte (`+7`, `andi #$1f`
//!   indexes the step tables, which are constants in the code);
//! - `h0_2fea` takes the slot in `d0` and, for kind 7 only, reads the slot index long `h0+$2faa`
//!   (whose flag it clears); `h0_2fae` clears that index itself and loops it over 0..5;
//! - kind 0 reads `char_x` (a long, PC-relative), compared as a word with the slot's `x + $10`;
//! - the probes read `collision_map` (`h3+$fa50`) at `((y + dy) & $ff) * $2e + (x' >> 3)`, `x'` the
//!   slot's x word plus `-1`, 0, `$20` or `$21`. A creature's x is a level coordinate, so `x' >> 3`
//!   runs to `$1fff`, far past the map's `$2e`-byte rows: the reach is `$ff * $2e + $1fff + 1`
//!   bytes, [`COLLISION_REACH`];
//! - the probe helpers take the slot in `a0`; `h0_3698` reads every flag and every slot's 8 bytes.
//!
//! Kinds 1 to 5 move x (kind 3 also y) by up to 5 before or after probing, so a slot's generator
//! lays the collision bytes on every row and column a probe can land on from where it starts,
//! clear or solid at even odds, and each probe goes both ways. The kind-7 counter is biased to the
//! `$1f`/`$20` edge of its `cmpi.b #$20`, kind 0's `char_x` to the slot's `x + $10`.

use super::{Gen, Generator, Setup, h};

pub const ALL: &[(&str, Generator)] = &[
    ("h0_2fae", g(slots_move)),
    ("h0_2fea", g(slot_move)),
    ("h0_320a", g(step_x)),
    ("h0_3222", g(step_x)),
    ("h0_32f8", g(step_xy)),
    ("h0_3322", g(step_xy)),
    ("h0_3530", g(probe)),
    ("h0_3566", g(probe)),
    ("h0_35a0", g(probe)),
    ("h0_35dc", g(probe)),
    ("h0_3698", g(sort)),
];

const fn g(case: fn(&mut Gen) -> Setup) -> Generator {
    Generator { base: None, case }
}

pub const CHAR_X: u32 = 0x1e5a;
pub const SLOT_FLAGS: u32 = 0x67da;
pub const SLOTS: u32 = 0x67ec;
pub const SLOT_INDEX: u32 = 0x2faa;
/// `collision_map`'s hunk 3 offset (`re/symbols.toml`).
pub const COLLISION_MAP: u32 = 0xfa50;
/// How far past `collision_map` a probe can read: row `$ff`, column `$ffff >> 3`.
pub const COLLISION_REACH: u32 = 0xff * 0x2e + 0x1fff + 1;

/// One variable a routine reads, for the test's write check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Var {
    /// The five flag bytes at `h0+$67da`.
    Flags,
    /// The five slots, eight bytes each.
    Slots,
    /// `h0+$2faa`, long.
    SlotIndex,
    /// `char_x`, long.
    CharX,
    /// `collision_map` and everything a probe reaches past it.
    Collision,
}

impl Var {
    /// Its absolute address and size in bytes.
    pub fn span(self) -> (u32, u32) {
        match self {
            Var::Flags => (h(0, SLOT_FLAGS), 5),
            Var::Slots => (h(0, SLOTS), 40),
            Var::SlotIndex => (h(0, SLOT_INDEX), 4),
            Var::CharX => (h(0, CHAR_X), 4),
            Var::Collision => (h(3, COLLISION_MAP), COLLISION_REACH),
        }
    }
}

/// What a routine reads and the conditional branches of its flow, callees included.
pub struct Cover {
    pub name: &'static str,
    /// Hunk 0 offset of its entry.
    pub entry: u32,
    pub reads: &'static [Var],
    /// Hunk 0 offsets of each `Bcc.w` and `DBcc`, listing order.
    pub branches: &'static [u32],
}

/// `h0_2fea`'s flow: the kind dispatch (`$3000`..`$303c`), kind 7's counter (`$3056`), kind 0's
/// side of the character (`$307c`) and its direction (`$309a`), the direction tests of kinds 1 to 5,
/// and the four probes.
const FLOW_2FEA: &[u32] = &[
    0x3000, 0x300a, 0x3014, 0x301e, 0x3028, 0x3032, 0x303c, 0x3056, 0x307c, 0x309a, 0x31c8, 0x31ee,
    0x32e4, 0x33b6, 0x3440, 0x3558, 0x3592, 0x35cc, 0x3608,
];

const FLOW_2FAE: &[u32] = &[
    0x2fc4, 0x2fe0, 0x3000, 0x300a, 0x3014, 0x301e, 0x3028, 0x3032, 0x303c, 0x3056, 0x307c, 0x309a,
    0x31c8, 0x31ee, 0x32e4, 0x33b6, 0x3440, 0x3558, 0x3592, 0x35cc, 0x3608,
];

use Var::*;

pub const COVER: &[Cover] = &[
    Cover {
        name: "h0_2fae",
        entry: 0x2fae,
        reads: &[Flags, Slots, CharX, Collision],
        branches: FLOW_2FAE,
    },
    Cover {
        name: "h0_2fea",
        entry: 0x2fea,
        reads: &[Slots, SlotIndex, CharX, Collision],
        branches: FLOW_2FEA,
    },
    Cover {
        name: "h0_320a",
        entry: 0x320a,
        reads: &[Slots],
        branches: &[],
    },
    Cover {
        name: "h0_3222",
        entry: 0x3222,
        reads: &[Slots],
        branches: &[],
    },
    Cover {
        name: "h0_32f8",
        entry: 0x32f8,
        reads: &[Slots],
        branches: &[],
    },
    Cover {
        name: "h0_3322",
        entry: 0x3322,
        reads: &[Slots],
        branches: &[],
    },
    Cover {
        name: "h0_3530",
        entry: 0x3530,
        reads: &[Slots, Collision],
        branches: &[0x3558],
    },
    Cover {
        name: "h0_3566",
        entry: 0x3566,
        reads: &[Slots, Collision],
        branches: &[0x3592],
    },
    Cover {
        name: "h0_35a0",
        entry: 0x35a0,
        reads: &[Slots, Collision],
        branches: &[0x35cc],
    },
    Cover {
        name: "h0_35dc",
        entry: 0x35dc,
        reads: &[Slots, Collision],
        branches: &[0x3608],
    },
    Cover {
        name: "h0_3698",
        entry: 0x3698,
        reads: &[Flags, Slots],
        branches: &[0x36f0, 0x36fa, 0x3704, 0x370e, 0x3716],
    },
];

/// The `collision_map` byte a probe reads for the x word `x` (with the probe's offset added) and
/// the y word `y`, the probe's row offset `dy` (`$18` or `$21`).
pub fn collision_at(x: u16, y: u16, dy: u32) -> u32 {
    let row = (y as u32 + dy) & 0xff;
    h(3, COLLISION_MAP) + row * 0x2e + (x >> 3) as u32
}

/// Clear or solid at even odds; solid is any non-zero byte.
fn solid(g: &mut Gen) -> u8 {
    if g.one_in(2) { 0 } else { g.next() as u8 | 1 }
}

/// A slot's x: mostly in a level (`$1900` wide), sometimes near 0 where `x - 1` wraps, or any word.
fn slot_x(g: &mut Gen) -> u16 {
    match g.below(10) {
        0 => g.below(4) as u16,
        1 => g.next() as u16,
        _ => g.below(0x1900) as u16,
    }
}

/// A slot's y: in a level (`$600` high) or any word; only its low byte reaches a probe.
fn slot_y(g: &mut Gen) -> u16 {
    if g.one_in(8) {
        g.next() as u16
    } else {
        g.below(0x600) as u16
    }
}

/// A kind: 0 to 7 (6 does nothing), sometimes 8 or any word, which also do nothing.
fn kind(g: &mut Gen) -> u16 {
    match g.below(12) {
        0 => g.next() as u16,
        1 => 8,
        _ => g.below(8) as u16,
    }
}

/// The step counter: any byte, but a third of the time on `$1f` or `$20`'s edge (kind 7's
/// `cmpi.b #$20` after `+1`) or `$7f`/`$ff` where the byte wraps.
fn counter(g: &mut Gen) -> u8 {
    if g.below(3) == 0 {
        g.pick(&[0x1e, 0x1f, 0x20, 0x7f, 0x9f, 0xff])
    } else {
        g.next() as u8
    }
}

/// Slot `i`: x, y, kind, direction and counter, and the collision bytes its probes can reach once
/// it has moved: rows `y + $17..=y + $22`, columns `(x - 8) >> 3..=(x + $29) >> 3`.
fn slot(g: &mut Gen, s: &mut Setup, i: u32) -> (u16, u16) {
    let o = h(0, SLOTS) + i * 8;
    let (x, y) = (slot_x(g), slot_y(g));
    s.w(o, x);
    s.w(o + 2, y);
    s.w(o + 4, kind(g));
    s.b(o + 6, g.next() as u8);
    s.b(o + 7, counter(g));
    let mut cells = std::collections::BTreeSet::new();
    for dy in 0x17..=0x22 {
        for dx in (-8i32..=0x29).step_by(4).chain([0x29]) {
            cells.insert(collision_at(x.wrapping_add(dx as u16), y, dy));
        }
    }
    for c in cells {
        s.b(c, solid(g));
    }
    (x, y)
}

/// `char_x` for kind 0: on, either side of or far from `x + $10` (compared as words), any long.
fn char_x(g: &mut Gen, s: &mut Setup, x: u16) {
    let p = x.wrapping_add(0x10);
    let lo = match g.below(6) {
        0 => p,
        1 => p.wrapping_sub(1),
        2 => p.wrapping_add(1),
        3 => g.next() as u16,
        _ => p.wrapping_add(g.below(0x200) as u16).wrapping_sub(0x100),
    };
    let hi = if g.one_in(4) { g.next() as u16 } else { 0 };
    s.l(h(0, CHAR_X), (hi as u32) << 16 | lo as u32);
}

/// `h0_2fae`: the five flags and slots, and `char_x` near one of them.
fn slots_move(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let mut xs = [0u16; 5];
    for i in 0..5 {
        let flag = if g.one_in(3) { 0 } else { g.next() as u8 | 1 };
        s.b(h(0, SLOT_FLAGS) + i, flag);
        xs[i as usize] = slot(g, &mut s, i).0;
    }
    let x = xs[g.below(5) as usize];
    char_x(g, &mut s, x);
    s
}

/// `h0_2fea` alone: `d0` the slot (upper words clear: `lsl.l #3` indexes with the whole long), the
/// index long kind 7 clears the flag of (mostly the same slot), the slot and `char_x`.
fn slot_move(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let i = g.below(5) as u32;
    s.d[0] = i;
    let index = if g.one_in(4) { g.below(5) as u32 } else { i };
    s.l(h(0, SLOT_INDEX), index);
    let (x, _) = slot(g, &mut s, i);
    char_x(g, &mut s, x);
    s
}

/// `a0` on a slot, mostly; the helpers use only `a0`, so any even address in the slots' page works
/// as well, but the slots are what the game passes.
fn slot_a0(g: &mut Gen, s: &mut Setup) -> u32 {
    let o = h(0, SLOTS) + g.below(5) as u32 * 8;
    s.a[0] = o;
    o
}

/// `h0_320a`, `h0_3222`: the counter and x.
fn step_x(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let o = slot_a0(g, &mut s);
    s.w(o, slot_x(g));
    s.b(o + 7, g.next() as u8);
    s
}

/// `h0_32f8`, `h0_3322`: the counter, x and y.
fn step_xy(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let o = slot_a0(g, &mut s);
    s.w(o, slot_x(g));
    s.w(o + 2, slot_y(g));
    s.b(o + 7, g.next() as u8);
    s
}

/// The four probes: x, y, and the one byte each can read (all four laid: the probe at hand reads
/// one of them, so each is clear or solid at even odds whichever it is).
fn probe(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let o = slot_a0(g, &mut s);
    let (x, y) = (slot_x(g), slot_y(g));
    s.w(o, x);
    s.w(o + 2, y);
    for (dx, dy) in [(0u16, 0x18), (0xffff, 0x21), (0x20, 0x18), (0x21, 0x21)] {
        s.b(collision_at(x.wrapping_add(dx), y, dy), solid(g));
    }
    s
}

/// `h0_3698`: the flags and slots, the y words drawn so the word compares (`$36ee`, `$370c`) and the
/// byte ones (`$36f8`, `$3702`) disagree: from a few values (ties), on a byte's sign edge, or any.
fn sort(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let mode = g.below(3);
    for i in 0..5 {
        s.b(h(0, SLOT_FLAGS) + i, g.next() as u8);
        let o = h(0, SLOTS) + i * 8;
        let bytes: Vec<u8> = (0..8).map(|_| g.next() as u8).collect();
        s.put(o, bytes);
        let y: u16 = match mode {
            0 => g.pick(&[0x10, 0x7f, 0x80, 0x17f, 0x180]),
            1 => g.below(0x600) as u16,
            _ => g.next() as u16,
        };
        s.w(o + 2, y);
    }
    s
}
