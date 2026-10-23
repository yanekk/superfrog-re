//! The 25 Hz pass's own routines (`level_frame_loop`, h0+$aa6), besides the objects': the buffer
//! flip, the tile animation queue, the secret passage, and the checks for pause and time.

use crate::frog::{FROG, f};
use crate::layout::h0;
use crate::level::collision::{MAP_ROWS, OBJ_WORLD_OFFSET, PASSAGE_CELL, marker};
use crate::level::scroll::{BUFFER_BYTES, SHOWN_BUFFER};
use crate::level::beam::{self, Cost};
use crate::level::{LevelCtx, n_clear};
use crate::mem::{Bus, TILE_PTRS};

/// `vbl_pass_flag` (h0+$9da): the vertical blank sets it, the flip clears it.
const PASS_FLAG: u32 = h0(0x9da);
/// `bob_buffer` (h0+$18be4): which display buffer the bobs were drawn into this pass.
const BOB_BUFFER: u32 = h0(0x18be4);
/// `ring_rows` (h0+$174d6).
const RING_ROWS: u32 = h0(0x174d6);
/// One plane of a display buffer.
const PLANE: u32 = 0x3b70;

/// h0+$172b4: the buffer the bobs were just drawn into is shown from the next vertical blank.
/// A frame may end before it (step 0, `level_frame_loop`'s call) or after each of its writes
/// (steps 1–3, [`LevelCtx::step_done`]): the next vertical blank latches the buffer then shown.
pub fn buffer_flip(c: &mut LevelCtx) {
    c.bus.set_w(PASS_FLAG, 0);
    c.step_done();
    c.bus.set_w(SHOWN_BUFFER, 1);
    c.step_done();
    if c.bus.w(BOB_BUFFER) != 0 {
        c.bus.set_w(SHOWN_BUFFER, 2);
    }
    c.step_done();
}

/// The ring position of map cell `cell` in display buffer 0 (`(cell − map_rows) / $260` gives
/// the ring row and the column, h0+$faa6).
fn ring_pos(b: &Bus, cell: u32) -> u32 {
    let d = cell.wrapping_sub(b.l(MAP_ROWS));
    let (q, r) = (d / 0x260, d % 0x260);
    let (row, rem) = if q > 0xffff {
        // divu overflow: the register keeps the dividend.
        (d as u16, (d >> 16) as u16)
    } else {
        (q as u16, r as u16)
    };
    let col = ((rem as i16 as i32 as u32) >> 1) as u16;
    let col = col.wrapping_add(col) as i16 as i32 as u32;
    let row4 = (row as i16 as i32 as u32 as u16).wrapping_mul(4);
    b.l(RING_ROWS.wrapping_add(row4 as i16 as i32 as u32))
        .wrapping_add(col)
        .wrapping_add(2)
}

/// Draw tile `tile` into the clean buffer at the ring position of `cell` and copy it into both
/// display buffers with the blitter (h0+$faa6..$fe30, h0+$7aaa): the cell's look changes on
/// screen at once, in the shown buffer too.
pub fn redraw_cell(c: &mut LevelCtx, cell: u32, tile: u16) {
    let b = &mut c.bus;
    let buffer0 = ring_pos(b, cell);
    let clean = buffer0.wrapping_add(2 * BUFFER_BYTES);
    let mut src = b.l(TILE_PTRS.wrapping_add((tile.wrapping_mul(4)) as i16 as i32 as u32));
    let mut a1 = clean;
    for p in 0..5 {
        if p > 0 {
            a1 = a1.wrapping_add(PLANE);
        }
        for line in 0..16u32 {
            b.set_w(a1.wrapping_add(0x30 * line), b.w(src));
            src = src.wrapping_add(2);
        }
    }
    c.spend(Cost::Redraw);
    c.custom(0x9a, 0x0010);
    c.custom(0x96, 0x8400);
    for dst in [buffer0, buffer0.wrapping_add(BUFFER_BYTES)] {
        for p in 0..5u32 {
            c.spend(Cost::RedrawPlane);
            c.blit_words(16, 0x09f0);
            let (s, d) = (clean.wrapping_add(PLANE * p), dst.wrapping_add(PLANE * p));
            for line in 0..16u32 {
                let v = c.bus.w(s.wrapping_add(0x30 * line) & 0x7_fffe);
                c.bus.set_w(d.wrapping_add(0x30 * line) & 0x7_fffe, v);
            }
        }
    }
    c.custom(0x96, 0x0400);
    c.custom(0x9a, 0x8010);
}

/// `tile_anim_queue` (h0+$fa68): four cells waiting to show their next tile; an end mark follows.
pub const TILE_ANIM_QUEUE: u32 = h0(0xfa68);

/// `tile_anim_draw` (h0+$fa7c): each queued cell steps to the next tile (its marker cleared) and
/// is redrawn. A frame may end before it (step 0) or after each slot (steps 1–4,
/// [`LevelCtx::step_done`]): a slot is freed only once its turn comes, and the next vertical
/// blank's pickups (`take_tile`) queue into the slots then free (T29: W2-4 7658, the original's
/// frame ending at h0+$fa8c with the queue full, its second coin taken a frame later).
pub fn tile_anim_draw(c: &mut LevelCtx) {
    c.walk_step(0);
    let mut slot = TILE_ANIM_QUEUE;
    loop {
        let cell = c.bus.l(slot);
        slot += 4;
        if cell == 0xffff_ffff {
            return;
        }
        if cell != 0 {
            c.bus.set_l(slot - 4, 0);
            let tile = (c.bus.w(cell) >> 6).wrapping_add(1);
            c.bus.set_w(cell, tile.wrapping_shl(6));
            redraw_cell(c, cell, tile);
        }
        c.step_done();
    }
}

/// `passage_toggle` (h0+$7770).
const PASSAGE_TOGGLE: u32 = h0(0x7770);
/// `passage_tiles` (h0+$7876): per world table, a pointer to its eight passage tiles.
const PASSAGE_TILES: u32 = h0(0x7876);

/// Markers a passage digs through: solid ground (`$3e`) and fake wall (`$26`).
fn diggable(m: u16) -> bool {
    m == 0x3e || m == 0x26
}

/// `passage_step` (h0+$7772): while the frog is in a secret passage (marker 38, docs/07), each
/// pass opens the passage's column at the current cell, up and down, then moves along it; at
/// its end the frog is free again.
pub fn passage_step(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(FROG + f::IN_PASSAGE) == 0 {
        return;
    }
    let cy = h0(0x16662);
    b.set_w(cy, b.w(cy).wrapping_sub(2));
    let t = !b.w(PASSAGE_TOGGLE);
    b.set_w(PASSAGE_TOGGLE, t);
    if t != 0 {
        b.set_w(cy, b.w(cy).wrapping_add(2));
    }
    let mut a1 = c.bus.l(PASSAGE_CELL).wrapping_add(0x260);
    loop {
        a1 = a1.wrapping_sub(0x260);
        if !diggable(marker(&c.bus, a1)) {
            break;
        }
        dig(c, a1);
    }
    let mut a1 = c.bus.l(PASSAGE_CELL);
    loop {
        a1 = a1.wrapping_add(0x260);
        if !diggable(marker(&c.bus, a1)) {
            break;
        }
        dig(c, a1);
    }
    let a1 = c.bus.l(PASSAGE_CELL).wrapping_add(2);
    for next in [a1, a1.wrapping_sub(4)] {
        if diggable(marker(&c.bus, next)) {
            c.bus.set_l(PASSAGE_CELL, next);
            return;
        }
    }
    let b = &mut c.bus;
    b.set_l(PASSAGE_CELL, 0);
    b.set_w(FROG + f::CAMERA_HELD, 0);
    b.set_w(FROG + f::FROZEN, 0);
    b.set_w(FROG + f::IN_PASSAGE, 0);
    c.sfx(0x16, 2);
    c.sample(1);
}

/// h0+$7a34: open the passage at `cell`: its neighbours' edges are mended, and it takes the
/// passage tile its surroundings call for.
fn dig(c: &mut LevelCtx, cell: u32) {
    let b = &c.bus;
    let k = (b.l(OBJ_WORLD_OFFSET) as u16 >> 8).wrapping_mul(4);
    let tiles = b.l(PASSAGE_TILES.wrapping_add(k as u32));
    // h0+$79b2: the neighbours.
    for (dm, at, idx) in [
        (0x3d, cell.wrapping_sub(2), 0x14),
        (0x3d, cell.wrapping_add(2), 0x4),
        (0x3f, cell.wrapping_add(0x260), 0x10),
        (0x3d, cell.wrapping_sub(0x260), 0x1c),
    ] {
        if marker(&c.bus, at) == dm {
            let t = c.bus.l(tiles + idx);
            set_cell_tile(c, at, t as u16);
        }
    }
    let b = &c.bus;
    let pick = if marker(b, cell) == 0x3e {
        8
    } else if marker(b, cell.wrapping_sub(2)) == 0x3d {
        0x18
    } else if marker(b, cell.wrapping_sub(0x260)) == 0x3d {
        0xc
    } else {
        0
    };
    let t = c.bus.l(tiles + pick);
    set_cell_tile(c, cell, t as u16);
}

/// h0+$7a40: `cell` shows tile `tile`, keeping its marker (a fake wall's is cleared), and is
/// redrawn if it is on or near the screen.
fn set_cell_tile(c: &mut LevelCtx, cell: u32, tile: u16) {
    let b = &mut c.bus;
    let m = b.w(cell) & 0x3f;
    let mut v = tile.wrapping_shl(6) | m;
    if m == 0x26 {
        v &= 0xffc0;
    }
    b.set_w(cell, v);
    let tile = v >> 6;
    let d = cell.wrapping_sub(b.l(MAP_ROWS));
    let (q, r) = (d / 0x260, d % 0x260);
    let (row, rem) = if q > 0xffff { (d as u16, (d >> 16) as u16) } else { (q as u16, r as u16) };
    let x = rem.wrapping_shl(3);
    let y = row.wrapping_shl(4);
    let left = b.w(h0(0x16666)).wrapping_sub(0x20);
    let top = b.w(h0(0x16662));
    let visible = !n_clear(left.wrapping_sub(x))
        && n_clear(left.wrapping_add(0x180).wrapping_sub(x))
        && !n_clear(top.wrapping_sub(y))
        && n_clear(top.wrapping_add(0x120).wrapping_sub(y));
    if visible {
        redraw_cell(c, cell, tile);
    }
}

/// `key_down` (h0+$16078), `logic_running` (h0+$9d6).
const KEY_DOWN: u32 = h0(0x16078);
const LOGIC_RUNNING: u32 = h0(0x9d6);
/// The raw key P.
const KEY_P: u8 = 0x19;

/// Where `pause` (h0+$7442) is: it keeps the main program in the middle of the pass, frame after
/// frame, `logic_running` clear (so the vertical blank does only its sound, pointer, keyboard,
/// timer and fades). Each holds the VPOS low byte it read past line `$38`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Pause {
    /// Waiting for the beam past line `$38` (VPOS's low byte, unsigned).
    Line,
    /// Waiting for fire to be let go.
    Release(u8),
    /// Waiting for P (a new `key_down`) or fire.
    Held(u8),
    /// Over: waiting for the beam on the line byte read at the start.
    Back(u8),
}

/// h0+$71d8: P pauses the game (`pause`, h0+$7442): `logic_running` cleared, the pass stopped
/// here until [`pause_run`] says it is over.
pub fn pause_check(c: &mut LevelCtx) -> bool {
    if c.bus.b(KEY_DOWN) != KEY_P {
        return false;
    }
    c.bus.set_w(LOGIC_RUNNING, 0);
    c.bus.st.pause = Some(Pause::Line);
    pause_run(c);
    true
}

/// `pause` from where it is: until the frame's end (false: the vertical blank comes, the pause
/// goes on in the next frame), or until it is over (true): the line it waited for at the start
/// come round again, `key_down` cleared and `logic_running` set, the pass to go on.
pub fn pause_run(c: &mut LevelCtx) -> bool {
    loop {
        let Some(p) = c.bus.st.pause else {
            return true;
        };
        let next = match p {
            // h0+$7448.
            Pause::Line => match line_where(c, |v| v >= 0x38) {
                Some(line) => {
                    wait_to(c, line);
                    Pause::Release(line as u8)
                }
                None => return false,
            },
            // h0+$7458.
            Pause::Release(v) => {
                if c.hw.fire {
                    return false;
                }
                c.bus.set_b(KEY_DOWN, 0);
                Pause::Held(v)
            }
            // h0+$7468.
            Pause::Held(v) => {
                if c.bus.b(KEY_DOWN) != KEY_P && !c.hw.fire {
                    return false;
                }
                Pause::Back(v)
            }
            // h0+$747c.
            Pause::Back(v) => match line_where(c, |b| b == v) {
                Some(line) => {
                    wait_to(c, line);
                    c.bus.set_b(KEY_DOWN, 0);
                    c.bus.set_w(LOGIC_RUNNING, 1);
                    c.bus.st.pause = None;
                    return true;
                }
                None => return false,
            },
        };
        c.bus.st.pause = Some(next);
    }
}

/// The first line from the beam's on, in this frame, whose VPOS low byte is `ok`.
fn line_where(c: &LevelCtx, ok: impl Fn(u8) -> bool) -> Option<u16> {
    (c.clock.line..beam::FRAME_LINES).find(|&l| ok(l as u8))
}

/// The CPU waits for the beam to reach `line`.
fn wait_to(c: &mut LevelCtx, line: u16) {
    c.wait_lines(line - c.clock.line);
}
