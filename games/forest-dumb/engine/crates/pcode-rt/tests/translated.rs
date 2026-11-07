//! The translator's output run (translator-trial T03). Each fixture is a made-up program written as
//! a P-code export by hand (`fixtures/*.json`) and its translation by `tools/pcode_rust.py`
//! (`fixtures/*.rs`), committed so this test needs no Python. `tools/test_pcode_rust.py` fails when a
//! translation here differs from what the translator now writes, which is how a translator change
//! reaches these tests.
//!
//! Each run calls the entry as a subroutine, as the judge does: a return address outside every hunk
//! on the stack, and after the run `PC` holds it and the stack is back where it was.

use pcode_rt::{Bus, CF, Image, PC, Regs, Stop, XF, ZF};

#[path = "fixtures/bcd.rs"]
mod bcd;
#[path = "fixtures/code_write.rs"]
mod code_write;
#[path = "fixtures/dbf_loop.rs"]
mod dbf_loop;
#[path = "fixtures/endless.rs"]
mod endless;
#[path = "fixtures/flags_call.rs"]
mod flags_call;
#[path = "fixtures/jump_table.rs"]
mod jump_table;
#[path = "fixtures/jump_table_hinted.rs"]
mod jump_table_hinted;
#[path = "fixtures/nested_call.rs"]
mod nested_call;
#[path = "fixtures/pushed_return.rs"]
mod pushed_return;
#[path = "fixtures/relative.rs"]
mod relative;
#[path = "fixtures/unsupported.rs"]
mod unsupported;

/// Hunk 0 at $1000 and hunk 1 at $3000: neither at 0, so an address left absolute shows.
const BASES: [u32; 2] = [0x1000, 0x3000];
const STACK: u32 = 0x8000;
const RETURN_TO: u32 = 0xf0_0000;

struct Ram(Vec<u8>);

impl Bus for Ram {
    fn rb(&mut self, a: u32) -> u8 {
        self.0[a as usize]
    }
    fn rw(&mut self, a: u32) -> u16 {
        u16::from_be_bytes([self.rb(a), self.rb(a + 1)])
    }
    fn rl(&mut self, a: u32) -> u32 {
        ((self.rw(a) as u32) << 16) | self.rw(a + 2) as u32
    }
    fn wb(&mut self, a: u32, v: u8) {
        self.0[a as usize] = v;
    }
    fn ww(&mut self, a: u32, v: u16) {
        self.0[a as usize..a as usize + 2].copy_from_slice(&v.to_be_bytes());
    }
    fn wl(&mut self, a: u32, v: u32) {
        self.0[a as usize..a as usize + 4].copy_from_slice(&v.to_be_bytes());
    }
}

type Routine = fn(&mut Regs, &mut dyn Bus, &Image) -> Result<(), Stop>;

/// Calls `f` with `RETURN_TO` pushed, on `ram`, with the registers `r`.
fn call(
    f: Routine,
    sizes: &[u32],
    code: &[(u32, u32, u32)],
    r: &mut Regs,
    ram: &mut Ram,
) -> Result<(), Stop> {
    let img = Image {
        bases: &BASES[..sizes.len()],
        sizes,
        code,
    };
    r.set_a(7, STACK - 4);
    ram.wl(STACK - 4, RETURN_TO);
    f(r, ram, &img)
}

fn ram() -> Ram {
    Ram(vec![0; 0x10000])
}

/// After a return to the caller: PC is the pushed address and the stack is as it was.
fn returned(r: &Regs) {
    assert_eq!(r.get(PC, 4) as u32, RETURN_TO, "PC after the run");
    assert_eq!(r.a(7), STACK, "SP after the run");
}

/// Ghidra's P-code for these, as exported, gives d0 = $48 (the compare's carry taken as X) and leaves
/// the memory sum unadjusted: the translator's corrections are what make them decimal.
#[test]
fn decimal_add_and_subtract_after_a_compare_keep_the_entry_x() {
    let (mut r, mut m) = (Regs::new(), ram());
    // cmpi.b #1,d3 with d3 = 0 borrows (C = 1) but leaves X at its entry 0.
    r.set_d(3, 0);
    r.set_d(0, 0xaaaa_aa19);
    r.set_d(1, 0x28);
    // A two-byte sum by abcd -(a1),-(a0) twice: $0999 + $0001.
    m.0[0x2000..0x2002].copy_from_slice(&[0x09, 0x99]);
    m.0[0x2100..0x2102].copy_from_slice(&[0x00, 0x01]);
    r.set_a(0, 0x2002);
    r.set_a(1, 0x2102);
    r.set_d(4, 0x10);
    r.set_d(2, 0x01);
    let f = bcd::entry("h0+$0").unwrap();
    call(f, bcd::HUNK_SIZES, bcd::CODE, &mut r, &mut m).unwrap();
    returned(&r);
    assert_eq!(r.d(0), 0xaaaa_aa47, "abcd d1,d0: 19 + 28, X 0, upper bytes kept");
    assert_eq!(&m.0[0x2000..0x2002], &[0x10, 0x00], "0999 + 0001, the carry through X");
    assert_eq!((r.a(0), r.a(1)), (0x2000, 0x2100), "each byte predecremented once");
    assert_eq!(r.d(4), 0x09, "sbcd d2,d4: 10 - 01");
    assert_eq!((r.get(XF, 1), r.get(CF, 1)), (0, 0));
    assert_eq!(r.get(ZF, 1), 0, "cleared by the first non-zero byte and never set again");
}

#[test]
fn a_dbf_loop_runs_its_count_plus_one_times() {
    let (mut r, mut m) = (Regs::new(), ram());
    r.set_d(0, 0xabcd_1234);
    let f = dbf_loop::entry("h0+$0").unwrap();
    call(f, dbf_loop::HUNK_SIZES, dbf_loop::CODE, &mut r, &mut m).unwrap();
    returned(&r);
    // moveq #3 then dbf: four passes of addq #2; dbf leaves the word at -1 and the high word alone.
    assert_eq!(r.d(1), 8);
    assert_eq!(r.d(0), 0x0000_ffff);
    assert_eq!(r.get(ZF, 1), 0, "flags of the last addq");
}

#[test]
fn a_nested_call_returns_through_both_levels() {
    let (mut r, mut m) = (Regs::new(), ram());
    let f = nested_call::entry("h0+$0").unwrap();
    call(f, nested_call::HUNK_SIZES, nested_call::CODE, &mut r, &mut m).unwrap();
    returned(&r);
    assert_eq!(r.d(2), 6, "moveq #5 in the inner call, addq #1 in the middle one");
    assert_eq!(r.d(3), 6, "the outer routine's move after its call returned");
    // The return addresses pushed are hunk-relative, resolved at the run's bases.
    assert_eq!(m.rl(STACK - 8), BASES[0] + 0x4);
    assert_eq!(m.rl(STACK - 12), BASES[0] + 0x12);
    // Each callee is an entry of its own too.
    assert!(nested_call::entry("h0+$10").is_some() && nested_call::entry("h0+$20").is_some());
    assert!(nested_call::entry("h0+$4").is_none());
}

#[test]
fn a_caller_branches_on_the_flags_its_callee_set() {
    let f = flags_call::entry("h0+$0").unwrap();
    for (d0, d1, d2) in [(5, 5, 2), (5, 6, 1), (0, 0xffff_ffff, 1)] {
        let (mut r, mut m) = (Regs::new(), ram());
        r.set_d(0, d0);
        r.set_d(1, d1);
        call(f, flags_call::HUNK_SIZES, flags_call::CODE, &mut r, &mut m).unwrap();
        returned(&r);
        assert_eq!(r.d(2), d2, "d0={d0:x} d1={d1:x}");
        assert_eq!(r.get(ZF, 1), (d0 == d1) as u64);
    }
}

/// The table the jump reads, as the loader leaves it: absolute addresses of hunk 0's two cases.
fn jump_table_ram() -> Ram {
    let mut m = ram();
    m.wl(BASES[1], BASES[0] + 0x10);
    m.wl(BASES[1] + 4, BASES[0] + 0x14);
    m
}

#[test]
fn a_jump_table_resolved_by_a_hint_runs_each_case() {
    let f = jump_table_hinted::entry("h0+$0").unwrap();
    for (d0, d1) in [(0, 10), (1, 20)] {
        let (mut r, mut m) = (Regs::new(), jump_table_ram());
        r.set_d(0, d0);
        call(f, jump_table_hinted::HUNK_SIZES, jump_table_hinted::CODE, &mut r, &mut m).unwrap();
        returned(&r);
        assert_eq!(r.d(1), d1, "case {d0}");
        assert_eq!(r.a(0), BASES[1], "the relocated lea points at hunk 1");
    }
}

#[test]
fn the_same_jump_table_without_its_hint_stops_untranslated() {
    let f = jump_table::entry("h0+$0").unwrap();
    for (d0, at) in [(0, "h0+$10"), (1, "h0+$14")] {
        let (mut r, mut m) = (Regs::new(), jump_table_ram());
        r.set_d(0, d0);
        let got = call(f, jump_table::HUNK_SIZES, jump_table::CODE, &mut r, &mut m);
        assert_eq!(got, Err(Stop::Untranslated(at.into())));
    }
    // A target outside every hunk is named by its absolute address.
    let (mut r, mut m) = (Regs::new(), ram());
    m.wl(BASES[1], 0x00df_f000);
    let got = call(f, jump_table::HUNK_SIZES, jump_table::CODE, &mut r, &mut m);
    assert_eq!(got, Err(Stop::Untranslated("$dff000".into())));
}

#[test]
fn a_store_into_its_own_code_stops_and_writes_nothing_there() {
    let (mut r, mut m) = (Regs::new(), ram());
    r.set_d(0, 0x1122_3344);
    r.set_d(1, 0x4e71);
    let f = code_write::entry("h0+$0").unwrap();
    let got = call(f, code_write::HUNK_SIZES, code_write::CODE, &mut r, &mut m);
    assert_eq!(got, Err(Stop::WriteIntoCode("h0+$2".into())));
    assert_eq!(m.rl(BASES[1] + 4), 0x1122_3344, "the data store before it landed");
    assert_eq!(m.rw(BASES[0] + 2), 0, "the code byte is untouched");
}

#[test]
fn an_endless_loop_stops_as_a_hang() {
    let (mut r, mut m) = (Regs::new(), ram());
    let f = endless::entry("h0+$0").unwrap();
    let got = call(f, endless::HUNK_SIZES, endless::CODE, &mut r, &mut m);
    assert_eq!(got, Err(Stop::Hang));
}

#[test]
fn an_unsupported_op_stops_only_when_its_instruction_runs() {
    let f = unsupported::entry("h0+$0").unwrap();
    let (mut r, mut m) = (Regs::new(), ram());
    call(f, unsupported::HUNK_SIZES, unsupported::CODE, &mut r, &mut m).unwrap();
    returned(&r);
    assert_eq!(r.d(0), 7, "d1 = 0 branches round the trap");

    let (mut r, mut m) = (Regs::new(), ram());
    r.set_d(1, 1);
    let got = call(f, unsupported::HUNK_SIZES, unsupported::CODE, &mut r, &mut m);
    assert_eq!(
        got,
        Err(Stop::Unsupported {
            op: "CALLOTHER __m68k_trap".into(),
            at: "h0+$6".into()
        })
    );
}

/// The callee overwrites its return address with d4 and returns there (68000 code may).
#[test]
fn a_return_to_another_pushed_address_continues_there() {
    let f = pushed_return::entry("h0+$0").unwrap();
    // The address after the call: the normal path, moveq #1 then addq #4.
    // h0+$8, a block of the caller past the moveq: only the addq runs.
    for (d4, d2) in [(BASES[0] + 0x6, 1), (BASES[0] + 0x8, 0)] {
        let (mut r, mut m) = (Regs::new(), ram());
        r.set_d(4, d4);
        call(f, pushed_return::HUNK_SIZES, pushed_return::CODE, &mut r, &mut m).unwrap();
        returned(&r);
        assert_eq!((r.d(2), r.d(3)), (d2, 4), "returned to {d4:x}");
    }
    // An address with no translation stops there.
    for (d4, at) in [(BASES[0] + 0x30, "h0+$30"), (0x00fc_0000, "$fc0000")] {
        let (mut r, mut m) = (Regs::new(), ram());
        r.set_d(4, d4);
        let got = call(f, pushed_return::HUNK_SIZES, pushed_return::CODE, &mut r, &mut m);
        assert_eq!(got, Err(Stop::Untranslated(at.into())));
    }
}

#[test]
fn branches_inside_one_instruction_take_either_way() {
    let f = relative::entry("h0+$0").unwrap();
    for (d0, d1, d2) in [(3, 3, 2), (3, 4, 1)] {
        let (mut r, mut m) = (Regs::new(), ram());
        r.set_d(0, d0);
        r.set_d(1, d1);
        call(f, relative::HUNK_SIZES, relative::CODE, &mut r, &mut m).unwrap();
        returned(&r);
        assert_eq!(r.d(2), d2);
    }
}

#[test]
fn code_lists_every_translated_instructions_bytes() {
    assert_eq!(dbf_loop::CODE, &[(0, 0, 0xc)]);
    assert_eq!(nested_call::CODE, &[(0, 0, 8), (0, 0x10, 0x16), (0, 0x20, 0x24)]);
    // Without its hint the jump table's cases are not translated, so they are not code.
    assert_eq!(jump_table::CODE, &[(0, 0, 0x10)]);
    assert_eq!(jump_table_hinted::CODE, &[(0, 0, 0x18)]);
    assert_eq!(code_write::HUNK_SIZES, &[0x20, 0x10]);
}
