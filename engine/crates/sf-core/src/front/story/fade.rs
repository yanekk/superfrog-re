//! The story's palette fades (docs/13): one algorithm in two copies, disk 3's boot program's
//! (`d3_c02` h0+$2ba92..$2be16, one record) and the intro's (`d3_c04` h0+$b4b4..$bb6c, four
//! records the vertical blank steps). A fade moves the colour words of a copper list (every
//! fourth byte pair from `list + 4`) one twelve-bit component at a time:
//!
//! - in (type 2): towards the target colours; each component's accumulator gains a fifteenth of
//!   its target a step and the colour takes the accumulator's top nibble;
//! - out (type 3): to black; the accumulator starts at the colour and loses a fifteenth a step;
//! - across (type 1, the intro's only): each component moves one step a frame up or down to the
//!   target's.
//!
//! A record's step runs once every `speed` calls. A fade is done (`done` set, `kind` cleared) on
//! the first step that changes nothing.

use super::mem::StoryMem;

/// Where one fade record keeps its fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rec {
    /// `kind` (word: 0 idle, 1 across, 2 in, 3 out); `speed`, `count`er, `done`, colour `count`
    /// and the two pointers follow at +2, +4, +6, +8, +$a and +$e.
    pub at: u32,
    /// Per component: across, a direction byte; in and out, a step word (`target / 15`).
    pub dirs: u32,
    pub steps: u32,
    pub accs: u32,
}

/// What one call of a record's step did, as the original's code went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Walk {
    /// The record is idle (`kind` 0, or past 3).
    Other,
    /// Its fade is over (`done` set).
    Done,
    /// Its counter has not reached its speed yet.
    Waiting,
    /// It stepped: each colour in turn; `changed` false when nothing changed (the fade is over).
    Stepped { colours: Vec<Colour>, changed: bool },
}

/// One colour word of a step: its address and, per component (red, green, blue in the code's
/// order), whether the code rewrote it, as the word after the component is cleared and after it
/// is set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Colour {
    pub at: u32,
    pub writes: [Option<(u16, u16)>; 3],
}

impl Colour {
    fn at(at: u32) -> Colour {
        Colour {
            at,
            writes: [None; 3],
        }
    }
}

pub const KIND: u32 = 0;
pub const SPEED: u32 = 2;
pub const COUNTER: u32 = 4;
pub const DONE: u32 = 6;
pub const COUNT: u32 = 8;
pub const TARGET: u32 = 0xa;
pub const LIST: u32 = 0xe;

impl Rec {
    /// The intro's records (`h0+$bb70`, `$bd62`, `$bf54`, `$c146`), as `a6` addresses them.
    pub const fn intro(at: u32) -> Rec {
        Rec {
            at,
            dirs: at + 0x12,
            steps: at + 0x72,
            accs: at + 0x132,
        }
    }

    pub fn done(&self, m: &StoryMem) -> bool {
        m.w(self.at + DONE) != 0
    }

    /// `fade_in` (h0+$b6d8 / boot h0+$2ba92 without its wait): towards the `count` colours at
    /// `target` (plain words), on the colour words of the copper moves at `list`.
    pub fn start_in(&self, m: &mut StoryMem, list: u32, target: u32, count: u16) {
        m.set_w(self.at + KIND, 2);
        m.clear(self.accs, 0xc0);
        for k in 0..count as u32 {
            let c = m.w(target + 2 * k);
            for (j, sh) in [0u32, 4, 8].into_iter().enumerate() {
                m.set_w(self.steps + 6 * k + 2 * j as u32, (c << sh & 0xf00) / 15);
            }
        }
        m.set_w(self.at + COUNT, count);
        m.set_l(self.at + LIST, list.wrapping_sub(2));
        m.set_l(self.at + TARGET, target.wrapping_sub(2));
        m.set_w(self.at + DONE, 0);
    }

    /// `fade_out` (h0+$b906 / boot h0+$2bc3e): the `count` colours of the copper moves at `list`
    /// to black.
    pub fn start_out(&self, m: &mut StoryMem, list: u32, count: u16) {
        m.set_w(self.at + KIND, 3);
        m.clear(self.accs, 0xc0);
        for k in 0..count as u32 {
            let c = m.w(list + 2 + 4 * k);
            for (j, sh) in [0u32, 4, 8].into_iter().enumerate() {
                m.set_w(self.steps + 6 * k + 2 * j as u32, (c << sh & 0xf00) / 15);
                m.set_w(self.accs + 6 * k + 2 * j as u32, (c << sh) & 0xf00);
            }
        }
        m.set_w(self.at + COUNT, count);
        m.set_l(self.at + LIST, list.wrapping_sub(2));
        m.set_w(self.at + DONE, 0);
    }

    /// `fade_across` (h0+$b4b4 without its wait): from the colour words at `from` (plain) to the
    /// ones at `to`, on the copper moves at `list`, which take `from`'s colours at once.
    pub fn start_across(&self, m: &mut StoryMem, from: u32, to: u32, list: u32, count: u16) {
        m.set_w(self.at + KIND, 1);
        m.set_w(self.at + COUNT, count);
        m.set_l(self.at + TARGET, to);
        m.set_l(self.at + LIST, list);
        for k in 0..count as u32 {
            let (a, b) = (m.w(from + 2 * k), m.w(to + 2 * k));
            for (j, mask) in [0xf00u16, 0xf0, 0xf].into_iter().enumerate() {
                let d: u8 = match (b & mask).cmp(&(a & mask)) {
                    std::cmp::Ordering::Less => 0xff,
                    std::cmp::Ordering::Greater => 1,
                    std::cmp::Ordering::Equal => 0,
                };
                m.set_b(self.dirs + 3 * k + j as u32, d);
            }
        }
        for k in 0..count as u32 {
            let c = m.w(from + 2 * k);
            m.set_w(list + 2 + 4 * k, c);
        }
        m.set_w(self.at + DONE, 0);
    }

    /// One call of the record's step (the vertical blank's, or the boot's main loop's): returns
    /// whether it changed a colour (the long case: the original's step then takes lines).
    pub fn step(&self, m: &mut StoryMem) -> bool {
        matches!(self.step_walk(m), Walk::Stepped { changed: true, .. })
    }

    /// [`Rec::step`], telling the path the original's code took and each colour's writes.
    pub fn step_walk(&self, m: &mut StoryMem) -> Walk {
        let kind = m.w(self.at + KIND);
        if !(1..=3).contains(&kind) {
            return Walk::Other;
        }
        if self.done(m) {
            return Walk::Done;
        }
        let counter = m.w(self.at + COUNTER).wrapping_add(1);
        m.set_w(self.at + COUNTER, counter);
        if (counter.wrapping_sub(m.w(self.at + SPEED)) as i16) < 0 {
            return Walk::Waiting;
        }
        m.set_w(self.at + COUNTER, 0);
        let mut colours = Vec::new();
        let changed = match kind {
            1 => self.step_across(m, &mut colours),
            2 => self.step_in(m, &mut colours),
            _ => self.step_out(m, &mut colours),
        };
        if !changed {
            m.set_w(self.at + DONE, 1);
            m.set_w(self.at + KIND, 0);
        }
        Walk::Stepped { colours, changed }
    }

    fn step_in(&self, m: &mut StoryMem, colours: &mut Vec<Colour>) -> bool {
        let n = m.w(self.at + COUNT) as u32;
        let (list, target) = (m.l(self.at + LIST), m.l(self.at + TARGET));
        let mut changed = false;
        for k in 0..n {
            let a = list + 4 * (k + 1);
            let t = m.w(target + 2 * (k + 1));
            let mut c = Colour::at(a);
            for (j, sh) in [0u32, 4, 8].into_iter().enumerate() {
                let mask = 0xf00u16 >> sh;
                if m.w(a) & mask != t & mask {
                    let clear = m.w(a) & !mask;
                    let acc_at = self.accs + 6 * k + 2 * j as u32;
                    let acc = m
                        .w(acc_at)
                        .wrapping_add(m.w(self.steps + 6 * k + 2 * j as u32));
                    m.set_w(acc_at, acc);
                    let v = (m.w(a) & !mask).wrapping_add((acc & 0xf00) >> sh);
                    m.set_w(a, v);
                    c.writes[j] = Some((clear, v));
                    changed = true;
                }
            }
            colours.push(c);
        }
        changed
    }

    fn step_out(&self, m: &mut StoryMem, colours: &mut Vec<Colour>) -> bool {
        let n = m.w(self.at + COUNT) as u32;
        let list = m.l(self.at + LIST);
        let mut changed = false;
        for k in 0..n {
            let a = list + 4 * (k + 1);
            let mut c = Colour::at(a);
            for (j, sh) in [0u32, 4, 8].into_iter().enumerate() {
                let mask = 0xf00u16 >> sh;
                if m.w(a) & mask != 0 {
                    let clear = m.w(a) & !mask;
                    let acc_at = self.accs + 6 * k + 2 * j as u32;
                    let acc = m
                        .w(acc_at)
                        .wrapping_sub(m.w(self.steps + 6 * k + 2 * j as u32));
                    m.set_w(acc_at, acc);
                    // The blue nibble comes down by a long shift: the accumulator's bits above
                    // $f00 are masked off first, so a word shift gives the same.
                    let v = (m.w(a) & !mask).wrapping_add((acc & 0xf00) >> sh);
                    m.set_w(a, v);
                    c.writes[j] = Some((clear, v));
                    changed = true;
                }
            }
            colours.push(c);
        }
        changed
    }

    fn step_across(&self, m: &mut StoryMem, colours: &mut Vec<Colour>) -> bool {
        let n = m.w(self.at + COUNT) as u32;
        let (list, target) = (m.l(self.at + LIST), m.l(self.at + TARGET));
        let mut same = 0;
        for k in 0..n {
            let a = list + 2 + 4 * k;
            let t = m.w(target + 2 * k);
            let mut col = Colour::at(a);
            for (j, sh) in [8u32, 4, 0].into_iter().enumerate() {
                let mask = 0xfu16 << sh;
                let c = m.w(a);
                if c & mask == t & mask {
                    same += 1;
                } else {
                    let d = m.b(self.dirs + 3 * k + j as u32);
                    let v = ((c >> sh & 0xf) as u8).wrapping_add(d) as u16 & 0xf;
                    m.set_w(a, c & !mask | v << sh);
                    col.writes[j] = Some((c & !mask, c & !mask | v << sh));
                }
            }
            colours.push(col);
        }
        same != 3 * n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec() -> Rec {
        Rec::intro(0x1000)
    }

    #[test]
    fn a_fade_in_reaches_the_target_and_a_fade_out_black() {
        let mut m = StoryMem::default();
        let (list, target) = (0x2000, 0x3000);
        for (k, c) in [0xfffu16, 0x842, 0x00f, 0x000].into_iter().enumerate() {
            m.set_w(list + 4 * k as u32, 0x180 + 2 * k as u16);
            m.set_w(target + 2 * k as u32, c);
        }
        let r = rec();
        m.set_w(r.at + SPEED, 1);
        r.start_in(&mut m, list, target, 4);
        let mut frames = 0;
        while !r.done(&m) {
            r.step(&mut m);
            frames += 1;
            assert!(frames < 40);
        }
        assert_eq!(
            (0..4).map(|k| m.w(list + 2 + 4 * k)).collect::<Vec<_>>(),
            [0xfff, 0x842, 0x00f, 0x000]
        );
        assert_eq!(m.w(r.at + KIND), 0);
        r.start_out(&mut m, list, 4);
        while !r.done(&m) {
            r.step(&mut m);
        }
        assert!((0..4).all(|k| m.w(list + 2 + 4 * k) == 0));
        // Across: one step a frame per component.
        r.start_across(&mut m, target, target + 0x10, list, 1);
        m.set_w(target + 0x10, 0xf0e);
        assert_eq!(m.w(list + 2), 0xfff);
    }
}
