//! The `character` routines' generators (plans/forest-dumb-level-1 T35, DESIGN §4.3): every entry of
//! `re/translate.toml` with `gen_task = "T35"`, named as there.
//!
//! What each routine reads is read off its listing (`tools/re_listing.py`), callees included, since
//! the original runs them whole: [`COVER`] names the variables per routine and the conditional
//! branches of its own flow. `tests/gen_character.rs` holds every generator to them: 2 000 states
//! with no hang or fault, every branch taken both ways by some state or named in
//! [`Cover::unreachable`] with why, and no byte poked that the routine does not read.
//!
//! The values are biased to the edges the branches test: the `$ff` ends of the step tables, the
//! signed compares against `$30` and `$a0`, the counter that wraps at `$f0`. A uniformly random
//! byte would reach most of them in 2 000 states but not the combinations `h0_3ea8`'s sound needs.

use super::{Gen, Generator, Setup, h};

/// One variable a routine reads: where (hunk 0 offset or an address), its size, how it is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Var {
    CharX,
    CharY,
    /// `h0+$3b28`, word, cleared by the jump: 0 or not.
    J3b28,
    /// `h0+$4b62`, word: the character is in the dying fall (`h0+$4b64`).
    Dying,
    /// `h0+$4b60`, word: the fall's counter, its high nibble indexing the fall's frames.
    DyingT,
    /// `h0+$4ecf`, byte: the jump's rise is running.
    Rising,
    /// `h0+$4ed0`, byte: the fall is running.
    Falling,
    /// `h0+$4ed1`..`$4ed6`, bytes: `char_probe_flags`' results. `$4ed1` and `$4ed2` hold 0-3, the
    /// rest 0 or 1.
    Probe1,
    Probe2,
    Probe3,
    Probe4,
    Probe5,
    Probe6,
    /// `h0+$4ece`, byte: facing, `$ff` left, 1 right.
    Facing,
    /// `h0+$4be4`, byte: the walk animation's counter, wraps at `$28`.
    WalkT,
    /// `h0+$4108`, byte: the slide's step, index into `h0+$40ac`/`$40cb`.
    SlideT,
    /// `h0+$4109`, byte: the jump has left the ground.
    Jumped,
    /// `h0+$410a`, byte: the jump's or fall's step, index into `h0+$44d4`, `$42ec`, `$431e`, `$43ba`.
    JumpT,
    /// `h0+$4710`, `$4714`, longs: run counters left and right, index into `h0+$49d0`.
    RunL,
    RunR,
    /// `h0+$41b4`, `$41b6`, words.
    J41b4,
    J41b6,
    /// `joy_flags`: left, right, up (`h0+$8df2`, `$8df4`, `$8df6`), words.
    JoyL,
    JoyR,
    JoyU,
    /// `h0+$54f0`..`$54fe`, bytes: `char_probe`'s samples, `$ff` or 0 (`sne`).
    Samples,
    /// `h0+$54ff`, byte: the sample `h0_4718` and `h0_48be` test for ground.
    Ground,
}

impl Var {
    /// The first byte and the byte count.
    pub fn span(self) -> (u32, u32) {
        use Var::*;
        let (off, n) = match self {
            CharX => (0x1e5a, 4),
            CharY => (0x1e5e, 4),
            J3b28 => (0x3b28, 2),
            Dying => (0x4b62, 2),
            DyingT => (0x4b60, 2),
            Rising => (0x4ecf, 1),
            Falling => (0x4ed0, 1),
            Probe1 => (0x4ed1, 1),
            Probe2 => (0x4ed2, 1),
            Probe3 => (0x4ed3, 1),
            Probe4 => (0x4ed4, 1),
            Probe5 => (0x4ed5, 1),
            Probe6 => (0x4ed6, 1),
            Facing => (0x4ece, 1),
            WalkT => (0x4be4, 1),
            SlideT => (0x4108, 1),
            Jumped => (0x4109, 1),
            JumpT => (0x410a, 1),
            RunL => (0x4710, 4),
            RunR => (0x4714, 4),
            J41b4 => (0x41b4, 2),
            J41b6 => (0x41b6, 2),
            JoyL => (0x8df2, 2),
            JoyR => (0x8df4, 2),
            JoyU => (0x8df6, 2),
            Samples => (0x54f0, 15),
            Ground => (0x54ff, 1),
        };
        (h(0, off), n)
    }

    fn poke(self, g: &mut Gen, s: &mut Setup) {
        use Var::*;
        let (at, _) = self.span();
        match self {
            CharX => s.l(at, coord(g, 0x1800)),
            CharY => s.l(at, coord(g, 0x600)),
            Dying => {
                let v = if g.one_in(3) { g.pick(&[1, 0xffff]) } else { 0 };
                s.w(at, v)
            }
            // The fall ends once (counter + 1) >> 4 reaches 8: $7f is the step that ends it.
            DyingT => {
                let v = match g.below(4) {
                    0 => g.pick(&[0x6f, 0x7e, 0x7f, 0x80]),
                    1 => g.next() as u16,
                    _ => g.below(0x90) as u16,
                };
                s.w(at, v)
            }
            Rising | Falling | Jumped => s.b(at, g.pick(&[0, 0, 0xff, 1])),
            Probe1 | Probe2 => s.b(at, g.below(4) as u8),
            Probe3 | Probe4 | Probe5 | Probe6 => s.b(at, g.below(2) as u8),
            Facing => s.b(at, g.pick(&[0xff, 1, 0])),
            WalkT => {
                let (low, any) = (g.below(0x28) as u8, g.next() as u8);
                s.b(at, g.pick(&[0x27, 0x28, low, any]))
            }
            // 3 starts the slide's sound; $1e in $40ac and $3c in $40cb hold the $ff that ends it.
            SlideT => s.b(
                at,
                match g.below(4) {
                    0 => 3,
                    1 => g.pick(&[0x1d, 0x1e, 0x3b, 0x3c]),
                    2 => g.next() as u8,
                    _ => g.below(0x40) as u8,
                },
            ),
            // The tables' $ff ends ($f9 in $44d4, $9b in $431e, $30 in $42ec), a byte about to wrap,
            // and the entries with bit 7 set that h0_4274 masks.
            JumpT => s.b(
                at,
                if g.one_in(2) {
                    g.pick(&[
                        0xf9, 0xf8, 0x9b, 0x9a, 0x30, 0x2f, 0xff, 0x1b, 0x4d, 0x98, 0xa8,
                    ])
                } else {
                    g.next() as u8
                },
            ),
            // Signed compares with $30 and $a0; $ef steps to the $f0 the counter checks; from $121
            // the table at h0+$49d0 holds $ff and the routine steps back $1e.
            RunL | RunR => s.l(
                at,
                match g.below(6) {
                    0 => 0,
                    1 => g.pick(&[0x2f, 0x30, 0x9f, 0xa0, 0xef, 0x120, 0x121, 0x13f]),
                    2 => g.next() as u32,
                    _ => g.below(0x140) as u32,
                },
            ),
            J3b28 | J41b4 | J41b6 | JoyL | JoyR | JoyU => {
                let v = if g.one_in(2) { 0 } else { g.pick(&[1, 0xffff]) };
                s.w(at, v)
            }
            Samples => {
                let v: Vec<u8> = (0..15).map(|_| g.pick(&[0, 0xff])).collect();
                s.put(at, v)
            }
            Ground => s.b(at, g.pick(&[0, 0xff])),
        }
    }
}

/// A position: mostly in a level's range (`max`), sometimes any long, sometimes near 0.
fn coord(g: &mut Gen, max: u64) -> u32 {
    match g.below(10) {
        0 => g.next() as u32,
        1 => g.below(0x40) as u32,
        _ => g.below(max) as u32,
    }
}

/// `collision_map` (`h3+$fa50`, 256 rows of `$2e` bytes): blocks of 8 bytes, each clear, solid or
/// noise, so the probe's samples come out every way and in runs, as a level's ground does.
pub fn collision_map(mem: &mut [u8]) {
    let mut g = Gen::new(0x5EED_0035);
    let at = h(3, COLLISION_MAP.0) as usize;
    for block in mem[at..at + COLLISION_MAP.1 as usize].chunks_mut(8) {
        let kind = g.below(3);
        for b in block {
            *b = match kind {
                0 => 0,
                1 => 0xff,
                _ => g.next() as u8,
            };
        }
    }
}

/// `collision_map`'s hunk 3 offset and size (`re/symbols.toml`).
pub const COLLISION_MAP: (u32, u32) = (0xfa50, 0x2e00);

/// What each routine reads, by [`Var`], and its own flow's conditional branches.
pub struct Cover {
    pub name: &'static str,
    /// Hunk 0 offset of its entry.
    pub entry: u32,
    pub reads: &'static [Var],
    /// Whether it reads `collision_map` (laid by [`collision_map`]).
    pub map: bool,
    /// Hunk 0 offsets of each `Bcc` and `DBcc` in its own flow (calls not followed), listing order.
    pub branches: &'static [u32],
    /// Branches no state can take one way: (offset, the way, why).
    pub unreachable: &'static [(u32, Way, &'static str)],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Way {
    Taken,
    NotTaken,
}

use Var::*;

const VHPOSR_WAIT: &str = "the raster wait's `beq` back to its `cmp`: the routine oracle steps VHPOSR on every read, so two reads never match";

pub const COVER: &[Cover] = &[
    Cover {
        name: "char_update",
        entry: 0x3e0c,
        // char_probe writes $54f0-$54ff and char_probe_flags $4ed1-$4ed6 before anything reads
        // them, so they are not inputs here.
        reads: &[
            CharX, CharY, Dying, DyingT, Rising, Falling, Facing, WalkT, SlideT, Jumped, JumpT,
            RunL, RunR, J3b28, J41b4, J41b6, JoyL, JoyR, JoyU,
        ],
        map: true,
        branches: &[0x3e26],
        unreachable: &[],
    },
    Cover {
        name: "char_step",
        entry: 0x3e30,
        reads: &[
            CharX, CharY, Dying, DyingT, Rising, Falling, Probe1, Probe2, Probe3, Probe4, Probe5,
            Probe6, Facing, WalkT, SlideT, Jumped, JumpT, RunL, RunR, J3b28, J41b4, J41b6, JoyL,
            JoyR, JoyU, Ground,
        ],
        map: false,
        branches: &[
            0x3e36, 0x3e44, 0x3e52, 0x3e60, 0x3e6e, 0x3e7c, 0x3e8a, 0x3e94, 0x3e9e, 0x4126, 0x41be,
            0x41c8, 0x41d2, 0x41fc, 0x4218, 0x4222, 0x4240, 0x4b7a, 0x4b84, 0x4bc0,
        ],
        unreachable: &[],
    },
    Cover {
        name: "h0_3ea8",
        entry: 0x3ea8,
        reads: &[
            Probe1, Probe2, Probe3, Probe4, RunL, RunR, SlideT, CharX, CharY, JumpT, Facing, JoyL,
            JoyR,
        ],
        map: false,
        branches: &[
            0x3eb2, 0x3ebe, 0x3ecc, 0x3eda, 0x3efc, 0x3f26, 0x3f42, 0x3f46, 0x3f80, 0x3f9c, 0x3fd4,
            0x3ffe, 0x401a, 0x401e, 0x4058, 0x407a,
        ],
        unreachable: &[
            (0x3f42, Way::Taken, VHPOSR_WAIT),
            (0x401a, Way::Taken, VHPOSR_WAIT),
        ],
    },
    Cover {
        name: "h0_410c",
        entry: 0x410c,
        reads: &[
            Probe1, JumpT, CharY, Facing, JoyR, JoyL, Probe3, Probe4, CharX,
        ],
        map: false,
        branches: &[0x4126],
        unreachable: &[],
    },
    Cover {
        name: "h0_416c",
        entry: 0x416c,
        reads: &[JumpT, CharY],
        map: false,
        branches: &[0x4194],
        unreachable: &[],
    },
    Cover {
        name: "h0_4274",
        entry: 0x4274,
        reads: &[JumpT, J41b4, J3b28, JoyU, CharY],
        map: false,
        branches: &[0x4286, 0x4290, 0x42be, 0x42c6, 0x42d0],
        unreachable: &[],
    },
    Cover {
        name: "h0_4696",
        entry: 0x4696,
        reads: &[JoyR, JoyL, Probe3, Probe4, CharX],
        map: false,
        branches: &[0x469c, 0x46a6, 0x46b2, 0x46e2],
        unreachable: &[],
    },
    Cover {
        name: "h0_4718",
        entry: 0x4718,
        reads: &[
            Probe3, Probe1, Probe2, Ground, CharY, Facing, WalkT, RunL, CharX,
        ],
        map: false,
        branches: &[
            0x4724, 0x4730, 0x4746, 0x4752, 0x475e, 0x476a, 0x4792, 0x479e, 0x483c,
        ],
        unreachable: &[],
    },
    Cover {
        name: "h0_47e2",
        entry: 0x47e2,
        reads: &[RunL, CharX],
        map: false,
        branches: &[0x47fc, 0x481c],
        unreachable: &[],
    },
    Cover {
        name: "h0_48be",
        entry: 0x48be,
        reads: &[
            Probe4, Probe1, Probe2, Ground, CharY, Facing, WalkT, RunR, CharX,
        ],
        map: false,
        branches: &[
            0x4880, 0x48ca, 0x48d6, 0x48ec, 0x48f8, 0x4904, 0x4910, 0x4938, 0x4944,
        ],
        unreachable: &[],
    },
    Cover {
        name: "h0_497c",
        entry: 0x497c,
        reads: &[RunR, CharX],
        map: false,
        branches: &[0x4996, 0x49b6],
        unreachable: &[],
    },
    Cover {
        name: "h0_4b0e",
        entry: 0x4b0e,
        reads: &[Probe1, Probe6, Probe5, Facing, Probe2],
        map: false,
        branches: &[0x4b16, 0x4b2a, 0x4b3a],
        unreachable: &[],
    },
    Cover {
        name: "char_probe",
        entry: 0x4f84,
        reads: &[CharX, CharY],
        map: true,
        branches: &[],
        unreachable: &[],
    },
    Cover {
        name: "char_probe_flags",
        entry: 0x4f90,
        reads: &[Samples],
        map: false,
        branches: &[
            0x4f9e, 0x4fac, 0x4fba, 0x4fd8, 0x4fe6, 0x4ffc, 0x500a, 0x5020, 0x502e, 0x5044, 0x5052,
            0x5060, 0x5076, 0x5084, 0x5092,
        ],
        unreachable: &[],
    },
];

/// The cover entry named `name`.
pub fn cover(name: &str) -> Option<&'static Cover> {
    COVER.iter().find(|c| c.name == name)
}

/// Every register random, then each variable `name` reads drawn.
fn state(g: &mut Gen, name: &str) -> Setup {
    let mut s = Setup::random(g);
    let reads = cover(name).expect("a T35 routine").reads;
    for v in reads {
        v.poke(g, &mut s);
    }
    // h0_3ea8's slide runs only on flat ground (`$4ed1` 1, `$4ed2` 0) with one run counter at
    // `$30` or more, the other not, and no wall that way: five draws that line up about once in
    // 2 000 states, too rarely to reach the `$ff` end of its tables. A quarter of the states slide.
    if reads.contains(&Probe1) && reads.contains(&SlideT) && g.one_in(4) {
        let fast = 0x30 + g.below(0x100) as u32;
        let slow = g.below(0x30) as u32;
        let left = g.one_in(2);
        s.b(Probe1.span().0, 1);
        s.b(Probe2.span().0, 0);
        s.b(if left { Probe3 } else { Probe4 }.span().0, 0);
        s.l(RunL.span().0, if left { fast } else { slow });
        s.l(RunR.span().0, if left { slow } else { fast });
    }
    s
}

macro_rules! generators {
    ($($name:literal => $f:ident, $base:expr;)*) => {
        $(fn $f(g: &mut Gen) -> Setup { state(g, $name) })*
        pub const ALL: &[(&str, Generator)] = &[
            $(($name, Generator { base: $base, case: $f }),)*
        ];
    };
}

generators! {
    "char_update" => char_update, Some(collision_map);
    "char_step" => char_step, None;
    "h0_3ea8" => h0_3ea8, None;
    "h0_410c" => h0_410c, None;
    "h0_416c" => h0_416c, None;
    "h0_4274" => h0_4274, None;
    "h0_4696" => h0_4696, None;
    "h0_4718" => h0_4718, None;
    "h0_47e2" => h0_47e2, None;
    "h0_48be" => h0_48be, None;
    "h0_497c" => h0_497c, None;
    "h0_4b0e" => h0_4b0e, None;
    "char_probe" => char_probe, Some(collision_map);
    "char_probe_flags" => char_probe_flags, None;
}
