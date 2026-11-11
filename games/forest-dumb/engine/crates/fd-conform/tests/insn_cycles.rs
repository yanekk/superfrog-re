//! The fixed cycle times (translator T11, DESIGN §2.6) on Moira, beyond the game's census: every
//! encoding of T09's shift_bit program (`fixtures/shift_bit.rs`: shifts in every size and form, bit
//! ops by immediate and by register, scc and tas on a register and on memory, swap). A counted
//! encoding equals Moira on every state; an uncounted one is exactly a data-dependent kind, T12's
//! (`tools/pcode_fix/cycles.py`, `is_dynamic`). The bcd and compare forms are judged in every column,
//! cycles included, by `insn_defects.rs`.

use fd_conform::judge::insn::{Columns, InsnRig, Site};

// Byte for byte as pcode_rust.py writes it, which test_shift_bit.py compares: never rustfmt it.
#[rustfmt::skip]
#[allow(dead_code, unused_imports, unused_mut, unused_variables)]
#[path = "fixtures/shift_bit.rs"]
mod fixed;

include!("fixtures/shift_bit_texts.rs");

const PROGRAM: &[u8] = include_bytes!("fixtures/shift_bit.bin");
const BASE: u32 = 0x1000;
const STATES: usize = 60;

fn rig() -> InsnRig {
    let mut mem = vec![0u8; 0x8000];
    mem[BASE as usize..BASE as usize + PROGRAM.len()].copy_from_slice(PROGRAM);
    InsnRig::new(mem, &[BASE], &[PROGRAM.len() as u32])
}

/// Whether the time of the instruction `text` depends on data: a shift by a register count, a
/// bset/bclr/bchg numbered by a register on a register, scc on a register.
fn dynamic(text: &str) -> bool {
    let (m, ops) = text.split_once(' ').unwrap_or((text, ""));
    let m = m.split('.').next().unwrap();
    let shift = ["asl", "asr", "lsl", "lsr", "rol", "ror", "roxl", "roxr"].contains(&m);
    let bit = ["bset", "bclr", "bchg"].contains(&m);
    let scc = m.starts_with('s') && m != "swap" && m != "sub";
    (shift && ops.starts_with('d') && ops.contains(','))
        || (bit && ops.starts_with('d') && ops.ends_with(",d0"))
        || (scc && ops == "d0")
}

#[test]
fn every_counted_shift_bit_encoding_takes_moiras_time() {
    let mut rig = rig();
    let cols = Columns::parse("cycles").unwrap();
    let (mut counted, mut uncounted) = (0, 0);
    for (r, text) in fixed::SITES.iter().zip(TEXTS) {
        let s = Site::of(r);
        let rep = rig.check(&s, STATES, "cycles", cols);
        assert_eq!(rep.fail, 0, "{} {text}: {:?}", s.at, rep.first);
        assert!(rep.pass > 0, "{} {text}: {}", s.at, rep.first_skip);
        assert_eq!(rep.cycles_counted, !dynamic(text), "{} {text}", s.at);
        if rep.cycles_counted {
            counted += 1;
        } else {
            uncounted += 1;
        }
    }
    eprintln!("counted {counted}, uncounted {uncounted}");
    assert!(
        counted > 60 && uncounted > 30,
        "counted {counted}, uncounted {uncounted}"
    );
}
