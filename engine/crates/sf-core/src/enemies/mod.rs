//! The enemies and hazards of the six worlds (T39–T44): each world's object types that hurt the
//! frog or that it can kill, and the scenery its maps place with them, each the original's
//! template with its routines (docs/09 "Types per world"), registered into the level's dispatch
//! with the marker spawners of its world's row of `obj_flag_jumptab` (h0+$178ec). What every
//! world shares is in [`common`].

pub mod common;
pub mod w2;

use crate::level::{Dispatch, register_routines};

/// Every enemy kind and spawner ported, into `d`.
pub fn register_all(d: &mut Dispatch) {
    register_routines(d, common::ROUTINES);
    w2::register_all(d);
}
