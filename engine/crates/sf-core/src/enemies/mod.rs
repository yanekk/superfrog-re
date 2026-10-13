//! The enemies, hazards and scenery of worlds 1–6 (T39–T44): one module per world, each kind the
//! original's template with its routines (docs/09 "Types per world"), registered into the level's
//! dispatch; what they share is in [`common`].

pub mod common;
pub mod w4;

use crate::level::{Dispatch, register_routines};

/// Every enemy kind and spawner ported, into `d`.
pub fn register_all(d: &mut Dispatch) {
    register_routines(d, common::ROUTINES);
    w4::register_all(d);
}
