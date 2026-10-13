//! The bonus level (world 7, `LB*` files; T75–T79): shooter mode, where the camera creeps right on
//! its own and the frog fires shots instead of throwing the spud. Its object kinds register into
//! the level's dispatch here: the frog's shots ([`shots`]), and what a shot
//! enemy turns into ([`death`]).

pub mod death;
pub mod shots;

use crate::level::{Dispatch, register, register_routines};

/// Every bonus-level kind, into `d`.
pub fn register_all(d: &mut Dispatch) {
    register(d, 110, &shots::SHOT);
    register(d, 111, &shots::SHOT_UP);
    register(d, 112, &shots::SHOT_DOWN);
    register_routines(d, death::ROUTINES);
}
