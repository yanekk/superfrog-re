//! A standard Amiga double-density disk image and the four slots of the Superfrog set.

use std::fmt;

/// Bytes in a double-density ADF: 2 sides × 80 cylinders × 11 sectors × 512 bytes.
pub const ADF_SIZE: usize = 901_120;

/// Which of the four Superfrog disks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DiskSlot {
    D1,
    D2,
    D3,
    D4,
}

impl DiskSlot {
    pub const ALL: [DiskSlot; 4] = [DiskSlot::D1, DiskSlot::D2, DiskSlot::D3, DiskSlot::D4];

    /// 1 to 4, as printed on the disk label.
    pub fn number(self) -> u8 {
        self.index() as u8 + 1
    }

    /// 0 to 3, the position in [`crate::DiskSet::disks`].
    pub fn index(self) -> usize {
        match self {
            DiskSlot::D1 => 0,
            DiskSlot::D2 => 1,
            DiskSlot::D3 => 2,
            DiskSlot::D4 => 3,
        }
    }
}

impl fmt::Display for DiskSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Disk {} of 4", self.number())
    }
}

/// One identified disk: its slot, the offered file it came from, and its 901 120 bytes.
#[derive(Clone, PartialEq, Eq)]
pub struct Adf {
    pub slot: DiskSlot,
    /// The name of the offered file (a zip or an `.adf`) this disk was read from.
    pub source: String,
    pub bytes: Vec<u8>,
}

impl fmt::Debug for Adf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Adf")
            .field("slot", &self.slot)
            .field("source", &self.source)
            .field("len", &self.bytes.len())
            .finish()
    }
}
