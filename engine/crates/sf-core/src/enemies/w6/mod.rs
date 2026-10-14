//! World 6 and the final level (`obj_world_offset` `$500`): every type but the witch (marker 1,
//! T18). Markers 12–20 spawn templates 101–106, 57, 58 and 67 (with its chain, 68); 101, 103 and
//! 105 fire 109, 107 and 108. Walked from `obj_flag_jumptab`'s W6 row (docs/09).

pub mod cannon;
pub mod charger;
pub mod dart;
pub mod flier;
pub mod mace;
pub mod shot;
pub mod thrower;
pub mod walker;

use crate::level::{Dispatch, register};

/// W6's row of `obj_flag_jumptab`.
const W6: u8 = 5;

/// Every W6 kind and spawner, into `d`.
pub fn register_all(d: &mut Dispatch) {
    register(d, 57, &dart::DART_LEFT);
    register(d, 58, &dart::DART_RIGHT);
    register(d, 67, &mace::MACE);
    register(d, 68, &mace::MACE_LINK);
    register(d, 101, &cannon::CANNON);
    register(d, 102, &charger::CHARGER);
    register(d, 103, &flier::FLIER);
    register(d, 104, &walker::WALKER);
    register(d, 105, &thrower::THROWER);
    register(d, 106, &walker::STRIDER);
    register(d, 107, &shot::SHOT);
    register(d, 108, &shot::FALLING_SHOT);
    register(d, 109, &cannon::CANNON_SHOT);
    d.register_marker(W6, 12, cannon::spawn_101);
    d.register_marker(W6, 13, charger::spawn_102);
    d.register_marker(W6, 14, flier::spawn_103);
    d.register_marker(W6, 15, walker::spawn_104);
    d.register_marker(W6, 16, thrower::spawn_105);
    d.register_marker(W6, 17, mace::spawn_67);
    d.register_marker(W6, 18, dart::spawn_57);
    d.register_marker(W6, 19, dart::spawn_58);
    d.register_marker(W6, 20, walker::spawn_106);
}
