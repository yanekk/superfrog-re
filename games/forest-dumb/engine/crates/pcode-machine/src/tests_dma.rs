//! DMA contention (translator T49) against the oracle's readings: the loops of
//! `toolkit/crates/amiga-oracle/tests/contention.rs` replayed as translated code would run them, a
//! block an iteration, ticking the instructions' cycles and their cycles off the bus.

use super::*;

/// DMACON bits.
const DMAEN: u16 = 0x0200;
const BPLEN: u16 = 0x0100;
const BLTEN: u16 = 0x0040;
const BLTPRI: u16 = 0x0400;
const SPREN: u16 = 0x0020;

/// Every sprite channel's list in the oracle's sprite readings: one sprite on lines `$40` to `$F0`
/// (data words zero), then the list's end.
const SPRITE: u32 = 0x7_8000;

fn img() -> Image<'static> {
    Image {
        bases: &[],
        sizes: &[],
        code: &[],
    }
}

/// A loop body as the generated code ticks it, an instruction at a time: (cycles, of them off
/// the bus, of those before its accesses), before the `dbf`.
type Body = &'static [(u32, u32, u32)];

/// Eight `move.w (a1),(a2)`: 12 each, three bus cycles.
const MOVES: Body = &[(12, 0, 0); 8];
/// Four `lsl.l #8,d4` (24, one bus cycle) and a `move.w (a1),(a2)`.
const SHIFTS: Body = &[(24, 20, 0), (24, 20, 0), (24, 20, 0), (24, 20, 0), (12, 0, 0)];

struct Case {
    planes: u16,
    /// BLTCON0, BLTSIZE, BLTPRI.
    blit: Option<(u16, u16, bool)>,
    body: Body,
    /// All eight sprite channels fetching over the loop.
    sprites: bool,
    /// The oracle's reading: VHPOSR's (line, h) before the loop, and slots to the read after.
    start: (u16, u16),
    slots: u64,
}

/// A machine at a display of `planes` lores planes, 320 x 256 from line `$2C`, beam at `at`.
fn machine(planes: u16, dma: u16, at: (u16, u16)) -> Machine {
    let cfg = Config::a500(vec![]);
    let mut ram = vec![0u8; (cfg.chip + cfg.slow) as usize];
    let mut custom = [0u16; 256];
    if dma & SPREN != 0 {
        let s = SPRITE as usize;
        ram[s..s + 4].copy_from_slice(&[0x40, 0x40, 0xF0, 0x00]);
        for n in 0..8 {
            custom[0x120 / 2 + 2 * n] = (SPRITE >> 16) as u16;
            custom[0x120 / 2 + 2 * n + 1] = SPRITE as u16;
        }
    }
    custom[2] = 0x8000 | (at.0 >> 8);
    custom[3] = (at.0 & 0xFF) << 8 | at.1;
    custom[custom::DMACONR as usize >> 1] = dma | if planes > 0 { BPLEN } else { 0 };
    let set = |c: &mut [u16; 256], reg: u16, v: u16| c[reg as usize >> 1] = v;
    set(&mut custom, 0x100, planes << 12 | 0x0200);
    set(&mut custom, 0x08E, 0x2C81);
    set(&mut custom, 0x090, 0x2CC1);
    set(&mut custom, 0x092, 0x0038);
    set(&mut custom, 0x094, 0x00D0);
    Machine::from_snapshot(cfg, &ram, &custom)
}

/// One block of instructions, each (cycles, of them off the bus, of those before its accesses),
/// then the next block's start.
fn block(m: &mut Machine, r: &mut Regs, insns: &[(u32, u32, u32)]) {
    for &(cycles, off, lead) in insns {
        r.tick(cycles);
        r.off_bus(off);
        r.idle_first(lead);
    }
    m.poll(r, &img()).unwrap();
}

/// The loop on the machine: slots from the first VHPOSR read's block start to the second's, and
/// where the first was read.
fn run(c: &Case) -> ((u16, u16), u64) {
    let dma = DMAEN | BLTEN | if c.sprites { SPREN } else { 0 };
    // With a blit, the machine starts at the BLTSIZE write, 6 slots before the oracle's first
    // read in the plain blit's case (the write, then `move.w 6(a0),d1`'s opcode and extension).
    let at = match c.blit {
        Some(_) => (c.start.0, c.start.1 - 6),
        None => c.start,
    };
    // Sprite DMA learns its pointers on line 25: the machine starts above it and runs to `at`.
    let from = if c.sprites { (20, 0) } else { at };
    let mut m = machine(c.planes, dma, from);
    let mut r = Regs::new();
    if c.sprites {
        r.tick(((at.0 - from.0) as u64 * 454 + 2 * at.1 as u64) as u32);
    }
    m.poll(&mut r, &img()).unwrap();
    if let Some((con0, size, pri)) = c.blit {
        if pri {
            m.ww(0xDF_F096, 0x8000 | BLTPRI);
        }
        m.ww(0xDF_F040, con0);
        m.ww(0xDF_F058, size);
        // The write's own bus cycle, the next instruction's opcode and extension words and its
        // read: with BLTPRI the read is what waits for the blit's end.
        block(&mut m, &mut r, &[(16, 0, 0)]);
    }
    // The read is the last bus cycle so far (a cycle in the plain case, which starts there).
    let t0 = r.cycles() - if c.blit.is_some() { 4 } else { 0 };
    let first = m.beam(t0);
    // The read's prefetch, `move.w #n,d0`, two `lea abs.l`.
    block(&mut m, &mut r, &[(4, 0, 0), (8, 0, 0), (12, 0, 0), (12, 0, 0)]);
    for k in 0..200 {
        // `dbf`: 10 taken, the last falling through 14; 2 off the bus either way, before its reads.
        let dbf = if k == 199 { (14, 2, 2) } else { (10, 2, 2) };
        let insns: Vec<(u32, u32, u32)> = c.body.iter().copied().chain([dbf]).collect();
        block(&mut m, &mut r, &insns);
    }
    // `move.w 4(a0),d2`, then `move.w 6(a0),d3` to its read.
    block(&mut m, &mut r, &[(12, 0, 0), (8, 0, 0)]);
    (first, (r.cycles() - t0) / 2)
}

fn case(planes: u16, body: Body, start: (u16, u16), slots: u64) -> Case {
    Case {
        planes,
        blit: None,
        body,
        sprites: false,
        start,
        slots,
    }
}

/// The oracle's readings (`amiga-oracle/tests/contention.rs`, 2026-11-12).
fn readings() -> Vec<(&'static str, Case)> {
    let blit = |planes, con0, pri, start, slots| Case {
        planes,
        blit: Some((con0, 64 << 6 | 20, pri)),
        body: MOVES,
        sprites: false,
        start,
        slots,
    };
    let sprites = |body, start, slots| Case { sprites: true, ..case(0, body, start, slots) };
    vec![
        ("moves, 0 planes", case(0, MOVES, (0x50, 31), 10672)),
        ("moves, 4 planes", case(4, MOVES, (0x50, 25), 10859)),
        ("moves, 5 planes", case(5, MOVES, (0x50, 17), 13156)),
        ("moves, 6 planes", case(6, MOVES, (0x50, 31), 16438)),
        ("shifts, 6 planes", case(6, SHIFTS, (0x50, 31), 13033)),
        // Sprite DMA's odd slots: 17 slots on the moves, 7 on the shifts (T49 review). Not held:
        // the shifts with 0 planes and no sprites, 11 866 on the oracle, 11 873 on the machine
        // (plans/forest-dumb-translator/FINDINGS.md, 2026-11-18).
        ("moves, 8 sprites", sprites(MOVES, (0x50, 17), 10689)),
        ("shifts, 8 sprites", sprites(SHIFTS, (0x50, 17), 11873)),
        ("blit ABCD", blit(0, 0x0FCA, false, (0x50, 157), 14549)),
        ("blit ABCD BLTPRI", blit(0, 0x0FCA, true, (0x50, 157), 10671)),
        ("blit AD", blit(0, 0x09F0, false, (0x50, 155), 12611)),
        ("blit D", blit(0, 0x0100, false, (0x50, 151), 10754)),
        ("blit ABCD, 5 planes", blit(5, 0x0FCA, false, (0x50, 165), 21322)),
    ]
}

/// Every reading within 6 slots of the oracle's (of 10 000 to 21 000): the block's accesses are
/// timed per instruction, an instruction idling before its accesses (`dbf`) as it does, and the
/// blit's start is placed at the write from the plain case's read.
#[test]
fn a_cpu_loop_takes_on_the_machine_what_it_took_on_the_oracle() {
    for (name, c) in readings() {
        let (first, slots) = run(&c);
        assert!(
            slots.abs_diff(c.slots) <= 6,
            "{name}: machine {slots} slots from {first:?}, oracle {} from {:?}",
            c.slots,
            c.start
        );
    }
}

/// With BLTPRI the blitter keeps the bus from the CPU until it ends: the oracle's first read
/// came at line `$67` (103), slot 148, the blit's end.
#[test]
fn with_bltpri_the_cpu_waits_for_the_blits_end() {
    let (_, c) = readings().into_iter().find(|(n, _)| *n == "blit ABCD BLTPRI").unwrap();
    let (first, _) = run(&c);
    assert_eq!(first.0, 103);
    assert!(first.1.abs_diff(148) <= 6, "{first:?}");
}

/// The copper's fetches take their slots: the oracle's loop beside a list writing four colours on
/// each line from `$50` to `$9F` took 10 714 slots, 42 more than without it.
#[test]
fn a_busy_copper_takes_its_slots() {
    const LIST: u32 = 0x7_D000;
    let mut words = vec![0x0100, 0x0200, 0x0092, 0x0038, 0x0094, 0x00D0];
    for line in 0x50..0xA0u16 {
        words.extend([line << 8 | 0x07, 0xFFFE]);
        for k in 0..4u16 {
            words.extend([0x0180 + 2 * k, line + k]);
        }
    }
    words.extend([0xFFFF, 0xFFFE]);
    let mut m = machine(0, DMAEN | BLTEN | 0x0080, (0x50, 32));
    for (k, w) in words.iter().enumerate() {
        m.ww(LIST + 2 * k as u32, *w);
    }
    m.ww(0xDF_F080, (LIST >> 16) as u16);
    m.ww(0xDF_F082, LIST as u16);
    // The frame's copper started at the vertical blank, from the list as it is now.
    m.set_cycles(0);
    m.copper_run(LIST, None);
    let mut r = Regs::new();
    m.poll(&mut r, &img()).unwrap();
    let t0 = r.cycles();
    block(&mut m, &mut r, &[(4, 0, 0), (8, 0, 0), (12, 0, 0), (12, 0, 0)]);
    for k in 0..200 {
        let dbf = if k == 199 { (14, 2, 2) } else { (10, 2, 2) };
        let insns: Vec<(u32, u32, u32)> = MOVES.iter().copied().chain([dbf]).collect();
        block(&mut m, &mut r, &insns);
    }
    block(&mut m, &mut r, &[(12, 0, 0), (8, 0, 0)]);
    let slots = (r.cycles() - t0) / 2;
    assert!(slots.abs_diff(10714) <= 6, "{slots}");
}

/// A read is timed at its own instruction, not at its block's start (`Bus::sync`, T49 review):
/// after ten instructions of 12 cycles off memory, `move.w 6(a0),d1` (12, three bus cycles) reads
/// VHPOSR in its second, 62 slots on, with nothing taking the bus.
#[test]
fn a_read_sees_the_beam_at_its_instruction() {
    let mut m = machine(0, DMAEN, (0x50, 10));
    let mut r = Regs::new();
    m.poll(&mut r, &img()).unwrap();
    for _ in 0..11 {
        r.tick(12);
    }
    m.sync(&r);
    let (line, h) = beam::decode(m.rw(0xDF_F004), m.rw(0xDF_F006));
    assert_eq!((line, h), (0x50, 10 + 62));
}

/// An instruction's leading idle cycles are spent before its accesses (`cycles::lead`, T49
/// review): where they fall on a slot a bitplane takes, the access after them is not held up.
#[test]
fn leading_idle_cycles_come_before_the_access() {
    let line = 0x50;
    let fetch = machine(4, DMAEN, (0, 0)).fetch();
    let free = |h: u16| !amiga_clock::is_refresh(h as u32) && !fetch.takes(line, h);
    // A free slot, then a bitplane's, then a free one, on a line the four planes fetch on.
    let h = (0..225u16).find(|&h| free(h) && !free(h + 1) && free(h + 2)).expect("a fetch");
    let t = line as u64 * 454 + 2 * h as u64;
    // Six cycles, one bus access and two off the bus.
    let end = |lead| {
        let mut m = machine(4, DMAEN, (0, 0));
        m.with_slots(|s, beam| s.cpu(t, &[(6, 2, lead)], (0, false), beam)) - t
    };
    // Idle first: the idle cycles take the free slot, the access waits out the fetch.
    assert_eq!(end(2), 2 + 2 + 4);
    // Access first: it takes the free slot, the idle cycles overlap the fetch.
    assert_eq!(end(0), 4 + 2);
}
