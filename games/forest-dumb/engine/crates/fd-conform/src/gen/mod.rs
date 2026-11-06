//! Every routine's generator, by name (plans/forest-dumb-level-1 DESIGN §4.3). A name is the
//! routine's in `re/symbols.toml`, or `hN+$off` where it has none yet. The translation service's
//! judge (T33) and an engine port's rig test look a generator up here, so it is written once.
//!
//! Each subsystem's generators live in their own module with a `pub const ALL` list; a generator
//! task (T35 to T39, T49) adds its module and appends one line to [`MODULES`], nothing else.

pub mod camera;
pub mod character;
pub mod character_anim;
pub mod drawing;
pub mod edge_redraw;
pub mod level_start;
pub mod objects;
pub mod objects_move;
pub mod rest;
pub mod spike;

pub use crate::routine::{Gen, Generator, Setup, h};

/// The generator service's candidate (T61, `tools/generate.py`): the module `FD_GEN` names, built in
/// by `build.rs`, or none. Listed first in [`MODULES`], so under its name it is the one looked up,
/// ahead of a committed generator of the same name. Never set outside the service's builds.
#[allow(unused_imports, dead_code, unused_variables, unused_mut, clippy::all)]
pub mod candidate {
    include!(concat!(env!("OUT_DIR"), "/candidate.rs"));
}

/// Every module's generators.
pub const MODULES: &[&[(&str, Generator)]] = &[
    candidate::ALL,
    spike::ALL,
    character::ALL,
    camera::ALL,
    objects::ALL,
    rest::ALL,
    drawing::ALL,
    edge_redraw::ALL,
    character_anim::ALL,
    objects_move::ALL,
    level_start::ALL,
];

/// The generator registered as `name`.
pub fn generator(name: &str) -> Option<Generator> {
    all().find(|(n, _)| *n == name).map(|(_, g)| g)
}

/// Every registered generator, with its name.
pub fn all() -> impl Iterator<Item = (&'static str, Generator)> {
    MODULES.iter().flat_map(|m| m.iter().copied())
}
