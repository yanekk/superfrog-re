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
pub const PORTED: &[Ported] = &[];
