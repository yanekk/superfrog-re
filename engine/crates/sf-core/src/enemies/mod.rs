//! The enemies and hazards of worlds 1–6 (T39–T44, docs/09 "Types per world"): each world's new
//! object types, the markers that spawn them (its row of `obj_flag_jumptab`, h0+$178ec) and what
//! they share ([`common`]), registered into the level's dispatch.

pub mod common;
pub mod w1;
pub mod w2;
pub mod w3;
pub mod w4;
pub mod w6;

use crate::level::{Dispatch, register_routines};

/// Every enemy kind and spawner ported so far, into `d`.
pub fn register_all(d: &mut Dispatch) {
    register_routines(d, common::ROUTINES);
    w1::register_all(d);
    w2::register_all(d);
    w3::register_all(d);
    w4::register_all(d);
    w6::register_all(d);
}
