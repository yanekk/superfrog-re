//! The seam's judge (translator T35, `judge::seam`, `fd-judge --seam`): a replacement against the
//! routine it replaces, on the same states.
//!
//! Made-up (no disks): a "translated" routine that stores d0 and d1 and returns, judged against a
//! replacement that does the same (passes), one that writes one byte differently (fails naming the
//! byte), one that returns somewhere else and one that panics.
//!
//! Full (the disks and the generated tree): `fd-translated`'s two adapters, `camera_update` and
//! `char_sprite_position`, against the translated routines they replace on 2 000 states each.

use fd_conform::judge::seam::{self, SeamReport};
use fd_conform::judge::translated::Entry;
use fd_conform::routine::{Gen, Generator, RegsCompared, Rig, Setup, h};
use pcode_rt::{Bus, Image, Mounted, Regs, Stop, rts};

const AT: u32 = 0x1000;
/// States a made-up check runs: each runs both sides on a thread of its own.
const CASES: usize = 50;

fn states() -> Generator {
    Generator {
        base: None,
        case: |g: &mut Gen| Setup::random(g),
    }
}

/// `move.l d0,($2000).l; move.w d1,($2004).l; rts`, as translated code would run it.
fn translated(r: &mut Regs, bus: &mut dyn Bus, _: &Image) -> Result<(), Stop> {
    bus.wl(0x2000, r.d(0));
    bus.ww(0x2004, r.d(1) as u16);
    rts(r, bus);
    Ok(())
}

fn same(r: &mut Regs, bus: &mut dyn Bus, _: &Image) -> Result<(), Stop> {
    let (d0, d1) = (r.d(0), r.d(1));
    // In another order and leaving d0 changed: neither shows unless registers are compared.
    bus.ww(0x2004, d1 as u16);
    bus.wl(0x2000, d0);
    r.set_d(0, 0);
    rts(r, bus);
    Ok(())
}

fn one_byte_off(r: &mut Regs, bus: &mut dyn Bus, _: &Image) -> Result<(), Stop> {
    bus.wl(0x2000, r.d(0));
    bus.ww(0x2004, r.d(1) as u16 ^ 1);
    rts(r, bus);
    Ok(())
}

fn no_rts(r: &mut Regs, bus: &mut dyn Bus, _: &Image) -> Result<(), Stop> {
    bus.wl(0x2000, r.d(0));
    bus.ww(0x2004, r.d(1) as u16);
    Ok(())
}

fn panics(_: &mut Regs, _: &mut dyn Bus, _: &Image) -> Result<(), Stop> {
    panic!("an overflow, say")
}

fn judge(replaced: Option<Entry>, replacing: Mounted, regs: RegsCompared) -> SeamReport {
    let rig = Rig::from_memory(vec![0; 0x10000]);
    seam::check(&rig, "made-up", CASES, AT, &states(), replaced, replacing, regs)
}

#[test]
fn a_replacement_that_does_the_same_passes() {
    let r = judge(Some(translated), same, RegsCompared::NONE);
    assert_eq!((r.passed, r.failures, r.replaced_stopped), (CASES, 0, 0), "{r:?}");
    assert!(r.pass());
}

#[test]
fn one_byte_written_differently_fails_naming_it() {
    let r = judge(Some(translated), one_byte_off, RegsCompared::NONE);
    assert_eq!(r.failures, CASES, "{r:?}");
    assert!(!r.pass());
    // The low byte of the word at $2004 is the one that differs, and only it.
    assert!(r.first.contains("$002005: replaced wrote $"), "{}", r.first);
    assert!(!r.first.contains("$002004"), "{}", r.first);
}

#[test]
fn registers_are_held_only_when_named() {
    let r = judge(Some(translated), same, RegsCompared::d(&[0]));
    assert!(r.failures > CASES - 5, "{r:?}");
    assert!(r.first.contains("d0: replaced $"), "{}", r.first);
}

#[test]
fn a_replacement_that_does_not_return_as_rts_does_fails() {
    let r = judge(Some(translated), no_rts, RegsCompared::NONE);
    assert_eq!(r.failures, CASES);
    assert!(r.first.contains("returned to: replaced $"), "{}", r.first);
    assert!(r.first.contains("a7: replaced $"), "{}", r.first);
}

#[test]
fn a_panicking_replacement_fails_and_no_translation_fails_every_state() {
    std::panic::set_hook(Box::new(|_| {}));
    let r = judge(Some(translated), panics, RegsCompared::NONE);
    let _ = std::panic::take_hook();
    assert_eq!(r.failures, CASES);
    assert!(r.first.contains("the replacing routine stopped: panicked: an overflow"), "{}", r.first);
    let r = judge(None, same, RegsCompared::NONE);
    assert_eq!((r.passed, r.failures), (0, CASES), "{r:?}");
    assert!(r.first.contains("the replaced routine stopped: untranslated: $001000"), "{}", r.first);
}

#[test]
#[ignore = "full: the disks and the generated tree (translate.py --build)"]
fn the_two_adapters_pass_against_the_translated_routines_on_2000_states() {
    assert!(
        fd_translated::TranslatedGame::generated(),
        "no generated tree: python3 -I games/forest-dumb/tools/translate.py --build"
    );
    let rig = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    for (name, off) in [("camera_update", 0x4d7e), ("char_sprite_position", 0x5462)] {
        let generator = fd_conform::generators::generator(name).unwrap();
        let key = rig.name(h(0, off));
        let replaced = fd_translated::tree::entry(&key);
        assert!(replaced.is_some(), "{key} is not in the tree");
        let replacing = fd_translated::seam::adapter((0, off)).unwrap();
        let r = seam::check(
            &rig,
            name,
            2000,
            h(0, off),
            &generator,
            replaced,
            replacing,
            RegsCompared::NONE,
        );
        assert!(r.pass(), "{name}: {r:?}");
        assert_eq!((r.passed, r.replaced_stopped), (2000, 0), "{name}: {r:?}");
    }
}
