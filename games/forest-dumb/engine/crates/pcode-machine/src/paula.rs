//! Paula's writes as the frame bar compares them (translator DESIGN §2.7): every write to DMACON,
//! INTENA, INTREQ, ADKCON or an audio channel's registers, in the order made, with the cycle count
//! and the beam position it was made at. The sound itself is `amiga-paula`'s, played from this list
//! by the app; the machine raises no audio interrupt, because the program enables none (T01:
//! level 4 is never taken; its INTENA writes are `$c018` and the OS's own back).

use amiga_paula::PaulaWrite;

pub use amiga_chips::is_paula;

/// The writes of the frame so far, oldest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PaulaLog {
    writes: Vec<(u64, PaulaWrite)>,
}

impl PaulaLog {
    /// Keep `value` written to `reg` at cycle count `at`, the beam then at `(line, h)`, if `reg` is
    /// Paula's.
    pub fn note(&mut self, reg: u16, value: u16, at: u64, (line, h): (u16, u16)) {
        if is_paula(reg) {
            self.writes.push((
                at,
                PaulaWrite {
                    reg,
                    value,
                    hpos: h,
                    vpos: line,
                },
            ));
        }
    }

    pub fn writes(&self) -> &[(u64, PaulaWrite)] {
        &self.writes
    }

    /// The frame's writes, leaving the log empty for the next.
    pub fn take(&mut self) -> Vec<(u64, PaulaWrite)> {
        std::mem::take(&mut self.writes)
    }
}
