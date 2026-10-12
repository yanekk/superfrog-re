//! The intro's bobs (docs/13 "Bobs"): up to seventeen objects, each an animation of frames cut
//! from a bank of bob graphics, drawn with the blitter into two display buffers in turn.
//!
//! An object (`$b0` bytes from h0+$8bb0, listed at h0+$8b6c) keeps a block per buffer, at `+0`
//! and `+$2c`: its position, the clean background to restore from, its animation and, once drawn
//! there, where it went and with what blit (`+$10` destination, `+$1e` size, `+$20` graphic,
//! `+$24`/`+$26` modulos). A frame descriptor (`$28` bytes) says where in the bank a frame is, its
//! size and offset, and the blit that cuts it out. A scene's frames come from its table of six
//! words a frame (source x and y, width, height, x and y offset), which `objects_init` turns into
//! the descriptors listed at h0+$97be.
//!
//! A draw (`bobs_draw` h0+$87ec) restores, in the buffer not shown, each object's last drawing
//! there from the clean background, then steps each object's animation and draws it with a
//! cookie-cut blit, last object first; the buffer is shown from the next vertical blank.

use super::clock::BlitClock;
use super::mem::StoryMem;
use super::{h0, h2, h3};
use crate::front::blit::Blit;

/// The object list: a long, then pointers to the objects up to a zero.
pub const LIST: u32 = h0(0x8b68);
/// The two display buffers (five planes of 200 rows of `$2c` bytes, `$2260` apart) and the clean
/// background they are restored from.
pub const BUFFER_A: u32 = h3(0x41dd0);
pub const BUFFER_B: u32 = h3(0x4c9b0);
pub const CLEAN: u32 = h3(0x57590);
pub const PLANE: u32 = 0x2260;
/// The bank of bob graphics (five planes of 320 × 1024, `$a000` apart) and the masks built from it.
pub const BANK: u32 = h3(0x5dd0);
pub const MASKS: u32 = h3(0x37dd0);
/// `buffer_toggle`: which buffer the next draw goes to.
pub const TOGGLE: u32 = h0(0x84e0);
/// The row table: each of 200 rows' byte offset in a buffer.
const ROWS: u32 = h0(0x84e4);
/// `draw_buffer` and `shown_buffer`: the buffer being drawn, and the one the vertical blank
/// points the display at.
pub const DRAWN: u32 = h0(0xb42e);
pub const SHOWN: u32 = h0(0xb432);
/// The scene's frame table pointer (`h0+$1c`) and the list of frame descriptors it fills.
pub const SCENE_TABLE: u32 = h0(0x1c);
const DESCRIPTORS: u32 = h0(0x97be);
/// The idle animation every object starts with (one frame, nothing in it).
const IDLE: u32 = h0(0xafc2);
/// The copper moves of the scene's five bitplane pointers.
pub const PLANES_COPPER: u32 = h2(0xdc66);

/// The object addresses, in list order.
pub fn objects(m: &StoryMem) -> Vec<u32> {
    let mut out = Vec::new();
    let mut at = LIST + 4;
    loop {
        let o = m.l(at);
        if o == 0 {
            return out;
        }
        out.push(o);
        at += 4;
    }
}

/// `set_planes` (h0+$83b4, h0+$83de with three planes and list h2+$db22): the copper's bitplane
/// pointer moves at `moves` to `count` planes from `at`, `rows` × `$c8` apart.
pub fn set_planes(m: &mut StoryMem, moves: u32, at: u32, row_bytes: u32, count: u32) {
    let stride = (row_bytes * 0xc8) & 0xffff;
    for k in 0..count {
        let v = at.wrapping_add(stride * k);
        m.set_w(moves + 8 * k + 6, v as u16);
        m.set_w(moves + 8 * k + 2, (v >> 16) as u16);
    }
}

/// `frame_prepare` (h0+$8724): a frame descriptor's blit, for buffers of `row_bytes` a row.
fn frame_prepare(m: &mut StoryMem, d: u32, row_bytes: u32) {
    let bytes = (m.w(d + 4) >> 3) as u32;
    m.set_w(d + 0x22, (row_bytes.wrapping_sub(bytes)) as u16);
    m.set_w(d + 0x20, m.w(d + 0x14).wrapping_sub(bytes as u16));
    m.set_w(d + 0x1a, m.w(d + 6) << 6 | (bytes >> 1) as u16);
    let at = (m.w(d + 2) as u32) * (m.w(d + 0x14) as u32) + (m.w(d) >> 3) as u32;
    m.set_l(d + 0x24, m.l(d + 0xc).wrapping_add(at));
    m.set_l(d + 0x1c, m.l(d + 0x10).wrapping_add(at));
}

/// `anim_prepare` (h0+$8718): every frame of the animation at `list` (entries of a frame pointer
/// and a duration, up to a negative long).
fn anim_prepare(m: &mut StoryMem, mut list: u32, row_bytes: u32) {
    loop {
        let d = m.l(list);
        frame_prepare(m, d, row_bytes);
        list += 8;
        if (m.l(list) as i32) < 0 {
            return;
        }
    }
}

/// `objects_init` (h0+$96d4) up to its two draws: every object back to its start, the scene's
/// frames from its table, the masks built from the bank, both buffers' pointers. The two draws
/// that end it are the caller's.
pub fn init(m: &mut StoryMem, clock: &mut BlitClock) {
    m.set_w(TOGGLE, 0);
    m.set_w(h0(0x1a), 0);
    for o in objects(m) {
        m.set_w(o, 0x17c);
        m.set_w(o + 2, 0x40);
        m.set_l(o + 0x28, IDLE);
    }
    // `frames_load` (h0+$868e): each descriptor from the table's six words.
    let mut table = m.l(SCENE_TABLE);
    let mut list = DESCRIPTORS;
    loop {
        let d = m.l(list);
        for k in 0..6 {
            let v = m.w(table + 2 * k);
            let v = if k == 2 { v.wrapping_add(16) } else { v };
            m.set_w(d + 2 * k, v);
        }
        table += 12;
        m.set_l(d + 0x10, BANK);
        m.set_l(d + 0xc, MASKS);
        m.set_w(d + 0x14, 0x28);
        m.set_l(d + 0x16, 0xa000);
        list += 8;
        if (m.l(list) as i32) < 0 {
            break;
        }
    }
    anim_prepare(m, IDLE, 0x2c);
    anim_prepare(m, DESCRIPTORS, 0x2c);
    // `objects_reset` (h0+$86c6): the row table, then each object's animation from its start and
    // its second block a copy of its first.
    for r in 0..200u32 {
        m.set_w(ROWS + 2 * r, (r * 0x2c) as u16);
    }
    for o in objects(m) {
        m.set_w(o + 0x14, 0);
        let a = m.l(o + 8);
        m.set_l(o + 0x16, a);
        m.set_w(o + 0x1a, m.w(a + 6));
        let block = m.read(o, 0x2c);
        m.write(o + 0x2c, &block);
        let a = m.l(o + 8);
        m.set_w(o + 0x1a, m.w(a + 6));
        anim_prepare(m, a, 0x2c);
    }
    // The masks: cleared, then each plane of the bank or-ed in (h0+$8408, h0+$8782).
    m.clear(MASKS, 0xa000);
    let size = (0x400u16 << 6) | (0x140 >> 4);
    for p in 0..5 {
        let b = Blit {
            con0: 0x0dfc,
            afwm: 0xffff,
            alwm: 0xffff,
            pt: [BANK + p * 0xa000, MASKS, 0, MASKS],
            size,
            ..Blit::default()
        };
        b.run(m);
        clock.blit(b.con0, size);
    }
}

/// `buffers_clear` (h0+$8314) after its fade waits: both buffers from the clean background.
pub fn clear_buffers(m: &mut StoryMem, clock: &mut BlitClock) {
    for to in [BUFFER_A, BUFFER_B] {
        let b = Blit {
            con0: 0x09f0,
            afwm: 0xffff,
            alwm: 0xffff,
            pt: [CLEAN, 0, 0, to],
            size: 0xfa16,
            ..Blit::default()
        };
        b.run(m);
        clock.blit(b.con0, b.size);
    }
}

/// `bobs_draw` (h0+$87de, `draw_frame` h0+$96b0's work): one draw of every object into the buffer
/// not shown, which becomes the one to show.
pub fn draw(m: &mut StoryMem, clock: &mut BlitClock) {
    let (mut d0, mut d1) = (BUFFER_A, BUFFER_B);
    let t = !m.w(TOGGLE);
    m.set_w(TOGGLE, t);
    let mut d2 = 0;
    if t != 0 {
        std::mem::swap(&mut d0, &mut d1);
        d2 = 0x2c;
    }
    m.set_l(DRAWN, d0);
    let objs = objects(m);
    // Restore each object's last drawing in this buffer.
    for &o in &objs {
        let a2 = o + d2;
        let dest = m.l(a2 + 0x10);
        clock.cpu(CPU_RESTORE);
        if dest == 0 {
            continue;
        }
        let size = m.w(a2 + 0x1e);
        let modulo = m.w(a2 + 0x26) as i16;
        let mut d6 = dest;
        let mut d7 = m.l(a2 + 4).wrapping_add(dest.wrapping_sub(d0));
        for _ in 0..5 {
            let b = Blit {
                con0: 0x09f0,
                afwm: 0xffff,
                alwm: 0xffff,
                pt: [d7, 0, 0, d6],
                modulo: [modulo, 0, 0, modulo],
                size,
                ..Blit::default()
            };
            b.run(m);
            clock.blit(b.con0, size);
            d6 = d6.wrapping_add(PLANE);
            d7 = d7.wrapping_add(PLANE);
        }
    }
    // Step and draw each, last first.
    for &o in objs.iter().rev() {
        clock.cpu(super::clock::CPU_PER_OBJECT);
        step_anim(m, o);
        draw_object(m, clock, o + d2, d0);
    }
    let drawn = m.l(DRAWN);
    m.set_l(SHOWN, drawn);
}

/// The CPU's time per object in the restore pass when nothing is restored.
const CPU_RESTORE: u32 = 20;

/// An object's animation: a new one asked for (`+$28`) starts; the current frame's count runs
/// down and the next entry (or the first again after a negative long) takes over.
fn step_anim(m: &mut StoryMem, o: u32) {
    let pos = m.l(o);
    m.set_l(o + 0x2c, pos);
    let new = m.l(o + 0x28);
    if new != 0 {
        m.set_l(o + 0x28, 0);
        m.set_l(o + 8, new);
        m.set_l(o + 0x16, new.wrapping_sub(8));
        m.set_w(o + 0x1a, 0);
    }
    let left = m.w(o + 0x1a).wrapping_sub(1);
    m.set_w(o + 0x1a, left);
    if (left as i16) < 0 {
        let mut e = m.l(o + 0x16).wrapping_add(8);
        m.set_l(o + 0x16, e);
        if (m.l(e) as i32) < 0 {
            e = m.l(o + 8);
            m.set_l(o + 0x16, e);
        }
        m.set_w(o + 0x1a, m.w(e + 6));
    }
    let e = m.l(o + 0x16);
    m.set_l(o + 0x42, e);
}

/// One object's block `a2` drawn into the buffer at `buffer`: clipped to the buffer's 352 × 200,
/// cut in with its mask; `+$10` keeps where it went (zero when nothing was drawn).
fn draw_object(m: &mut StoryMem, clock: &mut BlitClock, a2: u32, buffer: u32) {
    let a3 = m.l(m.l(a2 + 0x16));
    let mut a1 = buffer;
    m.set_l(a2 + 0x1e, m.l(a3 + 0x1a));
    m.set_l(a2 + 0x22, m.l(a3 + 0x1e));
    m.set_w(a2 + 0x26, m.w(a3 + 0x22));
    let width = m.w(a3 + 4);
    let height = m.w(a3 + 6);
    let mut d7 = m.w(a2).wrapping_add(m.w(a3 + 8));
    let mut a4 = m.l(a3 + 0x24);
    m.set_l(a2 + 0x10, 0);
    let add_w = |m: &mut StoryMem, at: u32, v: u16| {
        let x = m.w(at).wrapping_add(v);
        m.set_w(at, x);
    };
    // d1 carries a value between the clips, as the original's register does.
    let mut d1: u16 = 0;
    // Right edge (`$170`: the buffer's 352 pixels and the shift's extra word).
    let d6 = d7.wrapping_add(width).wrapping_sub(0x170);
    if (d6 as i16) >= 0 {
        if (d7.wrapping_sub(0x160) as i16) >= 0 {
            return;
        }
        let d5 = width.wrapping_sub(d6) >> 4;
        let cut = ((width >> 4).wrapping_sub(d5)) << 1;
        add_w(m, a2 + 0x26, cut);
        add_w(m, a2 + 0x24, cut);
        d1 = m.w(a3 + 0x1a) & 0xffc0 | d5;
        m.set_w(a2 + 0x1e, d1);
    }
    let mut d5: u16 = 0;
    if (d7 as i16) < 0 {
        // Left edge: whole words cut, and the rest drawn from the row above's end, so the
        // shifted first word falls left of the row.
        d1 = d7.wrapping_add(width);
        if (d1.wrapping_sub(0x10) as i16) < 0 {
            return;
        }
        let mut d6 = d7.wrapping_neg();
        d1 = d6;
        d6 >>= 4;
        add_w(m, a2 + 0x1e, d6.wrapping_neg());
        let bytes = (d6 << 1) as i16 as i32 as u32;
        let g = m.l(a2 + 0x20).wrapping_add(bytes);
        m.set_l(a2 + 0x20, g);
        a4 = a4.wrapping_add(bytes);
        add_w(m, a2 + 0x26, bytes as u16);
        add_w(m, a2 + 0x24, bytes as u16);
        let d6 = 0xfu16.wrapping_sub(d1 & 0xf);
        d7 = 0x150u16.wrapping_add(d6).wrapping_add(1);
        d5 = 2;
    }
    a1 = a1.wrapping_add((d7 >> 3) as u32);
    let mut y = m.w(a2 + 2).wrapping_add(m.w(a3 + 0xa));
    let d6 = y.wrapping_add(height).wrapping_sub(0xc8);
    if (d6 as i16) >= 0 {
        if (d6.wrapping_sub(height) as i16) >= 0 {
            return;
        }
        let size = m.w(a2 + 0x1e);
        d1 = ((size >> 6).wrapping_sub(d6)) << 6;
        m.set_w(a2 + 0x1e, size & 0x3f | d1);
    }
    if (y as i16) < 0 {
        let d6 = y.wrapping_neg();
        if (d6.wrapping_sub(height) as i16) >= 0 {
            return;
        }
        let size = m.w(a2 + 0x1e);
        let rows = ((size >> 6).wrapping_sub(d6)) << 6;
        m.set_w(a2 + 0x1e, size & 0x3f | rows);
        let skip = (m.w(a3 + 0x14) as u32 * d6 as u32) as u16 as i16 as i32 as u32;
        let g = m.l(a2 + 0x20).wrapping_add(skip);
        m.set_l(a2 + 0x20, g);
        a4 = a4.wrapping_add(skip);
        y = 0;
        let _ = y;
    } else {
        let at = (y as i16 as i32 * 2 - d5 as i32) as u32;
        a1 = a1.wrapping_add(m.w(ROWS.wrapping_add(at)) as u32);
    }
    let _ = d1;
    m.set_l(a2 + 0x10, a1);
    let size = m.w(a2 + 0x1e);
    let shift = (m.w(a2).wrapping_add(m.w(a3 + 8)) & 0xf) << 12;
    let modulo_cd = m.w(a2 + 0x26) as i16;
    let modulo_ab = m.w(a2 + 0x24) as i16;
    let mut g = m.l(a2 + 0x20);
    let stride = m.l(a3 + 0x16);
    for _ in 0..5 {
        let b = Blit {
            con0: shift | 0x0fca,
            con1: shift,
            afwm: 0xffff,
            alwm: 0x0000,
            pt: [a4, g, a1, a1],
            modulo: [modulo_ab, modulo_ab, modulo_cd, modulo_cd],
            size,
        };
        b.run(m);
        clock.blit(b.con0, size);
        a1 = a1.wrapping_add(PLANE);
        g = g.wrapping_add(stride);
    }
}
