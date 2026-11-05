//! The coverage report (plans/forest-dumb-level-1 DESIGN §4.1): how often the original executed each
//! routine of the port register during a case, from T31's counter (`Amiga::watch_pcs`). A ported
//! routine with no hits in any case is not proven by any case; a routine the case never reaches
//! shows zero, never disappears.

use std::fmt;

use fd_core::Ported;

use crate::HUNK_BASES;

/// Per case: each registered routine's executions of its first instruction on the original, in
/// register order, from the first step after the sync point to the last.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Coverage {
    pub hits: Vec<(&'static str, u32)>,
}

/// Where `p`'s first instruction is on the oracle.
pub fn address(p: &Ported) -> u32 {
    HUNK_BASES[p.hunk as usize] + p.offset
}

impl Coverage {
    /// Every routine of `register` at zero.
    pub fn new(register: &[Ported]) -> Coverage {
        Coverage {
            hits: register.iter().map(|p| (p.name, 0)).collect(),
        }
    }

    /// Add one step's counts, `(pc, count)` as `Amiga::pc_hits` gives them.
    pub fn add(&mut self, register: &[Ported], pc_hits: &[(u32, u32)]) {
        for (slot, p) in self.hits.iter_mut().zip(register) {
            let at = address(p);
            slot.1 += pc_hits
                .iter()
                .find(|(pc, _)| *pc == at)
                .map_or(0, |(_, n)| *n);
        }
    }

    /// The routines the case never reached.
    pub fn unreached(&self) -> Vec<&'static str> {
        self.hits
            .iter()
            .filter(|(_, n)| *n == 0)
            .map(|(name, _)| *name)
            .collect()
    }
}

impl fmt::Display for Coverage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.hits.is_empty() {
            return write!(f, "coverage: no routine is registered in fd-core's PORTED");
        }
        write!(f, "coverage on the original:")?;
        for (name, n) in &self.hits {
            write!(f, "\n  {name:<28} {n:>8}")?;
            if *n == 0 {
                write!(f, "  never reached")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REG: [Ported; 2] = [
        Ported {
            name: "a",
            hunk: 0,
            offset: 0x10,
            port: "x::a",
        },
        Ported {
            name: "b",
            hunk: 0,
            offset: 0x20,
            port: "x::b",
        },
    ];

    #[test]
    fn counts_add_up_per_routine_and_an_unreached_one_shows_zero() {
        let mut c = Coverage::new(&REG);
        let a = HUNK_BASES[0] + 0x10;
        c.add(&REG, &[(a, 2), (0x1f02, 1)]);
        c.add(&REG, &[(a, 3)]);
        assert_eq!(c.hits, vec![("a", 5), ("b", 0)]);
        assert_eq!(c.unreached(), vec!["b"]);
        let text = c.to_string();
        assert!(text.contains("never reached"), "{text}");
    }
}
