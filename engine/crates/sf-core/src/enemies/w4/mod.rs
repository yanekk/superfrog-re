//! World 4 (`obj_world_offset` `$300`): every type its markers spawn, walked from
//! `obj_flag_jumptab`'s W4 row (docs/09). Markers 1–3, 10–22, 24–27, 32 and 33 spawn templates
//! 135, 137, 136, 32, 27, 93, 139, 138, 141, 142, 69 (with its chain, 70), 59–62, 143, 144, 145,
//! 151 (whose spray is 152–159), 148, 146 and 147. W4 uses markers 24–28 as objects (docs/09).

pub mod crawler;
pub mod crusher;
pub mod dart;
pub mod floater;
pub mod fountain;
pub mod gate;
pub mod ledge;
pub mod mace;
pub mod pacer;
pub mod scenery;
pub mod spikes;
pub mod swooper;
pub mod walker;

use crate::level::{Dispatch, register};

/// W4's row of `obj_flag_jumptab`.
pub const W4: u8 = 3;

/// Every W4 kind and spawner, into `d`.
pub fn register_all(d: &mut Dispatch) {
    register(d, 27, &scenery::SCENERY_B);
    register(d, 32, &scenery::SCENERY_A);
    for (t, kind) in dart::DARTS.iter().enumerate() {
        register(d, 59 + t as u16, kind);
    }
    register(d, 69, &mace::MACE);
    register(d, 70, &mace::MACE_LINK);
    register(d, 93, &walker::WALKER);
    register(d, 135, &swooper::SWOOPER);
    register(d, 136, &pacer::PACER_RIGHT);
    register(d, 137, &pacer::PACER_LEFT);
    register(d, 138, &crawler::CRAWLER_RIGHT);
    register(d, 139, &crawler::CRAWLER_LEFT);
    register(d, 141, &crusher::CRUSHER_DOWN);
    register(d, 142, &crusher::CRUSHER_UP);
    register(d, 143, &floater::FLOATER);
    register(d, 144, &floater::FLOATER);
    register(d, 145, &spikes::SPIKES);
    register(d, 146, &gate::GATE_A);
    register(d, 147, &gate::GATE_B);
    register(d, 148, &ledge::LEDGE);
    register(d, 151, &fountain::FOUNTAIN);
    for t in 152..=159 {
        register(d, t, &fountain::SPRAY);
    }
    d.register_marker(W4, 1, swooper::spawn_135);
    d.register_marker(W4, 2, pacer::spawn_137);
    d.register_marker(W4, 3, pacer::spawn_136);
    d.register_marker(W4, 10, scenery::spawn_32);
    d.register_marker(W4, 11, scenery::spawn_27);
    d.register_marker(W4, 12, walker::spawn_93);
    d.register_marker(W4, 13, crawler::spawn_139);
    d.register_marker(W4, 14, crawler::spawn_138);
    d.register_marker(W4, 15, crusher::spawn_141);
    d.register_marker(W4, 16, crusher::spawn_142);
    d.register_marker(W4, 17, mace::spawn_69);
    d.register_marker(W4, 18, dart::spawn_59);
    d.register_marker(W4, 19, dart::spawn_60);
    d.register_marker(W4, 20, dart::spawn_61);
    d.register_marker(W4, 21, dart::spawn_62);
    d.register_marker(W4, 22, floater::spawn_143);
    d.register_marker(W4, 24, floater::spawn_144);
    d.register_marker(W4, 25, spikes::spawn_145);
    d.register_marker(W4, 26, fountain::spawn_151);
    d.register_marker(W4, 27, ledge::spawn_148);
    d.register_marker(W4, 32, gate::spawn_146);
    d.register_marker(W4, 33, gate::spawn_147);
}
