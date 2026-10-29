// The test zip writer is the toolkit's, beside the reader it tests (toolkit-extract T03).
#[path = "../../../../../toolkit/crates/amiga-disk/tests/common/zipwrite.rs"]
pub mod zipwrite;

// What `zipwrite` builds on, found as `super::` both here and when `zip.rs`'s own tests include it.
pub use sf_disk::zip::{crc32, write_stored};
