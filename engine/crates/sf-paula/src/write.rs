//! One write to a Paula-related custom register: the chip's own event, shared by the engine
//! (`sf-core`), the mixer (`sf-paula`) and the oracle (`sf-oracle`).
//!
//! Conformance compares the ordered `(reg, value)` list of a frame. `vpos`/`hpos` are the beam
//! position the original made the write at, recorded for diagnosis only: the engine is not
//! cycle-timed and cannot know them (DESIGN §2.1).

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PaulaWrite {
    /// Offset from 0xDFF000: DMACON 0x96, INTENA 0x9A, INTREQ 0x9C, ADKCON 0x9E, AUDx 0xA0..=0xDE.
    pub reg: u16,
    pub value: u16,
    pub hpos: u16,
    pub vpos: u16,
}

impl PaulaWrite {
    /// What conformance compares.
    pub fn reg_value(&self) -> (u16, u16) {
        (self.reg, self.value)
    }
}
