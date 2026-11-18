//! Game-free: how long a CPU loop takes on the beam while the chips take bus slots, measured on the
//! reference emulator, for a machine that models DMA contention to be held to. Kickstart 1.3 boots
//! to its insert-disk screen; then a vertical-blank handler poked into chip RAM takes the display
//! over (a copper list, lores bitplanes, the DMA the case chooses), waits for line `$50`, reads
//! VHPOSR, runs a loop from chip RAM, reads VPOSR and VHPOSR again and keeps the three words.
//!
//! Needs the Kickstart (`AMIGA_KICKSTART`, else `~/amiga-roms/kick1.3.rom`) and
//! `toolkit/tools/setup/oracle.sh`'s build.

use amiga_oracle::*;

/// The first frame showing the insert-disk hand (tests/boot.rs).
const HAND: u32 = 207;
/// Frames the handler runs before its words are read; every frame measures the same.
const FRAMES: u32 = 4;

const HANDLER: u32 = 0x7_f000;
const COPPER: u32 = 0x7_d000;
const BITPLANE: u32 = 0x6_0000;
/// Every sprite channel's list: one sprite from line `SPRITE_START` to `SPRITE_STOP`, then the end.
const SPRITE: u32 = 0x7_8000;
pub const SPRITE_START: u16 = 0x40;
pub const SPRITE_STOP: u16 = 0xf0;
/// The loop's word source and destination, and the blit's.
const DATA: u32 = 0x5_0000;
/// The handler's three words (VHPOSR before, VPOSR and VHPOSR after), then its frame count.
const RESULT: u32 = 0x7_e000;

/// DMACON bits.
const SET: u16 = 0x8000;
const BLTPRI: u16 = 0x0400;
const DMAEN: u16 = 0x0200;
const BPLEN: u16 = 0x0100;
const COPEN: u16 = 0x0080;
const BLTEN: u16 = 0x0040;
const SPREN: u16 = 0x0020;

/// One measurement's set-up.
#[derive(Clone, Copy)]
pub struct Case {
    /// Lores bitplanes BPLCON0 enables (0 to 6).
    pub planes: u16,
    /// A copper list that writes four colours on every line from `$50` to `$9f`.
    pub busy_copper: bool,
    /// A blit started just before the loop: BLTCON0 and BLTSIZE; `BLTPRI` set or not.
    pub blit: Option<(u16, u16, bool)>,
    /// All eight sprite channels fetching on every line the loop runs (`sprite`).
    pub sprites: bool,
    /// The loop's body (before its `dbf`), and its count.
    pub body: &'static [u16],
    pub count: u16,
}

/// `move.w (a1),(a2)`, eight times: 12 cycles each, two reads and a write, all on the bus.
pub const MOVES: &[u16] = &[0x3491; 8];
/// `lsl.l #8,d4` four times then one `move.w (a1),(a2)`: 24 cycles each, one read and 20
/// cycles off the bus.
pub const SHIFTS: &[u16] = &[0xE18C, 0xE18C, 0xE18C, 0xE18C, 0x3491];

fn words(ws: &[u16]) -> Vec<u8> {
    ws.iter().flat_map(|w| w.to_be_bytes()).collect()
}

fn hi(a: u32) -> u16 {
    (a >> 16) as u16
}

fn lo(a: u32) -> u16 {
    a as u16
}

/// The handler: set the display and DMA up, wait for line `$50`, maybe start the blit, read VHPOSR,
/// run the loop, read VPOSR and VHPOSR, keep them, count the frame, return.
fn handler(c: &Case) -> Vec<u8> {
    let dma = SET | DMAEN | COPEN | if c.planes > 0 { BPLEN } else { 0 } | BLTEN | if c.sprites { SPREN } else { 0 };
    let mut h = vec![
        0x48e7, 0xfffe, //                       movem.l d0-d7/a0-a6,-(sp)
        0x41f9, 0x00df, 0xf000, //               lea $dff000,a0
        0x317c, 0x7fff, 0x009a, //               move.w #$7fff,$9a(a0)   INTENA: all off
        0x317c, 0xc020, 0x009a, //               move.w #$c020,$9a(a0)   INTEN VERTB
        0x317c, 0x7fff, 0x0096, //               move.w #$7fff,$96(a0)   DMACON: all off
        0x217c, hi(COPPER), lo(COPPER), 0x0080, // move.l #COPPER,$80(a0) COP1LC
        0x3140, 0x0088, //                       move.w d0,$88(a0)       COPJMP1
        0x317c, dma, 0x0096, //                  move.w #dma,$96(a0)
        0x317c, 0x0070, 0x009c, //               move.w #$0070,$9c(a0)   INTREQ: level 3 done
        0x317c, 0x0070, 0x009c, //
        0x1028, 0x0006, //               wait:   move.b 6(a0),d0         VHPOSR's line
        0x0c00, 0x0050, //                       cmpi.b #$50,d0
        0x66f6, //                               bne.s wait
    ];
    if let Some((con0, size, pri)) = c.blit {
        if pri {
            h.extend([0x317c, SET | BLTPRI, 0x0096]);
        }
        h.extend([
            0x317c, 0xffff, 0x0044, //           BLTAFWM
            0x317c, 0xffff, 0x0046, //           BLTALWM
            0x317c, 0x0000, 0x0060, //           BLTCMOD
            0x317c, 0x0000, 0x0062, //           BLTBMOD
            0x317c, 0x0000, 0x0064, //           BLTAMOD
            0x317c, 0x0000, 0x0066, //           BLTDMOD
            0x217c, hi(DATA), lo(DATA), 0x0050, // BLTAPT
            0x217c, hi(DATA), lo(DATA), 0x004c, // BLTBPT
            0x217c, hi(DATA), lo(DATA), 0x0048, // BLTCPT
            0x217c, hi(DATA + 0x8000), lo(DATA + 0x8000), 0x0054, // BLTDPT
            0x317c, con0, 0x0040, //             BLTCON0
            0x317c, 0x0000, 0x0042, //           BLTCON1
            0x317c, size, 0x0058, //             BLTSIZE
        ]);
    }
    h.extend([
        0x3228, 0x0006, //                       move.w 6(a0),d1         before
        0x303c, c.count - 1, //                  move.w #count-1,d0
        0x43f9, hi(DATA), lo(DATA), //           lea DATA,a1
        0x45f9, hi(DATA + 2), lo(DATA + 2), //   lea DATA+2,a2
    ]);
    h.extend(c.body);
    let back = -(2 * c.body.len() as i32 + 2);
    h.extend([0x51c8, back as u16]); //         dbf d0,loop
    h.extend([
        0x3428, 0x0004, //                       move.w 4(a0),d2         VPOSR after
        0x3628, 0x0006, //                       move.w 6(a0),d3         VHPOSR after
        0x317c, 0x0400, 0x0096, //               move.w #$0400,$96(a0)   BLTPRI off
        0x33c1, hi(RESULT), lo(RESULT), //       move.w d1,RESULT
        0x33c2, hi(RESULT + 2), lo(RESULT + 2),
        0x33c3, hi(RESULT + 4), lo(RESULT + 4),
        0x5279, hi(RESULT + 6), lo(RESULT + 6), // addq.w #1,RESULT+6
        0x4cdf, 0x7fff, //                       movem.l (sp)+,d0-d7/a0-a6
        0x4e73, //                               rte
    ]);
    words(&h)
}

fn copper(c: &Case) -> Vec<u8> {
    let mut l = vec![
        0x0100,
        c.planes << 12 | 0x0200, // BPLCON0: lores, colour
        0x0102,
        0x0000,
        0x0104,
        0x0000,
        0x008e,
        0x2c81,
        0x0090,
        0x2cc1, // DIWSTRT/DIWSTOP: 320 x 256
        0x0092,
        0x0038,
        0x0094,
        0x00d0, // DDFSTRT/DDFSTOP
        0x0108,
        0x0000,
        0x010a,
        0x0000,
    ];
    for p in 0..6u16 {
        l.extend([0x00e0 + 4 * p, hi(BITPLANE), 0x00e2 + 4 * p, lo(BITPLANE)]);
    }
    // Only with sprites: the pointer moves would shift the busy copper's list against the beam.
    if c.sprites {
        for n in 0..8u16 {
            l.extend([0x0120 + 4 * n, hi(SPRITE), 0x0122 + 4 * n, lo(SPRITE)]);
        }
    }
    if c.busy_copper {
        for line in 0x50..0xa0u16 {
            l.extend([line << 8 | 0x07, 0xfffe]);
            for k in 0..4u16 {
                l.extend([0x0180 + 2 * k, line + k]);
            }
        }
    }
    l.extend([0xffff, 0xfffe]);
    words(&l)
}

/// What the loop took on the beam: the line and slot VHPOSR read before it, and the DMA slots
/// from that read to the one after it. Every frame measured must agree.
pub fn measure(c: &Case) -> ((u16, u16), u32) {
    let rom = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    let mut a = Amiga::power_on(&OracleConfig::new(rom, Vec::new(), HAND + FRAMES + 1)).unwrap();
    for _ in 0..HAND {
        a.step(FrameInput::NONE).unwrap();
    }
    a.poke(BITPLANE, &vec![0; 40 * 256]);
    a.poke(COPPER, &copper(c));
    a.poke(SPRITE, &sprite());
    a.poke(RESULT, &[0; 8]);
    a.poke(HANDLER, &handler(c));
    a.poke(0x6c, &HANDLER.to_be_bytes());
    let mut seen = Vec::new();
    for _ in 0..FRAMES {
        a.step(FrameInput::NONE).unwrap();
        let p = a.peek(RESULT, 8);
        let w = |i: usize| u16::from_be_bytes([p[i], p[i + 1]]);
        let (before, vpos, after) = (w(0), w(2), w(4));
        let start = (before >> 8, before & 0xff);
        let end = ((vpos & 1) << 8 | after >> 8, after & 0xff);
        let slots = (end.0 as u32 * 227 + end.1 as u32) - (start.0 as u32 * 227 + start.1 as u32);
        seen.push((start, slots, w(6)));
    }
    let ran = seen.last().unwrap().2;
    assert!(u32::from(ran) >= FRAMES - 1, "the handler ran {ran} times");
    // The first frame's handler starts from Kickstart's state; the later ones repeat each other.
    // The wait for line `$50` ends a slot or two apart from frame to frame, which moves the loop
    // against the fetch by as much: frame 1's reading is the one kept, the others within 2 slots.
    let (start, slots, _) = seen[1];
    for s in &seen[2..] {
        assert!(s.1.abs_diff(slots) <= 2, "frames disagree: {seen:?}");
    }
    (start, slots)
}

/// One sprite, its data words zero (DMA fetches them all the same), then the list's end.
pub fn sprite() -> Vec<u8> {
    let lines = (SPRITE_STOP - SPRITE_START) as usize;
    let mut w = vec![SPRITE_START << 8 | 0x40, SPRITE_STOP << 8];
    w.extend(std::iter::repeat_n(0u16, 2 * lines));
    w.extend([0, 0]);
    words(&w)
}

fn case(planes: u16) -> Case {
    Case {
        planes,
        busy_copper: false,
        blit: None,
        sprites: false,
        body: MOVES,
        count: 200,
    }
}

/// The readings `pcode-machine`'s contention tests hold the machine to (`tests_dma.rs`; learned
/// on: translator T49): where the loop's first VHPOSR read was, and the DMA slots to its second.
#[test]
fn a_cpu_loop_on_the_beam() {
    let blit = |planes, con0, pri| Case {
        blit: Some((con0, 64 << 6 | 20, pri)),
        ..case(planes)
    };
    let all: Vec<(&str, Case, (u16, u16), u32)> = vec![
        ("moves, 0 planes", case(0), (0x50, 31), 10672),
        ("moves, 4 planes", case(4), (0x50, 25), 10859),
        ("moves, 5 planes", case(5), (0x50, 17), 13156),
        ("moves, 6 planes", case(6), (0x50, 31), 16438),
        (
            "shifts, 6 planes",
            Case {
                body: SHIFTS,
                ..case(6)
            },
            (0x50, 31),
            13033,
        ),
        (
            "moves, 0 planes, busy copper",
            Case {
                busy_copper: true,
                ..case(0)
            },
            (0x50, 32),
            10714,
        ),
        // Sprite DMA's odd slots (T49 review): 17 slots on the moves, 7 on the shifts.
        ("shifts, 0 planes", Case { body: SHIFTS, ..case(0) }, (0x50, 31), 11866),
        (
            "moves, 8 sprites",
            Case {
                sprites: true,
                ..case(0)
            },
            (0x50, 17),
            10689,
        ),
        (
            "shifts, 8 sprites",
            Case {
                sprites: true,
                body: SHIFTS,
                ..case(0)
            },
            (0x50, 17),
            11873,
        ),
        ("blit ABCD 64x20", blit(0, 0x0fca, false), (0x50, 157), 14549),
        // BLTPRI holds the CPU off the bus until the blit ends, its first read included.
        ("blit ABCD 64x20 BLTPRI", blit(0, 0x0fca, true), (103, 148), 10671),
        ("blit AD 64x20", blit(0, 0x09f0, false), (0x50, 155), 12611),
        ("blit D 64x20", blit(0, 0x0100, false), (0x50, 151), 10754),
        ("blit ABCD 64x20, 5 planes", blit(5, 0x0fca, false), (0x50, 165), 21322),
    ];
    for (name, c, start, slots) in &all {
        assert_eq!(measure(c), (*start, *slots), "{name}");
    }
}
