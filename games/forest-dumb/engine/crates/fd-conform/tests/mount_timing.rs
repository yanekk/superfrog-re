//! The frame mount's timing rules (translator T56, DESIGN §2.9): each routine's path worked out
//! from the level it finds (`fd_translated::seam::rules`), walked through the original's
//! instructions (`seam::walk`), spends the cycles the translated routine counts, contention out,
//! on the routine judge's states for that routine (`judge::seam::cycles`).
//!
//! Full: the disks and the generated tree. A no-disk check keeps every ruled routine in the list.

use std::any::Any;
use std::sync::Arc;

use fd_conform::judge::seam;
use fd_conform::routine::{Rig, h};
use fd_translated::seam::frame::time_alone;
use fd_translated::seam::rules::RULED;

/// Each ruled routine and the routine judge's generator for it.
/// A routine with no generator of its own is checked on `game_frame`'s, whose states hold the
/// whole level.
const CHECKED: [(&str, u32); 10] = [
    ("game_frame", 0x9b06),
    ("game_frame", 0x94ba),
    ("camera_update", 0x4d7e),
    ("h0_513c", 0x513c),
    ("char_sprite_position", 0x5462),
    ("h0_9754", 0x9754),
    ("game_frame", 0x8dfc),
    ("game_frame", 0x20b0),
    ("game_frame", 0x2104),
    ("h0_21c0", 0x21c0),
];

/// States a routine is checked on.
const STATES: usize = 2000;

#[test]
fn every_ruled_routine_is_checked() {
    let mut checked: Vec<u32> = CHECKED.iter().map(|c| c.1).collect();
    checked.sort();
    let mut ruled = RULED.to_vec();
    ruled.sort();
    assert_eq!(checked, ruled);
}

#[test]
#[ignore = "full: the disks and the generated tree (translate.py --build); about a minute"]
fn each_rule_spends_the_translated_routines_cycles_on_its_judge_states() {
    assert!(
        fd_translated::TranslatedGame::generated(),
        "no generated tree: python3 -I games/forest-dumb/tools/translate.py --build"
    );
    let rig = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    let level: Arc<dyn Any + Send + Sync> = Arc::new(fd_translated::Level1::new(data));
    let only = std::env::var("MOUNT_TIMING_ONLY").ok();
    let mut failed = Vec::new();
    for (name, off) in CHECKED {
        if only
            .as_deref()
            .is_some_and(|o| o != name && o != format!("{off:x}"))
        {
            continue;
        }
        let generator = fd_conform::generators::generator(name).unwrap();
        let key = rig.name(h(0, off));
        let replaced = fd_translated::tree::entry(&key).unwrap_or_else(|| panic!("{key}"));
        let alone = move |r: &mut pcode_rt::Regs,
                          m: &mut fd_conform::judge::Mem,
                          img: &pcode_rt::Image| { time_alone(off, r, m, img) };
        let rep = seam::cycles(
            &rig,
            name,
            STATES,
            h(0, off),
            &generator,
            replaced,
            &alone,
            Some(level.clone()),
        );
        if !(rep.pass() && rep.passed == STATES) {
            failed.push(format!("h0+${off:x} on {name}: {rep:?}"));
        }
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}
