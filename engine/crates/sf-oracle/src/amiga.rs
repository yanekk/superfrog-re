//! The toolkit's emulator with Superfrog's disks named by slot.

use std::ops::{Deref, DerefMut};

use amiga_oracle::OracleError;
use sf_disk::DiskSlot;

use crate::config::OracleConfig;

/// [`amiga_oracle::Amiga`] powered on from Superfrog's set-up, whose disks are the four slots
/// (disk `n` at index `n - 1`). Everything else is the toolkit's, through `Deref`.
pub struct Amiga(amiga_oracle::Amiga);

impl Amiga {
    /// Read the ROM and the four disks (read-only), insert [`OracleConfig::boot`]'s disk and switch
    /// on. Frame 0 is next.
    pub fn power_on(cfg: &OracleConfig) -> Result<Amiga, OracleError> {
        amiga_oracle::Amiga::power_on(&cfg.generic()?).map(Amiga)
    }

    /// Swap the disk in `drive` for `slot`'s, as a player answering the game's prompt does. A
    /// drive the set-up did not connect ([`OracleConfig::drives`]) panics.
    pub fn insert_disk(&mut self, drive: u8, slot: DiskSlot) {
        self.0.insert_disk(drive as usize, slot.index());
    }

    /// Whether disk 1 is the disk last put in df0.
    pub fn disk1_in(&self) -> bool {
        self.0.df0() == Some(DiskSlot::D1.index())
    }

    /// The toolkit's machine, for code that takes it directly.
    pub fn into_inner(self) -> amiga_oracle::Amiga {
        self.0
    }
}

impl Deref for Amiga {
    type Target = amiga_oracle::Amiga;

    fn deref(&self) -> &amiga_oracle::Amiga {
        &self.0
    }
}

impl DerefMut for Amiga {
    fn deref_mut(&mut self) -> &mut amiga_oracle::Amiga {
        &mut self.0
    }
}
