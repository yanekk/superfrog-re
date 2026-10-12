//! The objects' side of the 25 Hz pass (docs/09): the spud, the slots' updates, hitboxes and
//! collisions, and the bobs. T14 runs a level with no object kinds registered (T15–T18 add them);
//! what runs here is what the original does whatever the objects: the frog's platform flag, the
//! hitbox list, the bob draw list and the bob drawer, which erases and draws every bob into the
//! back display buffer.

use crate::frog::{FROG, f};
use crate::layout::{h0, h3};
use crate::level::camera::{SHOOTER_MODE, SPRITES_HIDDEN};
use crate::level::{LevelCtx, n_clear};
use crate::mem::Bus;

/// `obj_slots` (h0+$1ba6c): 20 slots of `$7c` bytes.
pub const OBJ_SLOTS: u32 = h0(0x1ba6c);
const SLOT_SIZE: u32 = 0x7c;
const SLOTS: u32 = 20;
/// `hitboxes` (h0+$d350): 12-byte entries (x1, x2, y1, y2, slot), ending with a negative word.
pub const HITBOXES: u32 = h0(0xd350);
/// `spud_bob` (h0+$297b6): the spud's bob record; its level position is the long before it.
pub const SPUD_BOB: u32 = h0(0x297b6);
/// `bob_draw_list` (h0+$2970e): the bobs to draw, ending with 0; the long before it is 0 too.
pub const BOB_DRAW_LIST: u32 = h0(0x2970e);
/// `bob_free_list` (h0+$1ba16).
pub const BOB_FREE_LIST: u32 = h0(0x1ba16);
/// The spud's parked frame (`bob_records+$162c`).
const SPUD_PARKED: u32 = h0(0x2aefe);

/// `spud_update` (h0+$80f0): with no spud to throw, it is parked far off at x `$639c`.
pub fn spud_update(c: &mut LevelCtx) {
    let b = &mut c.bus;
    debug_assert_eq!(b.w(SHOOTER_MODE), 0, "the bonus level's spud is T19's");
    let off = b.w(FROG + f::SPUD_ON) == 0 || b.w(SPRITES_HIDDEN) != 0;
    debug_assert!(off, "a thrown spud is not ported (T15)");
    if off {
        park_spud(b);
    }
}

/// h0+$82d4: the spud out of play.
fn park_spud(b: &mut Bus) {
    b.set_w(FROG + 0x72, 0);
    b.set_w(FROG + 0x74, 0x639c);
    b.set_l(SPUD_BOB - 4, b.l(FROG + 0x74));
    b.set_l(SPUD_BOB + 0x28, SPUD_PARKED);
    b.set_l(SPUD_BOB + 0x42, SPUD_PARKED);
}

/// `obj_run_all` (h0+$1b324): each live slot's update. No kind is registered in T14; the
/// frog-only cases freeze the original's objects, so no slot is live.
pub fn obj_run_all(c: &mut LevelCtx) {
    for k in 0..SLOTS {
        let slot = OBJ_SLOTS + k * SLOT_SIZE;
        debug_assert_eq!(c.bus.l(slot + 0x40), 0, "objects are T15–T18's");
    }
}

/// `obj_build_hitboxes` (h0+$d2f6): every live slot's hitboxes, offset by its position.
pub fn obj_build_hitboxes(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let mut out = HITBOXES;
    for k in 0..SLOTS {
        let slot = OBJ_SLOTS + k * SLOT_SIZE;
        if b.l(slot + 0x40) == 0 {
            continue;
        }
        let mut hb = b.l(slot + 0x14);
        if hb == 0 {
            continue;
        }
        loop {
            let (dx, dy) = (b.w(hb + 4), b.w(hb + 6));
            let x1 = b.w(slot + 8).wrapping_add(dx);
            b.set_w(out, x1);
            b.set_w(out + 2, x1.wrapping_add(b.w(hb)));
            let y1 = b.w(slot + 0xa).wrapping_add(dy);
            b.set_w(out + 4, y1);
            b.set_w(out + 6, y1.wrapping_add(b.w(hb + 2)));
            b.set_l(out + 8, slot);
            out += 12;
            hb = hb.wrapping_add(8);
            if !n_clear(b.w(hb)) {
                break;
            }
        }
    }
    b.set_w(out, 0xffff);
}

/// Walk the hitbox list for boxes overlapping x `x1..x2`, y `y1..y2` (the original's tests: an
/// entry is skipped when it ends left of or starts right of the box, and likewise down).
fn overlapping(b: &Bus, x1: u16, x2: u16, y1: u16, y2: u16) -> Vec<u32> {
    let mut hits = Vec::new();
    let mut at = HITBOXES;
    while n_clear(b.w(at)) {
        let (ex1, ex2, ey1, ey2) = (b.w(at), b.w(at + 2), b.w(at + 4), b.w(at + 6));
        let slot = b.l(at + 8);
        at += 12;
        if !n_clear(ex2.wrapping_sub(x1))
            || n_clear(ex1.wrapping_sub(x2))
            || !n_clear(ey2.wrapping_sub(y1))
            || n_clear(ey1.wrapping_sub(y2))
        {
            continue;
        }
        hits.push(slot);
    }
    hits
}

/// `collide_all` (h0+$d0cc): the spud against the objects, then the frog (which is off any
/// platform until an object says otherwise). A touch runs the object's handler, which no kind
/// in T14 has.
pub fn collide_all(c: &mut LevelCtx) {
    let b = &c.bus;
    let (sx, sy) = (b.w(FROG + 0x74), b.w(FROG + 0x76));
    let hits = overlapping(b, sx, sx.wrapping_add(0x10), sy, sy.wrapping_add(0x10));
    debug_assert!(hits.is_empty(), "spud touches are T15–T18's");
    let b = &mut c.bus;
    if b.w(SPRITES_HIDDEN) == 0 {
        b.set_w(FROG + f::ON_PLATFORM, 0);
        let x1 = b.w(FROG + f::X).wrapping_add(4).wrapping_add(0xe);
        let mut y1 = b.w(FROG + f::Y).wrapping_add(4).wrapping_add(0x16);
        let (x2, y2) = (x1.wrapping_add(0xa), y1.wrapping_add(0x20));
        if b.w(FROG + f::CROUCH) != 0 {
            y1 = y1.wrapping_add(0xb);
        }
        let hits = overlapping(b, x1, x2, y1, y2);
        debug_assert!(hits.is_empty(), "frog touches are T15–T18's");
    }
    debug_assert_eq!(c.bus.w(SHOOTER_MODE), 0, "the bonus level's collisions are T19's");
}

/// `build_bob_draw_list` (h0+$19b42): the spud, the slots' bobs (those in front first), then
/// the free bobs that still have something on screen to erase.
pub fn build_bob_draw_list(c: &mut LevelCtx) {
    let b = &mut c.bus;
    let mut out = BOB_DRAW_LIST;
    b.set_l(out, SPUD_BOB);
    out += 4;
    for front in [true, false] {
        for k in 0..SLOTS {
            let slot = OBJ_SLOTS + k * SLOT_SIZE;
            let bob = b.l(slot + 0x50);
            if bob != 0 && ((b.l(slot + 0x26) as i32) >= 0) == front {
                b.set_l(out, bob);
                out += 4;
            }
        }
    }
    for k in 0..SLOTS {
        let bob = b.l(BOB_FREE_LIST + 4 * k);
        if bob == 0 {
            continue;
        }
        if [0x10, 0x56, 0x9c, 0xe2].iter().any(|&o| b.l(bob + o) != 0) {
            b.set_l(out, bob);
            out += 4;
        }
    }
    b.set_l(out, 0);
}

/// The bob drawer's variables (h0+$18bca..$18be4).
const BD_CAMERA_X: u32 = h0(0x18bca);
const BD_CAMERA_Y: u32 = h0(0x18bce);
/// The camera's low words, as the drawer reads them.
const BD_CX: u32 = h0(0x18bcc);
const BD_CY: u32 = h0(0x18bd0);
/// The play field's lines the current part covers: top and bottom (screen lines; the ring seam
/// splits the drawing in two), and the same for drawing into the clean buffer.
const BD_TOP: u32 = h0(0x18bd2);
const BD_BOTTOM: u32 = h0(0x18bd4);
const BD_CLEAN_TOP: u32 = h0(0x18bd6);
const BD_CLEAN_BOTTOM: u32 = h0(0x18bd8);
/// The column of the camera, as a byte offset.
const BD_COLUMN: u32 = h0(0x18bda);
/// Set while the seam's part is drawn.
const BD_SEAM: u32 = h0(0x18bde);
/// `bob_buffer`: toggles each pass, the buffer the bobs go into.
pub const BOB_BUFFER: u32 = h0(0x18be4);
/// `line_offsets` (h0+$18be8).
const LINE_OFFSETS: u32 = h0(0x18be8);
/// `ring_row_lines` (h0+$176c4).
const RING_ROW_LINES: u32 = h0(0x176c4);
const BUFFER_0: u32 = h3(0x10600);
const BUFFER_1: u32 = h3(0x22f30);
const BUFFER_BYTES: u32 = 0x12930;
const PLANE: u32 = 0x3b70;
const CLEAN: u32 = h3(0x35860);
/// A sub-record of a bob record (4 per bob, `$46` apart: the two buffers' main part and seam
/// part).
const SUB: u32 = 0x46;

/// One blit of the drawer: BLTCON0/1, masks, the four pointers and modulos, BLTSIZE.
#[derive(Clone, Copy)]
struct Blit {
    con0: u16,
    con1: u16,
    afwm: u16,
    alwm: u16,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
    amod: u16,
    bmod: u16,
    cmod: u16,
    dmod: u16,
    size: u16,
}

/// The blitter in area mode, ascending (what the drawer uses): A shifted and masked, B shifted,
/// C, minterm, into D.
fn blit(b: &mut Bus, x: Blit) {
    let h = match x.size >> 6 {
        0 => 1024,
        h => h as u32,
    };
    let w = match x.size & 0x3f {
        0 => 64,
        w => w as u32,
    };
    let (use_a, use_b, use_c, use_d) = (
        x.con0 & 0x800 != 0,
        x.con0 & 0x400 != 0,
        x.con0 & 0x200 != 0,
        x.con0 & 0x100 != 0,
    );
    let (ash, bsh) = ((x.con0 >> 12) as u32, (x.con1 >> 12) as u32);
    let mt = x.con0 as u8;
    let (mut pa, mut pb, mut pc, mut pd) = (x.a & 0x7_fffe, x.b & 0x7_fffe, x.c & 0x7_fffe, x.d & 0x7_fffe);
    let (mut aold, mut bold) = (0u16, 0u16);
    for _ in 0..h {
        for col in 0..w {
            let mut a = if use_a { b.w(pa) } else { 0 };
            if use_a {
                pa = pa.wrapping_add(2);
            }
            if col == 0 {
                a &= x.afwm;
            }
            if col == w - 1 {
                a &= x.alwm;
            }
            let bv = if use_b { b.w(pb) } else { 0 };
            if use_b {
                pb = pb.wrapping_add(2);
            }
            let cv = if use_c { b.w(pc) } else { 0 };
            if use_c {
                pc = pc.wrapping_add(2);
            }
            let ash_v = ((aold as u32) << 16 | a as u32) >> ash;
            let bsh_v = ((bold as u32) << 16 | bv as u32) >> bsh;
            let (sa, sb) = (ash_v as u16, bsh_v as u16);
            aold = a;
            bold = bv;
            let mut d = 0u16;
            for (bit, term) in [
                (0x80, sa & sb & cv),
                (0x40, sa & sb & !cv),
                (0x20, sa & !sb & cv),
                (0x10, sa & !sb & !cv),
                (0x08, !sa & sb & cv),
                (0x04, !sa & sb & !cv),
                (0x02, !sa & !sb & cv),
                (0x01, !sa & !sb & !cv),
            ] {
                if mt & bit != 0 {
                    d |= term;
                }
            }
            if use_d {
                b.set_w(pd, d);
                pd = pd.wrapping_add(2);
            }
        }
        if use_a {
            pa = pa.wrapping_add(x.amod as i16 as i32 as u32) & 0x7_fffe;
        }
        if use_b {
            pb = pb.wrapping_add(x.bmod as i16 as i32 as u32) & 0x7_fffe;
        }
        if use_c {
            pc = pc.wrapping_add(x.cmod as i16 as i32 as u32) & 0x7_fffe;
        }
        if use_d {
            pd = pd.wrapping_add(x.dmod as i16 as i32 as u32) & 0x7_fffe;
        }
    }
}

/// `bob_draw` (h0+$19120): the bobs of the draw list, erased from the back buffer (copied back
/// from the clean buffer where they were last drawn) and drawn again (cookie-cut, minterm `$ca`,
/// their mask over the background), in two parts split at the display ring's seam; then the
/// buffers swap (`bob_buffer`).
pub fn bob_draw(c: &mut LevelCtx) {
    let b = &mut c.bus;
    b.set_l(BD_CAMERA_X, b.l(h0(0x16664)));
    b.set_l(BD_CAMERA_Y, b.l(h0(0x16660)));
    // The seam: the screen line where the ring wraps.
    let mut d1 = (b.w(BD_CY) as u32).wrapping_add(0xffff_fff4);
    let d3 = d1 & 0xffff_0000 | (d1 as u16 & 0xf) as u32;
    let row = ((d1 as u16 >> 4).wrapping_mul(4)) >> 1;
    d1 = d1 & 0xffff_0000 | row as u32;
    let d6 = 0x130u16
        .wrapping_sub(b.w(RING_ROW_LINES.wrapping_add(d1 as u16 as i16 as i32 as u32)))
        .wrapping_sub(d3 as u16);
    let seam = 0x130u16.wrapping_sub(d6);
    b.set_w(BD_TOP, seam);
    b.set_w(BD_BOTTOM, 0x130);
    let pushed = 0x130u16.wrapping_sub(0x130u16.wrapping_sub(seam)) & 0xfff0;
    b.set_w(BD_CLEAN_TOP, (b.w(BD_TOP) & 0xfff0).wrapping_add(0x10));
    b.set_w(BD_CLEAN_BOTTOM, b.w(BD_BOTTOM));
    // (h0+$1918e: the drawer waits for the vertical blank's flag, which a pass on time finds set.)
    screen_positions(b);
    b.set_w(BD_SEAM, 0);
    // h0+$19216: the column, and which buffer is the back one.
    let col = ((b.w(BD_CX).wrapping_add(0xf) >> 4).wrapping_mul(2)) as u32;
    b.set_l(BD_COLUMN, col);
    let (mut back, mut other) = (BUFFER_0 + col, BUFFER_1 + col);
    let mut sub = 0u32;
    let t = !b.w(BOB_BUFFER);
    b.set_w(BOB_BUFFER, t);
    if t != 0 {
        std::mem::swap(&mut back, &mut other);
        sub = SUB;
    }
    draw_part(c, back, sub);
    // The part below the seam.
    let b = &mut c.bus;
    b.set_w(BD_TOP, 0);
    b.set_w(BD_BOTTOM, if n_clear(pushed) { pushed } else { 0 });
    b.set_w(BD_CLEAN_TOP, b.w(BD_TOP));
    b.set_w(BD_CLEAN_BOTTOM, b.w(BD_BOTTOM));
    let mut at = BOB_DRAW_LIST;
    loop {
        let bob = b.l(at).wrapping_add(sub);
        at += 4;
        b.set_l(bob + 0x8c, b.l(bob));
        b.set_w(bob + 0x8e, b.w(bob + 0x8e).wrapping_sub(0x130));
        if b.l(at) == 0 {
            break;
        }
    }
    b.set_w(BD_SEAM, 0xffff);
    draw_part(c, back, sub + 0x8c);
}

/// h0+$1909c: each listed bob's screen position from its level position (the long before the
/// record) and the camera, the ring's line under the camera's top added.
fn screen_positions(b: &mut Bus) {
    let d1 = (b.w(BD_CY) as u32).wrapping_add(0xffff_fff4);
    let line = d1 as u16 & 0xf;
    let row = (((d1 as u16 & 0xfff0) >> 4).wrapping_mul(4)) >> 1;
    let d6 = b
        .w(RING_ROW_LINES.wrapping_add(row as i16 as i32 as u32))
        .wrapping_add(line);
    let d5 = !(b.w(BD_CX).wrapping_add(0xf)) & 0xf;
    let d7 = 0xfu16.wrapping_sub(d5);
    let (cx, cy) = (b.w(BD_CX), b.w(BD_CY));
    let mut at = BOB_DRAW_LIST;
    loop {
        let bob = b.l(at);
        at += 4;
        let x = b.w(bob.wrapping_sub(4)).wrapping_sub(cx).wrapping_add(3);
        b.set_w(bob, x);
        let y = b.w(bob.wrapping_sub(2)).wrapping_sub(cy);
        b.set_w(bob + 2, y.wrapping_add(d6));
        b.set_w(bob, b.w(bob).wrapping_add(d7));
        if b.l(at) == 0 {
            break;
        }
    }
}

/// h0+$19242: one part of the drawing into the back buffer at `back` (with the camera's column),
/// the bobs' sub-records `sub` on: first (unless this is the seam's part) each bob's last
/// drawing is erased, going down the list; then each is drawn, going back up it.
fn draw_part(c: &mut LevelCtx, back: u32, sub: u32) {
    c.custom(0x96, 0x8400);
    let mut at = BOB_DRAW_LIST;
    if c.bus.w(BD_SEAM) != 0 {
        while c.bus.l(at) != 0 {
            at += 4;
        }
    } else {
        loop {
            let rec = c.bus.l(at).wrapping_add(sub);
            at += 4;
            for part in [rec, rec + 0x8c] {
                let dest = c.bus.l(part + 0x10);
                if dest != 0 {
                    erase(c, part, dest);
                }
                c.bus.set_l(part + 0x10, 0);
            }
            if c.bus.l(at) == 0 {
                break;
            }
        }
    }
    // h0+$1940e: draw, going back up the list.
    loop {
        at -= 4;
        let bob = c.bus.l(at);
        draw_bob(c, bob, back, sub);
        if c.bus.l(at - 4) == 0 {
            break;
        }
    }
    c.custom(0x96, 0x0400);
}

/// h0+$1928c: copy the clean background back over a bob's last drawing (five planes).
fn erase(c: &mut LevelCtx, part: u32, dest: u32) {
    let b = &mut c.bus;
    let size = b.w(part + 0x1e);
    let modulo = b.w(part + 0x26);
    let mut src = dest.wrapping_add(BUFFER_BYTES);
    if !n_clear_l(b.l(part + 0x2c).wrapping_sub(BUFFER_1)) {
        src = src.wrapping_add(BUFFER_BYTES);
    }
    c.custom(0x9a, 0x0010);
    for p in 0..5 {
        blit(
            &mut c.bus,
            Blit {
                con0: 0x09f0,
                con1: 0,
                afwm: 0xffff,
                alwm: 0xffff,
                a: src.wrapping_add(PLANE * p),
                b: 0,
                c: 0,
                d: dest.wrapping_add(PLANE * p),
                amod: modulo,
                bmod: 0,
                cmod: 0,
                dmod: modulo,
                size,
            },
        );
    }
    c.custom(0x9a, 0x8010);
}

fn n_clear_l(v: u32) -> bool {
    (v as i32) >= 0
}

/// h0+$19410: one bob's sub-record `sub`: its animation steps (in the main part), then it is
/// clipped to the part of the screen being drawn and cookie-cut into the back buffer, or, for a
/// bob drawn into the background (`+4` negative, h0+$19708), copied into it.
fn draw_bob(c: &mut LevelCtx, bob: u32, back: u32, sub: u32) {
    let b = &mut c.bus;
    b.set_l(bob + SUB, b.l(bob));
    let rec;
    let mut frame_at;
    if b.w(BD_SEAM) != 0 {
        rec = bob.wrapping_add(sub);
        frame_at = b.l(rec.wrapping_sub(0x76));
    } else {
        if b.l(bob + 0x28) != 0 {
            let start = b.l(bob + 0x28);
            b.set_l(bob + 0x28, 0);
            b.set_l(bob + 8, b.l(bob + 0x42));
            b.set_l(bob + 0x16, start.wrapping_sub(8));
            b.set_w(bob + 0x1a, 0);
        }
        let left = b.w(bob + 0x1a).wrapping_sub(1);
        b.set_w(bob + 0x1a, left);
        if !n_clear(left) {
            b.set_l(bob + 0x16, b.l(bob + 0x16).wrapping_add(8));
            let mut step = b.l(bob + 0x16);
            if (b.l(step) as i32) < 0 {
                step = b.l(bob + 8);
                b.set_l(bob + 0x16, step);
            }
            b.set_w(bob + 0x1a, b.w(step.wrapping_add(6)));
        }
        b.set_l(bob + 0x5c, b.l(bob + 0x16));
        rec = bob.wrapping_add(sub);
        frame_at = b.l(rec + 0x16);
    }
    if b.w(BD_TOP) == b.w(BD_BOTTOM) {
        return;
    }
    if (b.l(rec + 4) as i32) < 0 {
        return draw_background(c, rec, frame_at, back);
    }
    let fr = b.l(frame_at);
    frame_at = fr;
    let a3 = frame_at;
    b.set_l(rec + 0x2c, back);
    let mut a1 = back;
    b.set_l(rec + 0x1e, b.l(a3 + 0x1a));
    b.set_l(rec + 0x22, b.l(a3 + 0x1e));
    b.set_w(rec + 0x26, b.w(a3 + 0x22));
    let mut d7 = b.w(rec).wrapping_add(b.w(a3 + 8));
    let mut a4 = b.l(a3 + 0x24);
    b.set_l(rec + 0x10, 0);
    let width = b.w(a3 + 4);
    let d6 = d7.wrapping_add(width).wrapping_sub(0x180);
    if n_clear(d6) {
        if n_clear(d7.wrapping_sub(0x170)) {
            return;
        }
        let d5 = width.wrapping_sub(d6) >> 4;
        let d6 = ((width >> 4).wrapping_sub(d5)).wrapping_mul(2);
        b.set_w(rec + 0x26, b.w(rec + 0x26).wrapping_add(d6));
        b.set_w(rec + 0x24, b.w(rec + 0x24).wrapping_add(d6));
        b.set_w(rec + 0x1e, (b.w(a3 + 0x1a) & 0xffc0).wrapping_add(d5));
    }
    let mut d5 = 0u16;
    if !n_clear(d7) {
        if !n_clear(d7.wrapping_add(width).wrapping_sub(0x10)) {
            return;
        }
        let neg = d7.wrapping_neg();
        let words = neg >> 4;
        b.set_w(rec + 0x1e, b.w(rec + 0x1e).wrapping_sub(words));
        let bytes = words.wrapping_mul(2);
        let ext = bytes as i16 as i32 as u32;
        b.set_l(rec + 0x20, b.l(rec + 0x20).wrapping_add(ext));
        a4 = a4.wrapping_add(ext);
        b.set_w(rec + 0x26, b.w(rec + 0x26).wrapping_add(bytes));
        b.set_w(rec + 0x24, b.w(rec + 0x24).wrapping_add(bytes));
        let d6 = 0xfu16.wrapping_sub(neg & 0xf);
        d7 = 0x170u16.wrapping_add(d6).wrapping_add(1);
        d5 = 2;
    }
    a1 = a1.wrapping_add((d7 >> 3) as u32);
    let mut y = b.w(rec + 2).wrapping_add(b.w(a3 + 0xa));
    let height = b.w(a3 + 6);
    let below = y.wrapping_add(height).wrapping_sub(b.w(BD_BOTTOM));
    if n_clear(below) {
        if n_clear(below.wrapping_sub(height)) {
            return;
        }
        let s = b.w(rec + 0x1e);
        b.set_w(rec + 0x1e, s & 0x3f);
        let d1 = ((s >> 6).wrapping_sub(below)).wrapping_shl(6);
        b.set_w(rec + 0x1e, b.w(rec + 0x1e) | d1);
    }
    y = y.wrapping_sub(b.w(BD_TOP));
    let offset_index;
    if !n_clear(y) {
        let cut = y.wrapping_neg();
        if n_clear(cut.wrapping_sub(height)) {
            return;
        }
        let s = b.w(rec + 0x1e);
        b.set_w(rec + 0x1e, s & 0x3f);
        let d1 = ((s >> 6).wrapping_sub(cut)).wrapping_shl(6);
        b.set_w(rec + 0x1e, b.w(rec + 0x1e) | d1);
        let skip = (b.w(a3 + 0x14) as u32 * cut as u32) as u16 as i16 as i32 as u32;
        b.set_l(rec + 0x20, b.l(rec + 0x20).wrapping_add(skip));
        a4 = a4.wrapping_add(skip);
        offset_index = b.w(BD_TOP).wrapping_mul(2).wrapping_sub(d5);
    } else {
        offset_index = y.wrapping_add(b.w(BD_TOP)).wrapping_mul(2).wrapping_sub(d5);
    }
    a1 = a1.wrapping_add(line_offset_long(b, offset_index));
    if (b.w(rec) as i16) < 0 && !n_clear(b.w(rec + 2).wrapping_sub(1)) {
        a1 = a1.wrapping_sub(0x30);
    }
    let size = b.w(rec + 0x1e);
    if size & 0x3f == 0 || size & 0xffc0 == 0 {
        return;
    }
    b.set_l(rec + 0x10, a1);
    let shift = (b.w(rec).wrapping_add(b.w(a3 + 8)) & 0xf).wrapping_shl(12);
    let (cmod, bmod) = (b.w(rec + 0x26), b.w(rec + 0x24));
    let mut src = b.l(rec + 0x20);
    let step = b.l(a3 + 0x16);
    c.custom(0x9a, 0x0010);
    for p in 0..5u32 {
        blit(
            &mut c.bus,
            Blit {
                con0: shift | 0x0fca,
                con1: shift,
                afwm: 0xffff,
                alwm: 0x0000,
                a: a4,
                b: src,
                c: a1.wrapping_add(PLANE * p),
                d: a1.wrapping_add(PLANE * p),
                amod: bmod,
                bmod,
                cmod,
                dmod: cmod,
                size,
            },
        );
        src = src.wrapping_add(step);
    }
    c.custom(0x9a, 0x8010);
}

/// The long the drawer adds for a line: `move.w (a5,d7.w),d7` after `ext.l d7` of a line that is
/// never negative there: the table's word.
fn line_offset_long(b: &Bus, index: u16) -> u32 {
    let word = b.w(LINE_OFFSETS.wrapping_add(index as i16 as i32 as u32)) as u32;
    word
}

/// h0+$19708: a bob drawn into the background: copied (no mask) into the back buffer, or into
/// the clean buffer when its frame's y offset is `$fc00`; nothing erases it.
fn draw_background(c: &mut LevelCtx, rec: u32, frame_at: u32, back: u32) {
    let b = &mut c.bus;
    let mut a1 = back;
    let a3 = b.l(frame_at);
    let to_clean = b.w(a3 + 0xa) == 0xfc00;
    if to_clean {
        a1 = CLEAN.wrapping_add(b.l(BD_COLUMN));
    }
    b.set_w(rec, b.w(rec) & 0xfff0);
    b.set_l(rec + 0x1e, b.l(a3 + 0x1a));
    b.set_l(rec + 0x22, b.l(a3 + 0x1e));
    b.set_w(rec + 0x26, b.w(a3 + 0x22));
    let mut d7 = b.w(rec);
    let width = b.w(a3 + 4);
    let d6 = d7.wrapping_add(width).wrapping_sub(0x170);
    if n_clear(d6) {
        if n_clear(d7.wrapping_sub(0x170)) {
            return;
        }
        let d5 = width.wrapping_sub(d6) >> 4;
        let d6 = ((width >> 4).wrapping_sub(d5)).wrapping_mul(2);
        b.set_w(rec + 0x26, b.w(rec + 0x26).wrapping_add(d6));
        b.set_w(rec + 0x24, b.w(rec + 0x24).wrapping_add(d6));
        b.set_w(rec + 0x1e, (b.w(a3 + 0x1a) & 0xffc0).wrapping_add(d5));
    }
    let mut d5 = 0u16;
    if !n_clear(d7) {
        if !n_clear(d7.wrapping_add(width)) {
            return;
        }
        let neg = d7.wrapping_neg();
        let words = neg >> 4;
        b.set_w(rec + 0x1e, b.w(rec + 0x1e).wrapping_sub(words));
        let bytes = words.wrapping_mul(2);
        b.set_l(rec + 0x20, b.l(rec + 0x20).wrapping_add(bytes as i16 as i32 as u32));
        b.set_w(rec + 0x26, b.w(rec + 0x26).wrapping_add(bytes));
        b.set_w(rec + 0x24, b.w(rec + 0x24).wrapping_add(bytes));
        let d6 = 0xfu16.wrapping_sub(neg & 0xf);
        d7 = 0x170u16.wrapping_add(d6).wrapping_add(1);
        d5 = 2;
    }
    a1 = a1.wrapping_add((d7 >> 3) as u32);
    let mut y = b.w(rec + 2);
    if !to_clean {
        y = y.wrapping_add(b.w(a3 + 0xa));
    }
    let height = b.w(a3 + 6);
    let below = y.wrapping_add(height).wrapping_sub(b.w(BD_CLEAN_BOTTOM));
    if n_clear(below) {
        if n_clear(below.wrapping_sub(height)) {
            return;
        }
        let s = b.w(rec + 0x1e);
        b.set_w(rec + 0x1e, s & 0x3f);
        let d1 = ((s >> 6).wrapping_sub(below)).wrapping_shl(6);
        b.set_w(rec + 0x1e, b.w(rec + 0x1e) | d1);
    }
    y = y.wrapping_sub(b.w(BD_CLEAN_TOP));
    let index;
    if !n_clear(y) {
        let cut = y.wrapping_neg();
        if n_clear(cut.wrapping_sub(height)) {
            return;
        }
        let s = b.w(rec + 0x1e);
        b.set_w(rec + 0x1e, s & 0x3f);
        let d1 = ((s >> 6).wrapping_sub(cut)).wrapping_shl(6);
        b.set_w(rec + 0x1e, b.w(rec + 0x1e) | d1);
        let skip = (b.w(a3 + 0x14) as u32 * cut as u32) as u16 as i16 as i32 as u32;
        b.set_l(rec + 0x20, b.l(rec + 0x20).wrapping_add(skip));
        index = b.w(BD_CLEAN_TOP).wrapping_mul(2).wrapping_sub(d5);
    } else {
        index = y.wrapping_add(b.w(BD_CLEAN_TOP)).wrapping_mul(2).wrapping_sub(d5);
    }
    a1 = a1.wrapping_add(line_offset_long(b, index));
    if (b.w(rec) as i16) < 0 && !n_clear(b.w(rec + 2).wrapping_sub(1)) {
        a1 = a1.wrapping_sub(0x30);
    }
    b.set_l(rec + 0x10, 0);
    let size = b.w(rec + 0x1e);
    if size & 0x3f == 0 || size & 0xffc0 == 0 {
        return;
    }
    let (dmod, amod) = (b.w(rec + 0x26), b.w(rec + 0x24));
    let mut src = b.l(rec + 0x20);
    let step = b.l(a3 + 0x16);
    let shift = (b.w(rec) & 0xf).wrapping_shl(12);
    c.custom(0x9a, 0x0010);
    for p in 0..5u32 {
        blit(
            &mut c.bus,
            Blit {
                con0: shift | 0x09f0,
                con1: shift,
                afwm: 0xffff,
                alwm: 0xffff,
                a: src,
                b: 0,
                c: 0,
                d: a1.wrapping_add(PLANE * p),
                amod,
                bmod: 0,
                cmod: 0,
                dmod,
                size,
            },
        );
        src = src.wrapping_add(step);
    }
    c.custom(0x9a, 0x8010);
}
