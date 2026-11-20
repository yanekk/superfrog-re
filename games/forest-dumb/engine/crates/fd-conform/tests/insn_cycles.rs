//! The cycle times (translator T11 fixed, T12 data-dependent, DESIGN §2.6) on Moira, beyond the
//! game's census: every encoding of T09's shift_bit program (`fixtures/shift_bit.rs`: shifts in every
//! size and form, bit ops by immediate and by register, scc and tas on a register and on memory, swap).
//! Every encoding is counted and equals Moira on every state, the data-dependent ones (a shift by a
//! register, a bit op numbered by a register, scc on a register; `tools/pcode_fix/cycles.py`,
//! `is_dynamic`) by their run-time helpers. Multiply, divide, dbcc and movem are
//! `insn_cycles_dynamic.rs`'s. The bcd and compare forms are judged in every column,
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

#[test]
fn every_shift_bit_encoding_takes_moiras_time() {
    let mut rig = rig();
    let cols = Columns::parse("cycles").unwrap();
    for (r, text) in fixed::SITES.iter().zip(TEXTS) {
        let s = Site::of(r);
        let rep = rig.check(&s, STATES, "cycles", cols);
        assert_eq!(rep.fail, 0, "{} {text}: {:?}", s.at, rep.first);
        assert!(rep.pass > 0, "{} {text}: {}", s.at, rep.first_skip);
        assert!(rep.cycles_counted, "{} {text}: not counted", s.at);
    }
    assert!(fixed::SITES.len() > 90, "{} sites", fixed::SITES.len());
}
