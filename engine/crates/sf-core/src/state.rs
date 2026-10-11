//! The game's state: every original variable the engine models, as typed Rust, plus the
//! conversions to and from the original's memory that [`LAYOUT`] drives.

use crate::layout::{LAYOUT, Ram, VarId};
use crate::rng::Rng;

/// What the original keeps in memory, as far as the engine models it. A power-on state is all
/// zero, as the oracle's RAM is (FINDINGS 2026-10-11: RAM zeroed).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct State {
    pub rng: Rng,
}

impl State {
    /// The state the original's memory holds: every [`LAYOUT`] entry read from `ram`.
    pub fn transplant(ram: &dyn Ram) -> State {
        let mut s = State::default();
        for d in LAYOUT {
            let bytes: Vec<u8> = (0..d.width.len()).map(|k| ram.byte(d.addr + k)).collect();
            s.set_bytes(d.id, &bytes);
        }
        s
    }

    /// Every [`LAYOUT`] entry's value, in table order, one pair per [`crate::VarDef::pieces`] piece.
    pub fn probe(&self) -> Vec<(VarId, u32)> {
        LAYOUT
            .iter()
            .flat_map(|d| {
                let bytes = self.bytes(d.id);
                d.pieces()
                    .map(|p| {
                        let at = (p.addr - d.addr) as usize;
                        let v = bytes[at..at + p.width.len() as usize]
                            .iter()
                            .fold(0u32, |v, &b| v << 8 | b as u32);
                        (d.id, v)
                    })
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// Overwrite the bytes at `addr..addr + bytes.len()` that modelled variables cover, as a write
    /// to the original's memory would; bytes no variable covers are ignored. Conformance tooling
    /// uses it to plant a known divergence.
    pub fn poke(&mut self, addr: u32, bytes: &[u8]) {
        for d in LAYOUT {
            let mut cur = self.bytes(d.id);
            let mut hit = false;
            for (k, &b) in bytes.iter().enumerate() {
                let a = addr + k as u32;
                if (d.addr..d.addr + d.width.len()).contains(&a) {
                    cur[(a - d.addr) as usize] = b;
                    hit = true;
                }
            }
            if hit {
                self.set_bytes(d.id, &cur);
            }
        }
    }

    /// `id`'s value as the original stores it (big-endian, its [`crate::VarDef`]'s width).
    fn bytes(&self, id: VarId) -> Vec<u8> {
        match id {
            VarId::RngState => self.rng.to_bytes().to_vec(),
        }
    }

    fn set_bytes(&mut self, id: VarId, b: &[u8]) {
        match id {
            VarId::RngState => self.rng = Rng::from_bytes(b.try_into().expect("8 bytes")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{AmigaMemory, SLOW_BASE, def};

    fn memory_with_rng(bytes: [u8; 8]) -> AmigaMemory {
        let mut m = AmigaMemory {
            chip: vec![0; 512 * 1024],
            slow: vec![0; 512 * 1024],
        };
        let at = (def(VarId::RngState).addr - SLOW_BASE) as usize;
        m.slow[at..at + 8].copy_from_slice(&bytes);
        m
    }

    #[test]
    fn transplant_then_probe_gives_back_the_memory() {
        let s = State::transplant(&memory_with_rng([1, 2, 3, 4, 5, 6, 7, 8]));
        assert_eq!(
            s.rng,
            Rng {
                lo: 0x0102_0304,
                hi: 0x0506_0708
            }
        );
        assert_eq!(
            s.probe(),
            vec![
                (VarId::RngState, 0x0102_0304),
                (VarId::RngState, 0x0506_0708)
            ]
        );
    }

    #[test]
    fn poke_changes_only_covered_bytes() {
        let mut s = State::default();
        let at = def(VarId::RngState).addr;
        s.poke(at + 6, &[0xAA, 0xBB, 0xCC]);
        assert_eq!(s.rng, Rng { lo: 0, hi: 0xAABB });
        s.poke(0x100, &[1]);
        assert_eq!(s.rng, Rng { lo: 0, hi: 0xAABB });
    }
}
