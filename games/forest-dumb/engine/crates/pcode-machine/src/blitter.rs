//! The blitter on the machine (translator T24): a BLTSIZE write runs `amiga_chips::Blit` over chip
//! RAM at once, from the registers as last written, and the blitter stays busy until the time
//! `amiga_clock::word_times` gives its last word (DMACONR's BBUSY); then INTREQ's BLIT bit is set.
//! The memory effect is whole at the write: a program reading the destination before the busy bit
//! clears would see it early. It waits on BBUSY first, as the hardware asks.

use amiga_chips::{Blit, BlitMem, bltsize_words, word_times};

use crate::Machine;
use crate::beam;
use crate::journal::{Journal, Watch};

pub const BLTCON0: u16 = 0x040;
pub const BLTCON1: u16 = 0x042;
pub const BLTAFWM: u16 = 0x044;
pub const BLTALWM: u16 = 0x046;
pub const BLTCPT: u16 = 0x048;
pub const BLTBPT: u16 = 0x04C;
pub const BLTAPT: u16 = 0x050;
pub const BLTDPT: u16 = 0x054;
pub const BLTSIZE: u16 = 0x058;
pub const BLTCMOD: u16 = 0x060;
pub const BLTBMOD: u16 = 0x062;
pub const BLTAMOD: u16 = 0x064;
pub const BLTDMOD: u16 = 0x066;
pub const BLTCDAT: u16 = 0x070;
pub const BLTBDAT: u16 = 0x072;
pub const BLTADAT: u16 = 0x074;

/// INTREQ's blitter-done bit.
pub const INT_BLIT: u16 = 0x0040;

/// DMACON's DMAEN and BLTEN: both on for the blitter to run.
const BLIT_DMA: u16 = 0x0240;

/// What the blitter is doing.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Blitter {
    /// The cycle its busy flag clears and its interrupt is raised, while a blit runs.
    pub busy_until: Option<u64>,
    /// DMACONR's BZERO: every D result of the last blit was zero.
    pub zero: bool,
}

/// Chip RAM as the blit sees it: pointers are chip offsets (the blit masks them to `$7FFFE`),
/// mirrored into a smaller chip RAM; every changed byte is journalled at the blit's start.
struct View<'a> {
    chip: &'a mut [u8],
    journal: &'a mut Journal,
    watch: &'a mut Watch,
    at: u64,
}

impl BlitMem for View<'_> {
    fn blit_w(&self, a: u32) -> u16 {
        let i = (a as usize) & (self.chip.len() - 1) & !1;
        u16::from_be_bytes([self.chip[i], self.chip[i + 1]])
    }

    fn blit_set_w(&mut self, a: u32, v: u16) {
        let i = (a as usize) & (self.chip.len() - 1) & !1;
        for (k, b) in v.to_be_bytes().into_iter().enumerate() {
            let old = self.chip[i + k];
            if old != b {
                self.journal.record(self.at, (i + k) as u32, old);
                self.watch.written((i + k) as u32, self.at);
                self.chip[i + k] = b;
            }
        }
    }
}

impl Machine {
    /// BLTSIZE was written: run the blit, now.
    pub(crate) fn start_blit(&mut self) {
        let r = |reg: u16| self.custom[reg as usize >> 1];
        let (con0, con1) = (r(BLTCON0), r(BLTCON1));
        // The program's area blits are what the routine oracle saw; line and fill mode are not
        // modelled, and a run that needs them stops rather than draw something else.
        if con1 & 1 != 0 {
            return self.unsupported("blit line");
        }
        if con1 & 0x18 != 0 {
            return self.unsupported("blit fill");
        }
        if self.dmacon & BLIT_DMA != BLIT_DMA {
            return self.unsupported("blit with blitter DMA off");
        }
        let ptr = |reg: u16| (r(reg) as u32) << 16 | r(reg + 2) as u32;
        let blit = Blit {
            con0,
            con1,
            afwm: r(BLTAFWM),
            alwm: r(BLTALWM),
            pt: [ptr(BLTAPT), ptr(BLTBPT), ptr(BLTCPT), ptr(BLTDPT)],
            modulo: [BLTAMOD, BLTBMOD, BLTCMOD, BLTDMOD].map(|m| r(m) as i16),
            size: r(BLTSIZE),
        };
        let dat = [r(BLTADAT), r(BLTBDAT), r(BLTCDAT)];
        let at = self.now;
        let end = blit.run_with(
            dat,
            &mut View {
                chip: &mut self.chip,
                journal: &mut self.journal,
                watch: &mut self.watch,
                at,
            },
        );
        // The pointers carry on from where the blit left them: a following BLTSIZE write alone
        // continues the copy.
        for (reg, p) in [BLTAPT, BLTBPT, BLTCPT, BLTDPT].into_iter().zip(end.pt) {
            self.custom[reg as usize >> 1] = (p >> 16) as u16;
            self.custom[(reg as usize >> 1) + 1] = p as u16;
        }
        let (rows, words) = bltsize_words(blit.size);
        let fetch = self.fetch();
        let (line, h) = self.beam(at);
        let frames = self.cfg.frame_lines;
        let times = word_times(rows * words, line, h, frames, |l, c| fetch.takes(l, c));
        let (ll, lh) = *times.last().expect("a blit has a word");
        // `word_times` counts on past the frame's last line; offsets from the frame's start are
        // linear in the line, so the end lands in the next frame where it should.
        // Signed: in a snapshot's first frame the frame began before cycle 0.
        let into = (self.phase.wrapping_add(at)) % beam::frame_cycles(frames);
        let start = at as i64 - into as i64;
        let done = start + beam::offset(ll, lh) as i64 + 2;
        self.blitter = super::blitter::Blitter {
            busy_until: Some((done.max(0) as u64).max(at + 1)),
            zero: end.zero,
        };
    }

    /// DMACONR's BBUSY at the access time.
    pub(crate) fn blit_busy(&self) -> bool {
        self.blitter.busy_until.is_some_and(|e| e > self.now)
    }
}
