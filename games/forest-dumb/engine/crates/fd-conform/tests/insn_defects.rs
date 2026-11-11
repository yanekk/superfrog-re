//! Ghidra 12.0's known defects at the instruction level (translator T07, DESIGN §2.2): the bcd and
//! compare forms of a made-up program (`tools/test_pcode_export.py`'s `DEFECTS_H0`) as Ghidra exports
//! them (`fixtures/defects.json`, written by `tools/test_pcode_export_ghidra.py --write-defects`),
//! translated with `pcode_fix`'s corrections (`fixtures/defects.rs`, `pcode_rust.py --insn`), each
//! against Moira's step on generated states. Covers what the game's census lacks: nbcd in every
//! alterable mode, cmpm, `-(a7)` and `(a7)+` bytes. No disks.
//! Regenerate `defects.rs` with `python3 -I tools/pcode_rust.py engine/crates/fd-conform/tests/fixtures/defects.json --insn --out engine/crates/fd-conform/tests/fixtures/defects.rs`
//! from `games/forest-dumb/` (`tools/test_insn_check.py` checks it is current).

use fd_conform::judge::insn::{Columns, InsnRig, Site};
use pcode_rt::{Bus, CF, Image, Regs, Stop, XF};

// Byte for byte as pcode_rust.py writes it, which test_insn_check.py compares: never rustfmt it.
#[rustfmt::skip]
#[allow(dead_code, unused_imports)]
#[path = "fixtures/defects.rs"]
mod fixture;

const BASES: [u32; 2] = [0x1000, 0x2000];
const SIZES: [u32; 2] = [0x58, 0x10];
const STATES: usize = 500;

/// `DEFECTS_H0`, as test_insn_check.py checks against the Python original.
const H0: [&str; 3] = [
    "c101c109cf0f810181098f0f4800481048184820481f48274828001048301002",
    "48380400483900000004b001b050b081b0c1b1c90c0300010c5012340c801234",
    "5678b109b149b189bf0f600000064e754e7174024e754e71",
];
/// The long at `h0+$26` (nbcd's absolute operand) is relocated to hunk 1.
const RELOC: (u32, u32) = (0x26, 1);

fn rig() -> InsnRig {
    let hex = H0.concat();
    let code: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    assert_eq!(code.len() as u32, SIZES[0]);
    let mut mem = vec![0u8; 0x3000];
    let b = BASES[0] as usize;
    mem[b..b + code.len()].copy_from_slice(&code);
    let at = b + RELOC.0 as usize;
    let v = u32::from_be_bytes(mem[at..at + 4].try_into().unwrap()) + BASES[RELOC.1 as usize];
    mem[at..at + 4].copy_from_slice(&v.to_be_bytes());
    InsnRig::new(mem, &BASES, &SIZES)
}

fn mnemonic(s: &Site) -> u16 {
    u16::from_str_radix(s.encoding, 16).unwrap()
}

fn is_bcd_or_compare(w: u16) -> bool {
    let abcd_sbcd = matches!(w & 0xf1f0, 0xc100 | 0x8100 | 0xc108 | 0x8108);
    let nbcd = (w & 0xffc0) == 0x4800;
    let cmp = (w & 0xf000) == 0xb000 || (w & 0xff00) == 0x0c00; // cmp, cmpa, cmpm, cmpi
    abcd_sbcd || nbcd || cmp
}

#[test]
fn every_bcd_and_compare_form_passes_regs_ccr_and_memory_on_moira() {
    let cols = Columns::parse("regs,ccr,mem").unwrap();
    let mut rig = rig();
    let mut judged = 0;
    for row in fixture::SITES {
        let s = Site::of(row);
        if !is_bcd_or_compare(mnemonic(&s)) {
            continue;
        }
        let r = rig.check(&s, STATES, "test", cols);
        assert_eq!(r.fail, 0, "{} {}: {:?}", s.at, s.encoding, r.first);
        // Byte operands and even address registers: Moira takes no exception on any state.
        assert_eq!(r.skipped, 0, "{}: {}", s.at, r.first_skip);
        judged += 1;
    }
    assert_eq!(judged, 28);
}

#[test]
fn the_bra_w_ghidra_marks_call_return_is_a_branch_to_its_target() {
    let s = Site::of(fixture::SITES.iter().find(|r| r.1 == "h0+$4a").unwrap());
    let r = rig().check(&s, STATES, "test", Columns::ALL);
    assert_eq!((r.fail, r.skipped), (0, 0), "{:?}", r.first);
}

// A compare as the uncorrected sinc translates it: X takes C.
fn with_x_from_c(
    run: fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>,
) -> impl Fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop> {
    move |r, bus, img| {
        run(r, bus, img)?;
        r.set(XF, 1, r.get(CF, 1));
        Ok(())
    }
}

macro_rules! uncorrected {
    ($name:ident, $f:path) => {
        fn $name(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
            with_x_from_c($f)(r, bus, img)
        }
    };
}
uncorrected!(cmp_b, fixture::i_h0_2a);
uncorrected!(cmpa_l, fixture::i_h0_32);
uncorrected!(cmpi_b, fixture::i_h0_34);
uncorrected!(cmpm_b, fixture::i_h0_42);

#[test]
fn compares_leave_x_alone_on_states_with_x_set_and_clear() {
    // The corrected compares pass every state (above). Copying C into X, as Ghidra's sinc does,
    // fails exactly the states where X and C differ: some but not all, so the states hold X both
    // ways against C both ways, and the check sees X.
    let mut rig = rig();
    for (at, run) in [
        ("h0+$2a", cmp_b as fd_conform::judge::insn::Entry),
        ("h0+$32", cmpa_l),
        ("h0+$34", cmpi_b),
        ("h0+$42", cmpm_b),
    ] {
        let s = Site {
            run,
            ..Site::of(fixture::SITES.iter().find(|r| r.1 == at).unwrap())
        };
        let r = rig.check(&s, STATES, "test", Columns::ALL);
        assert!(
            r.fail > STATES / 10 && r.fail < STATES * 9 / 10,
            "{at}: {r:?}"
        );
        assert_eq!((r.ccr, r.regs, r.mem), (r.fail, 0, 0), "{at}");
        assert_eq!(r.first.unwrap().what, "X", "{at}");
    }
}
