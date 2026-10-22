//! A zip writer for tests: builds archives the reader in `sf_disk::zip` must take apart. Stored
//! entries are `sf_disk::zip::write_stored`'s; a deflated entry is written stored with its
//! compressed bytes, then its header fields are patched to say deflated.

#![allow(dead_code)]

use super::{crc32, write_stored};

#[derive(Clone, Copy)]
pub enum Method {
    Stored,
    Deflated,
}

/// A zip holding `files` (name, contents, method) in order, with a central directory.
pub fn write_zip(files: &[(&str, &[u8], Method)]) -> Vec<u8> {
    let packed: Vec<Vec<u8>> = files
        .iter()
        .map(|&(_, data, method)| match method {
            Method::Stored => data.to_vec(),
            Method::Deflated => miniz_oxide::deflate::compress_to_vec(data, 6),
        })
        .collect();
    let entries: Vec<(&str, &[u8])> = files
        .iter()
        .zip(&packed)
        .map(|(&(name, ..), p)| (name, p.as_slice()))
        .collect();
    let mut out = write_stored(&entries);
    let le32 = |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap());
    let le16 =
        |b: &[u8], at: usize| usize::from(u16::from_le_bytes(b[at..at + 2].try_into().unwrap()));
    let mut central = le32(&out, out.len() - 22 + 16) as usize;
    for &(_, data, method) in files {
        // The directory entry's local header offset, then method/CRC/size in both headers.
        let local = le32(&out, central + 42) as usize;
        if let Method::Deflated = method {
            for (method_at, crc_at, size_at) in [
                (local + 8, local + 14, local + 22),
                (central + 10, central + 16, central + 24),
            ] {
                out[method_at..method_at + 2].copy_from_slice(&8u16.to_le_bytes());
                out[crc_at..crc_at + 4].copy_from_slice(&crc32(data).to_le_bytes());
                out[size_at..size_at + 4].copy_from_slice(&(data.len() as u32).to_le_bytes());
            }
        }
        central += 46 + le16(&out, central + 28);
    }
    out
}
