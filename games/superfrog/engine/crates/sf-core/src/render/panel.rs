//! Lines Denise fetches from memory: the status panel and the display buffers (games/superfrog/docs/12 "Panel").
//!
//! The panel is five planes of 14 lines, `$2a0` bytes apart, shown with the same fetch as the
//! play field (23 words a line, modulo 2). Its two lowest lines fetch past the fifth plane into
//! what follows it in chip memory, the frog's sprite frames, as the original's do.

use super::Mem;
use amiga_chips::display::Planes;

/// The colour indices of one fetched line into `row`, whose pixel 0 is window position `left`.
pub(super) fn fetch(mem: &Mem, planes: &Planes, left: i32, row: &mut [u8]) {
    // Lores: a fetch unit of 8 slots takes plane 1 last, then 2, 3, 4, 5 at these slots.
    const SLOT: [u16; 6] = [7, 3, 5, 1, 6, 2];
    let first = mem.fetch_start.get();
    planes.fetch_line(left, row, |p, addr, len| {
        if first != 0 {
            mem.fetch_start.set(first + SLOT[p]);
        }
        mem.bytes(addr, len)
    });
}
