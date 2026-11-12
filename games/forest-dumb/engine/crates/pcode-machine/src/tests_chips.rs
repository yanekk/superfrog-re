//! The blitter, the copper and the picture on the machine (translator T24), on made-up memory.

use amiga_chips::copper::{self, Fetch, Write as CopWrite};
use amiga_chips::{AmigaMemory, Blit, BlitMem, display, word_times};

use super::*;

const FRAME: u64 = 313 * 454;

/// DMACON bits.
const DMAEN: u16 = 0x0200;
const BPLEN: u16 = 0x0100;
const COPEN: u16 = 0x0080;
const BLTEN: u16 = 0x0040;
const SPREN: u16 = 0x0020;

fn img() -> Image<'static> {
    Image {
        bases: &[],
        sizes: &[],
        code: &[],
    }
}

/// A machine from a snapshot of zeroed RAM at line 0's start, DMACON `dma`, the copper's list at
/// `cop1lc`, and `mem` written into chip RAM.
fn machine(dma: u16, cop1lc: u32, mem: &[(u32, &[u16])]) -> Machine {
    let cfg = Config::a500(vec![]);
    let mut ram = vec![0u8; (cfg.chip + cfg.slow) as usize];
    for &(at, words) in mem {
        for (k, w) in words.iter().enumerate() {
            let a = at as usize + 2 * k;
            ram[a..a + 2].copy_from_slice(&w.to_be_bytes());
        }
    }
    let mut custom = [0u16; 256];
    custom[2] = 0x8000; // VPOSR: line 0, long frame
    custom[custom::DMACONR as usize >> 1] = dma;
    custom[crate::copper::COP1LC as usize >> 1] = (cop1lc >> 16) as u16;
    custom[(crate::copper::COP1LC as usize >> 1) + 1] = cop1lc as u16;
    Machine::from_snapshot(cfg, &ram, &custom)
}

/// Bring the machine's time to `n` cycles, as a block start does.
fn at(m: &mut Machine, n: u64) -> Result<(), Stop> {
    let mut r = Regs::new();
    r.tick(n as u32);
    m.poll(&mut r, &img())
}

fn put(m: &mut Machine, reg: u16, v: u16) {
    m.ww(0xDF_F000 | reg as u32, v);
}

fn get(m: &mut Machine, reg: u16) -> u16 {
    m.rw(0xDF_F000 | reg as u32)
}

fn put_ptr(m: &mut Machine, reg: u16, p: u32) {
    put(m, reg, (p >> 16) as u16);
    put(m, reg + 2, p as u16);
}

/// Chip RAM as a flat array, for the blit run directly.
struct Flat(Vec<u8>);
impl BlitMem for Flat {
    fn blit_w(&self, a: u32) -> u16 {
        u16::from_be_bytes([self.0[a as usize], self.0[a as usize + 1]])
    }
    fn blit_set_w(&mut self, a: u32, v: u16) {
        self.0[a as usize..a as usize + 2].copy_from_slice(&v.to_be_bytes());
    }
}

/// Made-up words: every value different, nothing all zero.
fn pattern(seed: u16, n: usize) -> Vec<u16> {
    (0..n as u16)
        .map(|k| seed.wrapping_mul(31).wrapping_add(k.wrapping_mul(0x9E37)) ^ 0x5A5A)
        .collect()
}

#[test]
fn an_area_blit_through_the_registers_is_blit_run_directly() {
    use blitter::*;
    let (a, b, c, d) = (0x1000u32, 0x2000u32, 0x3000u32, 0x4000u32);
    for (con0, con1, adat) in [
        // A, B, C to D, cookie cut, both shifted by 4.
        (0x4FCA, 0x4000, 0),
        // Descending, A off: A is BLTADAT through the masks.
        (0x0BCA, 0x0002, 0xF0F0),
    ] {
        let words = [
            (a, pattern(1, 64)),
            (b, pattern(2, 64)),
            (c, pattern(3, 64)),
            (d, pattern(4, 64)),
        ];
        let mem: Vec<(u32, &[u16])> = words.iter().map(|(at, w)| (*at, &w[..])).collect();
        let mut m = machine(DMAEN | BLTEN, 0, &mem);
        let mut flat = Flat(m.chip().to_vec());
        at(&mut m, 1000).unwrap();
        let desc = con1 & 2 != 0;
        // A descending blit starts at its blocks' last word.
        let last = |p: u32| if desc { p + 6 * 12 - 4 - 2 } else { p };
        let blit = Blit {
            con0,
            con1,
            afwm: 0x7FFF,
            alwm: 0xFFF0,
            pt: [last(a), last(b), last(c), last(d)],
            modulo: [4, 4, 4, 4],
            size: 6 << 6 | 4, // 6 rows of 4 words
        };
        for (reg, v) in [
            (BLTCON0, blit.con0),
            (BLTCON1, blit.con1),
            (BLTAFWM, blit.afwm),
            (BLTALWM, blit.alwm),
            (BLTAMOD, 4),
            (BLTBMOD, 4),
            (BLTCMOD, 4),
            (BLTDMOD, 4),
            (BLTADAT, adat),
        ] {
            put(&mut m, reg, v);
        }
        for (reg, p) in [BLTAPT, BLTBPT, BLTCPT, BLTDPT].into_iter().zip(blit.pt) {
            put_ptr(&mut m, reg, p);
        }
        put(&mut m, BLTSIZE, blit.size);
        let end = blit.run_with([adat, 0, 0], &mut flat);
        assert_eq!(m.chip(), &flat.0[..], "BLTCON0 {con0:04x}");
        // The pointers are where the blit left them.
        let custom = *m.custom();
        let ptr = |reg: u16| {
            (custom[reg as usize >> 1] as u32) << 16 | custom[(reg as usize >> 1) + 1] as u32
        };
        assert_eq!([BLTAPT, BLTBPT, BLTCPT, BLTDPT].map(ptr), end.pt);

        // Busy until word_times' last word: 24 words from the beam at cycle 1000, no bitplanes.
        let (line, h) = beam::position(1000, 313);
        let times = word_times(24, line, h, 313, |_, _| false);
        let (ll, lh) = *times.last().unwrap();
        let done = beam::offset(ll, lh) + 2;
        assert!(done > 1000 + 24 * 4 - 8, "two slots a word: {done}");
        assert_ne!(
            get(&mut m, custom::DMACONR) & 0x4000,
            0,
            "busy at the write"
        );
        at(&mut m, done - 1).unwrap();
        assert_ne!(
            get(&mut m, custom::DMACONR) & 0x4000,
            0,
            "busy a cycle before"
        );
        assert_eq!(get(&mut m, irq::INTREQR) & INT_BLIT, 0);
        at(&mut m, done).unwrap();
        let dmaconr = get(&mut m, custom::DMACONR);
        assert_eq!(dmaconr & 0x4000, 0, "done");
        assert_eq!(dmaconr & 0x2000 != 0, end.zero, "BZERO");
        assert_eq!(dmaconr & 0x07FF, DMAEN | BLTEN);
        assert_eq!(
            get(&mut m, irq::INTREQR) & INT_BLIT,
            INT_BLIT,
            "the blitter's interrupt"
        );
        assert_eq!(m.fault(), None);
    }
}

#[test]
fn line_and_fill_blits_and_a_blit_with_its_dma_off_stop_the_run() {
    for (dma, con1, op) in [
        (DMAEN | BLTEN, 0x0001, "blit line"),
        (DMAEN | BLTEN, 0x0008, "blit fill"),
        (DMAEN | BLTEN, 0x0010, "blit fill"),
        (DMAEN, 0x0000, "blit with blitter DMA off"),
    ] {
        let mut m = machine(dma, 0, &[]);
        at(&mut m, 500).unwrap();
        put(&mut m, blitter::BLTCON0, 0x09F0);
        put(&mut m, blitter::BLTCON1, con1);
        put(&mut m, blitter::BLTSIZE, 0x0041);
        let stop = at(&mut m, 600).unwrap_err();
        assert_eq!(stop.to_string(), format!("unsupported: {op} at cycle 500"));
    }
}

/// A copper list: WAIT for line `line`, MOVE `reg` `value`, the end.
fn wait_move(line: u16, reg: u16, value: u16) -> Vec<u16> {
    vec![line << 8 | 1, 0xFFFE, reg, value, 0xFFFF, 0xFFFE]
}

/// The cycle the toolkit's copper makes the list's first write at, in a frame from cycle 0.
fn first_write_cycle(list: &[u16]) -> (u64, CopWrite) {
    let word = |a: u32| {
        list.get((a as usize - 0x400) / 2)
            .copied()
            .unwrap_or(0xFFFF)
    };
    let w = copper::run(&word, 0x400, Fetch::default(), 313)[0];
    (beam::offset(w.line, w.h), w)
}

#[test]
fn a_copper_move_to_intreq_raises_the_copper_interrupt_at_its_line() {
    let list = wait_move(0x40, irq::INTREQ, 0x8010);
    let mut m = machine(DMAEN | COPEN, 0x400, &[(0x400, &list)]);
    let (cyc, w) = first_write_cycle(&list);
    assert_eq!(w.line, 0x40);
    assert_eq!(m.frame_end(), FRAME);
    at(&mut m, cyc - 1).unwrap();
    assert_eq!(
        get(&mut m, irq::INTREQR) & crate::copper::INT_COPER,
        0,
        "not before its line"
    );
    at(&mut m, cyc).unwrap();
    assert_eq!(
        get(&mut m, irq::INTREQR) & crate::copper::INT_COPER,
        crate::copper::INT_COPER
    );
    // The handler clears it; the next frame's copper raises it again at the same place.
    put(&mut m, irq::INTREQ, 0x0010);
    assert_eq!(m.intreq() & crate::copper::INT_COPER, 0);
    at(&mut m, FRAME - 1).unwrap();
    let f = m.end_frame();
    assert!(
        f.writes.contains(&(cyc, irq::INTREQ, 0x8010)),
        "{:?}",
        f.writes
    );
    assert!(
        f.writes.contains(&(cyc, irq::INTREQ, 0x0010)),
        "the CPU's write, at its block"
    );
    at(&mut m, FRAME + cyc - 1).unwrap();
    assert_eq!(m.intreq() & crate::copper::INT_COPER, 0);
    at(&mut m, FRAME + cyc).unwrap();
    assert_eq!(
        m.intreq() & crate::copper::INT_COPER,
        crate::copper::INT_COPER
    );
    let f = m.end_frame();
    assert_eq!(f.writes, [(FRAME + cyc, irq::INTREQ, 0x8010)]);
}

/// The frame bar compares the CPU's custom writes with the oracle's, which keeps the CPU's alone
/// (translator T45): a copper MOVE to the same register as a CPU write is in `writes`, not in
/// `cpu_writes`, so the split is by who wrote, not by register.
#[test]
fn cpu_writes_hold_the_cpus_alone_not_the_coppers_to_the_same_register() {
    let colour = copper::reg::COLOR00;
    let list = wait_move(0x40, colour, 0x0111);
    let (cyc, _) = first_write_cycle(&list);
    let mut m = machine(DMAEN | COPEN, 0x400, &[(0x400, &list)]);
    at(&mut m, 0x10 * 454).unwrap();
    put(&mut m, colour, 0x0222);
    assert_eq!(m.cpu_writes(), [(0x10 * 454, colour, 0x0222)]);
    let f = m.end_frame();
    assert_eq!(
        f.writes,
        [(0x10 * 454, colour, 0x0222), (cyc, colour, 0x0111)]
    );
    assert_eq!(f.cpu_writes, [(0x10 * 454, colour, 0x0222)]);
    // Emptied with the frame, as `writes` is.
    assert!(m.cpu_writes().is_empty());
    let f = m.end_frame();
    assert_eq!(f.writes, [(FRAME + cyc, colour, 0x0111)]);
    assert!(f.cpu_writes.is_empty());
}

/// The frame runner takes state where the program marks its work's end, the CPU's write of
/// `INTREQ=$4010` (translator T30): the machine keeps RAM as it is at each such write, the CPU's
/// only, in order, and hands the copies with the frame.
#[test]
fn watch_write_keeps_ram_at_each_cpu_write_of_the_value_named() {
    let mark = (irq::INTREQ, 0x4010);
    let list = wait_move(0x40, irq::INTREQ, 0x4010);
    let mut m = machine(DMAEN | COPEN, 0x400, &[(0x400, &list)]);
    m.watch_write(Some(mark));
    m.ww(0x100, 0x1111);
    put(&mut m, irq::INTREQ, 0x4000);
    put(&mut m, irq::INTREQ, 0x4010);
    m.ww(0x100, 0x2222);
    put(&mut m, irq::INTREQ, 0x4010);
    m.ww(0x100, 0x3333);
    let f = m.end_frame();
    let kept: Vec<u16> = f
        .watched
        .iter()
        .map(|mem| u16::from_be_bytes([mem[0x100], mem[0x101]]))
        .collect();
    // Not the other INTREQ value, and not the copper's MOVE of the same value.
    assert_eq!(kept, [0x1111, 0x2222]);
    assert_eq!(f.watched[0].len(), (m.chip().len() + m.slow().len()));
    assert!(m.take_watched().is_empty(), "emptied with the frame");
    m.watch_write(None);
    put(&mut m, irq::INTREQ, 0x4010);
    assert!(m.take_watched().is_empty(), "nothing kept unwatched");
}

#[test]
fn a_list_word_the_cpu_rewrites_before_the_copper_reads_it_is_read_new() {
    let colour = copper::reg::COLOR00;
    let list = wait_move(0x40, colour, 0x0111);
    let (cyc, _) = first_write_cycle(&list);
    // Rewritten on line $10: the copper reads the new value on line $40.
    let mut m = machine(DMAEN | COPEN, 0x400, &[(0x400, &list)]);
    at(&mut m, 0x10 * 454).unwrap();
    m.ww(0x406, 0x0222);
    let f = m.end_frame();
    assert_eq!(f.writes, [(cyc, colour, 0x0222)]);
    // Rewritten after it was read: the old value stands this frame, the new one the next.
    let mut m = machine(DMAEN | COPEN, 0x400, &[(0x400, &list)]);
    at(&mut m, 0x41 * 454).unwrap();
    m.ww(0x406, 0x0222);
    let f = m.end_frame();
    assert_eq!(f.writes[0], (cyc, colour, 0x0111));
    let f = m.end_frame();
    assert_eq!(f.writes, [(FRAME + cyc, colour, 0x0222)]);
}

#[test]
fn a_cpu_strobe_of_copjmp1_starts_the_new_list_there() {
    let colour = copper::reg::COLOR00;
    let a = [colour, 0x0AAA, 0xFFFF, 0xFFFE];
    let b = [colour, 0x0BBB, 0xFFFF, 0xFFFE];
    let mut m = machine(DMAEN | COPEN, 0x400, &[(0x400, &a), (0x500, &b)]);
    let strobe = 0x80 * 454 + 20;
    at(&mut m, strobe).unwrap();
    put_ptr(&mut m, crate::copper::COP1LC, 0x500);
    put(&mut m, crate::copper::COPJMP1, 0);
    let f = m.end_frame();
    let made: Vec<_> = f.writes.iter().filter(|w| w.1 == colour).collect();
    assert_eq!(made.len(), 2, "{:?}", f.writes);
    assert_eq!(made[0].2, 0x0AAA);
    assert_eq!(made[1].2, 0x0BBB);
    assert_eq!(beam::position(made[1].0, 313).0, 0x80);
    assert!(made[1].0 > strobe);
    // The next frame starts at COP1LC, the new list.
    let f = m.end_frame();
    assert_eq!(f.writes[0].2, 0x0BBB);
    // A list that MOVEs to COPJMP2 is not followed: the run stops.
    let jump = [crate::copper::COPJMP2, 0, 0xFFFF, 0xFFFE];
    let mut m = machine(DMAEN | COPEN, 0x400, &[(0x400, &jump)]);
    m.end_frame();
    assert!(
        at(&mut m, FRAME)
            .unwrap_err()
            .to_string()
            .starts_with("unsupported: copper jump")
    );
}

/// A copper list for a lores screen of `planes` planes at `$10000`, `$20000` .., colours 0 to 3.
fn screen_list(planes: u16, extra: &[u16]) -> Vec<u16> {
    use copper::reg::*;
    let mut l = vec![
        BPLCON0,
        planes << 12 | 0x200,
        DIWSTRT,
        0x2C81,
        DIWSTOP,
        0x2CC1,
        DDFSTRT,
        0x38,
        DDFSTOP,
        0xD0,
        BPL1MOD,
        0,
        BPL2MOD,
        0,
        COLOR00,
        0x000,
        COLOR00 + 2,
        0xF00,
        COLOR00 + 4,
        0x0F0,
        COLOR00 + 6,
        0x00F,
    ];
    for p in 0..planes {
        l.extend([BPL1PTH + 4 * p, p + 1, BPL1PTH + 4 * p + 2, 0]);
    }
    l.extend(extra);
    l.extend([0xFFFF, 0xFFFE]);
    l
}

#[test]
fn a_two_plane_display_draws_the_expected_picture() {
    let list = screen_list(2, &[]);
    // Plane 1's first line: $FF00 repeated; plane 2's: $0F0F.
    let p1 = [0xFF00u16; 20];
    let p2 = [0x0F0Fu16; 20];
    let mem = [(0x400, &list[..]), (0x1_0000, &p1[..]), (0x2_0000, &p2[..])];
    let mut m = machine(DMAEN | COPEN | BPLEN, 0x400, &mem);
    let f = m.end_frame();
    let pic = &f.picture;
    assert_eq!(
        (pic.left, pic.top, pic.width, pic.height),
        (0x81, 0x2C, 320, 256)
    );
    // Line 0: pixels 0-3 plane 1 only (1), 4-7 both (3), 8-11 none (0), 12-15 plane 2 (2).
    let row: Vec<u8> = pic.pixels[..16].to_vec();
    assert_eq!(row, [1, 1, 1, 1, 3, 3, 3, 3, 0, 0, 0, 0, 2, 2, 2, 2]);
    assert_eq!(pic.rgb12(0, 0), 0xF00);
    assert_eq!(pic.rgb12(4, 0), 0x00F);
    assert_eq!(pic.rgb12(12, 0), 0x0F0);
    // Line 1 is the zeroed memory after line 0's 40 bytes.
    assert!(pic.pixels[320..640].iter().all(|&p| p == 0));
    // The toolkit's plain frame, from the same memory and the copper's writes, agrees.
    let word = |a: u32| u16::from_be_bytes([m.chip()[a as usize], m.chip()[a as usize + 1]]);
    let writes = copper::run(&word, 0x400, Fetch::default(), 313);
    let mem = AmigaMemory {
        chip: m.chip().to_vec(),
        slow: vec![],
    };
    assert_eq!(*pic, display::draw(&mem, &writes, Fetch::default()));
    // Every frame runs the list afresh: the next picture is the same.
    assert_eq!(m.end_frame().picture, *pic);
}

#[test]
fn a_sprite_shows_over_colour_0_inside_the_window() {
    use copper::reg::*;
    let list = screen_list(1, &[SPR0PTH, 0x3, SPR0PTH + 2, 0]);
    // VSTART $30, HSTART $50 (lores $A0), VSTOP $32; two lines of data.
    let spr = [0x3050, 0x3200, 0x8000, 0x0000, 0xC000, 0x8000, 0, 0];
    let mem = [(0x400, &list[..]), (0x3_0000, &spr[..])];
    let mut m = machine(DMAEN | COPEN | BPLEN | SPREN, 0x400, &mem);
    let pic = m.end_frame().picture;
    let px = |x: usize, y: usize| pic.pixels[y * 320 + x];
    // HSTART $A0 shows at window position $A1: picture x $20; line $30 is y 4.
    let (x, y) = (0xA1 - 0x81, 0x30 - 0x2C);
    assert_eq!(px(x, y), 17);
    assert_eq!(px(x + 1, y), 0);
    assert_eq!((px(x, y + 1), px(x + 1, y + 1)), (19, 17));
    assert_eq!(px(x, y + 2), 0, "VSTOP");
    // With sprite DMA off, nothing.
    let mut m = machine(DMAEN | COPEN | BPLEN, 0x400, &mem);
    assert!(m.end_frame().picture.pixels.iter().all(|&p| p == 0));
}

/// CLXDAT (translator T30): Denise ORs in each line's collisions as the beam passes it, and a
/// CPU read returns them with bit 15 and clears them. CLXCON `$0041` compares plane 1 with 1, so a
/// colour 1 pixel matches both playfields (bit 0), and sprite 0 over it sets bits 1 and 5.
#[test]
fn clxdat_reads_the_collisions_the_beam_has_passed_and_clears_them() {
    use copper::reg::*;
    let list = screen_list(1, &[SPR0PTH, 0x3, SPR0PTH + 2, 0, 0x098, 0x0041]);
    // Sprite 0 on line $30 at window position $A1, the picture's x $20.
    let spr = [0x3050, 0x3100, 0x8000, 0x0000, 0, 0];
    // Plane 1's row 4 (line $30): pixel $20 set.
    let pf = [0x8000u16];
    let mem = [(0x400, &list[..]), (0x3_0000, &spr[..]), (0x1_00A4, &pf[..])];
    let mut m = machine(DMAEN | COPEN | BPLEN | SPREN, 0x400, &mem);
    let line = |n: u64| n * amiga_clock::LINE_CYCLES as u64;
    at(&mut m, line(0x20)).unwrap();
    assert_eq!(get(&mut m, 0x00E), 0x8000, "the beam has not reached line $30");
    at(&mut m, line(0x40)).unwrap();
    assert_eq!(get(&mut m, 0x00E), 0x8023);
    assert_eq!(get(&mut m, 0x00E), 0x8000, "a read clears it");
    // The frame's end accounts the lines after the last read; the next frame's line $30 again.
    let f = m.end_frame();
    // BPLCON2 0 puts the playfield in front, yet an unattached sprite collides wherever it shows.
    assert_eq!(f.picture.pixels[4 * 320 + 0x20], 1, "the playfield in front");
    at(&mut m, line(313 + 0x40)).unwrap();
    assert_eq!(get(&mut m, 0x00E), 0x8023);
}

/// A machine started from a snapshot owes the program the collisions the original had not yet
/// read (`Amiga::clxdat`, in the snapshot's `custom[7]`).
#[test]
fn a_snapshots_clxdat_is_the_first_reads() {
    let cfg = Config::a500(vec![]);
    let ram = vec![0u8; (cfg.chip + cfg.slow) as usize];
    let mut custom = [0u16; 256];
    custom[2] = 0x8000;
    custom[0x00E >> 1] = 0x8006;
    let mut m = Machine::from_snapshot(cfg, &ram, &custom);
    assert_eq!(get(&mut m, 0x00E), 0x8006);
    assert_eq!(get(&mut m, 0x00E), 0x8000);
}

#[test]
fn a_ham_frame_is_refused_and_stops_the_run() {
    let mut list = screen_list(6, &[]);
    list[1] |= 0x0800;
    let mut m = machine(DMAEN | COPEN | BPLEN, 0x400, &[(0x400, &list)]);
    let f = m.end_frame();
    assert_eq!(f.picture.width, 0);
    assert!(
        at(&mut m, FRAME)
            .unwrap_err()
            .to_string()
            .starts_with("unsupported: display ham")
    );
}

#[test]
fn the_picture_reads_bitplanes_as_they_were_when_the_beam_fetched_them() {
    // The CPU fills plane 1's line at line $100 (y $D4): lines above show the old bytes.
    let list = screen_list(1, &[]);
    let mut m = machine(DMAEN | COPEN | BPLEN, 0x400, &[(0x400, &list)]);
    at(&mut m, 0x100 * 454).unwrap();
    for k in 0..256 * 40u32 {
        m.wb(0x1_0000 + k, 0xFF);
    }
    let pic = m.end_frame().picture;
    let row = |y: usize| &pic.pixels[y * 320..(y + 1) * 320];
    assert!(
        row(0xD3).iter().all(|&p| p == 0),
        "fetched before the write"
    );
    assert!(row(0xD5).iter().all(|&p| p == 1), "fetched after");
}

/// A snapshot part way down a frame: that frame began before cycle 0, and a blit in it still ends
/// a few cycles on (it underflowed the frame's start once).
#[test]
fn a_blit_in_a_mid_frame_snapshots_first_frame_ends_on_time() {
    let cfg = Config::a500(vec![]);
    let ram = vec![0u8; (cfg.chip + cfg.slow) as usize];
    let mut custom = [0u16; 256];
    custom[2] = 0x8000;
    custom[3] = 0x6400; // VHPOSR: line $64
    custom[custom::DMACONR as usize >> 1] = DMAEN | BLTEN;
    let mut m = Machine::from_snapshot(cfg, &ram, &custom);
    at(&mut m, 10).unwrap();
    put(&mut m, blitter::BLTCON0, 0x0100);
    put(&mut m, blitter::BLTSIZE, 0x0041);
    assert_ne!(get(&mut m, custom::DMACONR) & 0x4000, 0);
    at(&mut m, 200).unwrap();
    assert_eq!(get(&mut m, custom::DMACONR) & 0x4000, 0);
    assert_eq!(m.intreq() & blitter::INT_BLIT, blitter::INT_BLIT);
}

#[test]
fn a_sprite_outside_a_lines_window_shows_colour_0_when_the_window_narrows_part_way_down() {
    use copper::reg::*;
    // From line $80 the window starts at $A1, not $81: the picture still spans $81 (the lines above
    // keep it), but on line $90 a sprite at $91 is outside its line's window, one at $B1 inside.
    let list = screen_list(
        1,
        &[
            SPR0PTH,
            0x3,
            SPR0PTH + 2,
            0,
            SPR0PTH + 4,
            0x3,
            SPR0PTH + 6,
            0x100,
            0x8001,
            0xFFFE,
            DIWSTRT,
            0x2CA1,
        ],
    );
    let s0 = [0x9048, 0x9100, 0x8000, 0x0000, 0, 0];
    let s1 = [0x9058, 0x9100, 0x8000, 0x0000, 0, 0];
    let mem = [(0x400, &list[..]), (0x3_0000, &s0[..]), (0x3_0100, &s1[..])];
    let mut m = machine(DMAEN | COPEN | BPLEN | SPREN, 0x400, &mem);
    let pic = m.end_frame().picture;
    assert_eq!((pic.left, pic.width), (0x81, 320));
    let px = |x: i32, line: u16| pic.pixels[(line - 0x2C) as usize * 320 + (x - 0x81) as usize];
    assert_eq!(px(0xB1, 0x90), 17, "inside the window");
    assert_eq!(px(0x91, 0x90), 0, "left of the line's window");
}

/// CLXDAT bits: playfield against playfield, odd planes against sprite 0, even planes against
/// sprite 0, always set. The scene is `amiga-oracle`'s `tests/collision.rs`, whose readings on the
/// oracle these are (T31).
const PLF_PLF: u16 = 1;
const ODD_SPR0: u16 = 1 << 1;
const EVEN_SPR0: u16 = 1 << 5;
const ALWAYS: u16 = 0x8000;
/// CLXCON: ENBP1 (bit 6), MVBP1 (bit 0).
const ENBP1: u16 = 1 << 6;
const MVBP1: u16 = 1;

/// One lores plane, every pixel `set` or clear, sprite 0 (colour 1, lines $50 to $5F, at $90)
/// over it, CLXCON `clxcon`.
fn collision_scene(set: bool, clxcon: u16) -> Machine {
    use copper::reg::*;
    let list = screen_list(1, &[SPR0PTH, 0x3, SPR0PTH + 2, 0, collide::CLXCON, clxcon]);
    let mut spr = vec![0x5048, 0x6000];
    for _ in 0..16 {
        spr.extend([0xFFFF, 0x0000]);
    }
    spr.extend([0, 0]);
    let plane = vec![if set { 0xFFFF } else { 0 }; 20 * 256];
    let mem = [(0x400, &list[..]), (0x1_0000, &plane[..]), (0x3_0000, &spr[..])];
    machine(DMAEN | COPEN | BPLEN | SPREN, 0x400, &mem)
}

#[test]
fn clxdat_reads_the_collisions_of_the_lines_drawn_since_its_last_read_and_clears() {
    let mut m = collision_scene(true, ENBP1 | MVBP1);
    m.end_frame();
    at(&mut m, FRAME + 10).unwrap();
    // As the oracle: no even plane enabled, so playfield 2 matches everywhere.
    assert_eq!(
        get(&mut m, collide::CLXDAT),
        ALWAYS | EVEN_SPR0 | ODD_SPR0 | PLF_PLF
    );
    // A read clears it; line 0 of the next frame has drawn nothing since.
    assert_eq!(get(&mut m, collide::CLXDAT), ALWAYS);
}

#[test]
fn without_a_match_only_the_matching_playfields_bits_are_set() {
    // Plane clear, matching set: playfield 2 still matches (none enabled), playfield 1 not.
    let mut m = collision_scene(false, ENBP1 | MVBP1);
    m.end_frame();
    at(&mut m, FRAME + 10).unwrap();
    assert_eq!(get(&mut m, collide::CLXDAT), ALWAYS | EVEN_SPR0);
    // Plane set, matching clear.
    let mut m = collision_scene(true, ENBP1);
    m.end_frame();
    at(&mut m, FRAME + 10).unwrap();
    assert_eq!(get(&mut m, collide::CLXDAT) & (ODD_SPR0 | PLF_PLF), 0);
}

#[test]
fn a_read_part_way_down_sees_only_the_lines_above_the_beam() {
    let mut m = collision_scene(true, ENBP1 | MVBP1);
    // Line $40: the window's lines from $2C are drawn, the sprite (from $50) not yet.
    at(&mut m, 0x40 * 454).unwrap();
    assert_eq!(get(&mut m, collide::CLXDAT), ALWAYS | PLF_PLF);
    // Line $60: the sprite's lines since.
    at(&mut m, 0x60 * 454).unwrap();
    assert_eq!(
        get(&mut m, collide::CLXDAT),
        ALWAYS | EVEN_SPR0 | ODD_SPR0 | PLF_PLF
    );
    // The frame's end adds the rest of the window, read in the next frame.
    m.end_frame();
    at(&mut m, FRAME + 10).unwrap();
    assert_eq!(get(&mut m, collide::CLXDAT), ALWAYS | PLF_PLF);
}

#[test]
fn a_snapshots_unread_collisions_are_read_first() {
    let cfg = Config::a500(vec![]);
    let ram = vec![0u8; (cfg.chip + cfg.slow) as usize];
    let mut custom = [0u16; 256];
    custom[2] = 0x8000;
    custom[collide::CLXDAT as usize >> 1] = ALWAYS | EVEN_SPR0 | PLF_PLF;
    let mut m = Machine::from_snapshot(cfg, &ram, &custom);
    at(&mut m, 10).unwrap();
    assert_eq!(get(&mut m, collide::CLXDAT), ALWAYS | EVEN_SPR0 | PLF_PLF);
    assert_eq!(get(&mut m, collide::CLXDAT), ALWAYS);
}

/// Sprites 0 and 2, one pixel each (the first) at control word `pos` on lines $50 to $5F, over
/// a clear plane, CLXCON 0, DDFSTRT `ddf`.
fn two_sprites(pos: u16, ddf: u16) -> Machine {
    use copper::reg::*;
    let list = screen_list(
        1,
        &[
            DDFSTRT, ddf, SPR0PTH, 0x3, SPR0PTH + 2, 0, SPR0PTH + 8, 0x3, SPR0PTH + 10, 0,
            collide::CLXCON, 0,
        ],
    );
    let mut spr = vec![pos, 0x6000];
    for _ in 0..16 {
        spr.extend([0x8000, 0x0000]);
    }
    spr.extend([0, 0]);
    let plane = vec![0u16; 20 * 256];
    let mem = [(0x400, &list[..]), (0x1_0000, &plane[..]), (0x3_0000, &spr[..])];
    machine(DMAEN | COPEN | BPLEN | SPREN, 0x400, &mem)
}

/// Bit 9: sprite group 0/1 against group 2/3.
const S01_S23: u16 = 1 << 9;

#[test]
fn sprite_against_sprite_counts_from_the_fetch_start_and_past_the_window() {
    // The oracle (`amiga-oracle`'s collision scene, T31 review): sprite pixels from
    // 2 * DDFSTRT + 15 on collide, left of DIWSTRT ($81) and right of DIWSTOP ($1C1) too;
    // playfield against playfield (CLXCON 0) is set by the window either way.
    for (pos, ddf, want) in [
        (0x503F, 0x38, ALWAYS | S01_S23 | PLF_PLF), // pixel $7F
        (0x503E, 0x38, ALWAYS | PLF_PLF),           // pixel $7D
        (0x5037, 0x30, ALWAYS | S01_S23 | PLF_PLF), // pixel $6F
        (0x5036, 0x30, ALWAYS | PLF_PLF),           // pixel $6D
        (0x50E8, 0x38, ALWAYS | S01_S23 | PLF_PLF), // pixel $1D1
    ] {
        let mut m = two_sprites(pos, ddf);
        m.end_frame();
        at(&mut m, FRAME + 10).unwrap();
        assert_eq!(get(&mut m, collide::CLXDAT), want, "pos {pos:#06x} DDFSTRT {ddf:#x}");
    }
}
