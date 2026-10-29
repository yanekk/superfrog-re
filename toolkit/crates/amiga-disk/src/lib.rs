//! Pure: the disk-side formats any Amiga game's tools need. A zip reader and a stored-entry writer
//! (disk images come zipped; a bug recorder writes zips), SHA-1 to identify an image, the size of a
//! double-density ADF, and the readers for what is on the disks: hunk executables, IFF ILBM and
//! ANIM, plane-major bitmaps and the OCS palette. Each reader takes bytes and returns the parsed
//! form or a [`ReadError`] with the byte offset. Naming a particular game's disks and files is the
//! game's own crate's job.

pub mod anim;
mod error;
pub mod hunk;
pub mod iff;
pub mod palette;
pub mod planar;
mod read;
pub mod sha1;
pub mod zip;

pub use error::ReadError;

/// Bytes in a double-density ADF: 2 sides × 80 cylinders × 11 sectors × 512 bytes.
pub const ADF_SIZE: usize = 901_120;
