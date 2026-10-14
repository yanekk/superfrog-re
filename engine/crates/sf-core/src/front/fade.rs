//! The palette fades (h0+$151b2, $153a4, $155a0 and their vertical-blank steps $1528c, $15456,
//! $156aa): four records of `$1f2` bytes from h0+$157d8, each moving up to 32 colour words of a
//! copper list (the value word of each MOVE) towards a target, one step every `speed` frames.
//!
//! A record: `+0` mode (1 step by one, 2 fade in, 3 fade out, 0 idle), `+2` speed, `+4` frame
//! counter, `+6` done, `+8` colour count, `+$a` target palette, `+$e` the copper list (the word
//! before the first value), `+$12` per-channel directions (mode 1), `+$72` per-channel steps,
//! `+$132` per-channel accumulators, all as the original's code lays them out.

use crate::layout::h0;
use crate::state::State;

/// A fade record: its address and where its steps and accumulators are (the Accessing-disk
/// overlay carries its own copy of the code, with them two bytes further on).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rec {
    pub at: u32,
    steps: u32,
    accs: u32,
}

impl Rec {
    pub const fn main(at: u32) -> Rec {
        Rec {
            at,
            steps: at + 0x72,
            accs: at + 0x132,
        }
    }

    pub const fn overlay(at: u32) -> Rec {
        Rec {
            at,
            steps: at + 0x74,
            accs: at + 0x134,
        }
    }
}

/// The game's four records.
pub const RECORDS: [Rec; 4] = [
    Rec::main(h0(0x157d8)),
    Rec::main(h0(0x159ca)),
    Rec::main(h0(0x15bbc)),
    Rec::main(h0(0x15dae)),
];

const MODE: u32 = 0;
pub const SPEED: u32 = 2;
const COUNTER: u32 = 4;
pub const DONE: u32 = 6;
const COUNT: u32 = 8;
const TARGET: u32 = 0xa;
const LIST: u32 = 0xe;
const DIRS: u32 = 0x12;

/// What a fade set-up writes to Paula: interrupts off while the record changes, then on.
pub const INTENA_OFF: u16 = 0x4000;
pub const INTENA_ON: u16 = 0xc000;

/// The colour channels as the code takes them apart: (mask, shift to `$f00`).
const CHANNELS: [(u16, u32); 3] = [(0xf00, 0), (0x0f0, 4), (0x00f, 8)];

fn div15(v: u16) -> u16 {
    // divu.w #$f then ext.l: the quotient's word.
    (v as u32 / 15) as u16
}

/// `$151b2` (mode 1): `list` gets `from`'s colours now; each later step moves every channel one
/// towards `target`. `list` is the copper list's first MOVE.
pub fn set_and_step(s: &mut State, rec: Rec, from: u32, target: u32, list: u32, n: u16) {
    s.set_w(rec.at + MODE, 1);
    s.set_w(rec.at + COUNT, n);
    s.set_l(rec.at + TARGET, target);
    s.set_l(rec.at + LIST, list);
    let mut d = rec.at + DIRS;
    for k in 0..n as u32 {
        let (a, b) = (s.w(from + 2 * k), s.w(target + 2 * k));
        for (mask, _) in CHANNELS {
            let (a, b) = (a & mask, b & mask);
            s.set_b(
                d,
                if b < a {
                    0xff
                } else if b > a {
                    1
                } else {
                    0
                },
            );
            d += 1;
        }
    }
    for k in 0..n as u32 {
        let v = s.w(from + 2 * k);
        s.set_w(list + 2 + 4 * k, v);
    }
    s.set_w(rec.at + DONE, 0);
}

/// The 68000's `divu.w` time in CPU cycles for `dividend` / `divisor`, without its operand (Jorge
/// Cwik's exact count: 76 to 136 by the quotient's bits; 10 on overflow).
fn divu_cycles(dividend: u32, divisor: u16) -> u32 {
    if dividend >> 16 >= divisor as u32 {
        return 10;
    }
    let (mut d, h) = (dividend, (divisor as u32) << 16);
    let mut m = 38;
    for _ in 0..15 {
        let top = d & 0x8000_0000 != 0;
        d <<= 1;
        if top {
            d = d.wrapping_sub(h);
        } else {
            m += 2;
            if d >= h {
                d -= h;
                m -= 1;
            }
        }
    }
    m * 2
}

/// How long [`fade_in`]'s set-up takes with interrupts off (T100): clearing the accumulators,
/// then for each colour three channels' steps, each a `divu.w #$f` whose time depends on the
/// target (measured: `level_start`'s fade in, from line 299 to line 30 of the next frame).
pub fn fade_in_cycles(s: &State, target: u32, n: u16) -> u32 {
    const SETUP: u32 = 2_764;
    const COLOUR: u32 = 134;
    (0..n as u32)
        .map(|k| {
            let t = s.w(target + 2 * k);
            COLOUR
                + CHANNELS
                    .iter()
                    .map(|&(_, shift)| 4 + divu_cycles(((t << shift) & 0xf00) as u32, 15))
                    .sum::<u32>()
        })
        .sum::<u32>()
        + SETUP
}

/// `$153a4` (mode 2): fade `list`'s colours in towards `target`.
pub fn fade_in(s: &mut State, rec: Rec, list: u32, target: u32, n: u16) {
    s.set_w(rec.at + MODE, 2);
    for k in 0..0x30 {
        s.set_l(rec.accs + 4 * k, 0);
    }
    let mut st = rec.steps;
    for k in 0..n as u32 {
        let t = s.w(target + 2 * k);
        for (_, shift) in CHANNELS {
            s.set_w(st, div15((t << shift) & 0xf00));
            st += 2;
        }
    }
    s.set_w(rec.at + COUNT, n);
    s.set_l(rec.at + LIST, list.wrapping_sub(2));
    s.set_l(rec.at + TARGET, target.wrapping_sub(2));
    s.set_w(rec.at + DONE, 0);
}

/// `$155a0` (mode 3): fade `list`'s colours out to black.
pub fn fade_out(s: &mut State, rec: Rec, list: u32, n: u16) {
    s.set_w(rec.at + MODE, 3);
    for k in 0..0x30 {
        s.set_l(rec.accs + 4 * k, 0);
    }
    let mut st = rec.steps;
    for k in 0..n as u32 {
        let c = s.w(list + 2 + 4 * k);
        for (_, shift) in CHANNELS {
            s.set_w(st, div15((c << shift) & 0xf00));
            st += 2;
        }
    }
    let mut acc = rec.accs;
    for k in 0..n as u32 {
        let c = s.w(list + 2 + 4 * k);
        for (mask, shift) in CHANNELS {
            s.set_w(acc, (c & mask) << shift);
            acc += 2;
        }
    }
    s.set_w(rec.at + COUNT, n);
    s.set_l(rec.at + LIST, list.wrapping_sub(2));
    s.set_w(rec.at + DONE, 0);
}

/// Whether the record's fade has finished (`$1519e` waits for this).
pub fn done(s: &State, rec: Rec) -> bool {
    s.w(rec.at + DONE) != 0
}

/// The colour channels the fades in, then out, due a step this frame look at, in the order the
/// handler does: each one's word in its copper list, its mask, whether the step changes it, and
/// whether it is a fade in's (for how long the handler takes, and when the copper sees each).
pub fn due_channels(s: &State) -> Vec<(u32, u16, bool, bool)> {
    let mut out = Vec::new();
    for mode in [2, 3] {
        for rec in RECORDS {
            let due = s.w(rec.at + MODE) == mode
                && s.w(rec.at + DONE) == 0
                && (s.w(rec.at + COUNTER).wrapping_add(1) as i16) >= (s.w(rec.at + SPEED) as i16);
            if !due {
                continue;
            }
            let n = s.w(rec.at + COUNT) as u32;
            let (list, target) = (s.l(rec.at + LIST).wrapping_add(4), s.l(rec.at + TARGET).wrapping_add(2));
            for k in 0..n {
                for (mask, _) in CHANNELS {
                    let cur = s.w(list + 4 * k) & mask;
                    let changes = if mode == 2 { cur != s.w(target + 2 * k) & mask } else { cur != 0 };
                    out.push((list + 4 * k, mask, changes, mode == 2));
                }
            }
        }
    }
    out
}

/// The record is due a step this frame: its mode is `mode`, it is running and its counter reached
/// its speed (the counter restarts).
fn due(s: &mut State, rec: Rec, mode: u16) -> bool {
    if s.w(rec.at + MODE) != mode || s.w(rec.at + DONE) != 0 {
        return false;
    }
    let c = s.w(rec.at + COUNTER).wrapping_add(1);
    s.set_w(rec.at + COUNTER, c);
    if (c as i16) < (s.w(rec.at + SPEED) as i16) {
        return false;
    }
    s.set_w(rec.at + COUNTER, 0);
    true
}

fn finish(s: &mut State, rec: Rec) {
    s.set_w(rec.at + DONE, 1);
    s.set_w(rec.at + MODE, 0);
}

/// `$15456`: every fade-in record's step.
pub fn step_in(s: &mut State) {
    for rec in RECORDS {
        step_in_rec(s, rec);
    }
}

/// One record's step of that kind.
pub fn step_in_rec(s: &mut State, rec: Rec) {
    {
        if !due(s, rec, 2) {
            return;
        }
        let n = s.w(rec.at + COUNT) as u32;
        let (mut a0, mut a1) = (s.l(rec.at + LIST), s.l(rec.at + TARGET));
        let mut changed = 0;
        for k in 0..n {
            a0 += 4;
            a1 += 2;
            for (c, (mask, shift)) in CHANNELS.iter().enumerate() {
                let (st, acc) = (
                    rec.steps + 6 * k + 2 * c as u32,
                    rec.accs + 6 * k + 2 * c as u32,
                );
                let (cur, tgt) = (s.w(a0), s.w(a1));
                if cur & mask == tgt & mask {
                    continue;
                }
                let v = s.w(acc).wrapping_add(s.w(st));
                s.set_w(acc, v);
                let add = (v & 0xf00) >> shift;
                s.set_w(a0, (cur & !mask).wrapping_add(add));
                changed += 1;
            }
        }
        if changed == 0 {
            finish(s, rec);
        }
    }
}

/// `$156aa`: every fade-out record's step.
pub fn step_out(s: &mut State) {
    for rec in RECORDS {
        step_out_rec(s, rec);
    }
}

/// One record's step of that kind.
pub fn step_out_rec(s: &mut State, rec: Rec) {
    {
        if !due(s, rec, 3) {
            return;
        }
        let n = s.w(rec.at + COUNT) as u32;
        let mut a0 = s.l(rec.at + LIST);
        let mut changed = 0;
        for k in 0..n {
            a0 += 4;
            for (c, (mask, shift)) in CHANNELS.iter().enumerate() {
                let (st, acc) = (
                    rec.steps + 6 * k + 2 * c as u32,
                    rec.accs + 6 * k + 2 * c as u32,
                );
                let cur = s.w(a0);
                if cur & mask == 0 {
                    continue;
                }
                let v = s.w(acc).wrapping_sub(s.w(st));
                s.set_w(acc, v);
                let add = (v & 0xf00) >> shift;
                s.set_w(a0, (cur & !mask).wrapping_add(add));
                changed += 1;
            }
        }
        if changed == 0 {
            finish(s, rec);
        }
    }
}

/// `$1528c`: every mode-1 record's step: each channel one nearer its target.
pub fn step_set(s: &mut State) {
    for rec in RECORDS {
        step_set_rec(s, rec);
    }
}

/// One record's step of that kind.
pub fn step_set_rec(s: &mut State, rec: Rec) {
    {
        if !due(s, rec, 1) {
            return;
        }
        let n = s.w(rec.at + COUNT) as u32;
        let (mut a0, mut a1) = (s.l(rec.at + LIST) + 2, s.l(rec.at + TARGET));
        let mut d = rec.at + DIRS;
        let mut same = 0u32;
        for _ in 0..n {
            for (mask, shift) in CHANNELS {
                let (cur, tgt) = (s.w(a0), s.w(a1));
                if cur & mask == tgt & mask {
                    same += 1;
                    d += 1;
                    continue;
                }
                let dir = s.b(d);
                d += 1;
                // The channel as a byte, plus the direction, back in place (word arithmetic as
                // the original's: `lsr`, `add.b`, `lsl`, then masked into the word).
                let ch = ((cur & mask) >> (8 - shift)) as u8;
                let v = (((cur & mask) >> (8 - shift)) & 0xff00 | ch.wrapping_add(dir) as u16)
                    << (8 - shift);
                let keep = match shift {
                    0 => cur & 0x00ff,
                    4 => cur & 0x0f0f,
                    _ => cur & 0x0ff0,
                };
                s.set_w(a0, keep | v);
            }
            a0 += 4;
            a1 += 2;
        }
        if same / 3 == n {
            finish(s, rec);
        }
    }
}

/// The fades' steps on the level's copper list in the front end's handler (T100), timed from
/// `start` (CPU cycles from the frame's top, [`super::vbl::fade_start`]): a channel looked at and
/// one changed in a fade in (T37's), in a fade out, and from the last to the acknowledge, in CPU
/// cycles; from the display window's top line (and this far into it) the CPU gets 17 of every 20
/// (bitplane DMA). Fitted to the original's acknowledges and pictures in `frog-pit-w1-1`.
const IN_LOOK: u32 = 95;
const IN_CHANGE: u32 = 96;
const OUT_LOOK: u32 = 78;
const OUT_CHANGE: u32 = 86;
const IN_TAIL: u32 = 830;
const OUT_TAIL: u32 = 996;
const WINDOW_SLOW_FROM: u32 = 186;
const WINDOW_RATE: (u32, u32) = (17, 20);

/// When each changed channel of `channels` ([`due_channels`]) is written, and when the handler
/// acknowledges, in cycles from the frame's top: `start` the first channel's, `window_top` the
/// line the display window starts on.
pub fn level_step_times(
    channels: &[(u32, u16, bool, bool)],
    start: u32,
    window_top: u16,
) -> (Vec<u32>, u32) {
    let slow = window_top as u32 * super::BEAM_CYCLES_PER_LINE + WINDOW_SLOW_FROM;
    let wall = |work: u32| {
        let fast = if start < slow {
            work.min(slow - start)
        } else {
            0
        };
        start + fast + (work - fast) * WINDOW_RATE.1 / WINDOW_RATE.0
    };
    let mut work = 0;
    let mut writes = Vec::new();
    for &(_, _, changes, fade_in) in channels {
        work += if fade_in { IN_LOOK } else { OUT_LOOK };
        if changes {
            work += if fade_in { IN_CHANGE } else { OUT_CHANGE };
            writes.push(wall(work));
        }
    }
    let tail = if channels.iter().any(|c| c.3) {
        IN_TAIL
    } else {
        OUT_TAIL
    };
    (writes, wall(work + tail))
}

#[cfg(test)]
mod tests {
    use super::{divu_cycles, level_step_times};
    use crate::front::BEAM_CYCLES_PER_LINE;

    /// The 68000's `divu.w` by its quotient's bits (Cwik's count, checked against his table's
    /// extremes): a zero quotient is the slowest, an overflow stops at once.
    #[test]
    fn divu_takes_the_68000s_time() {
        assert_eq!(divu_cycles(0, 15), 136);
        assert_eq!(divu_cycles(0xf00, 15), 134);
        assert_eq!(divu_cycles(0x700, 15), 126);
        assert_eq!(divu_cycles(0xffff, 1), 106);
        assert_eq!(divu_cycles(0x1_0000, 1), 10);
    }

    /// A channel looked at and one changed, from the start; inside the display window the CPU
    /// gets 17 cycles of every 20, so the same work takes longer there.
    #[test]
    fn steps_slow_down_inside_the_window() {
        let ch = |changes| (0, 0x0f0, changes, true);
        let early = 10 * BEAM_CYCLES_PER_LINE;
        let (writes, end) = level_step_times(&[ch(false), ch(true)], early, 40);
        assert_eq!(writes, vec![early + 95 + 95 + 96]);
        assert_eq!(end, early + 286 + 830);
        let late = 50 * BEAM_CYCLES_PER_LINE;
        let (writes, end) = level_step_times(&[ch(true)], late, 40);
        assert_eq!(writes, vec![late + 191 * 20 / 17]);
        assert_eq!(end, late + (191 + 830) * 20 / 17);
    }
}
