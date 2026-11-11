//! The instruction check (translator T06) on made-up encodings: `tests/fixtures/insn.rs` is
//! `pcode_rust.py --insn` run on `fixtures/insn.json` (P-code in Ghidra's shape, hand-written), so a
//! right translation passes every column on Moira and a deliberately wrong one fails, naming the
//! column. No disks: the instructions' bytes are laid into memory here.
//! Regenerate the fixture with `python3 -I tools/pcode_rust.py engine/crates/fd-conform/tests/fixtures/insn.json --insn --out engine/crates/fd-conform/tests/fixtures/insn.rs`
//! from `games/forest-dumb/` (`tools/test_insn_check.py` checks it is current).

use fd_conform::judge::insn::{Columns, Entry, InsnReport, InsnRig, Site};
use pcode_rt::{Bus, CF, Image, Regs, Stop};

#[allow(dead_code, unused_imports)]
#[path = "fixtures/insn.rs"]
mod fixture;

const BASES: [u32; 2] = [0x1000, 0x2000];
const SIZES: [u32; 2] = [0x20, 0x10];
const STATES: usize = 500;

fn rig() -> InsnRig {
    let mut mem = vec![0u8; 0x3000];
    let mut put = |a: u32, b: &[u8]| mem[a as usize..a as usize + b.len()].copy_from_slice(b);
    put(0x1000, &[0x70, 0xff]); // moveq #-1,d0
    put(0x1002, &[0xd0, 0x81]); // add.l d1,d0
    put(0x1004, &[0xc1, 0x01]); // abcd d1,d0
    put(0x1006, &[0x0c, 0x03, 0x00, 0x01]); // cmpi.b #1,d3
    put(0x100a, &[0x66, 0xf4]); // bne.b $1000
    put(0x100c, &[0x30, 0xc0]); // move.w d0,(a0)+
    // move.l d0,(h1+$4).l, its address relocated to hunk 1's base as a loader would.
    put(0x100e, &[0x23, 0xc0, 0x00, 0x00, 0x20, 0x04]);
    InsnRig::new(mem, &BASES, &SIZES)
}

fn site(at: &str) -> Site<'static> {
    let row = fixture::SITES.iter().find(|r| r.1 == at).expect(at);
    Site::of(row)
}

fn with(at: &str, run: Entry) -> Site<'static> {
    Site { run, ..site(at) }
}

fn check(s: &Site, cols: Columns) -> InsnReport {
    rig().check(s, STATES, "test", cols)
}

#[test]
fn every_made_up_encoding_passes_every_column_on_moira() {
    let mut rig = rig();
    for row in fixture::SITES {
        let s = Site::of(row);
        let r = rig.check(&s, STATES, "test", Columns::ALL);
        assert_eq!(r.fail, 0, "{} {}: {:?}", s.at, s.encoding, r.first);
        assert_eq!(r.skipped, 0, "{}: {}", s.at, r.first_skip);
        assert_eq!(r.pass, STATES, "{}", s.at);
        // The generated code counts no cycles yet (T11): reported, never failed.
        assert!(!r.cycles_counted);
        assert!(
            r.json(&s, Columns::ALL)
                .contains("\"cycles\":\"not-counted\"")
        );
    }
}

fn add_with_c_flipped(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fixture::i_h0_2(r, bus, img)?;
    r.set(CF, 1, r.get(CF, 1) ^ 1);
    Ok(())
}

#[test]
fn a_wrong_carry_fails_every_state_in_the_ccr_column_naming_c() {
    let s = with("h0+$2", add_with_c_flipped);
    let r = check(&s, Columns::ALL);
    assert_eq!(
        (r.fail, r.ccr, r.regs, r.mem, r.stop),
        (STATES, STATES, 0, 0, 0)
    );
    let first = r.first.as_ref().unwrap();
    assert_eq!((first.column, first.what.as_str()), ("ccr", "C"));
    assert_ne!(first.moira, first.translated);
    let line = r.json(&s, Columns::ALL);
    assert!(line.contains("\"column\":\"ccr\",\"what\":\"C\""), "{line}");
    // Without the ccr column the same translation passes.
    let cols = Columns::parse("regs,mem").unwrap();
    assert_eq!(check(&s, cols).fail, 0);
}

fn moveq_into_d1(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let d1 = r.d(1);
    fixture::i_h0_0(r, bus, img)?;
    r.set_d(1, d1 ^ 1);
    Ok(())
}

fn move_one_byte_too_many(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let a0 = r.a(0);
    fixture::i_h0_c(r, bus, img)?;
    let b = bus.rb(a0 + 2);
    bus.wb(a0 + 2, !b);
    Ok(())
}

fn branch_never_taken(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    fixture::i_h0_a(r, bus, img)?;
    r.set(pcode_rt::PC, 4, img.h(0, 0xc) as u64);
    Ok(())
}

fn unsupported(_: &mut Regs, _: &mut dyn Bus, _: &Image) -> Result<(), Stop> {
    Err(Stop::Unsupported {
        op: "CALLOTHER".into(),
        at: "h0+$0".into(),
    })
}

#[test]
fn a_wrong_register_byte_or_branch_fails_in_its_own_column() {
    let r = check(&with("h0+$0", moveq_into_d1), Columns::ALL);
    assert_eq!((r.fail, r.regs, r.ccr), (STATES, STATES, 0));
    assert_eq!(r.first.unwrap().what, "d1");

    let r = check(&with("h0+$c", move_one_byte_too_many), Columns::ALL);
    assert_eq!((r.fail, r.mem, r.regs), (STATES, STATES, 0));
    assert_eq!(r.first.unwrap().column, "mem");

    // Only the states where Z is clear take the branch: about half fail, naming the PC.
    let r = check(&with("h0+$a", branch_never_taken), Columns::ALL);
    assert!(r.fail > STATES / 4 && r.fail < STATES * 3 / 4, "{r:?}");
    let first = r.first.unwrap();
    assert_eq!((first.what.as_str(), first.moira.as_str()), ("pc", "h0+$0"));

    let r = check(&with("h0+$0", unsupported), Columns::ALL);
    assert_eq!((r.fail, r.stop), (STATES, STATES));
    assert_eq!(r.first.unwrap().what, "unsupported: CALLOTHER at h0+$0");
}

#[test]
fn the_absolute_operand_is_seeded_and_written_where_the_original_writes() {
    let s = site("h0+$e");
    assert_eq!(s.seeds, &[(1, 4)]);
    let r = check(&s, Columns::ALL);
    assert_eq!(r.fail, 0, "{:?}", r.first);
}

#[test]
fn columns_parse_and_refuse_what_is_not_a_column() {
    assert_eq!(Columns::parse("regs,ccr,mem,cycles"), Ok(Columns::ALL));
    assert!(
        Columns::parse("regs,flags")
            .unwrap_err()
            .contains("`flags`")
    );
    assert!(Columns::parse("").is_err());
}
