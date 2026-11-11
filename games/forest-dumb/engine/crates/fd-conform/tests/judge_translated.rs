//! The judge's translated mode (translator-trial T04) on the main program from the owner's disks:
//! the stub tree built in without `FD_TRANSLATED` stops untranslated on every state (the negative
//! check), a hand-written `pcode-rt` entry for `h0+$5462` passes and the same with one constant
//! changed fails naming the register, and a store into translated code fails as `write-into-code`.

use std::process::Command;

use amiga_oracle::RETURN_SENTINEL;
use fd_conform::generators::{self, h};
use fd_conform::judge::translated::{self, Entry, tree};
use fd_conform::routine::{Generator, Rig, Setup};
use pcode_rt::{Bus, DEPTH_CAP, Hang, Image, Regs, Stop, fn_entry, load, store};

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
    Err(Stop::Hang { why: Hang::Steps })
}

/// Calls itself for ever, as generated code does (`fn_entry` first), each frame holding 128 KB: at
/// `DEPTH_CAP` frames that is more stack than a test thread's 2 MB, so only the judge's own thread
/// ([`translated::RUN_STACK`]) lets it reach the cap rather than overflow and abort.
fn recurses(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fn_entry(r)?;
    let mut frame = [0u8; 128 << 10];
    frame[r.depth as usize] = 1;
    std::hint::black_box(&mut frame);
    recurses(r, bus, img)?;
    Ok(())
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
    assert_eq!((j.stops.hangs, j.stops.first.as_str()), (10, "hang (steps)"));

    let j = judge(&mut r, 10, Some(recurses), CODE);
    assert_eq!(j.verdict.unwrap_err().failures, 10);
    assert_eq!((j.stops.hangs, j.stops.first.as_str()), (10, "hang (depth)"));
    assert!(DEPTH_CAP as usize * (128 << 10) > 2 << 20, "the frames outgrow a test thread");
    assert!(DEPTH_CAP as usize * (256 << 10) <= translated::RUN_STACK);
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

/// `movea.l a7,a0; movea.l (a7),a1; rts` at $2000 in empty memory: the stack pointer and the word
/// on top of it at entry, which the oracle sets to `STACK - 4` with its return address there.
fn stack_at_entry_rig() -> Rig {
    let mut mem = vec![0u8; 0x10_0000];
    for (i, w) in [0x204fu16, 0x2257, 0x4e75].iter().enumerate() {
        mem[0x2000 + i * 2..0x2002 + i * 2].copy_from_slice(&w.to_be_bytes());
    }
    let mut r = Rig::from_memory(mem);
    r.set_max_instr(1_000);
    r
}

fn stack_at_entry(r: &mut Regs, bus: &mut dyn Bus, _: &Image) -> Result<(), Stop> {
    let sp = r.a(7);
    r.set_a(0, sp);
    r.set_a(1, load(bus, sp, 4) as u32);
    r.set_a(7, sp.wrapping_add(4));
    Ok(())
}

#[test]
fn the_entry_sees_the_stack_the_original_sees() {
    let mut r = stack_at_entry_rig();
    let random = Generator {
        base: None,
        case: Setup::random,
    };
    let j = translated::check(
        &mut r,
        "stack",
        20,
        0x2000,
        &random,
        Some(stack_at_entry),
        &[],
    );
    let rep = j.verdict.unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(rep.passed, 20);
}

/// A made-up routine laid over hunk 0 (translator T15, DESIGN §2.5): `move.b d0,(a0); rts`, then a
/// `nop` it never runs. The tree lists all three as its code.
const SELF_AT: u32 = fd_conform::HUNK_BASES[0] + 0x5462;
const SELF_CODE: &[(u32, u32, u32)] = &[(0, 0x5462, 0x5468)];
/// The `nop`'s first byte: a code byte the routine writes on some states.
const SELF_TARGET: u32 = SELF_AT + 4;
/// Where it writes on the others: data.
const SELF_DATA: u32 = fd_conform::HUNK_BASES[1] + 0x100;

fn lay_self_writer(m: &mut [u8]) {
    let at = SELF_AT as usize;
    m[at..at + 6].copy_from_slice(&[0x10, 0x80, 0x4e, 0x75, 0x4e, 0x71]);
}

/// Random registers; `a0` at the routine's own `nop` one state in four, else at data; `d0`'s low
/// byte $12, so the write always changes the `nop`'s $4e.
fn self_writer_case(g: &mut fd_conform::routine::Gen) -> Setup {
    let mut s = Setup::random(g);
    s.d[0] = s.d[0] & !0xff | 0x12;
    s.a[0] = if g.one_in(4) { SELF_TARGET } else { SELF_DATA };
    s
}

const SELF_WRITER: Generator = Generator {
    base: Some(lay_self_writer),
    case: self_writer_case,
};

fn pop_return(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let sp = r.a(7);
    let ret = load(bus, sp, 4) as u32;
    r.set_a(7, sp.wrapping_add(4));
    if ret != RETURN_SENTINEL {
        return Err(Stop::Untranslated(img.describe(ret)));
    }
    Ok(())
}

/// The routine as translated code runs it: the byte store goes through `store`, which stops on a
/// translated instruction.
fn self_writer(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    store(bus, img, r.a(0), 1, (r.d(0) & 0xff) as u64)?;
    pop_return(r, bus, img)
}

/// Wrong on purpose: always stores into the `nop`, also where the original writes data.
fn always_into_code(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    store(bus, img, SELF_TARGET, 1, (r.d(0) & 0xff) as u64)?;
    pop_return(r, bus, img)
}

fn judge_self(r: &mut Rig, run: Entry, code: &[(u32, u32, u32)]) -> translated::Judged {
    translated::check(
        r,
        "self-writer",
        200,
        SELF_AT,
        &SELF_WRITER,
        Some(run),
        code,
    )
}

#[test]
fn states_where_the_original_writes_into_its_code_are_self_modifying_the_rest_judged() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let j = judge_self(&mut r, self_writer, SELF_CODE);
    let rep = j.verdict.unwrap_or_else(|f| panic!("{f}"));
    // About one state in four; the port never ran on them, so no stop was made.
    assert!(
        (25..=80).contains(&rep.self_modifying),
        "{} self-modifying",
        rep.self_modifying
    );
    assert_eq!(rep.passed + rep.self_modifying, 200);
    assert_eq!((rep.hangs, rep.faults), (0, 0));
    assert_eq!(j.stops, translated::Stops::default());
    let self_modifying = rep.self_modifying;

    // A translated store into code where the original writes data is still a failure, a stop.
    let j = judge_self(&mut r, always_into_code, SELF_CODE);
    let f = j.verdict.unwrap_err();
    assert_eq!(f.report.self_modifying, self_modifying, "the same states");
    assert_eq!(f.failures, 200 - self_modifying, "{f}");
    assert_eq!(j.stops.write_into_code, f.failures);
    assert_eq!(j.stops.first, "write-into-code: h0+$5466");

    // With no tree code the rule is off: every state is judged, the code write a plain write.
    let j = judge_self(&mut r, self_writer, &[]);
    let rep = j.verdict.unwrap_or_else(|f| panic!("{f}"));
    assert_eq!((rep.passed, rep.self_modifying), (200, 0));

    // The rig forgets the tree's code after the check: the survey judges every state.
    let rep = r.survey("self-writer", 200, SELF_AT, &SELF_WRITER);
    assert_eq!((rep.passed, rep.self_modifying), (200, 0));
}
