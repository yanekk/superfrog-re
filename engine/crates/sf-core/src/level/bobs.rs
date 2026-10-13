//! The objects' bobs as a level's set-up leaves them (h0+$1995a, called by every `level_init_wN`):
//! the frame records of the shared and the world's animations filled in from their tables
//! (position in the bob sheet, size, hot spot) and given their blit parameters, the line offsets
//! and the bob records' four sub-records, the bob sheet's mask built by the blitter (every plane
//! ORed together), then the frog's and the panel digits' hardware sprites.

use crate::frog;
use crate::layout::{h0, h2, h3};
use crate::level::camera;
use crate::level::LevelCtx;
use crate::level::objects::{BOB_DRAW_LIST, BOB_FREE_LIST, Blit, blit};
use crate::mem::Bus;

/// `bob_buffer` (h0+$18be4).
const BOB_BUFFER: u32 = h0(0x18be4);
/// `line_offsets` (h0+$18be8): `$13d` lines.
const LINE_OFFSETS: u32 = h0(0x18be8);
const SPUD_FRAMES: u32 = h0(0x2aefe);
/// The shared animations' frame list (h0+$2afb0) and their table (h0+$2c9b4); the world's frame
/// list (h0+$2d128), whose table `level_init_wN` sets (h0+$a02).
const SHARED_FRAMES: u32 = h0(0x2afb0);
const SHARED_TABLE: u32 = h0(0x2c9b4);
const WORLD_FRAMES: u32 = h0(0x2d128);
const WORLD_TABLE: u32 = h0(0xa02);
/// The bob sheet (`LnBO`, h3+0: five planes of 320 × 320) and its mask (h3+$48190).
const SHEET: u32 = h3(0);
const MASK: u32 = h3(0x48190);
const SHEET_ROW: u16 = 0x28;
const SHEET_PLANE: u32 = 0x3200;

/// h0+$1995a.
pub fn bob_setup(c: &mut LevelCtx) {
    let b = &mut c.bus;
    b.set_w(BOB_BUFFER, 0);
    park_free(b);
    frame_records(b, SHARED_FRAMES, SHARED_TABLE);
    let table = b.l(WORLD_TABLE);
    frame_records(b, WORLD_FRAMES, table);
    for list in [SPUD_FRAMES, SHARED_FRAMES, WORLD_FRAMES] {
        blit_params(b, list);
    }
    sub_records(b);
    let b = &mut c.bus;
    for k in 0..SHEET_PLANE {
        b.set_b(MASK + k, 0);
    }
    masks(c);
    let b = &mut c.bus;
    sprite_attach(b, camera::FROG_SPRITE_LEFT);
    sprite_attach(b, camera::FROG_SPRITE_RIGHT);
    let alt = b.w(camera::FINAL_LEVEL_FLAG) != 0 || b.w(camera::SHOOTER_MODE) != 0;
    let from = h0(0x19ac2) + if alt { 0x40 } else { 0 };
    for k in 0..0x40 {
        b.set_b(h2(0x6bd0 + 0x390) + k, b.b(from + k));
    }
    for rec in [h0(0x1b128), h0(0x1b144)] {
        sprite_attach(b, rec);
    }
    for rec in [h0(0x1b128), h0(0x1b144)] {
        frog::anim::sprite_to_copper(b, rec);
    }
    // The sprites' DMA off: the frog's sprites show once the copper list turns it on.
    c.custom(0x96, 0x0020);
}

/// h0+$84f0: each free bob record's position marked `$c180`, `$c180`.
fn park_free(b: &mut Bus) {
    let mut at = BOB_FREE_LIST;
    loop {
        let rec = b.l(at);
        if rec == 0 {
            break;
        }
        b.set_w(rec.wrapping_sub(4), 0xc180);
        b.set_w(rec.wrapping_sub(2), 0xc180);
        at += 4;
    }
}

/// h0+$18e8a: each frame record of the list at `list` (8-byte entries, the record first, ending
/// with a negative long) takes six words from `table` (x, y in the sheet, width less 16, height,
/// hot spot x, y) and the sheet's, the mask's and the plane's places.
fn frame_records(b: &mut Bus, mut list: u32, mut table: u32) {
    loop {
        let rec = b.l(list);
        for o in [0, 2, 4, 6, 8, 0xa] {
            let mut v = b.w(table);
            if o == 4 {
                v = v.wrapping_add(0x10);
            }
            b.set_w(rec + o, v);
            table = table.wrapping_add(2);
        }
        b.set_l(rec + 0x10, SHEET);
        b.set_l(rec + 0xc, MASK);
        b.set_w(rec + 0x14, SHEET_ROW);
        b.set_l(rec + 0x16, SHEET_PLANE);
        list = list.wrapping_add(8);
        if (b.l(list) as i32) < 0 {
            return;
        }
    }
}

/// h0+$18f6c: each frame record of the list at `list` gets its blit's modulos, size and the
/// addresses of its first word in the sheet and the mask (h0+$18f78), for a `$30`-byte line.
fn blit_params(b: &mut Bus, mut list: u32) {
    loop {
        let rec = b.l(list);
        let width = b.w(rec + 4) >> 3;
        b.set_w(rec + 0x22, 0x30u16.wrapping_sub(width));
        b.set_w(rec + 0x20, b.w(rec + 0x14).wrapping_sub(width));
        let size = b.w(rec + 6).wrapping_shl(6).wrapping_add(width >> 1);
        b.set_w(rec + 0x1a, size);
        let at = (b.w(rec + 2) as u32 * b.w(rec + 0x14) as u32).wrapping_add((b.w(rec) >> 3) as u32);
        b.set_l(rec + 0x24, at.wrapping_add(b.l(rec + 0xc)));
        b.set_l(rec + 0x1c, at.wrapping_add(b.l(rec + 0x10)));
        list = list.wrapping_add(8);
        if (b.l(list) as i32) < 0 {
            return;
        }
    }
}

/// h0+$18ef8: the line offsets (`$30` a line), and each bob record of the draw list started on its
/// animation's first step, its first sub-record copied to the second, then both to the third and
/// fourth.
fn sub_records(b: &mut Bus) {
    b.set_w(BOB_BUFFER, 0);
    for k in 0..0x13d {
        b.set_w(LINE_OFFSETS + 2 * k, (k * 0x30) as u16);
    }
    let first = BOB_DRAW_LIST;
    let mut at = first;
    loop {
        let rec = b.l(at);
        b.set_w(rec + 0x14, 0);
        let anim = b.l(rec + 8);
        b.set_l(rec + 0x16, anim);
        b.set_w(rec + 0x1a, b.w(anim + 6));
        for k in 0..0x23 {
            b.set_w(rec + 0x46 + 2 * k, b.w(rec + 2 * k));
        }
        let anim = b.l(rec + 8);
        b.set_w(rec + 0x1a, b.w(anim + 6));
        at += 4;
        if b.l(at) == 0 {
            break;
        }
    }
    let mut at = first;
    loop {
        let rec = b.l(at);
        at += 4;
        for k in 0..0x46 {
            b.set_w(rec + 0x8c + 2 * k, b.w(rec + 2 * k));
        }
        if b.l(at) == 0 {
            break;
        }
    }
}

/// h0+$18fd6: the mask cleared, then every plane of the sheet ORed into it.
fn masks(c: &mut LevelCtx) {
    let size = 0x140u16 << 6 | 0x14;
    c.custom(0x96, 0x8400);
    blit(
        &mut c.bus,
        Blit {
            con0: 0x0100,
            con1: 0,
            afwm: 0xffff,
            alwm: 0xffff,
            a: 0,
            b: 0,
            c: 0,
            d: MASK,
            amod: 0,
            bmod: 0,
            cmod: 0,
            dmod: 0,
            size,
        },
    );
    for p in 0..5 {
        let b = &mut c.bus;
        blit(
            b,
            Blit {
                con0: 0x0dfc,
                con1: 0,
                afwm: 0xffff,
                alwm: 0xffff,
                a: SHEET + p * SHEET_PLANE,
                b: MASK,
                c: 0,
                d: MASK,
                amod: 0,
                bmod: 0,
                cmod: 0,
                dmod: 0,
                size,
            },
        );
    }
    c.custom(0x96, 0x0400);
}

/// `$1b208`: point a sprite record's copper MOVEs at its sprite (its animation's first frame,
/// if it has one), its CTL bits ORed in.
fn sprite_attach(b: &mut Bus, rec: u32) {
    let anim = b.l(rec + 0x10);
    let data = if anim != 0 {
        b.set_l(rec + 0x14, anim);
        b.set_w(rec + 0x18, b.w(anim + 6));
        b.l(anim)
    } else {
        b.l(rec + 0xc)
    };
    let mv = b.l(rec + 8);
    b.set_w(mv + 0xe, b.w(mv + 0xe) | b.w(rec + 6));
    b.set_w(mv + 6, data as u16);
    b.set_w(mv + 2, (data >> 16) as u16);
}
