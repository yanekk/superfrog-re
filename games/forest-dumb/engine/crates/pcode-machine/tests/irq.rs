//! Interrupts taken at block starts (translator T23, DESIGN §2.8), on a made-up program: its P-code
//! written by hand (`fixtures/irq.json`) and translated by `tools/pcode_rust.py` (`fixtures/irq.rs`,
//! committed, kept current by `tools/test_pcode_rust.py`), so its `rte` is the translator's own.
//!
//! The program: `h0+$0` waits until the word at `$100` equals d2; `h0+$10` requests level 3 by an
//! INTREQ write d2 times, then waits as `h0+$0`. Handlers: `h0+$40` (level 3) counts at `$100` and
//! clears VERTB; `h0+$60` (level 2) requests level 3, reads `$100` into d3 a block later, clears
//! PORTS; `h0+$80` (level 3) requests level 2, reads `$102` into d4 a block later, counts at `$100`,
//! clears VERTB; `h0+$a0` (level 2) counts at `$102` and clears PORTS.

use pcode_machine::{Config, Machine, irq};
use pcode_rt::{Bus, Image, PC, Regs, SR, Stop, cycles as cy, ops};

#[path = "fixtures/irq.rs"]
mod prog;

const BASE: u32 = 0x4000;
const STACK: u32 = 0x8000;
const RETURN_TO: u32 = 0xF0_0000;
const VERTB: u16 = 5;
const PORTS: u16 = 3;
const L2: u32 = 0x68;
const L3: u32 = 0x6C;

fn img() -> Image<'static> {
    Image {
        bases: &[BASE],
        sizes: prog::HUNK_SIZES,
        code: prog::CODE,
    }
}

/// A zeroed A500 with the program's entry, INTENA set to `intena`, and the level 2 and 3 vectors
/// at the handlers `l2` and `l3` (hunk 0 offsets).
fn machine(intena: u16, l2: u32, l3: u32) -> Machine {
    let cfg = Config::a500(vec![BASE]);
    let mem = vec![0; (cfg.chip + cfg.slow) as usize];
    let mut m = Machine::from_snapshot(cfg, &mem, &[0; 256]);
    m.set_entry(prog::entry);
    m.wl(L2, BASE + l2);
    m.wl(L3, BASE + l3);
    m.ww(0xDF_F09A, 0x8000 | intena);
    m
}

/// Supervisor mode, mask 0, the system stack at `STACK`.
fn regs() -> Regs {
    let mut r = Regs::new();
    r.set(SR, 2, 0x2000);
    r.set_a(7, STACK);
    r
}

/// Calls `f` as a subroutine, as the judge does: `RETURN_TO` pushed, then checked on return.
fn call(m: &mut Machine, r: &mut Regs, f: prog::Routine) -> Result<(), Stop> {
    let sp = r.a(7) - 4;
    m.wl(sp, RETURN_TO);
    r.set_a(7, sp);
    f(r, m, &img())?;
    assert_eq!(r.get(PC, 4) as u32, RETURN_TO, "returned to its caller");
    Ok(())
}

fn word(m: &mut Machine, a: u32) -> u16 {
    m.rw(a)
}

#[test]
fn a_foreground_loop_resumes_after_each_handler_its_requests_ran() {
    let mut m = machine(irq::INTEN | 1 << VERTB, 0, 0x40);
    let mut r = regs();
    r.set_d(2, 3);
    call(&mut m, &mut r, prog::h0_10).unwrap();
    assert_eq!(word(&mut m, 0x100), 3, "the handler ran once per request");
    assert_eq!(r.d(1), 3, "the loop ran on after each");
    assert_eq!(m.intreq(), 0, "each handler cleared its request");
    // The loop's last compare leaves Z set; the system byte is what the frames restored.
    assert_eq!(
        (r.a(7), ops::sr_get(&r) & 0xFF00),
        (STACK, 0x2000),
        "every rte popped its frame"
    );
    assert_eq!(r.depth, 0);
}

#[test]
fn a_request_raised_by_a_device_is_taken_once_its_cycle_is_reached() {
    let mut m = machine(irq::INTEN | 1 << VERTB, 0, 0x40);
    m.raise(VERTB, 0);
    m.raise(VERTB, 500);
    let mut r = regs();
    r.set_d(2, 2);
    call(&mut m, &mut r, prog::h0_0).unwrap();
    assert_eq!(word(&mut m, 0x100), 2);
    assert!(
        r.cycles() >= 500,
        "the second waited for its cycle: {}",
        r.cycles()
    );
    // Cleared by the handler: polling on takes nothing more.
    for _ in 0..3 {
        m.poll(&mut r, &img()).unwrap();
    }
    assert_eq!(word(&mut m, 0x100), 2, "not re-taken");
}

/// The cycles one interrupt into `h0+$40` adds: the exception, then the handler's `rte`, the one
/// instruction there with its bytes in the export.
fn one_irq() -> u64 {
    (irq::IRQ_CYCLES + cy::of(cy::Op::Rte, cy::Size::L, cy::Ea::None, cy::Ea::None)) as u64
}

#[test]
fn masked_by_sr_until_the_mask_drops_then_the_frame_is_the_68000s() {
    let mut m = machine(irq::INTEN | 1 << VERTB, 0, 0x40);
    let mut r = regs();
    ops::sr_set(&mut r, 0x2315);
    m.raise(VERTB, 0);
    m.poll(&mut r, &img()).unwrap();
    assert_eq!(word(&mut m, 0x100), 0, "mask 3 holds level 3 off");
    assert_eq!((r.a(7), r.cycles()), (STACK, 0));
    ops::sr_set(&mut r, 0x2215);
    m.poll(&mut r, &img()).unwrap();
    assert_eq!(word(&mut m, 0x100), 1, "taken once the mask is below 3");
    assert_eq!(r.cycles(), one_irq());
    assert_eq!(
        ops::sr_get(&r),
        0x2215,
        "rte restored SR, condition codes too"
    );
    // The frame below the stack, as rte read it: SR, then the PC long.
    assert_eq!(word(&mut m, STACK - 6), 0x2215);
    assert_eq!(m.rl(STACK - 4), irq::IRQ_RETURN);
    assert_eq!(r.a(7), STACK);
}

#[test]
fn disabled_in_intena_or_with_the_master_off_is_not_taken() {
    for intena in [irq::INTEN | 1 << PORTS, 1 << VERTB] {
        let mut m = machine(intena, 0, 0x40);
        let mut r = regs();
        m.raise(VERTB, 0);
        m.poll(&mut r, &img()).unwrap();
        assert_eq!(word(&mut m, 0x100), 0, "INTENA ${intena:04x}");
        assert_eq!(m.rw(0xDF_F01E), 1 << VERTB, "INTREQR shows it requested");
        assert_eq!(m.rw(0xDF_F01C), intena, "INTENAR");
        // Enabling it lets the waiting request through.
        m.ww(0xDF_F09A, 0x8000 | irq::INTEN | 1 << VERTB);
        m.poll(&mut r, &img()).unwrap();
        assert_eq!(word(&mut m, 0x100), 1);
    }
}

#[test]
fn a_request_cleared_before_its_poll_is_not_taken() {
    let mut m = machine(irq::INTEN | 1 << VERTB, 0, 0x40);
    let mut r = regs();
    m.raise(VERTB, 0);
    m.ww(0xDF_F09C, 1 << VERTB);
    m.poll(&mut r, &img()).unwrap();
    assert_eq!(word(&mut m, 0x100), 0);
}

#[test]
fn level_3_is_taken_inside_a_level_2_handler() {
    let mut m = machine(irq::INTEN | 1 << VERTB | 1 << PORTS, 0x60, 0x40);
    m.raise(PORTS, 0);
    let mut r = regs();
    r.set_d(2, 1);
    call(&mut m, &mut r, prog::h0_0).unwrap();
    assert_eq!(
        r.d(3),
        1,
        "level 3 ran before the level 2 handler read the count"
    );
    assert_eq!(m.intreq(), 0);
    assert_eq!((r.a(7), ops::sr_get(&r) & 0xFF00), (STACK, 0x2000));
}

#[test]
fn level_2_waits_out_a_level_3_handler_then_is_taken() {
    let mut m = machine(irq::INTEN | 1 << VERTB | 1 << PORTS, 0xA0, 0x80);
    m.raise(VERTB, 0);
    let mut r = regs();
    r.set_d(2, 1);
    call(&mut m, &mut r, prog::h0_0).unwrap();
    assert_eq!(r.d(4), 0, "level 2 not taken inside level 3");
    assert_eq!(word(&mut m, 0x102), 1, "taken after level 3's rte");
    assert_eq!(m.intreq(), 0);
}

#[test]
fn from_user_mode_the_frame_goes_on_the_system_stack() {
    let mut m = machine(irq::INTEN | 1 << VERTB, 0, 0x40);
    let mut r = Regs::new();
    r.set_a(7, 0x7000);
    r.set(ops::ISP, 4, 0x9000);
    m.raise(VERTB, 0);
    m.poll(&mut r, &img()).unwrap();
    assert_eq!(word(&mut m, 0x100), 1);
    assert_eq!(
        word(&mut m, 0x9000 - 6),
        0x0000,
        "the user SR, pushed on the system stack"
    );
    assert_eq!((r.a(7), r.get(ops::ISP, 4)), (0x7000, 0x9000));
    assert_eq!(ops::sr_get(&r), 0);
}

#[test]
fn a_vector_with_no_translation_stops_naming_it() {
    let mut m = machine(irq::INTEN | 1 << VERTB, 0, 0x42);
    m.raise(VERTB, 0);
    let e = m.poll(&mut regs(), &img()).unwrap_err();
    assert_eq!(e, Stop::Untranslated("h0+$42".into()));

    let mut m = machine(irq::INTEN | 1 << VERTB, 0, 0x40);
    m.wl(L3, 0xFC_0D14);
    m.raise(VERTB, 0);
    let e = m.poll(&mut regs(), &img()).unwrap_err();
    assert_eq!(
        e,
        Stop::Untranslated("$fc0d14".into()),
        "a vector into Kickstart"
    );

    let cfg = Config::a500(vec![BASE]);
    let mem = vec![0; (cfg.chip + cfg.slow) as usize];
    let mut m = Machine::from_snapshot(cfg, &mem, &[0; 256]);
    m.wl(L3, BASE + 0x40);
    m.ww(0xDF_F09A, 0x8000 | irq::INTEN | 1 << VERTB);
    m.raise(VERTB, 0);
    let e = m.poll(&mut regs(), &img()).unwrap_err();
    assert_eq!(
        e,
        Stop::Untranslated("h0+$40".into()),
        "no entry handed over"
    );
}

#[test]
fn a_handler_leaving_by_rts_stops_the_run() {
    // h0+$0 with d2 = 0 returns at once by rts, popping the frame's SR and half its PC.
    let mut m = machine(irq::INTEN | 1 << VERTB, 0, 0x0);
    m.raise(VERTB, 0);
    let e = m.poll(&mut regs(), &img()).unwrap_err();
    assert!(
        matches!(&e, Stop::Unsupported { op, at } if op.contains("not by rte") && at == "h0+$0"),
        "{e:?}"
    );
}

#[test]
fn snapshot_intenar_and_intreqr_are_the_starting_state() {
    let cfg = Config::a500(vec![BASE]);
    let mem = vec![0; (cfg.chip + cfg.slow) as usize];
    let mut custom = [0u16; 256];
    custom[irq::INTENAR as usize / 2] = irq::INTEN | 1 << VERTB;
    custom[irq::INTREQR as usize / 2] = 1 << VERTB;
    let mut m = Machine::from_snapshot(cfg, &mem, &custom);
    m.set_entry(prog::entry);
    m.wl(L3, BASE + 0x40);
    assert_eq!(m.intena(), irq::INTEN | 1 << VERTB);
    m.poll(&mut regs(), &img()).unwrap();
    assert_eq!(
        word(&mut m, 0x100),
        1,
        "the snapshot's pending VERTB is taken"
    );
}
