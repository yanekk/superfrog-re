//! The blitter on the machine (translator T24): a BLTSIZE write runs `amiga_chips::Blit` over chip
//! RAM at once, from the registers as last written; its time is the bus's (`dma`, T49): the
//! blitter's micro-program runs slot by slot against the display's fetches and the CPU, DMACONR's
//! BBUSY clears at the last word's end and INTREQ's BLIT bit is set a slot after the last D write.
//! Chip RAM holds the whole result from the write on: a program reading the destination before
//! the busy bit clears would see it early (it waits on BBUSY first, as the hardware asks). The
//! picture and the copper see each D word from the slot the bus writes it in (T52): a blit racing
//! the beam is drawn part done, as on the original.

use amiga_chips::{Blit, BlitMem, bltsize_words};

use crate::Machine;
use crate::dma::BlitRun;
use crate::journal::Watch;

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
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Blitter {
    /// The blit running: where its micro-program has got to on the bus, and when its busy flag
    /// cleared (`dma`).
    pub(crate) run: Option<BlitRun>,
    /// DMACONR's BZERO: every D result of the last blit was zero.
    pub zero: bool,
    /// The running blit's changed bytes not yet journalled, (chip offset, byte replaced, D word
    /// number), in word order: each is journalled at the slot its word's D write runs in.
    pub(crate) pending: Vec<(u32, u8, u32)>,
}

/// Chip RAM as the blit sees it: pointers are chip offsets (the blit masks them to `$7FFFE`),
/// mirrored into a smaller chip RAM; every changed byte is kept with its D word's number, to be
/// journalled when the bus writes that word.
struct View<'a> {
    chip: &'a mut [u8],
    pending: &'a mut Vec<(u32, u8, u32)>,
    watch: &'a mut Watch,
    at: u64,
    word: u32,
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
                self.pending.push(((i + k) as u32, old, self.word));
                self.watch.written((i + k) as u32, self.at);
                self.chip[i + k] = b;
            }
        }
        self.word += 1;
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
        // A blit cut short by this one: what it had not written yet lands now.
        self.blit_journal(Some(at));
        let mut pending = Vec::new();
        let end = blit.run_with(
            dat,
            &mut View {
                chip: &mut self.chip,
                pending: &mut pending,
                watch: &mut self.watch,
                at,
                word: 0,
            },
        );
        // The pointers carry on from where the blit left them: a following BLTSIZE write alone
        // continues the copy.
        for (reg, p) in [BLTAPT, BLTBPT, BLTCPT, BLTDPT].into_iter().zip(end.pt) {
            self.custom[reg as usize >> 1] = (p >> 16) as u16;
            self.custom[(reg as usize >> 1) + 1] = p as u16;
        }
        let (rows, words) = bltsize_words(blit.size);
        // The bus decides when it ends (`dma`). vAmiga applies a BLTSIZE write a slot later
        // (`recordRegisterChange(DMA_CYCLES(1))`) and the blitter's first start-up step a slot
        // after that (`BLT_STRT1`): it takes no slot before the second after the write.
        self.blitter = Blitter {
            run: Some(BlitRun::new(con0, rows as u16, words as u16, at + 4)),
            zero: end.zero,
            pending,
        };
    }

    /// Journal the running blit's changed bytes whose D word the bus has written, at that word's
    /// slot; with `rest` also the others, at `rest`.
    pub(crate) fn blit_journal(&mut self, rest: Option<u64>) {
        let b = &mut self.blitter;
        let written = b.run.as_ref().map_or(&[][..], |r| &r.d_at[..]);
        let mut n = 0;
        for &(off, old, word) in &b.pending {
            let at = match written.get(word as usize) {
                Some(&at) => at,
                None => match rest {
                    Some(at) => at,
                    None => break,
                },
            };
            self.journal.record(at, off, old);
            n += 1;
        }
        b.pending.drain(..n);
    }

    /// The running blit's bytes not yet written by the bus, as journal entries for a view of the
    /// frame: not written within it.
    pub(crate) fn blit_unwritten(&self) -> Vec<(u64, u32, u8)> {
        self.blitter
            .pending
            .iter()
            .map(|&(off, old, _)| (u64::MAX, off, old))
            .collect()
    }

    /// DMACONR's BBUSY at the access time.
    pub(crate) fn blit_busy(&self) -> bool {
        self.blitter
            .run
            .as_ref()
            .is_some_and(|b| b.free_at.is_none_or(|f| f > self.now))
    }
}
