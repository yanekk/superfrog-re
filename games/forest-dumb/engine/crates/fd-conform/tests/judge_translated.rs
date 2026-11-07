//! The judge's translated mode (translator-trial T04) on the main program from the owner's disks:
//! the stub tree built in without `FD_TRANSLATED` stops untranslated on every state (the negative
//! check), a hand-written `pcode-rt` entry for `h0+$5462` passes and the same with one constant
//! changed fails naming the register, and a store into translated code fails as `write-into-code`.

use std::process::Command;

use amiga_oracle::RETURN_SENTINEL;
use fd_conform::generators::{self, h};
use fd_conform::judge::translated::{self, Entry, tree};
use fd_conform::routine::Rig;
use pcode_rt::{Bus, Image, Regs, Stop, load, store};

const ROUTINE: &str = "h0+$5462";
const GEN: &str = "char_sprite_position";

/// `h0+$5462` (`char_sprite_position`) by hand, as translated code is written: registers through
/// `Regs`, memory through `load`/`store`, the program's addresses through `img.h`, and a top-level
/// `rts` that pops the return address the judge put on the stack. `k` is the x offset, $3c in the
/// original.
fn sprite_position(r: &mut Regs, bus: &mut dyn Bus, img: &Image, k: u32) -> Result<(), Stop> {
    let lo = |hi: u32, v: u32| (hi & 0xffff_0000) | (v & 0xffff);
    let mut d3 = 0u32;
    let mut d0 = load(bus, img.h(0, 0x1e5a), 4) as u32;
    d0 = lo(d0, d0.wrapping_sub(load(bus, img.h(0, 0x1e62), 2) as u32));
    if d0 & 1 != 0 {
        d3 = 1;
    }
    d0 = (d0 >> 1).wrapping_add(k).rotate_left(16);
    let mut d1 = load(bus, img.h(0, 0x1e5e), 4) as u32;
    d1 = lo(d1, d1.wrapping_sub(load(bus, img.h(0, 0x1e64), 2) as u32));
    d1 = d1.wrapping_add(0x31);
    let mut d2 = d1;
    if d1 >> 8 & 1 != 0 {
        d0 |= 4;
    }
    d1 = d1.rotate_left(16) << 8;
    d2 = d2.wrapping_add(0x28);
    if d2 >> 8 & 1 != 0 {
        d0 |= 2;
    }
    d2 = (d2 & 0xff) << 8;
    d0 |= d1 | d2 | 0x80 | d3;
    for off in [0xd30, 0xdd8] {
        store(bus, img, img.h(1, off), 4, d0 as u64)?;
    }
    d0 = d0.wrapping_add(0x8_0000);
    for off in [0xe80, 0xf28] {
        store(bus, img, img.h(1, off), 4, d0 as u64)?;
    }
    for (n, v) in [d0, d1, d2, d3].into_iter().enumerate() {
        r.set_d(n, v);
    }
    let sp = r.a(7);
    let ret = load(bus, sp, 4) as u32;
    r.set_a(7, sp.wrapping_add(4));
    if ret != RETURN_SENTINEL {
        return Err(Stop::Untranslated(img.describe(ret)));
    }
    Ok(())
}

fn right(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    sprite_position(r, bus, img, 0x3c)
}

fn off_by_one(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    sprite_position(r, bus, img, 0x3d)
}

/// Writes a word over the routine's own first instruction.
fn writes_into_itself(_: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    store(bus, img, img.h(0, 0x5462), 2, 0x4e71)
}

fn never_ends(_: &mut Regs, _: &mut dyn Bus, _: &Image) -> Result<(), Stop> {
    Err(Stop::Hang)
}

/// The routine's instructions as a tree would list them in `CODE`.
const CODE: &[(u32, u32, u32)] = &[(0, 0x5462, 0x54c2)];

fn judge(
    r: &mut Rig,
    cases: usize,
    run: Option<Entry>,
    code: &[(u32, u32, u32)],
) -> translated::Judged {
    let generator = generators::generator(GEN).unwrap();
    translated::check(r, GEN, cases, h(0, 0x5462), &generator, run, code)
}

#[test]
fn the_stub_tree_stops_untranslated_on_every_state() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    // The test build has no FD_TRANSLATED.
    assert!(tree::entry(ROUTINE).is_none());
    assert!(tree::CODE.is_empty());
    let j = judge(&mut r, 200, tree::entry(ROUTINE), tree::CODE);
    let f = j.verdict.unwrap_err();
    assert_eq!(f.failures, 200, "{f}");
    assert_eq!(j.stops.untranslated, 200);
    assert_eq!(j.stops.first, "untranslated: h0+$5462");
    assert!(
        f.first.contains("the port stopped: untranslated: h0+$5462"),
        "{f}"
    );
}

#[test]
fn a_hand_translation_passes_and_one_constant_changed_fails_naming_d0() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let j = judge(&mut r, 2000, Some(right), CODE);
    let rep = j.verdict.unwrap_or_else(|f| panic!("{f}"));
    assert_eq!((rep.passed, rep.hangs, rep.faults), (2000, 0, 0));
    assert_eq!(j.stops, translated::Stops::default());

    let j = judge(&mut r, 50, Some(off_by_one), CODE);
    let f = j.verdict.unwrap_err();
    // Not every state: on some the changed bit is one the routine ORs in anyway.
    assert!(f.failures > 40, "{f}");
    assert!(f.first.contains("d0: original $"), "{f}");
    assert_eq!(
        j.stops,
        translated::Stops::default(),
        "a wrong value is no stop"
    );
}

#[test]
fn a_store_into_code_and_the_step_cap_are_stops_counted_by_kind() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let j = judge(&mut r, 30, Some(writes_into_itself), CODE);
    let f = j.verdict.unwrap_err();
    assert_eq!(f.failures, 30, "{f}");
    assert_eq!((j.stops.write_into_code, j.stops.untranslated), (30, 0));
    assert_eq!(j.stops.first, "write-into-code: h0+$5462");
    assert!(
        f.first
            .contains("the port stopped: write-into-code: h0+$5462"),
        "{f}"
    );

    // Without the routine in CODE the same word is a plain write, and a wrong one.
    let j = judge(&mut r, 30, Some(writes_into_itself), &[]);
    assert_eq!(j.stops.write_into_code, 0);
    assert!(j.verdict.unwrap_err().first.contains("byte h0+$5462"));

    let j = judge(&mut r, 10, Some(never_ends), CODE);
    assert_eq!(j.verdict.unwrap_err().failures, 10);
    assert_eq!((j.stops.hangs, j.stops.first.as_str()), (10, "hang"));
}

#[test]
fn fd_judge_translated_prints_the_stops_beside_the_verdict() {
    let out = Command::new(env!("CARGO_BIN_EXE_fd-judge"))
        .args([ROUTINE, "--gen", GEN, "--translated", "--cases", "20"])
        .env_remove("FD_TRANSLATED")
        .output()
        .unwrap();
    let line = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{line}{}",
        String::from_utf8_lossy(&out.stderr)
    );
    for want in [
        "\"pass\":false",
        "\"failures\":20",
        "\"untranslated\":20,\"write_into_code\":0,\"unsupported\":0,\"stopped_hangs\":0",
        "\"first_stop\":\"untranslated: h0+$5462\"}",
    ] {
        assert!(line.contains(want), "{want} not in {line}");
    }

    let both = Command::new(env!("CARGO_BIN_EXE_fd-judge"))
        .args([ROUTINE, "--gen", GEN, "--translated", "--survey"])
        .output()
        .unwrap();
    assert_eq!(both.status.code(), Some(2));
}
