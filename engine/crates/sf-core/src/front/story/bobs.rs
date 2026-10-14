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

use super::cpu::op;
use super::mem::StoryMem;
use super::sim::Prog;
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

/// `frame_prepare` (h0+$8724): a frame descriptor's blit, for buffers of `row_bytes` a row; its
/// instructions from its `bsr` on are `p`'s.
fn frame_prepare(m: &mut StoryMem, p: &mut Prog, d: u32, row_bytes: u32) {
    let bytes = (m.w(d + 4) >> 3) as u32;
    m.set_w(d + 0x22, (row_bytes.wrapping_sub(bytes)) as u16);
    m.set_w(d + 0x20, m.w(d + 0x14).wrapping_sub(bytes as u16));
    m.set_w(d + 0x1a, m.w(d + 6) << 6 | (bytes >> 1) as u16);
    let at = (m.w(d + 2) as u32) * (m.w(d + 0x14) as u32) + (m.w(d) >> 3) as u32;
    m.set_l(d + 0x24, m.l(d + 0xc).wrapping_add(at));
    m.set_l(d + 0x1c, m.l(d + 0x10).wrapping_add(at));
    p.ins(op::BSR);
    p.ins("rrp"); // movea.l (a2), a3
    p.ins(op::REG); // move.l d0, d2
    p.ins(op::LEA_ABS_L);
    p.ins(op::CLR_L_DN); // clr.l d7
    p.ins(op::MOVE_W_DI_DN);
    p.ins(SHIFT_3); // lsr.w #3, d7
    p.ins(op::REG);
    p.ins(op::REG); // sub.w d7, d2
    p.ins(MOVE_W_DN_DI);
    p.ins(op::MOVE_W_DI_DN);
    p.ins(op::REG);
    p.ins(MOVE_W_DN_DI);
    p.ins(op::REG);
    p.ins(SHIFT_1); // lsr.w #1, d6
    p.ins(op::MOVE_W_DI_DN);
    p.ins(SHIFT_6); // lsl.w #6, d5
    p.ins(op::REG);
    p.ins(MOVE_W_DN_DI);
    p.ins(op::MOVE_W_DI_DN);
    p.ins(op::MOVE_W_DI_DN);
    p.ins(op::REG); // ext.l d7
    // `mulu.w d6, d7`: d6 is `$28` (two bits set), 42 clocks.
    p.ins("pnnnnnnnnnnnnnnnnnnn");
    p.ins(op::MOVE_W_DI_DN);
    p.ins(SHIFT_3);
    p.ins(op::REG);
    p.ins(op::ADDQ_L_AN); // add.l d6, d7
    p.ins(op::REG);
    p.ins(ADD_L_DI_DN); // add.l $c(a3), d6
    p.ins(MOVE_L_DN_DI);
    p.ins(ADD_L_DI_DN);
    p.ins(MOVE_L_DN_DI);
    p.ins(op::RTS);
}

/// `lsr`/`lsl` by 1, 3 and 6 ([`op::shift`]).
const SHIFT_1: &str = "pnn";
const SHIFT_3: &str = "pnnnn";
const SHIFT_6: &str = "pnnnnnnn";
/// `move.w Dn, d(An)`; `move.l Dn, d(An)`; `add.l d(An), Dn`; `move.w (An)+, d(An)`.
const MOVE_W_DN_DI: &str = "pwp";
const MOVE_L_DN_DI: &str = "pw^wp";
const ADD_L_DI_DN: &str = "prrpn";
const MOVE_W_PI_DI: &str = "rpwp";

/// `anim_prepare` (h0+$8718): every frame of the animation at `list` (entries of a frame pointer
/// and a duration, up to a negative long), from its call (`p`'s, a `bsr` or a `jsr`) on.
fn anim_prepare(m: &mut StoryMem, p: &mut Prog, mut list: u32, row_bytes: u32) -> u32 {
    let mut n = 0;
    loop {
        let d = m.l(list);
        frame_prepare(m, p, d, row_bytes);
        n += 1;
        list += 8;
        p.ins(op::ADDQ_L_AN);
        p.ins("rrp"); // tst.l (a2)
        let more = (m.l(list) as i32) >= 0;
        p.branch(more, false);
        if !more {
            p.ins(op::RTS);
            return n;
        }
    }
}

/// The scene's set-up from `buffers_clear`'s first blit (h0+$834c, after its waits for the fades)
/// to its call of `objects_init` (h0+$7ae and its kind): both buffers from the clean background,
/// the scene's planes in the copper list, its frame table. The CPU's program ([`super::sim`]).
pub fn clear_buffers(m: &mut StoryMem) -> Prog {
    let mut p = Prog::new();
    p.ins_regs("pppWp", &[(DMACON, 0x8400)]);
    p.ins(op::BSR);
    p.wait_blit(true);
    p.ins(op::RTS);
    p.ins("ppppw^wp"); // move.l #-1, BLTAFWM
    p.ins_regs("ppppW^Wp", &[(BLTCON0, 0x09f0), (BLTCON1, 0)]);
    p.ins("pprrpww"); // clr.l BLTAMOD
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
        p.ins("ppppw^wp"); // BLTAPT
        p.ins("ppppw^wp"); // BLTDPT
        p.ins_regs("pppWp", &[(BLTSIZE, 0xfa16)]);
        p.ins(op::BSR);
        p.wait_blit(true);
        p.ins(op::RTS);
    }
    p.ins(op::RTS);
    // The scene's: `lea`, `move.l #$2c, d0`, `bsr.w set_planes` (h0+$83b4).
    p.ins(op::LEA_ABS_L);
    p.ins(op::MOVEA_L_IM);
    p.ins(op::BSR);
    p.ins(op::REG); // move.l a0, d1
    p.ins("ppnnnnnnnnnnnnnnnnnn"); // mulu.w #$c8, d0: three bits set, 44 clocks
    p.ins(op::REG); // ext.l
    p.ins(op::MOVEA_L_IM);
    p.ins(op::LEA_ABS_L);
    for k in 0..5 {
        p.ins(MOVE_W_DN_DI);
        p.ins(op::REG); // swap
        p.ins(MOVE_W_DN_DI);
        p.ins(op::REG);
        p.ins(op::ADDQ_L_AN); // addq.l #8, a0
        p.ins(op::ADDQ_L_AN); // add.l d0, d1
        p.ins(op::SUBQ_W_DN);
        p.branch(k < 4, false);
    }
    p.ins(op::RTS);
    p.ins("ppppw^wp"); // move.l #table, h0+$1c
    p
}

/// `objects_init` (h0+$96d4) up to its two draws, from its call: every object back to its start,
/// the scene's frames from its table, the masks cleared and built from the bank, both buffers'
/// pointers. The two draws that end it are the caller's. Its memory work done, the CPU's program
/// ([`super::sim`]).
pub fn init(m: &mut StoryMem) -> Prog {
    let mut p = Prog::new();
    p.ins(op::JSR_ABS_L);
    m.set_w(TOGGLE, 0);
    m.set_w(h0(0x1a), 0);
    p.ins(op::CLR_W_ABS_L);
    p.ins(op::CLR_W_ABS_L);
    p.ins(op::LEA_ABS_L);
    for o in objects(m) {
        m.set_w(o, 0x17c);
        m.set_w(o + 2, 0x40);
        m.set_l(o + 0x28, IDLE);
        p.ins("rrp"); // movea.l (a0)+, a1
        p.ins("pppn"); // cmpa.l #0, a1
        p.branch(false, false);
        p.ins(op::MOVE_W_IM_DI);
        p.ins(op::MOVE_W_IM_DI);
        p.ins("pppw^wp"); // move.l #idle, $28(a1)
        p.ins(op::BCC_TAKEN); // bra.b
    }
    p.ins("rrp");
    p.ins("pppn");
    p.branch(true, false);
    // `frames_load` (h0+$868e): each descriptor from the table's six words.
    for _ in 0..4 {
        p.ins(op::LEA_ABS_L);
    }
    p.ins("pprrp"); // movea.l h0+$1c, a1
    p.ins(op::MOVEA_L_IM);
    p.ins(op::MOVEA_L_IM);
    p.ins(op::BSR);
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
        p.ins("rrp"); // movea.l (a0), a5
        p.ins(MOVE_W_PI_DI);
        p.ins(MOVE_W_PI_DI);
        p.ins("rp"); // move.w (a1)+, d2
        p.ins(op::ANDI_W_DN); // addi.w #$10, d2
        p.ins(MOVE_W_DN_DI);
        for _ in 0..3 {
            p.ins(MOVE_W_PI_DI);
        }
        p.ins(MOVE_L_DN_DI);
        p.ins(MOVE_L_DN_DI);
        p.ins(MOVE_W_DN_DI);
        p.ins(MOVE_L_DN_DI);
        p.ins(op::ADDQ_L_AN);
        p.ins("rrp"); // tst.l (a0)
        list += 8;
        let more = (m.l(list) as i32) >= 0;
        p.branch(more, false);
        if !more {
            break;
        }
    }
    p.ins(op::RTS);
    // Each frame's blit: the idle animation's, then the scene's.
    for at in [IDLE, DESCRIPTORS] {
        p.ins(op::LEA_ABS_L);
        p.ins(op::LEA_ABS_L);
        p.ins(op::MOVEA_L_IM);
        p.ins(op::MOVEA_L_IM);
        p.ins(op::JSR_ABS_L);
        anim_prepare(m, &mut p, at, 0x2c);
    }
    // `objects_reset` (h0+$86c6): the row table, then each object's animation from its start and
    // its second block a copy of its first.
    p.ins(op::LEA_ABS_L);
    p.ins(op::MOVEA_L_IM);
    p.ins(op::MOVEA_L_IM);
    p.ins(op::JSR_ABS_L);
    p.ins(op::LEA_PC);
    p.ins(op::CLR_L_DN);
    p.ins(op::ANDI_W_DN); // move.w #$c8, d2
    for r in 0..200u32 {
        m.set_w(ROWS + 2 * r, (r * 0x2c) as u16);
        p.ins("wp"); // move.w d3, (a1)+
        p.ins(op::REG);
        p.ins(op::SUBQ_W_DN);
        p.branch(r < 199, false);
    }
    p.ins(op::ADDQ_L_AN);
    let objs = objects(m);
    for (i, &o) in objs.iter().enumerate() {
        m.set_w(o + 0x14, 0);
        let a = m.l(o + 8);
        m.set_l(o + 0x16, a);
        m.set_w(o + 0x1a, m.w(a + 6));
        let block = m.read(o, 0x2c);
        m.write(o + 0x2c, &block);
        let a = m.l(o + 8);
        m.set_w(o + 0x1a, m.w(a + 6));
        p.ins("rrp"); // movea.l (a0), a1
        p.ins(op::CLR_W_DI);
        p.ins(op::MOVEA_L_DI);
        p.ins(MOVE_L_DN_DI);
        p.ins("prpwp"); // move.w 6(a2), $1a(a1)
        p.ins(op::REG);
        p.ins(op::REG);
        p.ins(op::MOVEA_L_IM); // move.l #$2c, d7
        p.ins(op::ADDQ_L_AN); // adda.l d7, a2
        p.ins(op::REG);
        p.ins(SHIFT_1);
        for k in 0..0x16 {
            p.ins("rwp"); // move.w (a3)+, (a2)+
            p.ins(op::SUBQ_W_DN);
            p.branch(k < 0x15, false);
        }
        p.ins(op::MOVEA_L_DI);
        p.ins("prpwp");
        anim_prepare(m, &mut p, a, 0x2c);
        p.ins(op::ADDQ_L_AN);
        p.ins("rrp"); // tst.l (a0)
        p.branch(i + 1 < objs.len(), false);
    }
    p.ins(op::RTS);
    // The masks cleared (h0+$8408).
    m.clear(MASKS, 0xa000);
    p.ins(op::LEA_ABS_L);
    p.ins(op::MOVEA_L_IM);
    p.ins(op::BSR);
    for k in 0..CLEAR_TURNS {
        p.ins("rpw"); // clr.b (a0)+
        p.ins(op::ADDQ_L_AN); // subq.l #1, d0
        p.branch(k + 1 < CLEAR_TURNS, false);
    }
    p.ins(op::RTS);
    p.ins(op::LEA_ABS_L);
    p.ins(op::LEA_ABS_L);
    for _ in 0..3 {
        p.ins(op::MOVEA_L_IM);
    }
    p.ins(op::BSR);
    init_masks(m, &mut p);
    p
}

/// The masks' clearing's turns.
const CLEAR_TURNS: u32 = 0xa000;

/// `objects_init`'s blits (h0+$8782, from its first instruction): each plane of the bank or-ed
/// into the masks.
fn init_masks(m: &mut StoryMem, p: &mut Prog) {
    // 1024 rows (a height of 0) of 20 words.
    let size = 0x140u16 >> 4;
    p.ins(op::LEA_ABS_L);
    p.ins_regs("ppWp", &[(DMACON, 0x8400)]);
    p.ins("pppw^wp"); // move.l #-1, $44(a6)
    p.ins_regs("pppW^Wp", &[(BLTCON0, 0x0dfc), (BLTCON1, 0)]);
    p.ins(op::REG);
    p.ins(SHIFT_3);
    p.ins("pnnnnnnnnnnnnnnnnnn"); // mulu.w d2, d3: $400, one bit set, 40 clocks
    p.ins(SHIFT_6);
    p.ins("pnnnnn"); // lsr.w #4, d1
    p.ins(op::REG);
    p.ins("prrpww"); // clr.l $62(a6)
    p.ins(op::CLR_W_DI);
    for k in 0..5 {
        let b = Blit {
            con0: 0x0dfc,
            afwm: 0xffff,
            alwm: 0xffff,
            pt: [BANK + k * 0xa000, MASKS, 0, MASKS],
            size,
            ..Blit::default()
        };
        b.run(m);
        p.wait_blit(false);
        p.ins(MOVE_L_DN_DI);
        p.ins(MOVE_L_DN_DI);
        p.ins(MOVE_L_DN_DI);
        p.ins_regs("pWp", &[(BLTSIZE, size)]);
        p.ins(op::ADDQ_L_AN); // adda.l d3, a0
        p.ins(op::SUBQ_W_DN);
        p.branch(k < 4, false);
    }
    p.ins_regs("ppWp", &[(DMACON, 0x0400)]);
    p.ins(op::BSR);
    p.wait_blit(true);
    p.ins(op::RTS);
    p.ins(op::RTS);
}

/// `bobs_draw` (h0+$87ec, `draw_frame` h0+$96b0's work): one draw of every object into the buffer
/// not shown, which becomes the one to show once the draw is over (h0+$87de). Beside the memory
/// work, the CPU's instructions from h0+$87de's `bsr` to its `rts`, branch for branch, as the bus
/// accesses [`sim`] runs (T53).
pub fn draw(m: &mut StoryMem) -> Prog {
    let mut p = Prog::new();
    p.ins(op::BSR);
    let (mut d0, mut d1) = (BUFFER_A, BUFFER_B);
    p.ins(op::CLR_L_DN);
    let t = !m.w(TOGGLE);
    m.set_w(TOGGLE, t);
    p.ins("ppr^pw"); // not.w abs.l
    let mut d2 = 0;
    p.branch(t == 0, false);
    if t != 0 {
        std::mem::swap(&mut d0, &mut d1);
        d2 = 0x2c;
        p.ins("pn"); // exg
        p.ins(op::MOVEA_L_IM); // move.l #$2c, d2
    }
    p.ins(op::ADDQ_L_AN); // addq.l #2, d1
    m.set_l(DRAWN, d0);
    p.ins("ppwwp"); // move.l d0, abs.l
    p.wait_blit(true);
    p.ins(op::LEA_PC);
    p.ins(op::LEA_ABS_L);
    p.ins_regs("ppWp", &[(DMACON, 0x8400)]);
    p.ins_regs("pppW^Wp", &[(BLTAFWM, 0xffff), (BLTAFWM + 2, 0xffff)]);
    p.ins_regs("pppW^Wp", &[(BLTCON0, 0x09f0), (BLTCON1, 0)]);
    p.ins(op::ADDQ_L_AN); // addq.l #4, a0
    let objs = objects(m);
    // Restore each object's last drawing in this buffer.
    for (i, &o) in objs.iter().enumerate() {
        p.ins("rrp"); // movea.l (a0)+, a2
        p.ins(op::ADDQ_L_AN); // adda.l d2, a2
        let a2 = o + d2;
        let dest = m.l(a2 + 0x10);
        p.ins(op::MOVEA_L_DI); // move.l $10(a2), d6
        p.branch(dest == 0, true);
        if dest != 0 {
            let size = m.w(a2 + 0x1e);
            let modulo = m.w(a2 + 0x26);
            p.ins(op::MOVE_W_DI_DN);
            p.ins_regs("prpWp", &[(BLTDMOD, modulo)]);
            p.ins_regs("prpWp", &[(BLTAMOD, modulo)]);
            p.ins(op::REG); // move.l d6, d4
            p.ins(op::ADDQ_L_AN); // sub.l d0, d4
            p.ins(op::MOVEA_L_DI); // move.l 4(a2), d7
            p.ins(op::ADDQ_L_AN); // add.l d4, d7
            let mut d6 = dest;
            let mut d7 = m.l(a2 + 4).wrapping_add(dest.wrapping_sub(d0));
            for k in 0..5 {
                let b = Blit {
                    con0: 0x09f0,
                    afwm: 0xffff,
                    alwm: 0xffff,
                    pt: [d7, 0, 0, d6],
                    modulo: [modulo as i16, 0, 0, modulo as i16],
                    size,
                    ..Blit::default()
                };
                b.run(m);
                p.wait_blit(false);
                p.ins_regs("pW^Wp", &long(BLTAPT, d7));
                p.ins_regs("pW^Wp", &long(BLTDPT, d6));
                p.ins_regs("pWp", &[(BLTSIZE, size)]);
                if k < 4 {
                    p.ins(op::ADDQ_L_AN);
                    p.ins(op::ADDQ_L_AN);
                }
                d6 = d6.wrapping_add(PLANE);
                d7 = d7.wrapping_add(PLANE);
            }
        }
        p.wait_blit(false);
        p.ins("rrp"); // tst.l (a0)
        p.branch(i + 1 < objs.len(), true);
    }
    // Step and draw each, last first.
    p.ins(op::CLR_L_DN);
    for (i, &o) in objs.iter().rev().enumerate() {
        p.ins(op::CLR_L_DN);
        p.ins("nrrp"); // movea.l -(a0), a2
        step_anim(m, &mut p, o);
        p.ins(op::ADDQ_L_AN); // adda.l d2, a2
        draw_object(m, &mut p, o + d2, d0);
        p.wait_blit(false);
        p.ins(op::MOVEA_L_DI); // tst.l -4(a0)
        p.branch(i + 1 < objs.len(), true);
    }
    p.ins_regs("ppWp", &[(DMACON, 0x0400)]);
    p.ins(op::RTS);
    // `shown_buffer` takes `draw_buffer` when the draw is over: the caller's memory work (it can
    // be frames later), this program's last instructions.
    p.ins("pprrpwwpp"); // move.l abs.l, abs.l
    p.ins(op::RTS);
    p
}

/// The custom registers the draw writes.
const BLTCON0: u16 = 0x040;
const BLTCON1: u16 = 0x042;
const BLTAFWM: u16 = 0x044;
const BLTCPT: u16 = 0x048;
const BLTBPT: u16 = 0x04c;
const BLTAPT: u16 = 0x050;
const BLTDPT: u16 = 0x054;
const BLTSIZE: u16 = 0x058;
const BLTCMOD: u16 = 0x060;
const BLTBMOD: u16 = 0x062;
const BLTAMOD: u16 = 0x064;
const BLTDMOD: u16 = 0x066;
const DMACON: u16 = 0x096;

/// A long's two word writes to `reg`, high word first.
fn long(reg: u16, v: u32) -> [(u16, u16); 2] {
    [(reg, (v >> 16) as u16), (reg + 2, v as u16)]
}

/// An object's animation: a new one asked for (`+$28`) starts; the current frame's count runs
/// down and the next entry (or the first again after a negative long) takes over.
fn step_anim(m: &mut StoryMem, p: &mut Prog, o: u32) {
    let pos = m.l(o);
    m.set_l(o + 0x2c, pos);
    p.ins("prrpwwp"); // move.l 0(a2), $2c(a2)
    let new = m.l(o + 0x28);
    p.ins(op::MOVEA_L_DI); // tst.l $28(a2)
    p.branch(new == 0, false);
    if new != 0 {
        m.set_l(o + 0x28, 0);
        m.set_l(o + 8, new);
        m.set_l(o + 0x16, new.wrapping_sub(8));
        m.set_w(o + 0x1a, 0);
        p.ins(op::MOVEA_L_DI); // move.l $28(a2), d4
        p.ins("prr^pww"); // clr.l $28(a2)
        p.ins("pw^wp"); // move.l d4, 8(a2)
        p.ins(op::ADDQ_L_AN); // subq.l #8, d4
        p.ins("pw^wp"); // move.l d4, $16(a2)
        p.ins(op::CLR_W_DI);
    }
    let left = m.w(o + 0x1a).wrapping_sub(1);
    m.set_w(o + 0x1a, left);
    p.ins("pr^pw"); // subq.w #1, $1a(a2)
    p.branch((left as i16) >= 0, false);
    if (left as i16) < 0 {
        let mut e = m.l(o + 0x16).wrapping_add(8);
        m.set_l(o + 0x16, e);
        p.ins("prr^pww"); // addq.l #8, $16(a2)
        p.ins(op::MOVEA_L_DI); // movea.l $16(a2), a3
        p.ins("rrp"); // tst.l (a3)
        let wraps = (m.l(e) as i32) < 0;
        p.branch(!wraps, true);
        if wraps {
            e = m.l(o + 8);
            m.set_l(o + 0x16, e);
            p.ins(op::MOVEA_L_DI); // movea.l 8(a2), a3
            p.ins("pw^wp"); // move.l a3, $16(a2)
        }
        m.set_w(o + 0x1a, m.w(e + 6));
        p.ins("prpwp"); // move.w 6(a3), $1a(a2)
    }
    let e = m.l(o + 0x16);
    m.set_l(o + 0x42, e);
    p.ins("prrpwwp"); // move.l $16(a2), $42(a2)
}

/// One object's block `a2` drawn into the buffer at `buffer`: clipped to the buffer's 352 × 200,
/// cut in with its mask; `+$10` keeps where it went (zero when nothing was drawn).
fn draw_object(m: &mut StoryMem, p: &mut Prog, a2: u32, buffer: u32) {
    let a3 = m.l(m.l(a2 + 0x16));
    let mut a1 = buffer;
    p.ins(op::MOVEA_L_DI); // movea.l $16(a2), a3
    p.ins("rrp"); // movea.l (a3), a3
    p.ins(op::REG); // movea.l d0, a1
    m.set_l(a2 + 0x1e, m.l(a3 + 0x1a));
    m.set_l(a2 + 0x22, m.l(a3 + 0x1e));
    m.set_w(a2 + 0x26, m.w(a3 + 0x22));
    p.ins("prrpwwp");
    p.ins("prrpwwp");
    p.ins("prpwp");
    let width = m.w(a3 + 4);
    let height = m.w(a3 + 6);
    let mut d7 = m.w(a2).wrapping_add(m.w(a3 + 8));
    let mut a4 = m.l(a3 + 0x24);
    m.set_l(a2 + 0x10, 0);
    p.ins(op::MOVE_W_DI_DN); // move.w 0(a2), d7
    p.ins(op::MOVE_W_DI_DN); // add.w 8(a3), d7
    p.ins(op::MOVEA_L_DI); // movea.l $24(a3), a4
    p.ins("prr^pww"); // clr.l $10(a2)
    let add_w = |m: &mut StoryMem, at: u32, v: u16| {
        let x = m.w(at).wrapping_add(v);
        m.set_w(at, x);
    };
    // d1 carries a value between the clips, as the original's register does.
    let mut d1: u16 = 0;
    // Right edge (`$170`: the buffer's 352 pixels and the shift's extra word).
    let d6 = d7.wrapping_add(width).wrapping_sub(0x170);
    p.ins(op::REG); // move.w d7, d6
    p.ins(op::MOVE_W_DI_DN); // add.w 4(a3), d6
    p.ins(op::ANDI_W_DN); // subi.w #$170, d6
    p.branch((d6 as i16) < 0, false);
    if (d6 as i16) >= 0 {
        p.ins(op::ANDI_W_DN); // cmpi.w #$160, d7
        let off = (d7.wrapping_sub(0x160) as i16) >= 0;
        p.branch(off, true);
        if off {
            return;
        }
        let d5 = width.wrapping_sub(d6) >> 4;
        let cut = ((width >> 4).wrapping_sub(d5)) << 1;
        add_w(m, a2 + 0x26, cut);
        add_w(m, a2 + 0x24, cut);
        d1 = m.w(a3 + 0x1a) & 0xffc0 | d5;
        m.set_w(a2 + 0x1e, d1);
        p.ins(op::MOVE_W_DI_DN); // move.w 4(a3), d5
        p.ins(op::REG); // sub.w d6, d5
        p.ins_idle("p", shift(4)); // lsr.w #4, d5
        p.ins(op::MOVE_W_DI_DN); // move.w 4(a3), d6
        p.ins_idle("p", shift(4)); // lsr.w #4, d6
        p.ins(op::REG); // sub.w d5, d6
        p.ins_idle("p", shift(1)); // lsl.w #1, d6
        p.ins(op::ADD_W_DN_DI);
        p.ins(op::ADD_W_DN_DI);
        p.ins(op::MOVE_W_DI_DN); // move.w $1a(a3), d1
        p.ins(op::ANDI_W_DN);
        p.ins(op::REG); // add.w d5, d1
        p.ins("pwp"); // move.w d1, $1e(a2)
    }
    let mut d5: u16 = 0;
    p.ins(op::REG); // clr.w d5
    p.ins(op::REG); // move.w d7, d6
    p.branch((d7 as i16) >= 0, false);
    if (d7 as i16) < 0 {
        // Left edge: whole words cut, and the rest drawn from the row above's end, so the
        // shifted first word falls left of the row.
        d1 = d7.wrapping_add(width);
        p.ins(op::REG); // move.w d7, d1
        p.ins(op::MOVE_W_DI_DN); // add.w 4(a3), d1
        p.ins(op::ANDI_W_DN); // cmpi.w #$10, d1
        let off = (d1.wrapping_sub(0x10) as i16) < 0;
        p.branch(off, true);
        if off {
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
        p.ins(op::REG); // neg.w d6
        p.ins(op::REG); // move.w d6, d1
        p.ins_idle("p", shift(4)); // lsr.w #4, d6
        p.ins(op::ADD_W_DN_DI); // sub.w d6, $1e(a2)
        p.ins_idle("p", shift(1)); // lsl.w #1, d6
        p.ins(op::REG); // ext.l d6
        p.ins("prr^pww"); // add.l d6, $20(a2)
        p.ins(op::ADDQ_L_AN); // adda.l d6, a4
        p.ins(op::ADD_W_DN_DI);
        p.ins(op::ADD_W_DN_DI);
        p.ins(op::ANDI_W_DN); // move.w #$f, d6
        p.ins(op::ANDI_W_DN); // andi.w #$f, d1
        p.ins(op::REG); // sub.w d1, d6
        p.ins(op::ANDI_W_DN); // move.w #$150, d7
        p.ins(op::REG); // add.w d6, d7
        p.ins(op::ANDI_W_DN); // move.w #2, d5
        p.ins(op::ANDI_W_DN); // addi.w #1, d7
    }
    a1 = a1.wrapping_add((d7 >> 3) as u32);
    p.ins_idle("p", shift(3)); // lsr.w #3, d7
    p.ins(op::ADDQ_L_AN); // adda.l d7, a1
    let mut y = m.w(a2 + 2).wrapping_add(m.w(a3 + 0xa));
    let d6 = y.wrapping_add(height).wrapping_sub(0xc8);
    p.ins(op::MOVE_W_DI_DN); // move.w 2(a2), d7
    p.ins(op::MOVE_W_DI_DN); // add.w $a(a3), d7
    p.ins(op::REG); // move.w d7, d6
    p.ins(op::MOVE_W_DI_DN); // add.w 6(a3), d6
    p.ins(op::ANDI_W_DN); // subi.w #$c8, d6
    p.branch((d6 as i16) < 0, false);
    if (d6 as i16) >= 0 {
        p.ins(op::MOVE_W_DI_DN); // cmp.w 6(a3), d6
        let off = (d6.wrapping_sub(height) as i16) >= 0;
        p.branch(off, true);
        if off {
            return;
        }
        let size = m.w(a2 + 0x1e);
        d1 = ((size >> 6).wrapping_sub(d6)) << 6;
        m.set_w(a2 + 0x1e, size & 0x3f | d1);
        p.ins(op::MOVE_W_DI_DN); // move.w $1e(a2), d1
        p.ins("ppr^pw"); // andi.w #$3f, $1e(a2)
        p.ins_idle("p", shift(6)); // lsr.w #6, d1
        p.ins(op::REG); // sub.w d6, d1
        p.ins_idle("p", shift(6)); // lsl.w #6, d1
        p.ins(op::ADD_W_DN_DI); // or.w d1, $1e(a2)
    }
    p.ins(op::REG); // tst.w d7
    p.branch((y as i16) >= 0, false);
    if (y as i16) < 0 {
        let d6 = y.wrapping_neg();
        p.ins(op::REG); // move.w d7, d6
        p.ins(op::REG); // neg.w d6
        p.ins(op::MOVE_W_DI_DN); // cmp.w 6(a3), d6
        let off = (d6.wrapping_sub(height) as i16) >= 0;
        p.branch(off, true);
        if off {
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
        p.ins(op::REG); // move.w d1, d5
        p.ins(op::MOVE_W_DI_DN); // move.w $1e(a2), d1
        p.ins("ppr^pw"); // andi.w #$3f, $1e(a2)
        p.ins_idle("p", shift(6)); // lsr.w #6, d1
        p.ins(op::REG); // sub.w d6, d1
        p.ins_idle("p", shift(6)); // lsl.w #6, d1
        p.ins(op::ADD_W_DN_DI); // or.w d1, $1e(a2)
        p.ins(op::MOVE_W_DI_DN); // move.w $14(a3), d1
        p.ins_idle("p", 17 + d6.count_ones()); // mulu.w d6, d1
        p.ins(op::REG); // ext.l d1
        p.ins("prr^pww"); // add.l d1, $20(a2)
        p.ins(op::ADDQ_L_AN); // adda.l d1, a4
        p.ins(op::REG); // move.w d5, d1
        p.ins(op::BCC_TAKEN); // bra.b
    } else {
        let at = (y as i16 as i32 * 2 - d5 as i32) as u32;
        a1 = a1.wrapping_add(m.w(ROWS.wrapping_add(at)) as u32);
        p.ins(op::REG); // ext.l d7
        p.ins(op::REG); // add.w d7, d7
        p.ins(op::REG); // sub.w d5, d7
        p.ins("nprp"); // move.w (a5,d7.w), d7
        p.ins(op::ADDQ_L_AN); // adda.l d7, a1
    }
    let _ = d1;
    m.set_l(a2 + 0x10, a1);
    p.ins("pw^wp"); // move.l a1, $10(a2)
    let size = m.w(a2 + 0x1e);
    let shift_x = (m.w(a2).wrapping_add(m.w(a3 + 8)) & 0xf) << 12;
    let modulo_cd = m.w(a2 + 0x26);
    let modulo_ab = m.w(a2 + 0x24);
    let mut g = m.l(a2 + 0x20);
    let stride = m.l(a3 + 0x16);
    p.ins(op::MOVE_W_DI_DN); // move.w $1e(a2), d5
    p.ins(op::MOVE_W_DI_DN); // move.w 0(a2), d6
    p.ins(op::MOVE_W_DI_DN); // add.w 8(a3), d6
    p.ins(op::REG); // swap d6
    p.ins_idle("p", 6); // lsr.l #4, d6
    p.ins_regs("pWp", &[(BLTCON1, shift_x)]);
    p.ins(op::ANDI_W_DN); // ori.w #$fca, d6
    p.ins_regs("pWp", &[(BLTCON0, shift_x | 0x0fca)]);
    p.ins_regs("pppW^Wp", &[(BLTAFWM, 0xffff), (BLTAFWM + 2, 0)]);
    p.ins_regs("prpWp", &[(BLTCMOD, modulo_cd)]);
    p.ins_regs("prpWp", &[(BLTDMOD, modulo_cd)]);
    p.ins_regs("prpWp", &[(BLTBMOD, modulo_ab)]);
    p.ins_regs("prpWp", &[(BLTAMOD, modulo_ab)]);
    p.ins(op::MOVEA_L_DI); // move.l $20(a2), d1
    p.ins(op::MOVEA_L_DI); // move.l $16(a3), d4
    for k in 0..5 {
        let b = Blit {
            con0: shift_x | 0x0fca,
            con1: shift_x,
            afwm: 0xffff,
            alwm: 0x0000,
            pt: [a4, g, a1, a1],
            modulo: [
                modulo_ab as i16,
                modulo_ab as i16,
                modulo_cd as i16,
                modulo_cd as i16,
            ],
            size,
        };
        b.run(m);
        p.wait_blit(false);
        if k >= 3 {
            p.ins(op::ADDQ_L_AN); // adda.l d3, a1
            p.ins(op::ADDQ_L_AN); // add.l d4, d1
        }
        p.ins_regs("pW^Wp", &long(BLTCPT, a1));
        p.ins_regs("pW^Wp", &long(BLTDPT, a1));
        p.ins_regs("pW^Wp", &long(BLTBPT, g));
        p.ins_regs("pW^Wp", &long(BLTAPT, a4));
        p.ins_regs("pWp", &[(BLTSIZE, size)]);
        if k < 2 {
            p.ins(op::ADDQ_L_AN);
            p.ins(op::ADDQ_L_AN);
        }
        a1 = a1.wrapping_add(PLANE);
        g = g.wrapping_add(stride);
    }
}

/// A word shift's internal work by `n` bits, after its prefetch.
fn shift(n: u32) -> u32 {
    1 + n
}
