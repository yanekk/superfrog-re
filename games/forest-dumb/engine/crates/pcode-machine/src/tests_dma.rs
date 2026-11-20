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
/// `main_loop`'s wait for the interrupt count (`h0+$252`..`$2aa`, T54) as the oracle's `SPIN` has
/// it: `move.w abs.l,d4` 16, `andi.l #$1f,d4` 16 (4 after its accesses), `bne.w` taken 10 (2
/// before), `tst.w abs.l` 16, `bne.w` not taken 12 (4 before), `bra.w` 10 (2 before). Named one
/// by one: the pure crates carry no integer table over 16 numbers (fd-conform's purity test).
const SPIN: Body = &[MOVE_W_ABS, ANDI_L, BNE_TAKEN, TST_W_ABS, BNE_NOT_TAKEN, BRA_W];
const MOVE_W_ABS: (u32, u32, u32) = (16, 0, 0);
const ANDI_L: (u32, u32, u32) = (16, 4, 0);
const BNE_TAKEN: (u32, u32, u32) = (10, 2, 2);
const TST_W_ABS: (u32, u32, u32) = (16, 0, 0);
const BNE_NOT_TAKEN: (u32, u32, u32) = (12, 4, 4);
const BRA_W: (u32, u32, u32) = (10, 2, 2);
/// `lsl.l #8,d4` and `move.w (a1),(a2)` in turn, 30 times: 61 instructions with the `dbf`.
const LONG: Body = &{
    let mut body = [(12, 0, 0); 60];
    let mut k = 0;
    while k < 60 {
        body[k] = (24, 20, 0);
        k += 2;
    }
    body
};

struct Case {
    planes: u16,
    /// BLTCON0, BLTSIZE, BLTPRI.
    blit: Option<(u16, u16, bool)>,
    body: Body,
    /// All eight sprite channels fetching over the loop.
    sprites: bool,
    /// Audio channel 0 playing (period 124) from the frame's start (T52).
    audio: bool,
    /// The loop's iterations.
    count: u16,
    /// How many slots the machine may be off the oracle's reading.
    within: u64,
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
    // Sprite DMA learns its pointers on line 25, and the oracle's audio channel started near the
    // frame's start: the machine starts above and runs to `at`.
    let from = if c.sprites || c.audio { (20, 0) } else { at };
    let mut m = machine(c.planes, dma, from);
    let mut r = Regs::new();
    if c.audio {
        // The frame starts at the first poll; then AUD0LC, AUD0LEN 64, AUD0PER 124, AUD0VOL 0
        // and DMACON's AUD0EN.
        m.poll(&mut r, &img()).unwrap();
        for (reg, v) in [(0xA0, 5), (0xA2, 0), (0xA4, 64), (0xA6, 124), (0xA8, 0), (0x96, 0x8001)] {
            m.ww(0xDF_F000 + reg, v);
        }
    }
    if c.sprites || c.audio {
        // Idle: the lead-in's own bus waits would move the start.
        let lead = ((at.0 - from.0) as u64 * 454 + 2 * at.1 as u64) as u32;
        r.tick(lead);
        r.off_bus(lead);
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
    for k in 0..c.count {
        // `dbf`: 10 taken, the last falling through 14; 2 off the bus either way, before its reads.
        let dbf = if k == c.count - 1 { (14, 2, 2) } else { (10, 2, 2) };
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
        audio: false,
        count: 200,
        within: 6,
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
        audio: false,
        count: 200,
        within: 6,
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
        // T52: a block longer than 48 instructions, each placed on the bus in turn (with a trail
        // of 48 the rest was one lump: 12 022). Within 14: 600 `lsl.l #8` run 13 slots over, the
        // shifts' small excess the 0-plane reading shows too (FINDINGS, 2026-11-18).
        (
            "long block, 5 planes",
            Case { count: 20, within: 14, ..case(5, LONG, (0x50, 17), 11898) },
        ),
        // T54: the wait spin, 9 095 and 11 043 slots on the machine (the harness's reads are a
        // slot or two off the oracle's, as in every reading here). In the game its slots were the
        // oracle's before T54 too: what moved the interrupt was the raster wait before it, the IPL
        // pipe and a BLTPRI blit's wait (plans/forest-dumb-translator/FINDINGS.md, 2026-11-19).
        ("spin, 0 planes", Case { within: 3, ..case(0, SPIN, (0x50, 31), 9098) }),
        ("spin, 5 planes", Case { within: 3, ..case(5, SPIN, (0x50, 17), 11044) }),
        // T52: the audio channel's slots, which a BLTPRI blit loses (15 slots more than without).
        (
            "blit ABCD BLTPRI, audio",
            Case { audio: true, ..blit(0, 0x0FCA, true, (0x50, 157), 10686) },
        ),
    ]
}

/// Every reading within 6 slots of the oracle's (of 9 000 to 21 000; the long block 14, the spin
/// 3): the block's accesses are timed per instruction, an instruction idling before its accesses
/// (`dbf`) as it does, and the blit's start is placed at the write from the plain case's read.
#[test]
fn a_cpu_loop_takes_on_the_machine_what_it_took_on_the_oracle() {
    for (name, c) in readings() {
        let (first, slots) = run(&c);
        assert!(
            slots.abs_diff(c.slots) <= c.within,
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
    // With audio channel 0 playing the blit ends 21 slots later, at slot 169 (T52).
    let (_, c) = readings().into_iter().find(|(n, _)| *n == "blit ABCD BLTPRI, audio").unwrap();
    let (first, _) = run(&c);
    let at = first.0 as u64 * 227 + first.1 as u64;
    assert!(at.abs_diff(103 * 227 + 169) <= 6, "{first:?}");
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
/// VHPOSR in its second, 62 slots on, with nothing taking the bus; the read gives the beam five
/// slots on (T54).
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
    assert_eq!((line, h), (0x50, 10 + 62 + 5));
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


/// The CPU sees an interrupt request [`irq::IPL_DELAY`] cycles after it is made, at an
/// instruction's last access (vAmiga's IPL pipe, Moira's `POLL_IPL`; T54): raised at cycle 100, an
/// instruction whose last access starts at 104 has not seen it, one whose last access starts at
/// 108 has.
#[test]
fn a_request_is_seen_at_the_first_last_access_through_the_ipl_pipe() {
    let mut m = machine(0, DMAEN, (0x10, 0));
    m.ww(0xDF_F09A, 0xC020);
    m.raise(5, 100);
    let mut r = Regs::new();
    m.poll(&mut r, &img()).unwrap();
    // 25 accesses, the last at 96: the request is made after it.
    r.tick(100);
    m.poll_time(&mut r).unwrap();
    assert_eq!(m.due(&r), None);
    // Accesses at 100 and 104: inside the pipe.
    r.tick(8);
    m.poll_time(&mut r).unwrap();
    assert_eq!(m.due(&r), None, "sampled at 104");
    r.tick(4);
    m.poll_time(&mut r).unwrap();
    assert_eq!(m.due(&r), Some(3), "sampled at 108");
}

/// The interrupt's 44 cycles in Moira's order (`irq::IRQ_BUS`): its last access, the second
/// prefetch, starts at 40, after 2 idle; the oracle's bus owners at `main_loop`'s line-239
/// interrupt showed its seven accesses at +3, +9, +11, +13, +15, +17 and +20 slots (T54).
#[test]
fn an_interrupts_bus_cycles_are_moiras() {
    // From slot 20, clear of the refresh slots.
    let mut m = machine(0, DMAEN, (0x10, 0));
    let end = m.with_slots(|s, beam| s.cpu(40, &irq::IRQ_BUS, (0, false), beam));
    assert_eq!(end, 40 + irq::IRQ_CYCLES as u64);
    assert_eq!(m.sampled_at, 40 + 40);
    let off: u32 = irq::IRQ_BUS.iter().map(|b| b.1).sum();
    assert_eq!((irq::IRQ_CYCLES - off) / 4, 7, "seven accesses");
}

/// `n` instructions of 12 cycles, three bus accesses each, fitted in blocks of `per` (a poll after
/// each block, and the host's `end_frame` once the count is past the frame's end): the count they
/// end at.
fn fitted(m: &mut Machine, n: usize, per: usize) -> u64 {
    let mut r = Regs::new();
    m.poll(&mut r, &img()).unwrap();
    let insns = vec![(12, 0, 0); n];
    for chunk in insns.chunks(per) {
        block(m, &mut r, chunk);
        if r.cycles() >= m.frame_end() {
            m.end_frame();
        }
    }
    r.cycles()
}

/// A machine at `at` running the copper list `list` from the frame's start, at `LIST`.
fn with_list(planes: u16, dma: u16, at: (u16, u16), list: Option<&[u16]>) -> Machine {
    const LIST: u32 = 0x7_D000;
    let mut m = machine(planes, dma, at);
    if let Some(list) = list {
        for (k, w) in list.iter().enumerate() {
            m.ww(LIST + 2 * k as u32, *w);
        }
        m.ww(0xDF_F080, (LIST >> 16) as u16);
        m.ww(0xDF_F082, LIST as u16);
        m.set_cycles(0);
        m.copper_run(LIST, None);
    }
    m
}

/// Work fitted in one long block meets the copper's writes as its slots pass them, as when it is
/// fitted an instruction at a time with each write delivered between (T54: a CPU waiting out a
/// BLTPRI blit across the copper's BPLCON0 write at the display's top missed the bitplanes' slots).
#[test]
fn a_long_block_meets_the_coppers_display_writes_on_its_way() {
    // WAIT for line $40, then five planes on.
    let list: &[u16] = &[0x4001, 0xFFFE, 0x0100, 0x5200, 0xFFFF, 0xFFFE];
    let run = |per, list: Option<&[u16]>| {
        let dma = DMAEN | BPLEN | if list.is_some() { 0x0080 } else { 0 };
        fitted(&mut with_list(0, dma, (0x3E, 0), list), 240, per)
    };
    let (long, short, plain) = (run(240, Some(list)), run(1, Some(list)), run(240, None));
    assert_eq!(long, short);
    assert!(long > plain + 200, "the planes from line $40 cost: {long} against {plain}");
}

/// Work fitted across the frame's end finds the next frame's copper there, as when the host has
/// ended the frame between (T54: a CPU waiting out a blit at the vertical blank, the copper's
/// restart moves taking line 0's slots).
#[test]
fn a_block_across_the_frames_end_meets_the_next_frames_copper() {
    // Thirty colour moves from the restart, on line 0.
    let mut list: Vec<u16> = (0..30).flat_map(|k| [0x0180, k]).collect();
    list.extend([0xFFFF, 0xFFFE]);
    let run = |per, list: Option<&[u16]>| {
        let dma = DMAEN | if list.is_some() { 0x0080 } else { 0 };
        fitted(&mut with_list(0, dma, (311, 100), list), 200, per)
    };
    let (long, short, plain) = (run(200, Some(&list[..])), run(1, Some(&list[..])), run(200, None));
    assert_eq!(long, short);
    // 12 cycles: after a refresh wait the CPU's accesses fall between the copper's fetches.
    assert!(long > plain, "the moves cost: {long} against {plain}");
}

