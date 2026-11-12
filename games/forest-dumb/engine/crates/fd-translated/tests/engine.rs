//! `TranslatedGame` on the stub and on a made-up program (translator T28): no disks, no tree.
//!
//! The made-up program is pcode-machine's interrupt program (`tests/fixtures/irq.json`, translated by
//! `tools/pcode_rust.py` and kept current by `tools/test_pcode_rust.py`): `h0+$0` loops until the
//! word at `$100` equals d2, then returns; `h0+$40`, the level 3 handler, adds one to `$100` and
//! clears VERTB. A copper list whose first MOVE sets VERTB in INTREQ requests the handler once a
//! frame, at the frame's start.

use fd_translated::{Control, FrameOut, Program, Regs68k, Start, TranslatedGame};
use pcode_machine::{Config, Disks, Input, irq};
use pcode_rt::Stop;

#[rustfmt::skip]
#[path = "../../pcode-machine/tests/fixtures/irq.rs"]
mod prog;

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
    assert_eq!(g.run(&mut |_| Control::Stop), stop, "a stopped game stays stopped");
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
        assert!(out.writes.iter().any(|w| w.1 == irq::INTREQ && w.2 == 0x8020));
        if counts.len() == 5 {
            Control::Stop
        } else {
            Control::Next(Input::default())
        }
    });
    assert_eq!(stop, None, "each said stop");
    assert_eq!(counts, [1, 2, 3, 4, 5], "one interrupt a frame, every frame seen");
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
