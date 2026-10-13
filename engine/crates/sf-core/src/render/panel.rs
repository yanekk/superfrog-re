//! Lines Denise fetches from memory: the status panel and the display buffers (docs/12 "Panel").
//!
//! The panel is five planes of 14 lines, `$2a0` bytes apart, shown with the same fetch as the
//! play field (23 words a line, modulo 2). Its two lowest lines fetch past the fifth plane into
//! what follows it in chip memory, the frog's sprite frames, as the original's do.

use super::{Mem, Planes};

/// The colour indices of one fetched line into `row`, whose pixel 0 is window position `left`.
pub(super) fn fetch(mem: &Mem, planes: &Planes, left: i32, row: &mut [u8]) {
    row.fill(0);
    let words = planes.fetch.words() as i32;
    // Lores: a fetch unit of 8 slots takes plane 1 last, then 2, 3, 4, 5 at these slots.
    const SLOT: [u16; 6] = [7, 3, 5, 1, 6, 2];
    let first = mem.fetch_start.get();
    for p in 0..planes.fetch.planes() as usize {
        if first != 0 {
            mem.fetch_start.set(first + SLOT[p]);
        }
        let data = mem.bytes(planes.pt[p], 2 * words as u32);
        let start = planes.index(p, left);
        for (x, px) in row.iter_mut().enumerate() {
            let i = start + x as i32;
            if (0..16 * words).contains(&i) {
                let (byte, bit) = ((i / 8) as usize, 7 - i % 8);
                *px |= (data[byte] >> bit & 1) << p;
            }
        }
    }
}
