//! The `rest` routines' generators (plans/forest-dumb-level-1 T39, DESIGN §4.3): every entry of
//! `re/translate.toml` with `gen_task = "T39"`, named as there: `game_frame`, `countdown_step`,
//! `play_keys`, `help_key`, `esc_key` and `lives_minus_one`.
//!
//! What each reads is read off its flow listing (`tools/call_tree.py`'s decoding, 2026-11-05),
//! callees included, since the original runs them whole: [`COVER`] names the variables per routine
//! and the conditional branches of its own flow. `tests/gen_rest.rs` holds every generator to them,
//! as `tests/gen_character.rs` does T35's.
//!
//! The five small routines read a handful of hunk 0 words and bytes and no hardware, so they lay no
//! base. Their states lean to where the branches turn: the key codes the keys test (`$41` HELP,
//! `$75`/`$74` Esc down and up, `$cd`/`$cc` P down and up), `countdown_tick` at `$3e`/`$3f` and past,
//! and BCD counters at 0, 1 and on a borrow (`$xx00`). A counter is valid BCD in seven states of
//! eight and any word in the eighth, so a port's `sbcd` is held to the 68000's on bad digits too.
//!
//! `game_frame` runs the whole pass: fifteen split callees (the character, the camera, the drawing,
//! the objects) and the five small routines inline. Random memory under them faults or hangs, so its
//! base is the original's own RAM at the level sync point ([`crate::sync::reach_level_1`], from the
//! cache, chip RAM at 0 and slow RAM at `$c00000`): the play state every callee reads. Its pokes are
//! only what its translation unit's own code reads (the pause and key variables, the countdown, the
//! character's and camera's positions near where they stand, `input_log`'s index) and JOY1DAT.
//!
//! `sbcd` adds the X flag, and `lives_minus_one` reaches its `sbcd` with X as it was entered (no
//! instruction before it sets X). The rig enters every call with `sr = $2700`, X clear; what X its
//! callers leave is not measured, so a port treating X as clear matches the rig and no more.

use std::sync::OnceLock;

use super::{Gen, Generator, Setup, h};

/// One variable a routine reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Var {
    /// `last_key_byte` (`h0+$1f78`), byte: the raw key code.
    LastKey,
    /// `cheat_flag` (`h0+$780c`), word.
    Cheat,
    /// `esc_down` (`h0+$21b0`), word.
    EscDown,
    /// `pause_key_down` (`h0+$2102`), word.
    PauseDown,
    /// `paused` (`h0+$2100`), word.
    Paused,
    /// `countdown_tick` (`h0+$20ac`), long.
    Tick,
    /// `countdown` (`h0+$6a80`), BCD word.
    Countdown,
    /// `lives` (`h0+$6a84`), BCD word.
    Lives,
    /// `char_x`, `char_y` (`h0+$1e5a`, `$1e5e`), longs: read by `char_sprite_position` in
    /// `game_frame`'s unit. Drawn near the sync point's position, which the split callees live with.
    CharX,
    CharY,
    /// `camera_x`, `camera_y` (`h0+$1e62`, `$1e64`), words: read by `h0_513c` and
    /// `char_sprite_position`. Drawn as [`Var::CharX`].
    CameraX,
    CameraY,
    /// `input_log`'s index (`h0+$8ec0`), long: `read_joystick` stores at `$34c28` plus it unless it
    /// is `$2ee0`.
    InputIndex,
}

impl Var {
    /// Its address and size in bytes.
    pub fn span(self) -> (u32, u32) {
        let (off, n) = match self {
            Var::LastKey => (0x1f78, 1),
            Var::Cheat => (0x780c, 2),
            Var::EscDown => (0x21b0, 2),
            Var::PauseDown => (0x2102, 2),
            Var::Paused => (0x2100, 2),
            Var::Tick => (0x20ac, 4),
            Var::Countdown => (0x6a80, 2),
            Var::Lives => (0x6a84, 2),
            Var::CharX => (0x1e5a, 4),
            Var::CharY => (0x1e5e, 4),
            Var::CameraX => (0x1e62, 2),
            Var::CameraY => (0x1e64, 2),
            Var::InputIndex => (0x8ec0, 4),
        };
        (h(0, off), n)
    }

    fn poke(self, g: &mut Gen, s: &mut Setup) {
        let at = self.span().0;
        match self {
            Var::LastKey => {
                let k = if g.one_in(4) {
                    g.next() as u8
                } else {
                    g.pick(&[0x41, 0x75, 0x74, 0xcd, 0xcc, 0x00])
                };
                s.b(at, k);
            }
            Var::Cheat | Var::EscDown | Var::PauseDown | Var::Paused => s.w(at, flag(g)),
            Var::Tick => {
                let t = match g.below(6) {
                    0 => g.next() as u32,
                    1 => g.pick(&[0xffff_ffff, 0x7fff_ffff, 0x8000_0000, 0x7fff_ffbf]),
                    _ => 0x3a + g.below(10) as u32,
                };
                s.l(at, t);
            }
            Var::Countdown | Var::Lives => s.w(at, bcd(g)),
            Var::CharX | Var::CharY | Var::CameraX | Var::CameraY => {
                let n = self.span().1;
                let now = sync_value(at, n);
                // Within 24 pixels of where it stands: char_sprite_position's parity and
                // ninth-bit tests turn, and the callees still find the level around it.
                let v = now.wrapping_add(g.below(49) as u32).wrapping_sub(24);
                if n == 4 {
                    s.l(at, v);
                } else {
                    s.w(at, v as u16);
                }
            }
            Var::InputIndex => {
                let i = match g.below(4) {
                    0 => 0x2ee0,
                    1 => 0x2edf,
                    _ => g.below(0x2ee0) as u32,
                };
                s.l(at, i);
            }
        }
    }
}

/// A word flag: clear half the time, else 1 as the game sets it, or any non-zero word.
fn flag(g: &mut Gen) -> u16 {
    match g.below(4) {
        0 | 1 => 0,
        2 => 1,
        _ => g.next() as u16 | 0x8000,
    }
}

/// A BCD word as the game keeps one, biased to 0, 1 and a borrow out of the low byte; any word in
/// one draw of eight.
fn bcd(g: &mut Gen) -> u16 {
    let digit = |g: &mut Gen| g.below(10) as u16;
    match g.below(8) {
        0 => g.next() as u16,
        1 => g.pick(&[0x0000, 0x0001, 0x0100, 0x1000, 0x0010, 0x9999]),
        2 => (digit(g) << 12 | digit(g) << 8) & 0xff00,
        _ => digit(g) << 12 | digit(g) << 8 | digit(g) << 4 | digit(g),
    }
}

/// JOY1DAT (`$dff00c`), which `read_joystick` reads in `game_frame`'s unit.
pub const JOY1DAT: u32 = 0xdf_f00c;

/// What each routine reads, by [`Var`], and its own flow's conditional branches.
pub struct Cover {
    pub name: &'static str,
    /// Hunk 0 offset of its entry.
    pub entry: u32,
    pub reads: &'static [Var],
    /// Laid over the level sync point's RAM ([`sync_ram`]), and reads JOY1DAT.
    pub play: bool,
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

pub const COVER: &[Cover] = &[
    Cover {
        name: "game_frame",
        entry: 0x1fec,
        // Its unit's inline callees: play_keys (with help_key, esc_key), countdown_step, h0_513c,
        // char_sprite_position and read_joystick. $67da..$67ea, which it shuffles round h0_2c7e,
        // are a split callee's pointers and stay as the sync point holds them.
        reads: &[
            LastKey, Cheat, EscDown, PauseDown, Paused, Tick, Countdown, CharX, CharY, CameraX,
            CameraY, InputIndex,
        ],
        play: true,
        branches: &[0x200a],
        unreachable: &[],
    },
    Cover {
        name: "countdown_step",
        entry: 0x20b0,
        reads: &[Tick, Countdown],
        play: false,
        branches: &[0x20c4, 0x20d8],
        unreachable: &[],
    },
    Cover {
        name: "play_keys",
        entry: 0x2104,
        reads: &[LastKey, Cheat, EscDown, PauseDown, Paused],
        play: false,
        branches: &[0x2116, 0x2128, 0x2132],
        unreachable: &[],
    },
    Cover {
        name: "help_key",
        entry: 0x2146,
        reads: &[LastKey, Cheat],
        play: false,
        branches: &[0x214e, 0x215a],
        unreachable: &[],
    },
    Cover {
        name: "esc_key",
        entry: 0x2176,
        reads: &[LastKey, EscDown],
        play: false,
        branches: &[0x2180, 0x2192, 0x219c],
        unreachable: &[],
    },
    Cover {
        name: "lives_minus_one",
        entry: 0x69d6,
        reads: &[Lives],
        play: false,
        branches: &[0x69dc],
        unreachable: &[],
    },
];

/// The cover entry named `name`.
pub fn cover(name: &str) -> Option<&'static Cover> {
    COVER.iter().find(|c| c.name == name)
}

/// Chip and slow RAM at the level sync point, read once per process from the sync cache (a machine
/// with no cache boots to W1-1 first, about 16 s). Panics when the Kickstart or disks are missing:
/// a generator has no error path, and the rig needs the disks anyway.
pub fn sync_ram() -> &'static (Vec<u8>, Vec<u8>) {
    static RAM: OnceLock<(Vec<u8>, Vec<u8>)> = OnceLock::new();
    RAM.get_or_init(|| {
        let am = crate::sync::reach_level_1(0)
            .unwrap_or_else(|e| panic!("game_frame's base, the level sync point: {e}"));
        let m = am.memory();
        (m.chip, m.slow)
    })
}

/// Where slow RAM sits on the bus.
pub const SLOW_BASE: usize = 0xc0_0000;

/// `game_frame`'s base: the sync point's chip and slow RAM over the rig's memory.
fn play_ram(m: &mut [u8]) {
    let (chip, slow) = sync_ram();
    m[..chip.len()].copy_from_slice(chip);
    m[SLOW_BASE..SLOW_BASE + slow.len()].copy_from_slice(slow);
}

/// The big-endian value of `n` bytes at `a` in the sync point's RAM.
fn sync_value(a: u32, n: u32) -> u32 {
    let (chip, slow) = sync_ram();
    let byte = |a: u32| {
        let a = a as usize;
        if a >= SLOW_BASE {
            slow[a - SLOW_BASE]
        } else {
            chip[a]
        }
    };
    (0..n).fold(0, |v, k| v << 8 | byte(a + k) as u32)
}

/// Every register random, then each variable `name` reads drawn.
fn state(g: &mut Gen, name: &str) -> Setup {
    let c = cover(name).expect("a T39 routine");
    let mut s = Setup::random(g);
    for v in c.reads {
        v.poke(g, &mut s);
    }
    if c.play {
        // Paused in a quarter of the states: the pass then only runs the six calls before the
        // test and play_keys.
        s.w(Paused.span().0, if g.one_in(4) { flag(g) | 1 } else { 0 });
        s.hw_reads.push((JOY1DAT, g.next() as u8));
        s.hw_reads.push((JOY1DAT + 1, g.next() as u8));
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
    "game_frame" => game_frame, Some(play_ram);
    "countdown_step" => countdown_step, None;
    "play_keys" => play_keys, None;
    "help_key" => help_key, None;
    "esc_key" => esc_key, None;
    "lives_minus_one" => lives_minus_one, None;
}
