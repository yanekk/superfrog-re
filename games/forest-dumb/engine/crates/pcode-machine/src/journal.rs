//! Chip RAM as it was at a beam position. The copper and the display read chip RAM while the beam
//! moves, and the CPU and the blitter write it in the same frame; the machine runs the copper and
//! draws the picture from whole-frame views, so each chip write of the frame is journalled with
//! the byte it replaced, and a read "as of cycle t" takes the byte back from the first later write.
//!
//! [`Watch`] is the other half: which chip words the copper's current run read, and when, so a
//! write the copper has not read yet marks the run stale (it is run again before its next write is
//! delivered).

/// The frame's chip writes: (cycle, chip offset, byte replaced), in the order made.
#[derive(Debug, Default, Clone)]
pub struct Journal {
    log: Vec<(u64, u32, u8)>,
}

impl Journal {
    pub fn record(&mut self, at: u64, off: u32, old: u8) {
        self.log.push((at, off, old));
    }

    pub fn clear(&mut self) {
        self.log.clear();
    }

    /// The journal indexed by address, for reads as of a cycle.
    pub fn history(&self) -> History {
        let mut by_addr: Vec<(u32, u64, u8)> = self
            .log
            .iter()
            .map(|&(at, off, old)| (off, at, old))
            .collect();
        // Stable: one address's writes stay in the order made, which is cycle order.
        by_addr.sort_by_key(|e| e.0);
        History { by_addr }
    }
}

/// [`Journal::history`]: what a byte held at a given cycle.
#[derive(Debug, Default, Clone)]
pub struct History {
    by_addr: Vec<(u32, u64, u8)>,
}

impl History {
    /// Chip byte `off` as it was at cycle `at`: the byte the first write made after `at` replaced,
    /// or the byte now if none was. A write at `at` itself has happened (an access is timed at its
    /// block's start).
    pub fn byte(&self, chip: &[u8], off: u32, at: i64) -> u8 {
        let i = self.by_addr.partition_point(|e| e.0 < off);
        for &(a, t, old) in &self.by_addr[i..] {
            if a != off {
                break;
            }
            if t as i64 > at {
                return old;
            }
        }
        chip[off as usize]
    }

    /// The big-endian word at even chip offset `off` as of `at`.
    pub fn word(&self, chip: &[u8], off: u32, at: i64) -> u16 {
        (self.byte(chip, off, at) as u16) << 8 | self.byte(chip, off + 1, at) as u16
    }
}

/// The chip words the copper's current run read, each with the latest cycle it read it at.
#[derive(Debug, Default, Clone)]
pub struct Watch {
    /// One bit a chip word: whether `reads` holds it (a write's check is one bit test).
    bits: Vec<u64>,
    /// (word offset, latest cycle read), sorted by offset.
    reads: Vec<(u32, i64)>,
    /// A write landed on a word the run reads at or after the write's cycle.
    pub dirty: bool,
}

impl Watch {
    /// Watch `reads` (chip byte offset of each word read, cycle), replacing the last run's.
    pub fn set(&mut self, chip_len: usize, mut reads: Vec<(u32, i64)>) {
        if self.bits.len() != chip_len.div_ceil(128) {
            self.bits = vec![0; chip_len.div_ceil(128)];
        }
        for &(w, _) in &self.reads {
            self.bits[w as usize / 64] = 0;
        }
        for r in &mut reads {
            r.0 >>= 1;
        }
        reads.sort_by_key(|r| (r.0, std::cmp::Reverse(r.1)));
        reads.dedup_by_key(|r| r.0);
        for &(w, _) in &reads {
            self.bits[w as usize / 64] |= 1 << (w % 64);
        }
        self.reads = reads;
        self.dirty = false;
    }

    /// Forget the run: nothing is watched.
    pub fn clear(&mut self) {
        for &(w, _) in &self.reads {
            self.bits[w as usize / 64] = 0;
        }
        self.reads.clear();
        self.dirty = false;
    }

    /// Chip byte `off` was written at cycle `at`.
    pub fn written(&mut self, off: u32, at: u64) {
        let w = off >> 1;
        let Some(&bits) = self.bits.get(w as usize / 64) else {
            return;
        };
        if bits >> (w % 64) & 1 == 0 {
            return;
        }
        if let Ok(i) = self.reads.binary_search_by_key(&w, |r| r.0)
            && self.reads[i].1 >= at as i64
        {
            self.dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_byte_reads_as_it_was_at_a_cycle() {
        let chip = vec![9u8; 8];
        let mut j = Journal::default();
        // Byte 2 was 1, became 5 at cycle 10, then 9 at cycle 20; byte 3 was 7, became 9 at 15.
        j.record(10, 2, 1);
        j.record(15, 3, 7);
        j.record(20, 2, 5);
        let h = j.history();
        assert_eq!(h.byte(&chip, 2, 0), 1);
        assert_eq!(h.byte(&chip, 2, 9), 1);
        assert_eq!(h.byte(&chip, 2, 10), 5);
        assert_eq!(h.byte(&chip, 2, 19), 5);
        assert_eq!(h.byte(&chip, 2, 20), 9);
        assert_eq!(h.byte(&chip, 2, -5), 1);
        assert_eq!(h.word(&chip, 2, 12), 0x0507);
        assert_eq!(h.word(&chip, 2, 30), 0x0909);
        assert_eq!(h.byte(&chip, 4, 0), 9);
    }

    #[test]
    fn a_write_before_the_copper_reads_a_word_marks_the_run_stale() {
        let mut w = Watch::default();
        w.set(0x100, vec![(0x10, 50), (0x12, 60), (0x10, 40)]);
        w.written(0x20, 0);
        assert!(!w.dirty, "a word not read");
        w.written(0x11, 51);
        assert!(!w.dirty, "read at 50, before the write");
        w.written(0x13, 60);
        assert!(w.dirty, "read at 60, as the write lands");
        w.set(0x100, vec![(0x40, 5)]);
        assert!(!w.dirty);
        w.written(0x10, 0);
        assert!(!w.dirty, "the last run's words are forgotten");
    }
}
