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
    // T09, the camera (level/camera.rs).
    Ported {
        name: "camera_update",
        hunk: 0,
        offset: 0x4d7e,
        port: "level::camera::Camera::update",
    },
    Ported {
        name: "h0_4e9a",
        hunk: 0,
        offset: 0x4e9a,
        port: "level::camera::Camera::follow",
    },
    Ported {
        name: "h0_4ea6",
        hunk: 0,
        offset: 0x4ea6,
        port: "level::camera::Camera::follow",
    },
    Ported {
        name: "h0_4eb4",
        hunk: 0,
        offset: 0x4eb4,
        port: "level::camera::Camera::follow",
    },
    Ported {
        name: "h0_4ec2",
        hunk: 0,
        offset: 0x4ec2,
        port: "level::camera::Camera::follow",
    },
    Ported {
        name: "h0_513c",
        hunk: 0,
        offset: 0x513c,
        port: "level::camera::Camera::clamp",
    },
    Ported {
        name: "h0_51fc",
        hunk: 0,
        offset: 0x51fc,
        port: "level::camera::Camera::scroll",
    },
    Ported {
        name: "h0_526a",
        hunk: 0,
        offset: 0x526a,
        port: "level::camera::Camera::scroll",
    },
    Ported {
        name: "h0_52d8",
        hunk: 0,
        offset: 0x52d8,
        port: "level::camera::Camera::scroll",
    },
    Ported {
        name: "h0_5346",
        hunk: 0,
        offset: 0x5346,
        port: "level::camera::Camera::scroll",
    },
];
