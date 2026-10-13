//! The game's IFF ANIM player (docs/10 "ANIM"), ported from its code over the original's memory:
//! `h0+$ab02` opens a file (its first frame's ByteRun1 `BODY` into buffer A, copied to buffer B)
//! and `h0+$ab82` draws the next frame's `DLTA` (operation 5, byte vertical delta) into the buffer
//! not shown. The player's record (h0+$3b18c) and both buffers end as the original's do.
//!
//! Each call also gives the 68000 cycles the original's code takes, instruction by instruction as
//! the 68000's manual times them with memory that never waits; [`super::ending`] turns them into
//! beam time. A frame's decode can run for several frames, so its length decides when the next
//! frame shows (T35).

use crate::layout::{h0, h3};
use crate::state::State;

/// The player's record: `+0` bytes a row, `+4` rows, `+8` buffer A, `+$c` buffer B, `+$10` the
/// file, `+$14` bytes a row (`BMHD`), `+$16` rows, `+$18` planes, `+$19` which buffer is drawn
/// next ($00 B, $ff A), `+$1a` the next frame's `FORM`, `+$1e` the first delta's.
pub const RECORD: u32 = h0(0x3b18c);
pub const RECORD_LEN: u32 = 0x22;
/// Where `end_sequence` unpacks each file, and its two buffers.
pub const FILE: u32 = h3(0);
pub const BUFFER_A: u32 = h3(0x3d090);
pub const BUFFER_B: u32 = h3(0x49890);
/// One buffer: five planes of 256 rows of 40 bytes.
pub const BUFFER_LEN: u32 = 5 * 0x2800;

const FORM: u32 = u32::from_be_bytes(*b"FORM");
const ANIM: u32 = u32::from_be_bytes(*b"ANIM");
const ILBM: u32 = u32::from_be_bytes(*b"ILBM");
const BMHD: u32 = u32::from_be_bytes(*b"BMHD");
const BODY: u32 = u32::from_be_bytes(*b"BODY");
const ANHD: u32 = u32::from_be_bytes(*b"ANHD");
const DLTA: u32 = u32::from_be_bytes(*b"DLTA");

/// A short conditional branch: 10 cycles taken, 8 not.
fn bcc_b(taken: bool) -> u32 {
    if taken { 10 } else { 8 }
}

/// `mulu.w`: 38 cycles and 2 for each set bit of the multiplier word.
fn mulu(src: u32) -> u32 {
    38 + 2 * (src & 0xffff).count_ones()
}

/// `dbra` over `n` + 1 passes of a loop: 10 cycles a pass that branches back, 14 the last.
fn dbra(n: u32) -> u32 {
    10 * n + 14
}

/// Where a delta's decoding has got to, at the start of each column and each op: for timing it
/// against the original stopped mid-way (calibration).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    pub cycles: u32,
    /// The op list's read pointer (`a0`), the write pointer (`a1`), columns left (`d6`) and
    /// planes left (`d5`).
    pub a0: u32,
    pub a1: u32,
    pub d6: u16,
    pub d5: u8,
}

/// `h0+$ab02` as `end_sequence` calls it (`$a890`, `$a95e`): the ANIM at [`FILE`] opened on
/// [`BUFFER_A`] and [`BUFFER_B`], 40 bytes a row, 256 rows. Returns the cycles from the caller's
/// first `lea` to the open's `rts`.
pub fn open(s: &mut State) -> u32 {
    let (row_bytes, rows) = (0x28u32, 0x100u32);
    let rec = RECORD;
    // Four `lea`s, `moveq`, `move.l #`, `bsr.w`; the open's `movem` (15 registers) and the
    // record's first five longs (`movem` to memory), `move.b #0`, `clr.l`.
    let mut c = 4 * 12 + 4 + 12 + 18 + 128 + 4 + 52 + 16 + 24;
    s.set_l(rec, row_bytes);
    s.set_l(rec + 4, rows);
    s.set_l(rec + 8, BUFFER_A);
    s.set_l(rec + 0xc, BUFFER_B);
    s.set_l(rec + 0x10, FILE);
    s.set_b(rec + 0x19, 0);
    s.set_l(rec + 0x1e, 0);
    let mut a2 = FILE;
    // Two tags checked; an unknown file is the original's endless red screen, left out here.
    c += 12 + 14 + 8 + 8 + 12 + 14 + 8;
    if s.l(a2) != FORM || s.l(a2 + 8) != ANIM {
        return c;
    }
    a2 += 12;
    c += 4 + 16 + 16 + 16;
    let (a0, body) = ilbm_body(s, a2, BUFFER_A, row_bytes, rows);
    c += body;
    s.set_l(rec + 0x1a, a0);
    // The first frame copied to buffer B a long at a time (`dbra` counts the low word).
    let planes = s.b(rec + 0x18) as u32;
    let n = ((row_bytes * planes * rows).wrapping_sub(1) >> 2) & 0xffff;
    let bytes = s.read(BUFFER_A, (n + 1) * 4);
    s.write(BUFFER_B, &bytes);
    c += 16 + 16 + 16 + 16 + 4 + 12 + mulu(planes) + 16 + mulu(rows) + 8 + 12;
    c += (n + 1) * 20 + dbra(n) + 10 + 132 + 16;
    c
}

/// `h0+$ac96`: the `FORM ILBM` at `at`; its `BMHD` into the record and its `BODY` (ByteRun1,
/// planes interleaved by row) into `dest`, `row_bytes` apart a row and a plane `row_bytes *
/// rows` long. Returns where the `BODY`'s data ended (`a0`) and the cycles, `bsr` and `rts` in.
fn ilbm_body(s: &mut State, at: u32, dest: u32, row_bytes: u32, rows: u32) -> (u32, u32) {
    let rec = RECORD;
    let mut c = 18 + 120 + 4 + 4 + 12 + 14 + 12 + 8 + 12 + 14 + 12;
    let mut a0 = at;
    if s.l(a0) != FORM || s.l(a0 + 8) != ILBM {
        return (a0, c);
    }
    a0 += 12;
    let (mut width_bytes, mut height, mut planes) = (0u32, 0u32, 0u32);
    loop {
        let tag = s.l(a0);
        a0 += 4;
        c += 12 + 14;
        if tag == BMHD {
            c += 10;
            width_bytes = (s.w(a0 + 4) >> 3) as u32;
            height = s.w(a0 + 6) as u32;
            planes = s.b(a0 + 12) as u32;
            s.set_w(rec + 0x14, width_bytes as u16);
            s.set_w(rec + 0x16, height as u16);
            s.set_b(rec + 0x18, planes as u8);
            a0 += 4 + 4 + 4 + 1 + 0xb;
            c += 8 + 8 + 12 + 12 + 8 + 12 + 8 + 8 + 12 + 12 + 10;
            continue;
        }
        c += 8 + 14;
        if tag == BODY {
            c += 10;
            break;
        }
        // Skipped without the IFF pad byte, as the original does.
        let len = s.l(a0);
        a0 = a0.wrapping_add(4).wrapping_add(len);
        c += 8 + 12 + 8 + 10;
    }
    a0 += 4;
    let plane = row_bytes.wrapping_mul(rows) & 0xffff;
    c += 8 + 4 + 4 + mulu(rows);
    let mut row = dest;
    for r in 0..height {
        let mut a1 = row;
        c += 4 + 4 + 4;
        for p in 0..planes {
            let (next, cycles) = byterun1_row(s, a0, a1, width_bytes);
            a0 = next;
            a1 = a1.wrapping_add(plane);
            c += 18 + cycles + 8 + 4 + bcc_b(p + 1 < planes);
        }
        row = row.wrapping_add(row_bytes);
        c += 4 + 8 + 4 + bcc_b(r + 1 < height);
    }
    c += 10 + 124 + 16;
    (a0, c)
}

/// `h0+$ad2a`: one row of `bytes` bytes of ByteRun1 from `a0` to `a1`; a control byte `n` copies
/// `n` + 1 bytes when positive, else repeats the next byte `-n` + 1 times (`-128` too). Returns the
/// read pointer after it and the cycles, `rts` in (the caller's `bsr` out).
fn byterun1_row(s: &mut State, mut a0: u32, mut a1: u32, bytes: u32) -> (u32, u32) {
    let mut c = 24 + 4 + 4;
    let mut d2 = (bytes as u16).wrapping_sub(1) as i16;
    loop {
        c += 4;
        if d2 < 0 {
            c += 10;
            break;
        }
        c += 8 + 8;
        let n = s.b(a0) as i8;
        a0 += 1;
        if n >= 0 {
            c += 8 + 4;
            let k = n as u32;
            for _ in 0..=k {
                let v = s.b(a0);
                a0 += 1;
                s.set_b(a1, v);
                a1 += 1;
                d2 = d2.wrapping_sub(1);
            }
            c += (k + 1) * (12 + 4) + dbra(k) + 10;
        } else {
            c += 10 + 4 + 4 + 8;
            let k = (-(n as i16)) as u32;
            let v = s.b(a0);
            a0 += 1;
            for _ in 0..=k {
                s.set_b(a1, v);
                a1 += 1;
                d2 = d2.wrapping_sub(1);
            }
            c += (k + 1) * (8 + 4) + dbra(k) + 10;
        }
    }
    c += 28 + 16;
    (a0, c)
}

/// `h0+$ab82` as the end sequence calls it (`lea` of the record, `bsr`, `tst.w d0`): the next
/// frame's delta into the buffer not shown, then the record moved on. Returns whether the file
/// has ended (`d0` ≠ 0) and the cycles, the caller's branch on `d0` out.
pub fn next(s: &mut State) -> (bool, u32) {
    next_traced(s, &mut |_| {})
}

/// [`next`], reporting [`Progress`] at the start of every column and every op.
pub fn next_traced(s: &mut State, trace: &mut dyn FnMut(Progress)) -> (bool, u32) {
    let rec = RECORD;
    let mut c = 12 + 18 + 120 + 4 + 16 + 8 + 8 + 4;
    let mut a5 = (s.l(rec + 0x1a).wrapping_add(1)) & !1;
    let end = |c: u32| (true, c + 4 + 4 + 124 + 16);
    c += 12 + 14;
    if s.l(a5) != FORM {
        return end(c + 10);
    }
    c += 12 + 8 + 12 + 14;
    if s.l(a5 + 8) != ILBM {
        return end(c + 10);
    }
    c += 12;
    a5 += 12;
    loop {
        let tag = s.l(a5);
        a5 += 4;
        c += 12 + 14;
        if tag == ANHD {
            c += 10 + 8 + 8 + 8;
            if s.b(a5 + 4) != 5 {
                return end(c + 10);
            }
            a5 += 4 + 1 + 0x13 + 4 + 0x10;
            c += 12 + 12 + 12 + 12 + 10;
            continue;
        }
        c += 8 + 14;
        if tag == DLTA {
            c += 10;
            break;
        }
        let len = s.l(a5);
        a5 = a5.wrapping_add(4).wrapping_add(len);
        c += 8 + 12 + 8 + 10;
    }
    let len = s.l(a5);
    a5 += 4;
    let a3 = a5.wrapping_add(len);
    let a4 = a5;
    let row_bytes = s.l(rec);
    let planes = s.b(rec + 0x18);
    let toggle = s.b(rec + 0x19);
    let mut a1 = if toggle != 0 {
        s.l(rec + 8)
    } else {
        s.l(rec + 0xc)
    };
    c += 12 + 4 + 8 + 4 + 16 + 12 + 12 + bcc_b(toggle == 0) + 16 + if toggle != 0 { 10 } else { 0 };
    let columns = s.w(rec + 0x14);
    let mut d5 = planes;
    loop {
        let mut a2 = a1;
        let mut a0 = a4.wrapping_add(s.l(a5));
        a5 += 4;
        c += 12 + 4 + 12 + 12;
        let mut d6 = columns;
        loop {
            trace(Progress {
                cycles: c,
                a0,
                a1,
                d6,
                d5,
            });
            let mut d3 = s.b(a0);
            a0 += 1;
            c += 8;
            if d3 == 0 {
                c += 10;
            } else {
                c += 8;
                loop {
                    trace(Progress {
                        cycles: c,
                        a0,
                        a1,
                        d6,
                        d5,
                    });
                    let op = s.b(a0);
                    a0 += 1;
                    c += 4 + 8;
                    if op & 0x80 != 0 {
                        c += 10 + 8 + 4;
                        let k = (op & 0x7f) as u32;
                        let n = k.wrapping_sub(1) & 0xffff;
                        for _ in 0..=n {
                            let v = s.b(a0);
                            a0 += 1;
                            s.set_b(a1, v);
                            a1 = a1.wrapping_add(row_bytes as u16 as i16 as u32);
                        }
                        c += (n + 1) * (12 + 8) + dbra(n);
                    } else if op == 0 {
                        c += 8 + 10 + 4 + 8 + 8 + 4;
                        let k = s.b(a0) as u32;
                        let v = s.b(a0 + 1);
                        a0 += 2;
                        let n = k.wrapping_sub(1) & 0xffff;
                        for _ in 0..=n {
                            s.set_b(a1, v);
                            a1 = a1.wrapping_add(row_bytes as u16 as i16 as u32);
                        }
                        c += (n + 1) * (8 + 8) + dbra(n) + 10;
                    } else {
                        c += 8 + 8 + mulu(row_bytes) + 8 + 10;
                        let skip = (op as u32 * (row_bytes & 0xffff)) & 0xffff;
                        a1 = a1.wrapping_add(skip as u16 as i16 as u32);
                    }
                    d3 = d3.wrapping_sub(1);
                    c += 4 + bcc_b(d3 != 0);
                    if d3 == 0 {
                        break;
                    }
                }
            }
            a2 += 1;
            a1 = a2;
            d6 = d6.wrapping_sub(1);
            c += 8 + 4 + 4 + bcc_b(d6 != 0);
            if d6 == 0 {
                break;
            }
        }
        let rows = s.l(rec + 4);
        a2 = a2
            .wrapping_sub(row_bytes)
            .wrapping_add((row_bytes & 0xffff) * (rows & 0xffff));
        a1 = a2;
        d5 = d5.wrapping_sub(1);
        c += 18 + 16 + 16 + mulu(rows) + 8 + 4 + 4 + bcc_b(d5 != 0);
        if d5 == 0 {
            break;
        }
    }
    let next = a3.wrapping_add(1) & !1;
    s.set_l(rec + 0x1a, next);
    s.set_b(rec + 0x19, !toggle);
    let first = s.l(rec + 0x1e) == 0;
    if first {
        s.set_l(rec + 0x1e, next);
    }
    c += 4 + 8 + 16 + 4 + 16 + 16 + 16 + bcc_b(!first) + if first { 16 } else { 0 } + 4 + 10;
    (false, c + 124 + 16 + 4)
}

/// `$aac0` / `$aaca`: the end sequence's copper list (h2+$5efc) shows `buffer` (six bitplane
/// pointers from it, a plane apart) and becomes the next frame's list.
pub fn show_pointers(s: &mut State, buffer: u32) {
    let mut p = buffer;
    for k in 0..6 {
        let at = crate::layout::h2(0x6024) + 8 * k;
        s.set_w(at + 6, p as u16);
        s.set_w(at + 2, (p >> 16) as u16);
        p = p.wrapping_add(0x2800);
    }
}
