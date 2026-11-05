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
    // T08: the movement. `game_frame` is T63's, once every routine it calls is ported.
    port("char_update", 0x3e0c, "character::Character::update"),
    port("char_step", 0x3e30, "character::Character::step"),
    port("h0_3ea8", 0x3ea8, "character::Character::idle"),
    port("h0_410c", 0x410c, "character::Character::land_or_fall"),
    port("h0_416c", 0x416c, "character::Character::fall_step"),
    port("h0_4274", 0x4274, "character::Character::rise_step"),
    port("h0_4696", 0x4696, "character::Character::move_x"),
    port("h0_4718", 0x4718, "character::Character::left_pressed"),
    port("h0_47e2", 0x47e2, "character::Character::run_left_step"),
    port("h0_48be", 0x48be, "character::Character::right_pressed"),
    port("h0_497c", 0x497c, "character::Character::run_right_step"),
    port("h0_4b0e", 0x4b0e, "character::Character::up_pressed"),
    port("char_probe", 0x4f84, "character::Character::probe"),
    port(
        "char_probe_flags",
        0x4f90,
        "character::Character::probe_flags",
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
