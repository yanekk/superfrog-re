//! Carve a disk into its `ATN!` chunks: a port of `carve_chunks` in `tools/atn.py` (docs/01).
//!
//! Chunks sit back to back on 512-byte sector boundaries. The carver looks for the magic only at
//! sector starts, so the stray sub-sector `ATN!` bytes inside the loader code on disks 1 and 3 are
//! never seen, and a header failing the size sanity check is stepped over a sector at a time.

use std::fmt;

use crate::{Adf, DiskSlot};

const MAGIC: &[u8; 4] = b"ATN!";
const SECTOR: usize = 512;
const HEADER: usize = 12;
/// The Python carver's sanity bounds, kept identical so both find the same chunks.
const MAX_COMP: u32 = 900_000;
pub(crate) const MAX_DECOMP: u32 = 4_000_000;

/// One `ATN!` chunk as it sits on the disk, still compressed.
#[derive(Clone, PartialEq, Eq)]
pub struct Chunk {
    pub disk: DiskSlot,
    /// 1-based position on its disk, as in `docs/05-chunk-inventory.txt` and `d1_c01` file names.
    pub index: u16,
    /// Byte offset of the magic in the ADF.
    pub offset: u32,
    pub decomp_size: u32,
    /// Size of the compressed region including the 12-byte header.
    pub comp_size: u32,
    /// From the magic to the next sector boundary (clipped to the disk's end): the compressed
    /// stream and the trailer in the sector padding after it, both of which the decompressor needs.
    pub raw: Vec<u8>,
}

impl fmt::Debug for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Chunk")
            .field("disk", &self.disk)
            .field("index", &self.index)
            .field("offset", &format_args!("{:#x}", self.offset))
            .field("decomp_size", &self.decomp_size)
            .field("comp_size", &self.comp_size)
            .field("raw_len", &self.raw.len())
            .finish()
    }
}

/// Every chunk on `adf`, in disk order: the same chunks, in the same order, as `tools/atn.py`.
pub fn carve(adf: &Adf) -> Vec<Chunk> {
    let data = &adf.bytes;
    let n = data.len();
    let be32 = |at: usize| u32::from_be_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
    let mut chunks = Vec::new();
    let mut o = 0;
    while o + HEADER <= n {
        if &data[o..o + 4] == MAGIC {
            let (decomp_size, comp_size) = (be32(o + 4), be32(o + 8));
            if 0 < comp_size
                && comp_size < MAX_COMP
                && 0 < decomp_size
                && decomp_size < MAX_DECOMP
                && o + HEADER + comp_size as usize <= n
            {
                let end = o + (HEADER + comp_size as usize).div_ceil(SECTOR) * SECTOR;
                chunks.push(Chunk {
                    disk: adf.slot,
                    index: chunks.len() as u16 + 1,
                    offset: o as u32,
                    decomp_size,
                    comp_size,
                    raw: data[o..end.min(n)].to_vec(),
                });
                o = end;
                continue;
            }
        }
        o += SECTOR;
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk(bytes: Vec<u8>) -> Adf {
        Adf {
            slot: DiskSlot::D2,
            source: "test.adf".into(),
            bytes,
        }
    }

    fn header(at: usize, buf: &mut [u8], decomp: u32, comp: u32) {
        buf[at..at + 4].copy_from_slice(MAGIC);
        buf[at + 4..at + 8].copy_from_slice(&decomp.to_be_bytes());
        buf[at + 8..at + 12].copy_from_slice(&comp.to_be_bytes());
    }

    #[test]
    fn back_to_back_chunks_on_sector_boundaries() {
        let mut b = vec![0u8; 8 * SECTOR];
        header(0, &mut b, 100, 600); // 612 bytes -> two sectors
        header(2 * SECTOR, &mut b, 50, 500); // exactly one sector
        let c = carve(&disk(b.clone()));
        assert_eq!(c.len(), 2, "{c:?}");
        assert_eq!((c[0].index, c[0].offset, c[0].raw.len()), (1, 0, 1024));
        assert_eq!((c[1].index, c[1].offset, c[1].raw.len()), (2, 1024, 512));
        assert_eq!(
            (c[1].decomp_size, c[1].comp_size, c[1].disk),
            (50, 500, DiskSlot::D2)
        );
        assert_eq!(c[1].raw, b[1024..1536]);
    }

    #[test]
    fn magic_off_a_sector_start_or_failing_sanity_is_skipped() {
        let mut b = vec![0u8; 8 * SECTOR];
        header(100, &mut b, 10, 10); // not sector aligned
        header(SECTOR, &mut b, 0, 10); // decomp 0
        header(2 * SECTOR, &mut b, 10, 0); // comp 0
        header(3 * SECTOR, &mut b, 10, MAX_COMP); // comp too large
        header(4 * SECTOR, &mut b, MAX_DECOMP, 10); // decomp too large
        header(5 * SECTOR, &mut b, 10, 3 * SECTOR as u32); // runs off the end
        header(6 * SECTOR, &mut b, 10, 20); // fine
        let c = carve(&disk(b));
        assert_eq!(c.len(), 1, "{c:?}");
        assert_eq!((c[0].index, c[0].offset), (1, 6 * SECTOR as u32));
    }

    #[test]
    fn a_chunk_ending_inside_the_last_sector_is_clipped_to_the_disk() {
        let mut b = vec![0u8; SECTOR + 100];
        header(SECTOR, &mut b, 10, 80);
        let c = carve(&disk(b));
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].raw.len(), 100);
    }

    #[test]
    fn short_or_empty_input_gives_nothing() {
        assert!(carve(&disk(Vec::new())).is_empty());
        assert!(carve(&disk(b"ATN!".to_vec())).is_empty());
    }
}
