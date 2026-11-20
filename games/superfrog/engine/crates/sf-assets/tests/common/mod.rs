//! The owner's disks, unpacked once per test binary (found by `sf_conform::disks`; a missing disk
//! fails the test with where it looked).

#![allow(dead_code)]

use std::sync::OnceLock;

use sf_assets::{ChunkId, GameData, catalogue, decode, unpack};
use sf_disk::{DiskSet, Offered, identify};

pub fn disks() -> &'static DiskSet {
    static SET: OnceLock<DiskSet> = OnceLock::new();
    SET.get_or_init(|| {
        let files: Vec<Offered> = sf_conform::disks::read_disk_files()
            .unwrap_or_else(|e| panic!("{e}"))
            .into_iter()
            .map(|(name, bytes)| Offered { name, bytes })
            .collect();
        identify(&files).unwrap_or_else(|e| panic!("{e:?}"))
    })
}

pub fn chunks() -> &'static [(ChunkId, Vec<u8>)] {
    static CHUNKS: OnceLock<Vec<(ChunkId, Vec<u8>)>> = OnceLock::new();
    CHUNKS.get_or_init(|| unpack(disks()).unwrap_or_else(|e| panic!("{e}")))
}

pub fn data() -> &'static GameData {
    static DATA: OnceLock<GameData> = OnceLock::new();
    DATA.get_or_init(|| decode(chunks()).unwrap_or_else(|e| panic!("{e}")))
}

pub fn chunk(id: ChunkId) -> &'static [u8] {
    &chunks().iter().find(|(c, _)| *c == id).unwrap().1
}

/// The `nth` chunk the catalogue calls `name`.
pub fn named(name: &str, nth: usize) -> ChunkId {
    catalogue(chunks())
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .filter(|(n, _)| *n == name)
        .nth(nth)
        .unwrap_or_else(|| panic!("{name} #{nth} not in the catalogue"))
        .1
}
