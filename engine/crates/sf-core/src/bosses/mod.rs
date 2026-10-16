//! The final level's boss (T18, docs/09 "Bosses"): the one boss in the game. No world has a boss
//! of its own; each ends with its `LnET` picture (T183).

pub mod witch;

use crate::level::{Dispatch, register};

/// The final level's row of `obj_flag_jumptab`: W6's (`obj_world_offset` `$500`).
const FINAL: u8 = 5;

/// The witch and her marker, into `d`.
pub fn register_all(d: &mut Dispatch) {
    register(d, 134, &witch::WITCH);
    d.register_marker(FINAL, 1, witch::spawn_134);
}
