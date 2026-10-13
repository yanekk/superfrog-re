//! The bonus level (world 7, `LB*` files; T75–T79): shooter mode, where the camera creeps right on
//! its own and the frog fires shots instead of throwing the spud. Its object kinds register into
//! the level's dispatch here: the enemies of markers 1–6 ([`enemies`]).

pub mod enemies;

use crate::level::Dispatch;

/// Every bonus-level kind, into `d`.
pub fn register_all(d: &mut Dispatch) {
    enemies::register_all(d);
}
