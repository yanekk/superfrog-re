//! `load_hero` (h0+$9172), which the boot and the end sequence call: the frog's graphics (`HERO`,
//! four planes of 384 rows) and a mirrored copy of them cut into attached hardware-sprite frames
//! at h3+$4c0b0 (`hero_sprite_frames` h0+$19ce0), with their table at h0+$19ddc. The original
//! keeps `HERO` and its mirror in hunk 1 (h1+$0, h1+$f000); the engine reads them from the file.

use super::{Cx, at};
use crate::layout::{VarId, def};

/// `hero_rects` (h0+$1a6f4): a 12-byte rectangle per frame, `+0` x, `+2` y, `+6` rows, `+8` a long
/// the table keeps; the frog's 96, then its mirror image's 96.
const HERO_RECTS: u32 = at(0x1a6f4);
/// The table's two interleaved lists: the frog's records, its mirror image's 12 bytes on.
const TABLE: [u32; 2] = [at(0x19ddc), at(0x19de8)];
/// One plane of `HERO`.
const PLANE: usize = 0x3c00;

/// The whole of `load_hero` once its load is over, at once.
pub(super) fn load_hero(cx: &mut Cx) {
    // $172d6: h1+$20d00..$30fe0 cleared, the level's map with it.
    cx.s.map.fill(0);
    let hero = cx.data.file("HERO").to_vec();
    // Without the disks (a test's empty data) there is no table to walk.
    if hero.is_empty() {
        return;
    }
    // $19cb2: every long bit-reversed, so 32-pixel strips mirrored in place.
    let mirror: Vec<u8> = hero
        .chunks(4)
        .flat_map(|l| {
            let mut b = [0u8; 4];
            b[..l.len()].copy_from_slice(l);
            u32::from_be_bytes(b).reverse_bits().to_be_bytes()
        })
        .collect();
    let frames = def(VarId::HeroFrames);
    cx.s.write(frames.addr, &vec![0; frames.width.len() as usize]);
    // $19d8a: each record points at its frames, (rows + 1) * 16 bytes on from the last.
    let (mut rect, mut to) = (HERO_RECTS, frames.addr);
    for list in TABLE {
        let mut rec = list;
        loop {
            let rows = cx.w(rect + 6);
            let size = ((rows.wrapping_add(1) as u8 as i8 as i16 as u16) as u32) << 4;
            cx.s.set_l(rec, to);
            to = to.wrapping_add(size);
            let v = cx.l(rect + 8);
            cx.s.set_l(rec + 4, v);
            cx.s.set_w(rec + 8, rows);
            rect += 12;
            rec += 24;
            if (cx.l(rec) as i32) < 0 {
                break;
            }
        }
    }
    // $19d1e: each rectangle's two 16-pixel columns, each as two attached sprites (planes 0 and
    // 1, then 2 and 3): a word of each plane a row, then a free long.
    let mut rect = HERO_RECTS;
    for (list, image) in TABLE.into_iter().zip([&hero, &mirror]) {
        let mut rec = list;
        loop {
            let mut out = cx.l(rec);
            let x = cx.w(rect) as i16 as i32 as u32;
            let y = cx.w(rect + 2) as u32;
            let rows = cx.w(rect + 6);
            let row = (y.wrapping_mul(0x28) as u16 as i16 as i32) as u32;
            let column = row.wrapping_add((x >> 4).wrapping_mul(2));
            for c in [column, column.wrapping_add(2)] {
                for pair in [0, 2 * PLANE as u32] {
                    let mut from = c.wrapping_add(pair);
                    for _ in 0..rows {
                        for plane in [0, PLANE as u32] {
                            let o = from.wrapping_add(plane) as usize;
                            let w = [
                                image.get(o).copied().unwrap_or(0),
                                image.get(o + 1).copied().unwrap_or(0),
                            ];
                            cx.s.write(out, &w);
                            out = out.wrapping_add(2);
                        }
                        from = from.wrapping_add(0x28);
                    }
                    out = out.wrapping_add(4);
                }
            }
            rect += 12;
            rec += 24;
            if (cx.l(rec) as i32) < 0 {
                break;
            }
        }
    }
}
