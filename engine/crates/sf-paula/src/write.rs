//! One write to a Paula-related custom register: the chip's own event, shared by the engine
//! (`sf-core`), the mixer (`sf-paula`) and the oracle (`sf-oracle`).
//!
//! Conformance compares the ordered `(reg, value)` list of a frame. `vpos`/`hpos` are the beam
//! position the write was made at: the original's (the oracle's), or where the engine's model of
//! the original's timing puts it, which is not cycle-exact (DESIGN §2.1). The mixer plays each
//! write at its position: a channel turned off and on again restarts its note only if Paula had
//! the time between to let it go idle (T213).

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
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
