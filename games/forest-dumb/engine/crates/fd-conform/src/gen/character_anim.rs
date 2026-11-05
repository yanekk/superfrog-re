//! The second share of the `character` routines' generators (plans/forest-dumb-level-1 T53, DESIGN
//! §4.3): every entry of `re/translate.toml` with `gen_task = "T53"`, named as there. The walk
//! animation's frame pickers, which set `char_anim_ptr` from a pointer table in hunk 0, and
//! `read_joystick`, which turns JOY1DAT into `joy_flags` and appends them to `input_log`.
//!
//! As in [`super::character`], [`COVER`] names what each routine reads, read off its listing
//! (`tools/re_listing.py`), and the conditional branches of its flow; `tests/gen_character_anim.rs`
//! holds every generator to them. The pointer tables (`h0+$4edc`, `$4f30`, `$4ef0`..`$4f60`) are the
//! program's own and are read, never poked. The state variables are [`character::Var`]'s, drawn as
//! T35 draws them, so the two shares agree on what a state of the character looks like.

use super::character::{self, Way};
use super::{Gen, Generator, Setup, h};

/// One input of a T53 routine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Var {
    /// A state variable T35 already draws: `Facing` (`h0+$4ece`), `WalkT` (`$4be4`), `Probe1`
    /// (`$4ed1`), `Probe2` (`$4ed2`).
    Char(character::Var),
    /// `input_log_index` (`h0+$8ec0`), long: the next free byte of `input_log`, which is full at
    /// `$2ee0`.
    LogIndex,
    /// JOY1DAT (`$dff00c`), word, a hardware read: laid in `Setup::hw_reads`, not poked.
    Joy1dat,
}

/// `input_log_index`'s hunk 0 offset, and the value at which `input_log` is full.
pub const LOG_INDEX: u32 = 0x8ec0;
pub const LOG_FULL: u32 = 0x2ee0;
/// JOY1DAT.
pub const JOY1DAT: u32 = 0xdf_f00c;

impl Var {
    /// The first byte and the byte count.
    pub fn span(self) -> (u32, u32) {
        match self {
            Var::Char(v) => v.span(),
            Var::LogIndex => (h(0, LOG_INDEX), 4),
            Var::Joy1dat => (JOY1DAT, 2),
        }
    }

    fn poke(self, g: &mut Gen, s: &mut Setup) {
        match self {
            Var::Char(v) => v.poke(g, s),
            // Mostly a log with room, sometimes full or one short of it. The game never takes it
            // past $2ee0, and a larger index would only write further past `input_log`.
            Var::LogIndex => s.l(
                h(0, LOG_INDEX),
                match g.below(4) {
                    0 => LOG_FULL,
                    1 => g.pick(&[0, LOG_FULL - 1]),
                    _ => g.below(LOG_FULL as u64) as u32,
                },
            ),
            // Half the states only the four direction bits a joystick sets (9, 8, 1, 0), every
            // combination of them, half any word: the routine tests bits 9 and 1, and 9^8, 1^0.
            Var::Joy1dat => {
                let v = if g.one_in(2) {
                    let bits = g.below(16) as u16;
                    (bits & 3) | ((bits & 0xc) << 6)
                } else {
                    g.next() as u16
                };
                s.hw_reads.push((JOY1DAT, (v >> 8) as u8));
                s.hw_reads.push((JOY1DAT + 1, v as u8));
            }
        }
    }
}

/// What each routine reads, by [`Var`], and its own flow's conditional branches.
pub struct Cover {
    pub name: &'static str,
    /// Hunk 0 offset of its entry.
    pub entry: u32,
    pub reads: &'static [Var],
    /// Each `Bcc` in its flow, listing order: (hunk 0 offset, byte length of the instruction just
    /// before it). The length is 0 where the branch is a `Bcc.w` the test can detour alone; it is
    /// the instruction's length where the test moves that instruction with the branch (a `Bcc.b`,
    /// or a branch out of a `bra.w`'s reach of the test's stub). That instruction is position
    /// independent and no branch lands on the branch itself.
    pub branches: &'static [(u32, u32)],
    /// Branches no state can take one way: (offset, the way, why).
    pub unreachable: &'static [(u32, Way, &'static str)],
}

use character::Var::{Facing, Probe1, Probe2, WalkT};

pub const COVER: &[Cover] = &[
    Cover {
        name: "char_step_4be6",
        entry: 0x4be6,
        reads: &[Var::Char(WalkT)],
        branches: &[(0x4bfe, 0)],
        unreachable: &[],
    },
    Cover {
        name: "h0_4c28",
        entry: 0x4c28,
        reads: &[Var::Char(WalkT)],
        branches: &[(0x4c40, 0)],
        unreachable: &[],
    },
    Cover {
        name: "h0_4c6a",
        entry: 0x4c6a,
        reads: &[Var::Char(Probe1), Var::Char(Facing), Var::Char(Probe2)],
        branches: &[
            (0x4c72, 0),
            (0x4c7c, 0),
            (0x4c88, 0),
            (0x4c94, 0),
            (0x4cac, 0),
            (0x4cb8, 0),
            (0x4cd0, 0),
            (0x4cdc, 0),
            (0x4cf4, 0),
            (0x4d00, 0),
            (0x4d16, 0),
        ],
        unreachable: &[],
    },
    Cover {
        name: "h0_4d32",
        entry: 0x4d32,
        reads: &[Var::Char(Facing)],
        branches: &[(0x4d38, 0)],
        unreachable: &[],
    },
    Cover {
        name: "h0_4d54",
        entry: 0x4d54,
        reads: &[Var::Char(Facing)],
        branches: &[(0x4d5a, 0)],
        unreachable: &[],
    },
    Cover {
        name: "read_joystick",
        entry: 0x8dfc,
        reads: &[Var::Joy1dat, Var::LogIndex],
        // The four `beq.b` follow a `btst #n,Dn` (4 bytes); the `beq.w` from $8e6e on are out of a
        // bra.w's reach and follow a `cmpi.l #imm,d1` or a `tst.w abs.l` (6 bytes).
        branches: &[
            (0x8e26, 4),
            (0x8e34, 4),
            (0x8e48, 4),
            (0x8e56, 4),
            (0x8e6e, 6),
            (0x8e7e, 6),
            (0x8e8c, 6),
            (0x8e9a, 6),
            (0x8ea8, 6),
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
    for v in cover(name).expect("a T53 routine").reads {
        v.poke(g, &mut s);
    }
    s
}

macro_rules! generators {
    ($($name:literal => $f:ident;)*) => {
        $(fn $f(g: &mut Gen) -> Setup { state(g, $name) })*
        pub const ALL: &[(&str, Generator)] = &[
            $(($name, Generator { base: None, case: $f }),)*
        ];
    };
}

generators! {
    "char_step_4be6" => char_step_4be6;
    "h0_4c28" => h0_4c28;
    "h0_4c6a" => h0_4c6a;
    "h0_4d32" => h0_4d32;
    "h0_4d54" => h0_4d54;
    "read_joystick" => read_joystick;
}
