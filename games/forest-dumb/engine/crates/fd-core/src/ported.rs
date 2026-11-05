//! The port register (plans/forest-dumb-level-1 DESIGN §4.1; `toolkit/docs/porting-routines.md` §4):
//! every routine of the original ported into this crate, by hunk and offset, with the Rust function
//! that ports it. Its name is the routine's `kind = "func"` entry in `re/symbols.toml`, at the same
//! hunk and offset; `fd-conform/tests/ported.rs` fails on a mismatch and on two entries for one
//! address, because parallel ports of one routine were the first game's most repeated silent gap.
//!
//! Porting tasks only append to [`PORTED`]. A routine another task registered is called, never
//! ported a second time. The coverage report (T06) counts how often the original ran each entry.

/// One ported routine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ported {
    /// Its name in `re/symbols.toml`.
    pub name: &'static str,
    pub hunk: u8,
    /// Byte offset within the hunk, as `hN+$off` in the listing.
    pub offset: u32,
    /// The Rust function that ports it, as a path from this crate's root (`level::char_step`).
    pub port: &'static str,
}

/// Every ported routine. Append only.
pub const PORTED: &[Ported] = &[
    // T51: the animation frame pickers and the joystick read.
    port("char_step_4be6", 0x4be6, "character::Anim::walk_left"),
    port("h0_4c28", 0x4c28, "character::Anim::walk_right"),
    port("h0_4c6a", 0x4c6a, "character::Anim::frame_from_probes"),
    port("h0_4d32", 0x4d32, "character::Anim::frame_4d32"),
    port("h0_4d54", 0x4d54, "character::Anim::frame_4d54"),
    port(
        "read_joystick",
        0x8dfc,
        "character::JoyFlags::read_joystick",
    ),
];

/// A hunk 0 routine's entry.
const fn port(name: &'static str, offset: u32, port: &'static str) -> Ported {
    Ported {
        name,
        hunk: 0,
        offset,
        port,
    }
}
