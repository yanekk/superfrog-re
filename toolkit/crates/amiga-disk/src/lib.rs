//! Pure: the disk-side formats any Amiga game's tools need. A zip reader and a stored-entry writer
//! (disk images come zipped; a bug recorder writes zips), SHA-1 to identify an image, and the size
//! of a double-density ADF. Naming a particular game's disks is the game's own crate's job.

pub mod sha1;
pub mod zip;

/// Bytes in a double-density ADF: 2 sides × 80 cylinders × 11 sectors × 512 bytes.
pub const ADF_SIZE: usize = 901_120;
