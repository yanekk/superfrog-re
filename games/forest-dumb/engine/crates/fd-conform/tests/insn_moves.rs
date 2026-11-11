//! The move and control family at the instruction level (translator T10, DESIGN §2.2): the forms of a
//! made-up program (`tools/test_pcode_export.py`'s `MOVES_H0`) as Ghidra exports them
//! (`fixtures/moves.json`, written by `tools/test_pcode_export_ghidra.py --write-moves`), translated
//! with `pcode_fix`'s corrections (`fixtures/moves.rs`, `pcode_rust.py --insn`), each against Moira's
//! step on generated states. Covers what the game's census lacks: SR and CCR both ways, `rtr`,
//! `move usp`, `movem` with its own register in the list, bytes through a7, `link`, `unlk`, `pea`,
//! `exg`, `dbcc`. No disks.
//! Regenerate `moves.rs` with `python3 -I tools/pcode_rust.py engine/crates/fd-conform/tests/fixtures/moves.json --insn --out engine/crates/fd-conform/tests/fixtures/moves.rs`
//! from `games/forest-dumb/` (`tools/test_insn_check.py` checks it is current).

use fd_conform::judge::insn::{Columns, InsnRig, Site};

// Byte for byte as pcode_rust.py writes it, which test_insn_check.py compares: never rustfmt it.
#[rustfmt::skip]
#[allow(dead_code, unused_imports)]
#[path = "fixtures/moves.rs"]
mod fixture;

const BASES: [u32; 1] = [0x1000];
const SIZES: [u32; 1] = [0x88];
const STATES: usize = 500;

/// `MOVES_H0`, as test_insn_check.py checks against the Python original.
const H0: [&str; 5] = [
    "4e734e7746c046d046fc201540c040e744c044fc0015027cf8ff007c07000a7c",
    "0711023c00f0003c00050a3c001f4e604e6948e7fffe4cdf7fff48a180804c98",
    "000348e0c0804cd801011f00101f1f1f1ed84e56fff84e5e485048781234487a",
    "0010c188c34f57c8fffe51c900044e714e904eba00044ed06100000461024e75",
    "66024e714e754e71",
];

fn rig() -> InsnRig {
    let hex = H0.concat();
    let code: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    assert_eq!(code.len() as u32, SIZES[0]);
    let mut mem = vec![0u8; 0x2000];
    let b = BASES[0] as usize;
    mem[b..b + code.len()].copy_from_slice(&code);
    InsnRig::new(mem, &BASES, &SIZES)
}

fn check(rig: &mut InsnRig, at: &str) -> fd_conform::judge::insn::InsnReport {
    let s = Site::of(fixture::SITES.iter().find(|r| r.1 == at).unwrap());
    rig.check(&s, STATES, "test", Columns::parse("regs,ccr,mem").unwrap())
}

/// The returns pop a random address: Moira takes an address error on the odd half, never judged.
const RETURNS: [&str; 3] = ["h0+$0", "h0+$2", "h0+$7e"];

#[test]
fn every_move_and_control_form_passes_regs_ccr_and_memory_on_moira() {
    let mut rig = rig();
    let mut bad = Vec::new();
    for row in fixture::SITES {
        let r = check(&mut rig, row.1);
        if r.fail > 0 {
            bad.push(format!("{} {}: {} fail, {:?}", row.1, row.0, r.fail, r.first));
        }
        if RETURNS.contains(&row.1) {
            assert!(r.pass > STATES / 3, "{}: {} judged", row.1, r.pass);
        } else {
            assert_eq!(r.skipped, 0, "{}: {}", row.1, r.first_skip);
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
    assert_eq!(fixture::SITES.len(), 44);
}

#[test]
fn movem_to_predecrement_stores_in_reverse_and_leaves_an_decremented() {
    // movem.l d0-a6,-(a7); movem.w d0/a0,-(a1) (Ghidra steps 4 a word); movem.l d0-d1/a0,-(a0),
    // which stores a0 as it was before the instruction. regs holds An, mem every byte's place.
    let mut rig = rig();
    for at in ["h0+$32", "h0+$3a", "h0+$42"] {
        let r = check(&mut rig, at);
        assert_eq!((r.fail, r.skipped), (0, 0), "{at}: {:?}", r.first);
    }
}

#[test]
fn byte_moves_through_a7_step_it_by_two() {
    // move.b d0,-(a7); (a7)+,d0; (a7)+,-(a7); (a0)+,(a7)+
    let mut rig = rig();
    for at in ["h0+$4a", "h0+$4c", "h0+$4e", "h0+$50"] {
        let r = check(&mut rig, at);
        assert_eq!((r.fail, r.skipped), (0, 0), "{at}: {:?}", r.first);
    }
}

#[test]
fn writes_to_sr_and_rte_restore_the_mask_and_writes_to_ccr_keep_it() {
    // The regs column compares T, S and the mask, and the stack pointer not in a7; the states'
    // values and popped words set S and the mask both ways. move <ea>,sr; andi/ori/eori #,sr; rte;
    // then move <ea>,ccr; andi/ori/eori #,ccr; rtr, which must leave the system byte alone.
    let mut rig = rig();
    for at in [
        "h0+$4", "h0+$6", "h0+$8", "h0+$16", "h0+$1a", "h0+$1e", "h0+$0", "h0+$10", "h0+$12",
        "h0+$22", "h0+$26", "h0+$2a", "h0+$2", "h0+$c", "h0+$e", "h0+$2e", "h0+$30",
    ] {
        let r = check(&mut rig, at);
        assert_eq!(r.fail, 0, "{at}: {:?}", r.first);
        assert!(r.pass > STATES / 3, "{at}");
    }
}
