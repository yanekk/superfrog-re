//! Every routine's generator, by name (plans/forest-dumb-level-1 DESIGN §4.3). A name is the
//! routine's in `re/symbols.toml`, or `hN+$off` where it has none yet. The translation service's
//! judge (T33) and an engine port's rig test look a generator up here, so it is written once.
//!
//! Each subsystem's generators live in their own module with a `pub const ALL` list; a generator
//! task (T35 to T39, T49) adds its module and appends one line to [`MODULES`], nothing else.

pub mod camera;
pub mod character;
pub mod objects;
pub mod rest;
pub mod spike;

pub use crate::routine::{Gen, Generator, Setup, h};

/// Every module's generators.
pub const MODULES: &[&[(&str, Generator)]] =
    &[spike::ALL, character::ALL, camera::ALL, objects::ALL, rest::ALL];

/// The generator registered as `name`.
pub fn generator(name: &str) -> Option<Generator> {
    all().find(|(n, _)| *n == name).map(|(_, g)| g)
}

/// Every registered generator, with its name.
pub fn all() -> impl Iterator<Item = (&'static str, Generator)> {
    MODULES.iter().flat_map(|m| m.iter().copied())
}
