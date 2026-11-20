//! The arith family's corrections (translator T08, `tools/pcode_fix/arith.py`) on made-up encodings,
//! against Moira's step: `fixtures/arith.rs` is translated with every correction, `fixtures/arith_raw.rs`
//! without arith's, from `fixtures/arith.json` (P-code in Ghidra's shape). Each correction passes with
//! it and fails without it, in its own column. No disks.
//! Regenerate the fixtures with `python3 -I tools/test_pcode_fix_arith.py --write` from
//! `games/forest-dumb/` (which also checks they are current).

use std::sync::Arc;

use fd_conform::judge::Mem;
use fd_conform::judge::insn::{Columns, InsnReport, InsnRig, Site};
use pcode_rt::{Bus, CF, Image, Regs, Stop, XF, ZF};

// Byte for byte as pcode_rust.py writes them, which the Python half compares: never rustfmt them.
#[rustfmt::skip]
#[allow(dead_code, unused_imports, unused_variables, unused_mut, unreachable_code)]
#[path = "fixtures/arith.rs"]
mod fixed;
#[rustfmt::skip]
#[allow(dead_code, unused_imports, unused_variables, unused_mut, unreachable_code)]
#[path = "fixtures/arith_raw.rs"]
mod raw;

const BASES: [u32; 2] = [0x1000, 0x2000];
const SIZES: [u32; 2] = [0x30, 0x10];
const STATES: usize = 500;
/// The words after the two absolute clrs: what their read of a write-only register writes.
const AFTER_CLR_W: u16 = 0x4e71;
const AFTER_CLR_L: u16 = 0x7001;

fn program() -> Vec<u8> {
    let mut mem = vec![0u8; 0x3000];
    let mut put = |a: u32, b: &[u8]| mem[a as usize..a as usize + b.len()].copy_from_slice(b);
    put(0x1000, &[0xd1, 0x81]); // addx.l d1,d0
    put(0x1002, &[0x91, 0x81]); // subx.l d1,d0
    put(0x1004, &[0x40, 0x80]); // negx.l d0
    put(0x1006, &[0x80, 0xc1]); // divu.w d1,d0
    put(0x1008, &[0x81, 0xfc, 0x00, 0x03]); // divs.w #3,d0
    put(0x100c, &[0x80, 0xfc, 0x00, 0x00]); // divu.w #0,d0
    put(0x1010, &[0x42, 0x79, 0x00, 0xdf, 0xf0, 0xa8]); // clr.w $dff0a8 (AUD0VOL)
    put(0x1016, &AFTER_CLR_W.to_be_bytes());
    put(0x1018, &[0x42, 0xb9, 0x00, 0xdf, 0xf0, 0xa8]); // clr.l $dff0a8 (AUD0VOL, AUD0DAT)
    put(0x101e, &AFTER_CLR_L.to_be_bytes());
    put(0x1020, &[0x42, 0x50]); // clr.w (a0)
    mem
}

fn rig() -> InsnRig {
    InsnRig::new(program(), &BASES, &SIZES)
}

type Row = (
    &'static str,
    &'static str,
    u32,
    u32,
    u32,
    &'static [(u32, u32)],
    fixed::Routine,
);

fn site<'a>(rows: &'a [Row], at: &str) -> Site<'a> {
    Site::of(rows.iter().find(|r| r.1 == at).expect(at))
}

fn check(s: &Site) -> InsnReport {
    rig().check(s, STATES, "arith", Columns::parse("regs,ccr,mem").unwrap())
}

/// Every state passes, apart from the few Moira skips (a random zero divisor's trap).
fn passes(at: &str) {
    let r = check(&site(fixed::SITES, at));
    assert_eq!(r.fail, 0, "{at}: {:?}", r.first);
    assert!(r.skipped < STATES / 20, "{at}: {}", r.first_skip);
    assert_eq!(r.pass + r.skipped, STATES, "{at}");
}

/// Without arith's correction the site fails, the first failure in `column`, naming `what`.
fn fails_raw(at: &str, column: &str, what: &str) -> InsnReport {
    let r = check(&site(raw::SITES, at));
    assert!(r.fail > 0, "{at} passes without its correction");
    let first = r.first.as_ref().unwrap();
    assert_eq!(
        (first.column, first.what.as_str()),
        (column, what),
        "{at}: {first:?}"
    );
    r
}

#[test]
fn addx_sets_x_as_c_and_fails_without_it() {
    passes("h0+$0");
    let r = fails_raw("h0+$0", "ccr", "X");
    assert_eq!((r.regs, r.mem), (0, 0));
}

#[test]
fn divu_and_divs_match_moira_overflow_included_and_fail_without_it() {
    passes("h0+$6");
    passes("h0+$8");
    // Random 32-bit dividends overflow a word quotient most of the time, as the census check found.
    for at in ["h0+$6", "h0+$8"] {
        let r = check(&site(raw::SITES, at));
        assert!(r.regs > STATES / 4 && r.ccr > STATES / 4, "{at}: {r:?}");
        assert_eq!(r.mem, 0, "{at}");
    }
}

/// Run one translated instruction on `r`, over the fixture program at its bases.
fn run(f: fixed::Routine, r: &mut Regs) -> (Result<(), Stop>, Mem) {
    let mut mem = Mem::new(Arc::new(program_padded()), &[]);
    let img = Image {
        bases: &BASES,
        sizes: &SIZES,
        code: fixed::CODE,
    };
    let res = f(r, &mut mem as &mut dyn Bus, &img);
    (res, mem)
}

fn program_padded() -> Vec<u8> {
    let mut m = program();
    m.resize(1 << 24, 0);
    m
}

#[test]
fn divu_overflow_sets_v_and_leaves_the_destination() {
    let mut r = Regs::new();
    r.set_d(0, 0x0010_0000);
    r.set_d(1, 0x0000_0002);
    let (res, _) = run(fixed::i_h0_6, &mut r);
    assert_eq!(res, Ok(()));
    assert_eq!(r.d(0), 0x0010_0000);
    let flags = [pcode_rt::NF, ZF, pcode_rt::VF, CF].map(|o| r.get(o, 1));
    assert_eq!(flags, [1, 0, 1, 0]);
}

#[test]
fn division_by_zero_is_the_trap_stop_not_a_panic() {
    // Moira takes vector 5 on every state: the rig skips them all, never judges them.
    let r = check(&site(fixed::SITES, "h0+$c"));
    assert_eq!((r.skipped, r.fail), (STATES, 0), "{}", r.first_skip);

    let mut regs = Regs::new();
    regs.set_d(0, 1234);
    let (res, _) = run(fixed::i_h0_c, &mut regs);
    let want = Stop::Trap {
        vector: 5,
        at: "h0+$c".into(),
    };
    assert_eq!(res, Err(want.clone()));
    assert_eq!(want.to_string(), "trap: vector 5 at h0+$c");
    assert_eq!(regs.d(0), 1234, "the destination is left as it was");
    // A zero divisor in a register traps too.
    let mut regs = Regs::new();
    assert!(matches!(
        run(fixed::i_h0_6, &mut regs).0,
        Err(Stop::Trap { vector: 5, .. })
    ));
    // Without the correction P-code's division by zero gives 0 and runs on.
    let mut regs = Regs::new();
    regs.set_d(0, 1234);
    assert_eq!(
        raw::i_h0_c(
            &mut regs,
            &mut Mem::new(Arc::new(program_padded()), &[]),
            &Image {
                bases: &BASES,
                sizes: &SIZES,
                code: raw::CODE,
            }
        ),
        Ok(())
    );
}

#[test]
fn clr_of_a_write_only_register_writes_the_next_word_first() {
    passes("h0+$10");
    passes("h0+$18");
    passes("h0+$20");
    fails_raw("h0+$10", "mem", "custom writes");
    fails_raw("h0+$18", "mem", "custom writes");

    let mut r = Regs::new();
    let (_, mem) = run(fixed::i_h0_10, &mut r);
    assert_eq!(mem.hw_writes, vec![(0xa8, AFTER_CLR_W), (0xa8, 0)]);
    let (_, mem) = run(fixed::i_h0_18, &mut r);
    assert_eq!(
        mem.hw_writes,
        vec![
            (0xa8, AFTER_CLR_L),
            (0xaa, AFTER_CLR_L),
            (0xaa, 0),
            (0xa8, 0)
        ]
    );
}

/// addx, subx and negx: Z is cleared by a nonzero result and otherwise left as it was.
#[test]
fn extended_ops_only_ever_clear_z() {
    type F = fixed::Routine;
    // (instruction, d0, d1, X) giving a zero result, and one giving a nonzero result.
    let cases: [(&str, F, (u32, u32, u64), (u32, u32, u64)); 3] = [
        ("addx", fixed::i_h0_0, (0xffff_ffff, 0, 1), (1, 2, 0)),
        ("subx", fixed::i_h0_2, (5, 4, 1), (5, 1, 0)),
        ("negx", fixed::i_h0_4, (0, 0, 0), (3, 0, 0)),
    ];
    for (name, f, zero, nonzero) in cases {
        for z_before in [0, 1] {
            let mut r = Regs::new();
            r.set_d(0, zero.0);
            r.set_d(1, zero.1);
            r.set(XF, 1, zero.2);
            r.set(ZF, 1, z_before);
            run(f, &mut r).0.unwrap();
            assert_eq!(r.d(0), 0, "{name}");
            assert_eq!(r.get(ZF, 1), z_before, "{name}: a zero result keeps Z");

            let mut r = Regs::new();
            r.set_d(0, nonzero.0);
            r.set_d(1, nonzero.1);
            r.set(XF, 1, nonzero.2);
            r.set(ZF, 1, z_before);
            run(f, &mut r).0.unwrap();
            assert_ne!(r.d(0), 0, "{name}");
            assert_eq!(r.get(ZF, 1), 0, "{name}: a nonzero result clears Z");
        }
    }
}
