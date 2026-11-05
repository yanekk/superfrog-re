//! Game-free: Denise reports collisions in CLXDAT as the hardware does. Kickstart 1.3 boots to its
//! insert-disk screen; then a vertical-blank handler poked into chip RAM takes the display over: a
//! copper list shows one lores bitplane (all set, or all clear) and sprite 0 (16 lines, colour 1)
//! over it, with a CLXCON the test chooses. The handler copies CLXDAT ($dff00e, cleared on read)
//! once a frame, so the copy holds what the last whole frame collided.
//!
//! Needs the Kickstart (`AMIGA_KICKSTART`, else `~/amiga-roms/kick1.3.rom`) and
//! `toolkit/tools/setup/oracle.sh`'s build.

use amiga_oracle::*;

/// The first frame showing the insert-disk hand (tests/boot.rs).
const HAND: u32 = 207;
/// Frames the handler runs before its copy is read.
const FRAMES: u32 = 6;

const HANDLER: u32 = 0x7_f000;
const COPPER: u32 = 0x7_d000;
const SPRITE: u32 = 0x7_c000;
const NULL_SPRITE: u32 = 0x7_c100;
const BITPLANE: u32 = 0x6_0000;
/// The handler's CLXDAT copy (word), then its frame count (word).
const COPY: u32 = 0x7_e000;

/// CLXCON bits: ENBP1 (bit 6) and MVBP1 (bit 0).
const ENBP1: u16 = 1 << 6;
const MVBP1: u16 = 1 << 0;
/// CLXDAT bits: playfield against playfield (0), odd planes against sprite 0 (1), even planes
/// against sprite 0 (5); bit 15 reads 1 on OCS.
const PLF_PLF: u16 = 1 << 0;
const ODD_SPR0: u16 = 1 << 1;
const EVEN_SPR0: u16 = 1 << 5;
const ALWAYS: u16 = 1 << 15;

fn words(ws: &[u16]) -> Vec<u8> {
    ws.iter().flat_map(|w| w.to_be_bytes()).collect()
}

/// The handler: copy CLXDAT and count the frame, restart the copper on the list, turn on DMA for
/// copper, bitplanes and sprites only, acknowledge level 3, return.
fn handler() -> Vec<u8> {
    words(&[
        0x48e7, 0x8080, // movem.l d0/a0,-(sp)
        0x41f9, 0x00df, 0xf000, // lea $dff000,a0
        0x3028, 0x000e, // move.w $e(a0),d0      CLXDAT
        0x33c0, 0x0007, 0xe000, // move.w d0,COPY
        0x5279, 0x0007, 0xe002, // addq.w #1,COPY+2
        0x317c, 0x7fff, 0x0096, // move.w #$7fff,$96(a0)   DMACON: all off
        0x217c, 0x0007, 0xd000, 0x0080, // move.l #COPPER,$80(a0)  COP1LC
        0x3140, 0x0088, // move.w d0,$88(a0)     COPJMP1
        0x317c, 0x83a0, 0x0096, // move.w #$83a0,$96(a0)   DMAEN BPLEN COPEN SPREN
        0x317c, 0x0070, 0x009c, // move.w #$0070,$9c(a0)   INTREQ: level 3 acknowledged
        0x317c, 0x0070, 0x009c, //
        0x4cdf, 0x0101, // movem.l (sp)+,d0/a0
        0x4e73, // rte
    ])
}

fn copper(clxcon: u16) -> Vec<u8> {
    let mut l = vec![
        0x0100, 0x1200, // BPLCON0: one plane, colour
        0x0102, 0x0000, 0x0104, 0x0000, //
        0x008e, 0x2c81, 0x0090, 0x2cc1, // DIWSTRT/DIWSTOP: 320 x 256
        0x0092, 0x0038, 0x0094, 0x00d0, // DDFSTRT/DDFSTOP
        0x0108, 0x0000, 0x010a, 0x0000, //
        0x00e0, (BITPLANE >> 16) as u16, 0x00e2, BITPLANE as u16, //
        0x0120, (SPRITE >> 16) as u16, 0x0122, SPRITE as u16,
    ];
    for k in 1..8u16 {
        l.extend([0x0120 + 4 * k, (NULL_SPRITE >> 16) as u16]);
        l.extend([0x0122 + 4 * k, NULL_SPRITE as u16]);
    }
    l.extend([0x0098, clxcon, 0x0180, 0x0000, 0x0182, 0x0fff, 0x01a2, 0x0f00]);
    l.extend([0xffff, 0xfffe]);
    words(&l)
}

/// Sprite 0 at horizontal $90, lines $50 to $5f, every pixel colour 1.
fn sprite() -> Vec<u8> {
    let mut s = vec![0x5048, 0x6000];
    for _ in 0..16 {
        s.extend([0xffff, 0x0000]);
    }
    s.extend([0, 0]);
    words(&s)
}

/// CLXDAT as the handler last copied it, after the display ran with the bitplane `set` (every
/// pixel 1) or clear, under `clxcon`.
fn clxdat(set: bool, clxcon: u16) -> u16 {
    let rom = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    let mut a = Amiga::power_on(&OracleConfig::new(rom, Vec::new(), HAND + FRAMES + 1)).unwrap();
    for _ in 0..HAND {
        a.step(FrameInput::NONE).unwrap();
    }
    a.poke(BITPLANE, &vec![if set { 0xff } else { 0 }; 40 * 256]);
    a.poke(SPRITE, &sprite());
    a.poke(NULL_SPRITE, &[0; 8]);
    a.poke(COPPER, &copper(clxcon));
    a.poke(COPY, &[0; 4]);
    a.poke(HANDLER, &handler());
    // The level 3 autovector last, so the handler is complete before it can be taken.
    a.poke(0x6c, &HANDLER.to_be_bytes());
    for _ in 0..FRAMES {
        a.step(FrameInput::NONE).unwrap();
    }
    let p = a.peek(COPY, 4);
    let ran = u16::from_be_bytes([p[2], p[3]]);
    assert!(u32::from(ran) >= FRAMES - 1, "the handler ran {ran} times in {FRAMES} frames");
    u16::from_be_bytes([p[0], p[1]])
}

#[test]
fn a_sprite_over_a_matching_playfield_pixel_sets_its_bit() {
    let got = clxdat(true, ENBP1 | MVBP1);
    assert_eq!(got & ODD_SPR0, ODD_SPR0, "CLXDAT {got:#06x}");
    assert_eq!(got & ALWAYS, ALWAYS, "CLXDAT {got:#06x}");
    // No even plane is enabled, so even planes match everywhere: the even bit and the
    // playfield-to-playfield bit are set wherever the odd planes match.
    assert_eq!(got, ALWAYS | EVEN_SPR0 | ODD_SPR0 | PLF_PLF, "CLXDAT {got:#06x}");
}

#[test]
fn without_a_match_the_bit_stays_clear() {
    // The plane is set but CLXCON matches a clear pixel; then the plane is clear, matching set.
    for (set, clxcon) in [(true, ENBP1), (false, ENBP1 | MVBP1)] {
        let got = clxdat(set, clxcon);
        assert_eq!(got & ODD_SPR0, 0, "plane set {set}, CLXCON {clxcon:#06x}: {got:#06x}");
        assert_eq!(got & PLF_PLF, 0, "plane set {set}, CLXCON {clxcon:#06x}: {got:#06x}");
    }
}
