//! A minimal zip writer for tests: builds archives the reader in `sf_disk::zip` must take apart.

#![allow(dead_code)]

#[derive(Clone, Copy)]
pub enum Method {
    Stored,
    Deflated,
}

/// A zip holding `files` (name, contents, method) in order, with a central directory.
pub fn write_zip(files: &[(&str, &[u8], Method)]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut dir = Vec::new();
    for &(name, data, method) in files {
        let (code, packed) = match method {
            Method::Stored => (0u16, data.to_vec()),
            Method::Deflated => (8u16, miniz_oxide::deflate::compress_to_vec(data, 6)),
        };
        let crc = sf_crc32(data);
        let local = out.len() as u32;
        let common = |v: &mut Vec<u8>| {
            v.extend_from_slice(&20u16.to_le_bytes()); // version needed
            v.extend_from_slice(&0u16.to_le_bytes()); // flags
            v.extend_from_slice(&code.to_le_bytes());
            v.extend_from_slice(&0u32.to_le_bytes()); // time, date
            v.extend_from_slice(&crc.to_le_bytes());
            v.extend_from_slice(&(packed.len() as u32).to_le_bytes());
            v.extend_from_slice(&(data.len() as u32).to_le_bytes());
            v.extend_from_slice(&(name.len() as u16).to_le_bytes());
            v.extend_from_slice(&0u16.to_le_bytes()); // extra
        };
        out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        common(&mut out);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&packed);

        dir.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        dir.extend_from_slice(&20u16.to_le_bytes()); // version made by
        common(&mut dir);
        dir.extend_from_slice(&[0u8; 8]); // comment len, disk, internal attrs, external attrs
        dir.extend_from_slice(&[0u8; 2]);
        dir.extend_from_slice(&local.to_le_bytes());
        dir.extend_from_slice(name.as_bytes());
    }
    let dir_at = out.len() as u32;
    out.extend_from_slice(&dir);
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&[0u8; 4]); // this disk, directory disk
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(files.len() as u16).to_le_bytes());
    out.extend_from_slice(&(dir.len() as u32).to_le_bytes());
    out.extend_from_slice(&dir_at.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment
    out
}

fn sf_crc32(data: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in data {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    !c
}
