//! The frame mount's timing rules (translator T56, DESIGN §2.9): each routine's path worked out
//! from the level it finds (`fd_translated::seam::rules`), walked through the original's
//! instructions (`seam::walk`), spends the cycles the translated routine counts, contention out,
//! on the routine judge's states for that routine (`judge::seam::cycles`).
//!
//! Full: the disks and the generated tree. A no-disk check keeps every ruled routine in the list.

use std::any::Any;
use std::sync::Arc;

use fd_conform::generators::{self, Gen, Generator, Setup};
use fd_conform::judge::seam;
use fd_conform::routine::{Rig, h};
use fd_translated::seam::frame::time_alone;
use fd_translated::seam::rules::RULED;

/// Each ruled routine and the routine judge's generator for it.
/// A routine with no generator of its own is checked on [`varied`]: `game_frame`'s states, which
/// hold the whole level at its sync point, with what those routines read varied.
const CHECKED: [(&str, u32); 14] = [
    ("h0_3698", 0x3698),
    ("h0_24ce", 0x24ce),
    ("varied", 0x9b06),
    ("varied", 0x94ba),
    ("camera_update", 0x4d7e),
    ("h0_513c", 0x513c),
    ("char_sprite_position", 0x5462),
    ("h0_9754", 0x9754),
    ("varied", 0x8dfc),
    ("varied", 0x20b0),
    ("varied", 0x2104),
    ("h0_21c0", 0x21c0),
    ("varied", 0x2c7e),
    ("varied", 0x2d4c),
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

/// `game_frame`'s state with the creatures (flags, drawn flags, places, kinds, facings, phases),
/// the fade (on, its pass, the colours), the sprite's flicker count and the spring varied: what the
/// routines checked on it branch on, which the sync point holds at one value.
fn varied(g: &mut Gen) -> Setup {
    let mut s = (generators::generator("game_frame").unwrap().case)(g);
    let h0 = |o: u32| h(0, o);
    for i in 0..5u32 {
        s.b(h0(0x67da + i), g.pick(&[0, 0xff]));
        s.b(h0(0x67e6 + i), g.pick(&[0, 0xff]));
        let at = h0(0x67ec + 8 * i);
        s.w(at, g.below(0x1800) as u16);
        s.w(at + 2, g.below(0x560) as u16);
        s.w(at + 4, g.below(8) as u16);
        s.b(at + 6, g.pick(&[0, 1, 0xff]));
        s.b(at + 7, g.below(0x100) as u8);
    }
    s.b(h0(0x94b4), g.pick(&[0, 0xff]));
    s.l(h0(0x94b6), g.below(0x20) as u32);
    for i in 0..32 {
        s.w(h(1, 0x13e + 4 * i), g.below(0x1000) as u16);
    }
    s.b(
        h0(0x9b04),
        if g.one_in(2) { 0 } else { g.below(0x100) as u8 },
    );
    s.l(
        h0(0x21b4),
        g.below(0x1800) as u32 | if g.one_in(4) { 0x1_0000 } else { 0 },
    );
    s.l(h0(0x21b8), g.below(0x560) as u32);
    s.w(h0(0x21bc), g.pick(&[0, 1, 2, 5, 6, 7]));
    s.w(h0(0x21be), g.pick(&[0, 1]));
    s
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
        let generator = if name == "varied" {
            let gf = generators::generator("game_frame").unwrap();
            Generator {
                base: gf.base,
                case: varied,
            }
        } else {
            generators::generator(name).unwrap()
        };
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
