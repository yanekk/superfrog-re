//! The judge's translated mode on `pcode-machine` (translator T27, DESIGN §2.8, §4), on the main
//! program from the owner's disks: a hand translation of `h0+$5462` passes on the machine as on the
//! judge's `Mem` and one changed constant fails; the hardware reads a case gives reach the routine
//! through the machine's devices as the oracle answers them; a blit lands in memory and its busy
//! bit is the machine's; the entry sees the original's stack; `fd-judge --machine` prints its line.

use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use amiga_oracle::RETURN_SENTINEL;
use fd_conform::HUNK_BASES;
use fd_conform::generators::{self, h};
use fd_conform::judge::translated::{self, Entry, Hardware, tree};
use fd_conform::routine::{Gen, Generator, Rig, STACK, Setup};
use pcode_rt::{Bus, Image, Regs, Stop, load, store};

const ROUTINE: &str = "h0+$5462";
const GEN: &str = "char_sprite_position";
/// The routine's instructions as a tree would list them in `CODE`.
const CODE: &[(u32, u32, u32)] = &[(0, 0x5462, 0x54c2)];

/// The top-level `rts`: pop the judge's return address.
fn pop_return(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let sp = r.a(7);
    let ret = load(bus, sp, 4) as u32;
    r.set_a(7, sp.wrapping_add(4));
    if ret != RETURN_SENTINEL {
        return Err(Stop::Untranslated(img.describe(ret)));
    }
    Ok(())
}

/// `h0+$5462` (`char_sprite_position`) by hand, as `judge_translated.rs` writes it; `k` is the x
/// offset, $3c in the original.
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
    pop_return(r, bus, img)
}

fn right(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    sprite_position(r, bus, img, 0x3c)
}

fn off_by_one(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    sprite_position(r, bus, img, 0x3d)
}

fn rig() -> Rig {
    Rig::new().unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn a_hand_translation_passes_on_the_machine_as_on_mem_and_a_wrong_one_fails() {
    let mut r = rig();
    let generator = generators::generator(GEN).unwrap();
    for hw in [Hardware::Mem, Hardware::Machine] {
        let j = translated::check_on(
            &mut r,
            GEN,
            2000,
            h(0, 0x5462),
            &generator,
            Some(right),
            CODE,
            hw,
        );
        let rep = j.verdict.unwrap_or_else(|f| panic!("{hw:?}: {f}"));
        assert_eq!((rep.passed, rep.hangs, rep.faults), (2000, 0, 0), "{hw:?}");
        assert_eq!(j.stops, translated::Stops::default(), "{hw:?}");
    }
    let j = translated::check_on(
        &mut r,
        GEN,
        50,
        h(0, 0x5462),
        &generator,
        Some(off_by_one),
        CODE,
        Hardware::Machine,
    );
    let f = j.verdict.unwrap_err();
    assert!(f.failures > 40, "{f}");
    assert!(f.first.contains("d0: original $"), "{f}");

    // No entry: every state stops untranslated, on the machine too.
    let j = translated::check_on(
        &mut r,
        GEN,
        20,
        h(0, 0x5462),
        &generator,
        None,
        CODE,
        Hardware::Machine,
    );
    assert_eq!(j.verdict.unwrap_err().failures, 20);
    assert_eq!(j.stops.first, "untranslated: h0+$5462");
}

/// Where the made-up routines below are laid: over `h0+$5462`, in the program's own memory.
const AT: u32 = HUNK_BASES[0] + 0x5462;

fn lay(m: &mut [u8], code: &[u16]) {
    for (i, w) in code.iter().enumerate() {
        let a = AT as usize + 2 * i;
        m[a..a + 2].copy_from_slice(&w.to_be_bytes());
    }
}

/// `move.w $dff002,d0; move.w $dff00c,d1; move.w $dff00e,d2; move.b $bfe001,d3;
/// move.b $dff006,d4; rts`: DMACONR, JOY1DAT, CLXDAT, CIA-A's PRA and VHPOSR's line byte.
const READS: [u16; 16] = [
    0x3039, 0x00df, 0xf002, 0x3239, 0x00df, 0xf00c, 0x3439, 0x00df, 0xf00e, 0x1639, 0x00bf, 0xe001,
    0x1839, 0x00df, 0xf006, 0x4e75,
];

fn lay_reads(m: &mut [u8]) {
    lay(m, &READS);
}

/// Random registers; JOY1DAT's and CLXDAT's bytes given or not (a byte not given reads 0),
/// DMACONR's high or low byte or both, PRA always. Where a case gives none of a register's bytes
/// the machine's own model answers: DMACONR reads the judge's DMACON, CIA-A's PRA its port lines,
/// where the oracle reads 0.
fn reads_case(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    let dmaconr = match g.below(3) {
        0 => vec![0xdf_f002],
        1 => vec![0xdf_f003],
        _ => vec![0xdf_f002, 0xdf_f003],
    };
    for byte in dmaconr {
        s.hw_reads.push((byte, g.next() as u8));
    }
    for reg in [0xdf_f00cu32, 0xdf_f00e] {
        for byte in [reg, reg + 1] {
            if !g.one_in(3) {
                s.hw_reads.push((byte, g.next() as u8));
            }
        }
    }
    s.hw_reads.push((0xbf_e001, g.next() as u8));
    s
}

const READER: Generator = Generator {
    base: Some(lay_reads),
    case: reads_case,
};

/// [`READS`] translated: each read through the bus, the low word or byte of its register.
fn reads(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let set = |r: &mut Regs, n: usize, v: u64, mask: u32| {
        r.set_d(n, r.d(n) & !mask | v as u32 & mask);
    };
    for (n, a) in [(0, 0xdf_f002), (1, 0xdf_f00c), (2, 0xdf_f00e)] {
        let v = load(bus, a, 2);
        set(r, n, v, 0xffff);
    }
    let v = load(bus, 0xbf_e001, 1);
    set(r, 3, v, 0xff);
    let v = load(bus, 0xdf_f006, 1);
    set(r, 4, v, 0xff);
    pop_return(r, bus, img)
}

#[test]
fn the_hardware_reads_a_case_gives_reach_the_routine_through_the_machines_devices() {
    let mut r = rig();
    for hw in [Hardware::Mem, Hardware::Machine] {
        let j = translated::check_on(&mut r, "reads", 500, AT, &READER, Some(reads), &[], hw);
        let rep = j.verdict.unwrap_or_else(|f| panic!("{hw:?}: {f}"));
        assert_eq!(rep.passed, 500, "{hw:?}");
    }
}

/// Chip RAM the blit reads and writes: above the last chip hunk, below the top of chip RAM.
const SRC: u32 = 0x7_f000;
const DST: u32 = 0x7_f800;

/// `move.w #$09f0,$dff040; move.w #0,$dff042; move.l #-1,$dff044; move.l a0,$dff050;
/// move.l a1,$dff054; move.l #0,$dff064; move.w #$0042,$dff058; .w btst #6,$dff002; bne.s .w; rts`:
/// two words copied from (a0) to (a1) by the blitter, then a wait on its busy bit.
const BLIT: [u16; 33] = [
    0x33fc, 0x09f0, 0x00df, 0xf040, 0x33fc, 0x0000, 0x00df, 0xf042, 0x23fc, 0xffff, 0xffff, 0x00df,
    0xf044, 0x23c8, 0x00df, 0xf050, 0x23c9, 0x00df, 0xf054, 0x23fc, 0x0000, 0x0000, 0x00df, 0xf064,
    0x33fc, 0x0042, 0x00df, 0xf058, 0x0839, 0x0006, 0x00df, 0xf002, 0x66f6,
];

fn lay_blit(m: &mut [u8]) {
    let mut code = BLIT.to_vec();
    code.push(0x4e75);
    lay(m, &code);
}

fn blit_case(g: &mut Gen) -> Setup {
    let mut s = Setup::random(g);
    s.a[0] = SRC;
    s.a[1] = DST;
    s.l(SRC, g.next() as u32);
    s
}

const BLITTER: Generator = Generator {
    base: Some(lay_blit),
    case: blit_case,
};

/// How many times [`blit`]'s wait found the blitter busy, over every state run.
static BUSY: AtomicUsize = AtomicUsize::new(0);

/// [`BLIT`] translated: the stores in order, then the wait as generated code runs it, a block a
/// pass (`poll`, its cycles, the read).
fn blit(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    store(bus, img, 0xdf_f040, 2, 0x09f0)?;
    store(bus, img, 0xdf_f042, 2, 0)?;
    store(bus, img, 0xdf_f044, 4, 0xffff_ffff)?;
    store(bus, img, 0xdf_f050, 4, r.a(0) as u64)?;
    store(bus, img, 0xdf_f054, 4, r.a(1) as u64)?;
    store(bus, img, 0xdf_f064, 4, 0)?;
    store(bus, img, 0xdf_f058, 2, 0x0042)?;
    loop {
        // A block's accesses are timed at its start: the read sees the blitter as of the poll.
        bus.poll(r, img)?;
        if load(bus, 0xdf_f002, 1) & 0x40 == 0 {
            break;
        }
        BUSY.fetch_add(1, Ordering::Relaxed);
        r.tick(30);
    }
    pop_return(r, bus, img)
}

#[test]
fn a_blit_lands_in_memory_and_its_busy_bit_is_the_machines() {
    let mut r = rig();
    for (b, n) in HUNK_BASES.iter().zip(r.hunk_sizes()) {
        assert!(
            b + n <= SRC || *b >= 0x8_0000,
            "a hunk under the blit's memory"
        );
    }
    let j = translated::check_on(
        &mut r,
        "blit",
        200,
        AT,
        &BLITTER,
        Some(blit),
        &[],
        Hardware::Mem,
    );
    j.verdict.unwrap_or_else(|f| panic!("Mem: {f}"));
    assert_eq!(BUSY.swap(0, Ordering::Relaxed), 0, "Mem's DMACONR reads 0");
    let j = translated::check_on(
        &mut r,
        "blit",
        200,
        AT,
        &BLITTER,
        Some(blit),
        &[],
        Hardware::Machine,
    );
    let rep = j.verdict.unwrap_or_else(|f| panic!("Machine: {f}"));
    assert_eq!(rep.passed, 200);
    // Two words take the blitter some cycles: every state waited at least once.
    assert!(BUSY.load(Ordering::Relaxed) >= 200, "{BUSY:?}");
}

/// `movea.l a7,a0; movea.l (a7),a1; rts`: the stack pointer and the word on top of it at entry.
fn lay_stack(m: &mut [u8]) {
    lay(m, &[0x204f, 0x2257, 0x4e75]);
}

fn stack_at_entry(r: &mut Regs, bus: &mut dyn Bus, _: &Image) -> Result<(), Stop> {
    let sp = r.a(7);
    r.set_a(0, sp);
    r.set_a(1, load(bus, sp, 4) as u32);
    r.set_a(7, sp.wrapping_add(4));
    Ok(())
}

#[test]
fn the_entry_sees_the_stack_the_original_sees_on_the_machine() {
    let mut r = rig();
    let g = Generator {
        base: Some(lay_stack),
        case: Setup::random,
    };
    let j = translated::check_on(
        &mut r,
        "stack",
        20,
        AT,
        &g,
        Some(stack_at_entry as Entry),
        &[],
        Hardware::Machine,
    );
    let rep = j.verdict.unwrap_or_else(|f| panic!("{f}"));
    assert_eq!(rep.passed, 20);
    assert_eq!(r.last_regs().unwrap().a[0], STACK - 4);
}

#[test]
fn fd_judge_machine_prints_its_line_and_needs_translated() {
    assert!(tree::entry(ROUTINE).is_none(), "the test build has no tree");
    let out = Command::new(env!("CARGO_BIN_EXE_fd-judge"))
        .args([
            ROUTINE,
            "--gen",
            GEN,
            "--translated",
            "--machine",
            "--cases",
            "20",
        ])
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
        "\"failures\":20",
        "\"untranslated\":20,",
        "\"outside\":0,\"traps\":0,\"hardware\":\"machine\",\"first_stop\":\"untranslated: h0+$5462\"}",
    ] {
        assert!(line.contains(want), "{want} not in {line}");
    }
    let alone = Command::new(env!("CARGO_BIN_EXE_fd-judge"))
        .args([ROUTINE, "--gen", GEN, "--machine"])
        .output()
        .unwrap();
    assert_eq!(alone.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&alone.stderr).contains("--machine judges a translated tree"));
}
