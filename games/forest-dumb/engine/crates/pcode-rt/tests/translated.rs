//! The translator's output run (translator-trial T03). Each fixture is a made-up program written as
//! a P-code export by hand (`fixtures/*.json`) and its translation by `tools/pcode_rust.py`
//! (`fixtures/*.rs`), committed so this test needs no Python. `tools/test_pcode_rust.py` fails when a
//! translation here differs from what the translator now writes, which is how a translator change
//! reaches these tests.
//!
//! Each run calls the entry as a subroutine, as the judge does: a return address outside every hunk
//! on the stack, and after the run `PC` holds it and the stack is back where it was.

use pcode_rt::{Bus, CF, DEPTH_CAP, Hang, Image, PC, Regs, STEP_CAP_GLOBAL, Stop, XF, ZF};

#[path = "fixtures/bcd.rs"]
mod bcd;
#[path = "fixtures/call_loop.rs"]
mod call_loop;
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
#[path = "fixtures/mutual_call.rs"]
mod mutual_call;
#[path = "fixtures/nested_call.rs"]
mod nested_call;
#[path = "fixtures/pushed_return.rs"]
mod pushed_return;
#[path = "fixtures/relative.rs"]
mod relative;
#[path = "fixtures/self_call.rs"]
mod self_call;
#[path = "fixtures/shared_call.rs"]
mod shared_call;
// The same program written as a directory of two parts (translator T04, `--out DIR --parts 2`).
#[path = "fixtures/shared_call/mod.rs"]
mod shared_call_split;
#[path = "fixtures/unsupported.rs"]
mod unsupported;
#[path = "fixtures/os_call.rs"]
mod os_call;

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

/// [`Ram`] with one hand-written routine mounted at `at` (the seam, translator T35).
struct Mounting(Ram, (u32, u32), pcode_rt::Mounted);

impl Bus for Mounting {
    fn mounted(&self, k: (u32, u32)) -> Option<pcode_rt::Mounted> {
        (k == self.1).then_some(self.2)
    }
    fn rb(&mut self, a: u32) -> u8 {
        self.0.rb(a)
    }
    fn rw(&mut self, a: u32) -> u16 {
        self.0.rw(a)
    }
    fn rl(&mut self, a: u32) -> u32 {
        self.0.rl(a)
    }
    fn wb(&mut self, a: u32, v: u8) {
        self.0.wb(a, v)
    }
    fn ww(&mut self, a: u32, v: u16) {
        self.0.ww(a, v)
    }
    fn wl(&mut self, a: u32, v: u32) {
        self.0.wl(a, v)
    }
}

/// `moveq #9,d2` and the original's `rts`, by hand.
fn nine(r: &mut Regs, bus: &mut dyn Bus, _: &Image) -> Result<(), Stop> {
    r.set_d(2, 9);
    pcode_rt::rts(r, bus);
    Ok(())
}

#[test]
fn a_mounted_routine_runs_in_place_of_the_translated_one_wherever_it_is_called() {
    let f = nested_call::entry("h0+$0").unwrap();
    // The innermost (`moveq #5`) replaced: the middle routine's `addq #1` runs on the mounted 9.
    for (at, d2) in [((0, 0x20), 10), ((0, 0x10), 9)] {
        let (mut r, ram) = (Regs::new(), ram());
        let mut m = Mounting(ram, at, nine);
        let img = Image {
            bases: &BASES[..1],
            sizes: nested_call::HUNK_SIZES,
            code: nested_call::CODE,
        };
        r.set_a(7, STACK - 4);
        m.wl(STACK - 4, RETURN_TO);
        f(&mut r, &mut m, &img).unwrap();
        returned(&r);
        assert_eq!((r.d(2), r.d(3)), (d2, d2), "mounted at {at:?}");
        assert_eq!(r.depth, 0, "a mounted routine counts no translated frame");
    }
    // Mounted at the entry the host calls: the translated code never runs.
    let (mut r, mut m) = (Regs::new(), Mounting(ram(), (0, 0), nine));
    let img = Image {
        bases: &BASES[..1],
        sizes: nested_call::HUNK_SIZES,
        code: nested_call::CODE,
    };
    r.set_a(7, STACK - 4);
    m.wl(STACK - 4, RETURN_TO);
    f(&mut r, &mut m, &img).unwrap();
    returned(&r);
    assert_eq!((r.d(2), r.d(3)), (9, 0));
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
    assert_eq!(got, Err(Stop::Hang { why: Hang::Steps }));
    assert_eq!(r.steps, STEP_CAP_GLOBAL + 1);
}

/// `bsr` to itself: each call pushes and nests one more frame, so before the stack could overflow
/// the depth cap stops it.
#[test]
fn a_routine_calling_itself_stops_as_a_depth_hang() {
    let (mut r, mut m) = (Regs::new(), ram());
    let f = self_call::entry("h0+$0").unwrap();
    let got = call(f, self_call::HUNK_SIZES, self_call::CODE, &mut r, &mut m);
    assert_eq!(got, Err(Stop::Hang { why: Hang::Depth }));
    assert_eq!(got.unwrap_err().to_string(), "hang (depth)");
    assert_eq!(r.depth, DEPTH_CAP + 1);
    // DEPTH_CAP calls ran, each pushing its return address.
    assert_eq!(r.a(7), STACK - 4 - 4 * DEPTH_CAP);
    assert_eq!(r.steps, DEPTH_CAP as u64, "one block a frame, the last stopped on entry");
}

#[test]
fn two_routines_calling_each_other_by_jsr_stop_as_a_depth_hang() {
    let (mut r, mut m) = (Regs::new(), ram());
    let f = mutual_call::entry("h0+$0").unwrap();
    let got = call(f, mutual_call::HUNK_SIZES, mutual_call::CODE, &mut r, &mut m);
    assert_eq!(got, Err(Stop::Hang { why: Hang::Depth }));
    assert_eq!(r.depth, DEPTH_CAP + 1);
    // Return addresses alternate: h0+$6 from the first, h0+$16 from the second.
    assert_eq!(m.rl(STACK - 8), BASES[0] + 0x6);
    assert_eq!(m.rl(STACK - 12), BASES[0] + 0x16);
}

/// Every pass of the loop goes through a call that returns, so no one call runs long: only the
/// count across calls sees it. The depth goes back to 1 after each return.
#[test]
fn a_loop_spread_across_calls_stops_as_a_steps_hang() {
    let (mut r, mut m) = (Regs::new(), ram());
    let f = call_loop::entry("h0+$0").unwrap();
    let got = call(f, call_loop::HUNK_SIZES, call_loop::CODE, &mut r, &mut m);
    assert_eq!(got, Err(Stop::Hang { why: Hang::Steps }));
    assert_eq!(got.unwrap_err().to_string(), "hang (steps)");
    assert_eq!(r.steps, STEP_CAP_GLOBAL + 1);
    // Two blocks a pass (the caller's, the callee's); the step past the cap is a caller block,
    // after STEP_CAP_GLOBAL / 2 calls, each adding 1 to d0.
    assert_eq!(r.d(0) as u64, STEP_CAP_GLOBAL / 2);
    assert_eq!(r.depth, 1);
    assert_eq!(r.a(7), STACK - 4, "every call returned");
}

/// A run's counters go on from where the host left them: a second call does not start again at 0.
#[test]
fn the_step_count_carries_across_runs_until_the_host_resets_it() {
    let f = dbf_loop::entry("h0+$0").unwrap();
    let (mut r, mut m) = (Regs::new(), ram());
    call(f, dbf_loop::HUNK_SIZES, dbf_loop::CODE, &mut r, &mut m).unwrap();
    let one = r.steps;
    assert_eq!(r.depth, 0, "back to the host's depth after a return");
    call(f, dbf_loop::HUNK_SIZES, dbf_loop::CODE, &mut r, &mut m).unwrap();
    assert_eq!(r.steps, 2 * one);
    r.steps = STEP_CAP_GLOBAL - 1;
    let got = call(f, dbf_loop::HUNK_SIZES, dbf_loop::CODE, &mut r, &mut m);
    assert_eq!(got, Err(Stop::Hang { why: Hang::Steps }));
}

/// `Ram` that counts `poll`s, and stops on the `stop_at`th with an `Unsupported` naming it.
struct Polled {
    ram: Ram,
    polls: u64,
    stop_at: u64,
}

impl Bus for Polled {
    fn rb(&mut self, a: u32) -> u8 {
        self.ram.rb(a)
    }
    fn rw(&mut self, a: u32) -> u16 {
        self.ram.rw(a)
    }
    fn rl(&mut self, a: u32) -> u32 {
        self.ram.rl(a)
    }
    fn wb(&mut self, a: u32, v: u8) {
        self.ram.wb(a, v)
    }
    fn ww(&mut self, a: u32, v: u16) {
        self.ram.ww(a, v)
    }
    fn wl(&mut self, a: u32, v: u32) {
        self.ram.wl(a, v)
    }
    fn poll(&mut self, r: &mut Regs, _img: &Image) -> Result<(), Stop> {
        self.polls += 1;
        // Polled after the block was counted, so the count it sees includes its own block.
        assert_eq!(r.steps, self.polls);
        if self.polls == self.stop_at {
            return Err(Stop::Unsupported { op: "poll".into(), at: format!("{}", self.polls) });
        }
        Ok(())
    }
}

#[test]
fn poll_is_called_once_per_block_and_its_stop_ends_the_run() {
    // dbf_loop: moveq #3,d0 / addq #2,d1 / dbf d0 / rts. Blocks: the moveq's, then the loop body
    // four times (the dbf's branch back is a block boundary), then the rts's: 1 + 4 + 1.
    let f = dbf_loop::entry("h0+$0").unwrap();
    let mut r = Regs::new();
    let mut b = Polled { ram: ram(), polls: 0, stop_at: u64::MAX };
    let img = Image { bases: &BASES[..1], sizes: dbf_loop::HUNK_SIZES, code: dbf_loop::CODE };
    r.set_a(7, STACK - 4);
    b.ram.wl(STACK - 4, RETURN_TO);
    f(&mut r, &mut b, &img).unwrap();
    returned(&r);
    assert_eq!(b.polls, 6);
    assert_eq!(r.steps, 6);

    // Across calls too, and a stop from poll ends the run with nothing after it run.
    let f = nested_call::entry("h0+$0").unwrap();
    let img = Image { bases: &BASES[..1], sizes: nested_call::HUNK_SIZES, code: nested_call::CODE };
    let mut r = Regs::new();
    let mut b = Polled { ram: ram(), polls: 0, stop_at: u64::MAX };
    r.set_a(7, STACK - 4);
    b.ram.wl(STACK - 4, RETURN_TO);
    f(&mut r, &mut b, &img).unwrap();
    assert_eq!(b.polls, 3, "one block in each of three functions");
    let mut r = Regs::new();
    let mut b = Polled { ram: ram(), polls: 0, stop_at: 3 };
    r.set_a(7, STACK - 4);
    b.ram.wl(STACK - 4, RETURN_TO);
    let got = f(&mut r, &mut b, &img);
    assert_eq!(got, Err(Stop::Unsupported { op: "poll".into(), at: "3".into() }));
    assert_eq!(r.d(2), 0, "the innermost block's moveq #5 never ran");
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

/// A run resumed at any instruction (translator T28: a host starting from a snapshot of the
/// original) starts there: the instructions of its block before it are skipped.
#[test]
fn a_run_resumed_mid_block_runs_from_that_instruction_on() {
    // h0+$a, the rts after `addq.l #4,d3`: the addq is skipped.
    // h0+$6, the moveq after the bsr: moveq, then the addq and the rts.
    // h0+$2, the bsr's block start: the call too (d4 gives it the normal return).
    for (at, d2, d3) in [(0xa, 0, 0), (0x6, 1, 4), (0x2, 1, 4)] {
        let f = pushed_return::resume((0, at)).expect("every instruction resumes");
        let (mut r, mut m) = (Regs::new(), ram());
        r.set_d(4, BASES[0] + 0x6);
        let img = Image {
            bases: &BASES[..1],
            sizes: pushed_return::HUNK_SIZES,
            code: pushed_return::CODE,
        };
        r.set_a(7, STACK - 4);
        m.wl(STACK - 4, RETURN_TO);
        f((0, at), &mut r, &mut m, &img).unwrap();
        returned(&r);
        assert_eq!((r.d(2), r.d(3), r.depth), (d2, d3, 0), "resumed at h0+${at:x}");
    }
    // Inside an instruction, or no instruction: nothing to resume.
    assert!(pushed_return::resume((0, 0x4)).is_none());
    assert!(pushed_return::resume((0, 0x30)).is_none());
    // A function's own entry is run by that function.
    assert_eq!(
        pushed_return::resume((0, 0x20)).map(|f| f as usize),
        Some(pushed_return::h0_20_from as pushed_return::Resume as usize)
    );
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

/// The split tree runs exactly as the one-module tree: the caller and one helper in one part, the
/// other helper and the routine both helpers call in the other.
#[test]
fn a_tree_split_into_parts_runs_as_the_unsplit_one() {
    let run = |f: Routine, sizes: &[u32], code: &[(u32, u32, u32)]| {
        let (mut r, mut m) = (Regs::new(), ram());
        r.set_d(1, 10);
        call(f, sizes, code, &mut r, &mut m).unwrap();
        returned(&r);
        (r, m.0)
    };
    let (one, one_mem) = run(shared_call::entry("h0+$0").unwrap(), shared_call::HUNK_SIZES, shared_call::CODE);
    let (split, split_mem) = run(
        shared_call_split::entry("h0+$0").unwrap(),
        shared_call_split::HUNK_SIZES,
        shared_call_split::CODE,
    );
    assert_eq!((one.d(1), one.d(2), one.d(3)), (12, 1, 2), "the shared routine ran twice");
    assert_eq!(one, split, "registers");
    assert!(one_mem == split_mem, "memory");
    assert_eq!(shared_call::CODE, shared_call_split::CODE);
    for at in ["h0+$0", "h0+$10", "h0+$20", "h0+$30"] {
        assert!(shared_call_split::entry(at).is_some(), "{at}");
    }
}

/// A library call (`jsr -408(a6)`) leaves the hunks; on plain memory, which has no OS, the bus's
/// default `call_outside` stops naming the vector's address (translator T26).
#[test]
fn a_call_outside_every_hunk_goes_to_the_bus_which_by_default_stops() {
    let (mut r, mut m) = (Regs::new(), ram());
    m.wl(4, 0xc000);
    let f = os_call::entry("h0+$0").unwrap();
    let e = call(f, os_call::HUNK_SIZES, os_call::CODE, &mut r, &mut m).unwrap_err();
    assert_eq!(e, Stop::Untranslated(format!("${:06x}", 0xc000 - 408)));
    assert_eq!(r.a(7), STACK - 8, "the jsr pushed its return address");
    assert_eq!(m.rl(STACK - 8), BASES[0] + 0xe);
}
