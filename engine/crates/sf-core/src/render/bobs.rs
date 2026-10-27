//! Bobs in widescreen's side strips (real-widescreen DESIGN §2.9, D6).
//!
//! The display buffers are 384 pixels a row, so the original's drawer (`level::objects::bob_draw`)
//! clips or culls a bob past them; what it would have drawn there is the strips'. Each pass
//! records, for the buffer it draws, every bob in draw order ([`WideBob`]); the renderer draws the
//! shown buffer's list into the strips here, cookie-cut from the bob sheet with its mask as the
//! drawer's blits do, so a strip bob continues the centre's pixel for pixel.

use super::Mem;
use super::playfield::Field;
use crate::layout::{WideBob, WideBobs};

/// A bob's level x as drawn, less its level position and its frame's x offset: the drawer puts
/// a bob at buffer pixel `x + offset + 18` (`screen_positions`' `+ 3` and the camera's column),
/// and buffer pixel `p` shows level x `p - 16` (`playfield`).
const DX: i32 = 2;
/// The same for y: the drawer puts a bob on ring line `y - 12 + offset` (its camera less 12, the
/// ring row's line), and ring line `r` shows level row `r - 16`.
const DY: i32 = -28;

/// Where `bob`'s frame's first pixel shows in level space, and whether it is cut with its mask.
/// A background bob is copied from its 16-pixel column (the drawer clears the position's low
/// four bits and adds no x offset).
pub(super) fn placed(bob: &WideBob, mem: &Mem) -> (i32, i32) {
    let (x, y) = (bob.x as i32, bob.y as i32);
    let yoff = mem.word(bob.frame + 0xa) as i16 as i32;
    if bob.background {
        ((x + DX) & !15, y + yoff + DY)
    } else {
        (
            x + mem.word(bob.frame + 8) as i16 as i32 + DX,
            y + yoff + DY,
        )
    }
}

/// Whether level row `y` of `bob` is in the buffer as `list`'s pass left it: in the rows the
/// drawer covered (a background bob's part starts a row lower), and not in a map row the
/// scroller has drawn over since.
pub(super) fn drawn(bob: &WideBob, list: &WideBobs, y: i32) -> bool {
    let top = if bob.background {
        list.clean_top
    } else {
        list.top
    };
    (top as i32..list.bottom as i32).contains(&y) && !list.wiped(y)
}

/// Draw the bob whose frame record is at `frame` into `p` with its first pixel at level
/// position `at`, on the level rows `drawn` says only: cookie-cut with its mask (`masked`, minterm `$ca`), else copied whole (a
/// background bob, minterm `$f0`). The frame record holds its BLTSIZE at `+$1a`, its graphics at
/// `+$1c`, their modulo at `+$20`, the plane step at `+$16` and its mask at `+$24`; a masked bob's
/// blit has a last word of shift room its mask clears.
pub(crate) fn draw_bob(
    p: &mut Field,
    at: (i32, i32),
    drawn: &dyn Fn(i32) -> bool,
    frame: u32,
    mem: &Mem,
    masked: bool,
) {
    let size = mem.word(frame + 0x1a);
    let long = |a: u32| (mem.word(a) as u32) << 16 | mem.word(a + 2) as u32;
    let (height, words) = crate::front::blit::bltsize_words(size);
    let (src, mask, step) = (long(frame + 0x1c), long(frame + 0x24), long(frame + 0x16));
    let stride = (2 * words as i32 + mem.word(frame + 0x20) as i16 as i32) as u32;
    let shown = if masked { words - 1 } else { words };
    if shown == 0 {
        return;
    }
    let bytes = 2 * shown;
    for r in 0..height {
        let y = at.1 + r as i32;
        if !drawn(y) || !p.has_row(y) {
            continue;
        }
        let row = |a: u32| mem.bytes(a.wrapping_add(r.wrapping_mul(stride)), bytes);
        let m = if masked { Some(row(mask)) } else { None };
        let planes: [_; 5] =
            std::array::from_fn(|k| row(src.wrapping_add(step.wrapping_mul(k as u32))));
        for i in 0..16 * shown {
            let (byte, bit) = ((i / 8) as usize, 7 - (i % 8));
            if m.as_ref().is_some_and(|m| m[byte] >> bit & 1 == 0) {
                continue;
            }
            let v = (0..5).fold(0u8, |v, k| v | (planes[k][byte] >> bit & 1) << k);
            p.set(at.0 + i as i32, y, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{DisplayData, Mem};
    use super::*;
    use crate::layout::{VarId, WIDE_BOB_ENTRY, WIDE_BOB_HEAD, h0, h3};
    use crate::state::State;

    /// A frame record at `at`: two rows of `words` words of BLTSIZE from h3+0 (plane step `$400`,
    /// a 40-byte sheet row), its mask at h3+$2000, offsets `dx`, `dy`.
    fn frame(s: &mut State, at: u32, words: u16, dx: i16, dy: i16) {
        s.set_w(at + 8, dx as u16);
        s.set_w(at + 0xa, dy as u16);
        s.set_l(at + 0x16, 0x400);
        s.set_w(at + 0x1a, 2 << 6 | words);
        s.set_l(at + 0x1c, h3(0));
        s.set_w(at + 0x20, 40 - 2 * words);
        s.set_l(at + 0x24, h3(0x2000));
    }

    /// The sheet: plane 0 all set, plane 2 set on row 1; the mask `$f0f0` on every word.
    fn sheet(s: &mut State) {
        for r in 0..2 {
            for w in 0..3 {
                s.set_w(h3(r * 40 + 2 * w), 0xffff);
                s.set_w(h3(0x800 + r * 40 + 2 * w), if r == 1 { 0xffff } else { 0 });
                s.set_w(h3(0x2000 + r * 40 + 2 * w), 0xf0f0);
            }
        }
    }

    const F: u32 = h0(0x2b000);

    #[test]
    fn a_masked_bob_is_cookie_cut_and_its_shift_word_is_not_drawn() {
        let mut s = State::default();
        sheet(&mut s);
        frame(&mut s, F, 2, 0, 0);
        let data = DisplayData::default();
        let mem = Mem::new(&s, &data);
        let mut f = Field::blank(0, 0, 40, 3, 9);
        draw_bob(&mut f, (4, 0), &|_| true, F, &mem, true);
        let row: Vec<u8> = (0..40).map(|x| f.at(x, 0)).collect();
        // Pixels 4..20, mask 1111 0000 1111 0000: colour 1 where set, the field elsewhere.
        assert_eq!(&row[..4], &[9; 4]);
        assert_eq!(
            &row[4..20],
            &[1, 1, 1, 1, 9, 9, 9, 9, 1, 1, 1, 1, 9, 9, 9, 9]
        );
        assert_eq!(
            &row[20..],
            &[9; 20],
            "the shift word is the blit's, not the bob's"
        );
        assert_eq!((f.at(4, 1), f.at(8, 1)), (5, 9), "row 1: planes 0 and 2");
        assert_eq!(f.at(4, 2), 9);
        let mut f = Field::blank(0, 0, 40, 3, 9);
        draw_bob(&mut f, (4, 0), &|y| y == 1, F, &mem, true);
        assert_eq!(
            (f.at(4, 0), f.at(4, 1)),
            (9, 5),
            "only the rows the drawer covered"
        );
    }

    #[test]
    fn a_background_bob_is_copied_whole() {
        let mut s = State::default();
        sheet(&mut s);
        s.set_w(h3(2), 0);
        frame(&mut s, F, 2, 0, 0);
        let data = DisplayData::default();
        let mem = Mem::new(&s, &data);
        let mut f = Field::blank(0, 0, 40, 3, 9);
        draw_bob(&mut f, (0, 0), &|y| y < 1, F, &mem, false);
        let row: Vec<u8> = (0..40).map(|x| f.at(x, 0)).collect();
        assert_eq!(&row[..16], &[1; 16]);
        assert_eq!(&row[16..32], &[0; 16], "colour 0 is drawn: no mask");
        assert_eq!(&row[32..], &[9; 8]);
        assert_eq!(f.at(0, 1), 9, "row 1 is not one the drawer covered");
    }

    /// The shown buffer's list, in its order: a later bob over an earlier one; a background bob
    /// at its 16-pixel column with no x offset; a masked one at its position, offset and `DX`.
    #[test]
    fn the_list_is_drawn_in_order_where_the_drawer_puts_each_bob() {
        let mut s = State::default();
        sheet(&mut s);
        frame(&mut s, F, 2, 3, 30);
        let g = F + 0x30;
        frame(&mut s, g, 2, 3, 30);
        s.set_l(g + 0x1c, h3(0x800));
        let list = crate::layout::def(VarId::WideBobs0).addr;
        // The pass covered rows 0..200.
        s.set_w(list + 6, 200);
        let mut put = |k: u32, x: i16, y: i16, frame: u32, background: bool| {
            let at = list + WIDE_BOB_HEAD + k * WIDE_BOB_ENTRY;
            s.set_w(at, x as u16);
            s.set_w(at + 2, y as u16);
            s.set_l(at + 4, frame);
            s.set_w(at + 8, background as u16);
            s.set_w(list, k as u16 + 1);
        };
        // A background bob at x 21 (column 16), colour 1 then 5; over it a masked one at
        // 21 + 3 + 2, whose plane 0 is the sheet's plane 2 (colour 0, then 1).
        put(0, 21, 100, F, true);
        put(1, 21, 100, g, false);
        let data = DisplayData::default();
        let mem = Mem::new(&s, &data);
        let mut f = Field::blank(0, 90, 64, 20, 9);
        let row = super::super::Row {
            under: super::super::Under::Field {
                y: 0,
                x0: 0,
                buffer: 0,
            },
            priority: 0,
            sprites: [None; 8],
        };
        f.draw_bobs(&s, &mem, &[row]);
        let y = 100 + 30 + DY;
        assert_eq!(
            (f.at(15, y), f.at(16, y), f.at(47, y), f.at(48, y)),
            (9, 1, 1, 9)
        );
        // Row 1 of the masked bob from x 26: its mask set on its first four pixels.
        assert_eq!(
            (25..34).map(|x| f.at(x, y + 1)).collect::<Vec<_>>(),
            [5, 1, 1, 1, 1, 5, 5, 5, 5]
        );
        // Its row 0 is colour 0, drawn where the mask is set.
        assert_eq!((f.at(26, y), f.at(30, y)), (0, 1));
    }
}
