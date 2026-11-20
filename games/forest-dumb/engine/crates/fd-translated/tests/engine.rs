//! `TranslatedGame` on the stub and on a made-up program (translator T28): no disks, no tree.
//!
//! The made-up program is pcode-machine's interrupt program (`tests/fixtures/irq.json`, translated by
//! `tools/pcode_rust.py` and kept current by `tools/test_pcode_rust.py`): `h0+$0` loops until the
//! word at `$100` equals d2, then returns; `h0+$40`, the level 3 handler, adds one to `$100` and
//! clears VERTB. A copper list whose first MOVE sets VERTB in INTREQ requests the handler once a
//! frame, at the frame's start.

use fd_translated::{Control, FrameOut, Program, Regs68k, Start, TranslatedGame};
use pcode_machine::{Config, Disks, Input, irq};
use pcode_rt::{Bus, Image, PC, Regs, Stop, ops};

#[rustfmt::skip]
#[path = "../../pcode-machine/tests/fixtures/irq.rs"]
mod prog;

/// `h0+$0`: `bsr $10; bsr $20; rts`, both calling `$30`: two calls in one block.
#[rustfmt::skip]
#[path = "../../pcode-rt/tests/fixtures/shared_call.rs"]
mod shared_call;

const BASE: u32 = 0x4000;
const STACK: u32 = 0x8000;
const LIST: u32 = 0x2000;
const COUNT: usize = 0x100;

const PROG: Program = Program {
    code: prog::CODE,
    hunk_sizes: prog::HUNK_SIZES,
    entry: prog::entry,
    resume: prog::resume,
};

/// Zeroed RAM with the level 3 vector at `h0+$40` and the copper list; the custom registers with
/// copper DMA and the VERTB interrupt on, COP1LC at the list and the beam at line 0.
fn memory() -> (Vec<u8>, [u16; 256]) {
    let cfg = Config::a500(vec![BASE]);
    let mut mem = vec![0u8; (cfg.chip + cfg.slow) as usize];
    mem[0x6C..0x70].copy_from_slice(&(BASE + 0x40).to_be_bytes());
    // MOVE #$8020,INTREQ; then the end of the list (WAIT for a position never reached).
    for (i, w) in [0x009Cu16, 0x8020, 0xFFFF, 0xFFFE].iter().enumerate() {
        let a = LIST as usize + 2 * i;
        mem[a..a + 2].copy_from_slice(&w.to_be_bytes());
    }
    let mut custom = [0u16; 256];
    custom[0x080 >> 1] = (LIST >> 16) as u16;
    custom[0x082 >> 1] = LIST as u16;
    custom[0x002 >> 1] = 0x0280; // DMACONR: DMAEN, COPEN
    custom[irq::INTENAR as usize >> 1] = irq::INTEN | 1 << 5;
    (mem, custom)
}

/// Supervisor, mask 0, at `pc` with d2 = `until` and the stack at `sp`.
fn regs(pc: u32, until: u32, sp: u32) -> Regs68k {
    let mut r = Regs68k {
        pc,
        sr: 0x2000,
        ..Regs68k::default()
    };
    r.d[2] = until;
    r.a[7] = sp;
    r
}

fn word(out: &FrameOut, a: usize) -> u16 {
    u16::from_be_bytes([out.chip[a], out.chip[a + 1]])
}

#[test]
fn on_the_stub_a_run_stops_untranslated_at_once() {
    assert_eq!(
        TranslatedGame::generated(),
        !fd_translated::tree::CODE.is_empty(),
        "generated() is whether a tree is built in"
    );
    let (mem, custom) = memory();
    let mut g = TranslatedGame::with_program(
        Program::STUB,
        &[BASE],
        Start::Snapshot {
            mem: &mem,
            regs: regs(BASE, 0, STACK),
            custom: &custom,
            disks: Disks::default(),
        },
    );
    let mut frames = Vec::new();
    let stop = g.run(&mut |out| {
        frames.push(out.stop.clone());
        Control::Next(Input::default())
    });
    assert!(matches!(stop, Some(Stop::Untranslated(_))), "{stop:?}");
    assert_eq!(frames, vec![stop.clone()], "one frame, carrying the stop");
    assert_eq!(
        g.run(&mut |_| Control::Stop),
        stop,
        "a stopped game stays stopped"
    );
}

#[test]
fn each_is_called_once_a_frame_and_the_handler_runs_once_in_each() {
    let (mem, custom) = memory();
    let mut g = TranslatedGame::with_program(
        PROG,
        &[BASE],
        Start::Snapshot {
            mem: &mem,
            regs: regs(BASE, 0xFFFF, STACK),
            custom: &custom,
            disks: Disks::default(),
        },
    );
    let mut counts = Vec::new();
    let mut ends = Vec::new();
    let stop = g.run(&mut |out| {
        assert_eq!(out.stop, None);
        counts.push(word(out, COUNT));
        ends.push(out.writes.iter().map(|w| w.0).max());
        // The copper's MOVE to INTREQ is a write of the frame.
        assert!(
            out.writes
                .iter()
                .any(|w| w.1 == irq::INTREQ && w.2 == 0x8020)
        );
        if counts.len() == 5 {
            Control::Stop
        } else {
            Control::Next(Input::default())
        }
    });
    assert_eq!(stop, None, "each said stop");
    assert_eq!(
        counts,
        [1, 2, 3, 4, 5],
        "one interrupt a frame, every frame seen"
    );
    // The frames are a frame apart: 313 lines of 227 slots, two cycles a slot.
    let frame = 313 * 227 * 2;
    assert!(
        g.regs().cycles() >= 4 * frame && g.regs().cycles() < 5 * frame + 1000,
        "{} cycles",
        g.regs().cycles()
    );
    assert_eq!(g.run(&mut |_| Control::Stop), None, "a host stop is final");
}

#[test]
fn a_program_that_returns_stops_with_a_last_frame_naming_it() {
    let (mut mem, custom) = memory();
    // The loop returns once the count reaches 3, to $f00000, outside every hunk.
    mem[STACK as usize - 4..STACK as usize].copy_from_slice(&0x00F0_0000u32.to_be_bytes());
    let mut g = TranslatedGame::with_program(
        PROG,
        &[BASE],
        Start::Snapshot {
            mem: &mem,
            regs: regs(BASE, 3, STACK - 4),
            custom: &custom,
            disks: Disks::default(),
        },
    );
    let mut frames = Vec::new();
    let stop = g.run(&mut |out| {
        frames.push((word(out, COUNT), out.stop.clone()));
        Control::Next(Input::default())
    });
    let want = Stop::Untranslated("$f00000".into());
    assert_eq!(stop, Some(want.clone()));
    assert_eq!(
        frames,
        [(1, None), (2, None), (3, Some(want))],
        "two whole frames, then the third's stop"
    );
}

/// A snapshot taken inside the handler, mid-block, with the foreground interrupted mid-block too:
/// the run resumes the handler there, and its `rte` resumes the foreground at the stacked PC.
#[test]
fn a_start_inside_a_handler_returns_to_the_interrupted_foreground() {
    let (mut mem, custom) = memory();
    // The exception frame the 68000 pushed: SR (mask 0, supervisor), then the PC of the loop's
    // `cmp.w d2,d0` (`h0+$4`), with d0 already loaded.
    let sp = STACK - 6;
    mem[sp as usize..sp as usize + 2].copy_from_slice(&0x2000u16.to_be_bytes());
    mem[sp as usize + 2..sp as usize + 6].copy_from_slice(&(BASE + 4).to_be_bytes());
    // The handler has added one already (count 1) and is at its INTREQ write (`h0+$44`); the
    // request it clears is still set.
    mem[COUNT..COUNT + 2].copy_from_slice(&1u16.to_be_bytes());
    let mut custom = custom;
    custom[irq::INTREQR as usize >> 1] = 1 << 5;
    let mut r = regs(BASE + 0x44, 0xFFFF, sp);
    r.sr = 0x2300;
    let mut g = TranslatedGame::with_program(
        PROG,
        &[BASE],
        Start::Snapshot {
            mem: &mem,
            regs: r,
            custom: &custom,
            disks: Disks::default(),
        },
    );
    let mut counts = Vec::new();
    let stop = g.run(&mut |out| {
        counts.push(word(out, COUNT));
        if counts.len() == 3 {
            Control::Stop
        } else {
            Control::Next(Input::default())
        }
    });
    assert_eq!(stop, None);
    // The resumed handler's own count, then one a frame from the copper's requests.
    assert_eq!(counts, [2, 3, 4]);
    assert_eq!(g.regs().a(7), STACK, "the frame was popped");
}

/// A level 3 handler that runs past a frame's end: 1.5 frames of blocks, then the made-up
/// program's own handler (count plus one, `rte`).
fn slow_handler(
    r: &mut pcode_rt::Regs,
    bus: &mut dyn pcode_rt::Bus,
    img: &pcode_rt::Image,
) -> Result<(), Stop> {
    for _ in 0..213 {
        r.tick(1000);
        pcode_rt::block_start(bus, r, img)?;
    }
    prog::h0_40(r, bus, img)
}

/// The frame boundary is seen inside a handler, not when it returns: the host runs handlers on its
/// own bus (`enter_interrupt`, the handler, `leave_interrupt`), so a handler's blocks reach its
/// `poll`. Handlers run on the machine's bus would hand the first frame over after the handler's
/// count, a frame late.
#[test]
fn a_frame_that_ends_inside_a_handler_is_handed_over_there() {
    let (mem, custom) = memory();
    let slow = Program {
        entry: |a| match a {
            "h0+$40" => Some(slow_handler),
            _ => prog::entry(a),
        },
        ..PROG
    };
    let mut g = TranslatedGame::with_program(
        slow,
        &[BASE],
        Start::Snapshot {
            mem: &mem,
            regs: regs(BASE, 0xFFFF, STACK),
            custom: &custom,
            disks: Disks::default(),
        },
    );
    let mut counts = Vec::new();
    let stop = g.run(&mut |out| {
        counts.push(word(out, COUNT));
        if counts.len() == 3 {
            Control::Stop
        } else {
            Control::Next(Input::default())
        }
    });
    assert_eq!(stop, None);
    // Frame 1 ends mid-handler, before its count; the handler returns in frame 2, whose VERTB,
    // held pending meanwhile, starts it again at once, and frame 3 ends inside that one.
    assert_eq!(counts, [0, 1, 1], "frame 1 handed over inside the handler");
}

/// The level 3 handler by hand, counting two a frame: `addq.w #2,($100).w`, VERTB cleared, `rte`.
/// It spends the `rte`'s 20 cycles and polls before leaving, as a translated handler's last block
/// does: the CPU samples its IPL lines only as the machine times its accesses, so a handler that
/// never polls leaves VERTB seen pending after the clear (Paula's 8-cycle IPL delay, T54), and its
/// `rte` took the interrupt again forever (the 25 GB run of 2026-11-20; `stuck_interrupt` now
/// stops it).
fn handler_by_two(r: &mut Regs, bus: &mut dyn Bus, img: &Image) -> Result<(), Stop> {
    let n = bus.rw(COUNT as u32);
    bus.ww(COUNT as u32, n.wrapping_add(2));
    bus.ww(irq::INTREQ as u32 | 0xdf_f000, 0x20);
    r.tick(20);
    bus.poll(r, img)?;
    let sp = r.a(7);
    let sr = bus.rw(sp);
    let pc = bus.rl(sp + 2);
    r.set_a(7, sp + 6);
    ops::sr_set(r, sr as u64);
    r.set(PC, 4, pc as u64);
    Ok(())
}

#[test]
fn a_mounted_routine_runs_in_place_of_the_translated_one_in_a_run() {
    let (mem, custom) = memory();
    let mut g = TranslatedGame::with_program(
        PROG,
        &[BASE],
        Start::Snapshot {
            mem: &mem,
            regs: regs(BASE, 0xFFFF, STACK),
            custom: &custom,
            disks: Disks::default(),
        },
    );
    // The machine enters the handler through the tree's `entry`, so the seam reaches it too.
    g.mount((0, 0x40), handler_by_two);
    assert_eq!(g.seam().addresses().collect::<Vec<_>>(), [(0, 0x40)]);
    let mut counts = Vec::new();
    let stop = g.run(&mut |out| {
        assert_eq!(out.stop, None);
        counts.push(word(out, COUNT));
        if counts.len() == 4 {
            Control::Stop
        } else {
            Control::Next(Input::default())
        }
    });
    assert_eq!(stop, None);
    assert_eq!(counts, [2, 4, 6, 8], "the mounted handler, once a frame");
}

/// A watched routine's returns land in the frame each happened in (translator T32): the handler
/// `h0+$40` returns once a frame; the foreground loop, never entered through its entry here
/// (the run resumes it from the snapshot's PC), is not seen, nor is a routine nobody watches.
#[test]
fn watched_returns_are_reported_in_their_frame() {
    let (mem, custom) = memory();
    let mut g = TranslatedGame::with_program(
        PROG,
        &[BASE],
        Start::Snapshot {
            mem: &mem,
            regs: regs(BASE, 0xFFFF, STACK),
            custom: &custom,
            disks: Disks::default(),
        },
    );
    g.watch_returns(&[(0, 0x40), (0, 0x0)]);
    let mut returns = Vec::new();
    g.run(&mut |out| {
        returns.push(out.returns.clone());
        assert!(out.inside.is_empty(), "the handler ends within its frame");
        if returns.len() == 4 {
            Control::Stop
        } else {
            Control::Next(Input::default())
        }
    });
    assert_eq!(
        returns,
        vec![vec![(0, 0x40)]; 4],
        "one handler return a frame"
    );

    let mut g = TranslatedGame::with_program(
        PROG,
        &[BASE],
        Start::Snapshot {
            mem: &mem,
            regs: regs(BASE, 0xFFFF, STACK),
            custom: &custom,
            disks: Disks::default(),
        },
    );
    let mut returns = Vec::new();
    g.run(&mut |out| {
        returns.extend(out.returns.iter().copied());
        if out.chip[COUNT + 1] == 3 {
            Control::Stop
        } else {
            Control::Next(Input::default())
        }
    });
    assert!(returns.is_empty(), "nothing watched, nothing reported");
}

/// A case's `set` with `when` (translator T33): the write is made as the handler `h0+$40` is
/// entered, before it adds one to `$100`, once; a write waiting for a routine the run never enters
/// through its entry (the foreground, resumed from the snapshot's PC) is never made.
#[test]
fn a_write_at_a_routine_is_made_on_its_entry_once() {
    let (mem, custom) = memory();
    let mut g = TranslatedGame::with_program(
        PROG,
        &[BASE],
        Start::Snapshot {
            mem: &mem,
            regs: regs(BASE, 0xFFFF, STACK),
            custom: &custom,
            disks: Disks::default(),
        },
    );
    g.write_at((0, 0x40), COUNT as u32, &[0x00, 0x50]);
    g.write_at((0, 0x0), COUNT as u32 + 2, &[0xAB]);
    let mut seen = Vec::new();
    g.run(&mut |out| {
        seen.push((word(out, COUNT), out.chip[COUNT + 2], out.writes_at_pending));
        if seen.len() == 3 {
            Control::Stop
        } else {
            Control::Next(Input::default())
        }
    });
    assert_eq!(
        seen,
        [(0x51, 0, 1), (0x52, 0, 1), (0x53, 0, 1)],
        "the handler's write once, before its add; the foreground's never"
    );
}

/// `jsr a; jsr b` in one block reaches `b`'s entry with no block of the caller between: `a`'s
/// return is reported there, before `b`'s, and `b`'s when the program returns and stops.
#[test]
fn a_return_followed_by_a_call_in_the_same_block_is_seen_in_order() {
    let prog = Program {
        code: shared_call::CODE,
        hunk_sizes: shared_call::HUNK_SIZES,
        entry: shared_call::entry,
        resume: shared_call::resume,
    };
    let (mut mem, _) = memory();
    mem[STACK as usize - 4..STACK as usize].copy_from_slice(&0x00F0_0000u32.to_be_bytes());
    let mut g = TranslatedGame::with_program(
        prog,
        &[BASE],
        Start::Snapshot {
            mem: &mem,
            regs: regs(BASE, 0, STACK - 4),
            custom: &[0; 256],
            disks: Disks::default(),
        },
    );
    g.watch_returns(&[(0, 0x10), (0, 0x20), (0, 0x30)]);
    let mut returns = Vec::new();
    let stop = g.run(&mut |out| {
        returns.extend(out.returns.iter().copied());
        Control::Next(Input::default())
    });
    assert_eq!(stop, Some(Stop::Untranslated("$f00000".into())));
    assert_eq!(returns, [(0, 0x30), (0, 0x10), (0, 0x30), (0, 0x20)]);
}

/// A run started at line 200 has a first frame of the 113 lines left, then whole frames.
#[test]
fn a_run_started_at_a_line_has_a_short_first_frame() {
    let (mem, custom) = memory();
    let mut g = TranslatedGame::with_program(
        PROG,
        &[BASE],
        Start::Snapshot {
            mem: &mem,
            regs: regs(BASE, 0xFFFF, STACK),
            custom: &custom,
            disks: Disks::default(),
        },
    );
    g.start_at_line(200);
    let mut frames = 0;
    g.run(&mut |_| {
        frames += 1;
        if frames == 2 {
            Control::Stop
        } else {
            Control::Next(Input::default())
        }
    });
    let line = 227 * 2;
    let c = g.regs().cycles();
    assert!(
        c >= (113 + 313) * line && c < (114 + 313) * line,
        "{c} cycles, {} lines",
        c / line
    );
}

/// Power-on runs in user mode on `POWER_ON_STACK` with a supervisor stack for the first
/// interrupt: without one the 68000 pushes the exception frame below address 0 (translator T32,
/// `outside-memory: $fffffc` at play's first level 3 interrupt).
#[test]
fn power_on_has_a_supervisor_stack() {
    let g = TranslatedGame::with_program(
        Program::STUB,
        &[BASE],
        Start::PowerOn {
            program: &[],
            disks: Disks::default(),
        },
    );
    assert_eq!(ops::sr_get(g.regs()) & 0x2000, 0, "user mode");
    assert_eq!(g.regs().a(7), fd_translated::POWER_ON_STACK);
    assert_eq!(
        g.regs().get(ops::ISP, 4) as u32,
        fd_translated::POWER_ON_SUPERVISOR_STACK
    );
}

/// `shared_call` (`bsr $10; bsr $20; rts`, each callee `bsr $30` then an add) as a one-hunk
/// program file, assembled by hand from its listing.
fn shared_call_file() -> Vec<u8> {
    let mut code = [0u8; 0x40];
    for (at, bytes) in [
        (0x00, &[0x61, 0x00, 0x00, 0x0E][..]), // bsr.w $10
        (0x04, &[0x61, 0x00, 0x00, 0x1A]),      // bsr.w $20
        (0x08, &[0x4E, 0x75]),                  // rts
        (0x10, &[0x61, 0x00, 0x00, 0x1E]),      // bsr.w $30
        (0x14, &[0x52, 0x82]),                  // addq.l #1,d2
        (0x16, &[0x4E, 0x75]),
        (0x20, &[0x61, 0x00, 0x00, 0x0E]),      // bsr.w $30
        (0x24, &[0x54, 0x83]),                  // addq.l #2,d3
        (0x26, &[0x4E, 0x75]),
        (0x30, &[0x52, 0x81]),                  // addq.l #1,d1
        (0x32, &[0x4E, 0x75]),
    ] {
        code[at..at + bytes.len()].copy_from_slice(bytes);
    }
    let longs = (code.len() / 4) as u32;
    let mut f = Vec::new();
    // HUNK_HEADER, no names, one hunk (0 to 0) of `longs`; HUNK_CODE; HUNK_END.
    for l in [0x3F3, 0, 1, 0, 0, longs, 0x3E9, longs] {
        f.extend_from_slice(&u32::to_be_bytes(l));
    }
    f.extend_from_slice(&code);
    f.extend_from_slice(&u32::to_be_bytes(0x3F2));
    f
}

fn shared_call_prog() -> Program {
    Program {
        code: shared_call::CODE,
        hunk_sizes: shared_call::HUNK_SIZES,
        entry: shared_call::entry,
        resume: shared_call::resume,
    }
}

/// The program returning from its power-on entry, as the original's KONIEC returns to AmigaDOS
/// (translator T41): the run ends with no stop, its last frame marked `exited`.
#[test]
fn a_return_from_the_power_on_entry_ends_the_run_with_no_stop() {
    let file = shared_call_file();
    let mut g = TranslatedGame::with_program(
        shared_call_prog(),
        &[BASE],
        Start::PowerOn {
            program: &file,
            disks: Disks::default(),
        },
    );
    let mut last = Vec::new();
    let stop = g.run(&mut |out| {
        last.push((out.stop.clone(), out.exited));
        Control::Next(Input::default())
    });
    assert_eq!(stop, None, "no stop");
    assert!(g.exited());
    assert_eq!(last, [(None, true)], "one last frame, exited and not stopped");
    assert_eq!(
        (g.regs().d(1), g.regs().d(2), g.regs().d(3)),
        (2, 1, 2),
        "the program ran whole"
    );
    assert_eq!(g.regs().a(7), fd_translated::POWER_ON_STACK + 4);
    assert_eq!(g.run(&mut |_| Control::Stop), None, "an exited game stays ended");
}

/// The same return to address 0 with the same stack, from a snapshot rather than the power-on
/// entry, is not the program's exit: it stops `untranslated` as before.
#[test]
fn a_return_to_zero_not_from_the_power_on_entry_still_stops_untranslated() {
    let (mem, _) = memory();
    let sp = fd_translated::POWER_ON_STACK;
    let mut g = TranslatedGame::with_program(
        shared_call_prog(),
        &[BASE],
        Start::Snapshot {
            mem: &mem,
            regs: Regs68k {
                pc: BASE,
                sr: 0,
                usp: sp,
                ssp: STACK,
                a: [0, 0, 0, 0, 0, 0, 0, sp],
                ..Regs68k::default()
            },
            custom: &[0; 256],
            disks: Disks::default(),
        },
    );
    let mut exited = Vec::new();
    let stop = g.run(&mut |out| {
        exited.push(out.exited);
        Control::Next(Input::default())
    });
    assert_eq!(stop, Some(Stop::Untranslated("$000000".into())));
    assert!(!g.exited());
    assert_eq!(exited, [false]);
}

/// A program whose every address resumes in one routine that leaves for address 0 at the top
/// level, as an `rts` to the long under the power-on stack would, with a7 and the mode given.
macro_rules! to_zero {
    ($name:ident, $supervisor:expr, $a7:expr) => {
        fn $name(_: (u32, u32)) -> Option<fd_translated::Resume> {
            Some(|_, r, _, _| {
                if $supervisor {
                    ops::sr_set(r, 0x2000);
                }
                r.set_a(7, $a7);
                r.set(PC, 4, 0);
                Ok(())
            })
        }
    };
}
to_zero!(exit_exactly, false, fd_translated::POWER_ON_STACK + 4);
to_zero!(stack_unmoved, false, fd_translated::POWER_ON_STACK);
to_zero!(in_supervisor, true, fd_translated::POWER_ON_STACK + 4);

/// Only a return to the exit's address with the stack the entry's `rts` leaves (a7 =
/// `POWER_ON_STACK` + 4), in user mode, is the program's exit (DESIGN §2.10); the same address
/// with the stack unmoved, or in supervisor mode, stops `untranslated` (T41 review).
#[test]
fn only_the_entrys_own_return_to_zero_is_the_exit() {
    let file = shared_call_file();
    for (resume, exits) in [
        (exit_exactly as fn(_) -> _, true),
        (stack_unmoved, false),
        (in_supervisor, false),
    ] {
        let mut g = TranslatedGame::with_program(
            Program { resume, ..shared_call_prog() },
            &[BASE],
            Start::PowerOn {
                program: &file,
                disks: Disks::default(),
            },
        );
        let stop = g.run(&mut |_| Control::Next(Input::default()));
        let want = (!exits).then(|| Stop::Untranslated("$000000".into()));
        assert_eq!((stop, g.exited()), (want, exits), "exits: {exits}");
    }
}

/// From a snapshot the program's exit is the one the case runner reads off the original
/// (`TranslatedGame::exit_to`, title T02): the same return to 0 as above ends the run with no stop
/// once it is given with the stack its `rts` leaves, and still stops `untranslated` given another.
#[test]
fn a_snapshot_given_the_entrys_return_exits_there_and_only_there() {
    let (mem, _) = memory();
    let sp = fd_translated::POWER_ON_STACK;
    for (exit_sp, exits) in [(sp + 4, true), (sp, false)] {
        let mut g = TranslatedGame::with_program(
            shared_call_prog(),
            &[BASE],
            Start::Snapshot {
                mem: &mem,
                regs: Regs68k {
                    pc: BASE,
                    sr: 0,
                    usp: sp,
                    ssp: STACK,
                    a: [0, 0, 0, 0, 0, 0, 0, sp],
                    ..Regs68k::default()
                },
                custom: &[0; 256],
                disks: Disks::default(),
            },
        );
        g.exit_to(0, exit_sp);
        let mut last = Vec::new();
        let stop = g.run(&mut |out| {
            last.push((out.stop.is_some(), out.exited));
            Control::Next(Input::default())
        });
        let want = (!exits).then(|| Stop::Untranslated("$000000".into()));
        assert_eq!((stop, g.exited()), (want, exits), "exit stack {exit_sp:#x}");
        assert_eq!(last, [(!exits, exits)], "exit stack {exit_sp:#x}");
    }
}

/// A case's `enters` goal (title T02): the handler `h0+$40`, entered once a frame, is reported in
/// each frame; the foreground, resumed from the snapshot's PC and never entered through its entry,
/// is not; nothing is reported unwatched.
#[test]
fn watched_entries_are_reported_in_their_frame() {
    let (mem, custom) = memory();
    for watch in [&[(0u8, 0x40u32), (0, 0x0)][..], &[]] {
        let mut g = TranslatedGame::with_program(
            PROG,
            &[BASE],
            Start::Snapshot {
                mem: &mem,
                regs: regs(BASE, 0xFFFF, STACK),
                custom: &custom,
                disks: Disks::default(),
            },
        );
        g.watch_entries(watch);
        let mut seen = Vec::new();
        g.run(&mut |out| {
            seen.push(out.entries.clone());
            if seen.len() == 3 {
                Control::Stop
            } else {
                Control::Next(Input::default())
            }
        });
        let want = if watch.is_empty() { vec![] } else { vec![(0, 0x40)] };
        assert_eq!(seen, vec![want; 3], "watching {watch:?}");
    }
}
