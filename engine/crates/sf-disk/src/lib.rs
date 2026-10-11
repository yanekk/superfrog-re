//! Pure: ADF/zip bytes -> identified disks -> chunks -> ATN! decompression (DESIGN §3.2).
//!
//! Import's first stage (DESIGN §2.2): [`identify`] takes the files the player offered (zipped or
//! raw ADFs, as bytes the shell read), finds the four Superfrog disks by SHA-1 and names every
//! problem by disk slot or file; [`carve`] cuts a disk into its `ATN!` chunks exactly as
//! `tools/atn.py` does.

mod adf;
mod carve;
mod identify;
mod sha1;
pub mod zip;

pub use adf::{ADF_SIZE, Adf, DiskSlot};
pub use carve::{Chunk, carve};
pub use identify::{DiskSet, ImportError, Offered, identify, known_sha1};
pub use sha1::sha1_hex;
