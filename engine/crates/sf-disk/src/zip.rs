//! Just enough of the zip format to take disk images out of an archive: stored and deflated
//! entries, read through the central directory, each checked against its CRC-32. Every malformed
//! input is an `Err` naming what is wrong, never a panic. And just enough to write one: stored
//! entries only, for the bug recorder's zip (bug-recorder DESIGN §2.5).

/// One file taken out of a zip.
#[derive(Clone, PartialEq, Eq)]
pub struct Entry {
    /// The name inside the archive, folders included (`disks/Disk 1.adf`).
    pub name: String,
    pub bytes: Vec<u8>,
}

const LOCAL: u32 = 0x0403_4b50;
const CENTRAL: u32 = 0x0201_4b50;
const END: u32 = 0x0605_4b50;
const END_LEN: usize = 22;
const MAX_COMMENT: usize = 0xFFFF;

/// Whether `bytes` start like a zip archive (a local header, or the end record of an empty one).
pub fn is_zip(bytes: &[u8]) -> bool {
    matches!(le32(bytes, 0), Some(LOCAL | END))
}

/// The entries of `zip` whose name `want` accepts, in archive order. Folders are skipped. An entry
/// that claims more than `max_size` bytes is refused before it is inflated.
pub fn read_entries(
    zip: &[u8],
    want: impl Fn(&str) -> bool,
    max_size: usize,
) -> Result<Vec<Entry>, String> {
    let end = find_end(zip).ok_or("it is not a zip archive (no end-of-archive record)")?;
    let field = |at: usize| le16(zip, end + at).ok_or("its end-of-archive record is cut short");
    if field(4)? != 0 || field(6)? != 0 {
        return Err("it is one part of a zip split across several files".into());
    }
    let count = field(10)?;
    let dir = le32(zip, end + 16).ok_or("its end-of-archive record is cut short")?;
    if count == 0xFFFF || dir == 0xFFFF_FFFF {
        return Err("it is a zip64 archive, which is not supported".into());
    }
    let mut out = Vec::new();
    let mut p = dir as usize;
    for _ in 0..count {
        let cut = || "its file list is cut short or damaged".to_string();
        if le32(zip, p) != Some(CENTRAL) {
            return Err(cut());
        }
        let h = |at: usize| le16(zip, p + at).map(usize::from).ok_or_else(cut);
        let w = |at: usize| le32(zip, p + at).ok_or_else(cut);
        let (flags, method, crc) = (h(8)?, h(10)?, w(16)?);
        let (packed, size, local) = (w(20)?, w(24)?, w(42)?);
        let (name_len, extra_len, comment_len) = (h(28)?, h(30)?, h(32)?);
        let name = zip.get(p + 46..p + 46 + name_len).ok_or_else(cut)?;
        let name = String::from_utf8_lossy(name).into_owned();
        p += 46 + name_len + extra_len + comment_len;
        if name.ends_with('/') || !want(&name) {
            continue;
        }
        if flags & 1 != 0 {
            return Err(format!("“{name}” inside it is encrypted"));
        }
        if packed == 0xFFFF_FFFF || size == 0xFFFF_FFFF {
            return Err(format!(
                "“{name}” inside it is a zip64 entry, which is not supported"
            ));
        }
        if size as usize > max_size {
            return Err(format!(
                "“{name}” inside it is {size} bytes, too large to be a disk image"
            ));
        }
        let data = entry_data(zip, local as usize, packed as usize)
            .ok_or_else(|| format!("“{name}” inside it is cut short or damaged"))?;
        let bytes = match method {
            0 => data.to_vec(),
            8 => miniz_oxide::inflate::decompress_to_vec_with_limit(data, size as usize)
                .map_err(|_| format!("“{name}” inside it does not decompress"))?,
            m => {
                return Err(format!(
                    "“{name}” inside it uses compression method {m}, which is not supported"
                ));
            }
        };
        if bytes.len() != size as usize || crc32(&bytes) != crc {
            return Err(format!("“{name}” inside it fails its checksum"));
        }
        out.push(Entry { name, bytes });
    }
    Ok(out)
}

/// The `packed` bytes of the entry whose local header is at `at`.
fn entry_data(zip: &[u8], at: usize, packed: usize) -> Option<&[u8]> {
    if le32(zip, at)? != LOCAL {
        return None;
    }
    let start = at + 30 + usize::from(le16(zip, at + 26)?) + usize::from(le16(zip, at + 28)?);
    zip.get(start..start.checked_add(packed)?)
}

/// Offset of the end-of-central-directory record: the last one, searched back over a comment.
fn find_end(zip: &[u8]) -> Option<usize> {
    let last = zip.len().checked_sub(END_LEN)?;
    (last.saturating_sub(MAX_COMMENT)..=last)
        .rev()
        .find(|&at| le32(zip, at) == Some(END))
}

fn le16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        b.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

/// A zip holding `entries` (name, contents) in order, stored (no compression), with a central
/// directory: what [`read_entries`] reads back. Times are zero (1980-01-01), so the same entries
/// always make the same bytes. Panics past the plain zip format's limits (65 535 entries, 4 GiB),
/// which no caller comes near.
pub fn write_stored(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let count = u16::try_from(entries.len()).expect("a zip holds at most 65 534 entries");
    assert!(count < 0xFFFF, "a zip holds at most 65 534 entries");
    let mut out = Vec::new();
    let mut dir = Vec::new();
    for &(name, data) in entries {
        let size = u32::try_from(data.len()).ok().filter(|&s| s < 0xFFFF_FFFF);
        let size = size.expect("a zip entry is under 4 GiB");
        let name_len = u16::try_from(name.len()).expect("a zip entry's name is under 64 KiB");
        let local = u32::try_from(out.len()).expect("a zip is under 4 GiB");
        let crc = crc32(data);
        // The fields a local header and its directory entry share, from "version needed" on.
        let common = |v: &mut Vec<u8>| {
            v.extend_from_slice(&20u16.to_le_bytes()); // version needed
            v.extend_from_slice(&0u16.to_le_bytes()); // flags
            v.extend_from_slice(&0u16.to_le_bytes()); // method: stored
            v.extend_from_slice(&0u32.to_le_bytes()); // time, date
            v.extend_from_slice(&crc.to_le_bytes());
            v.extend_from_slice(&size.to_le_bytes()); // packed
            v.extend_from_slice(&size.to_le_bytes()); // size
            v.extend_from_slice(&name_len.to_le_bytes());
            v.extend_from_slice(&0u16.to_le_bytes()); // extra
        };
        out.extend_from_slice(&LOCAL.to_le_bytes());
        common(&mut out);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(data);

        dir.extend_from_slice(&CENTRAL.to_le_bytes());
        dir.extend_from_slice(&20u16.to_le_bytes()); // version made by
        common(&mut dir);
        dir.extend_from_slice(&[0u8; 10]); // comment len, disk, internal and external attrs
        dir.extend_from_slice(&local.to_le_bytes());
        dir.extend_from_slice(name.as_bytes());
    }
    let dir_at = u32::try_from(out.len()).expect("a zip is under 4 GiB");
    out.extend_from_slice(&dir);
    out.extend_from_slice(&END.to_le_bytes());
    out.extend_from_slice(&[0u8; 4]); // this disk, directory disk
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&(dir.len() as u32).to_le_bytes());
    out.extend_from_slice(&dir_at.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment
    out
}

/// The zip (IEEE 802.3) CRC-32 of `data`.
pub fn crc32(data: &[u8]) -> u32 {
    const TABLE: [u32; 256] = {
        let mut t = [0u32; 256];
        let mut i = 0;
        while i < 256 {
            let mut c = i as u32;
            let mut k = 0;
            while k < 8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
                k += 1;
            }
            t[i] = c;
            i += 1;
        }
        t
    };
    !data.iter().fold(!0u32, |c, &b| {
        TABLE[((c ^ u32::from(b)) & 0xFF) as usize] ^ (c >> 8)
    })
}

impl std::fmt::Debug for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry")
            .field("name", &self.name)
            .field("len", &self.bytes.len())
            .finish()
    }
}

#[cfg(test)]
#[path = "../tests/common/zipwrite.rs"]
mod zipwrite;

#[cfg(test)]
mod tests {
    use super::zipwrite::{Method, write_zip};
    use super::*;

    fn all(_: &str) -> bool {
        true
    }

    #[test]
    fn crc32_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn stored_and_deflated_entries_come_out_whole() {
        let big: Vec<u8> = (0..5000u32).map(|i| (i * 7 % 251) as u8).collect();
        let z = write_zip(&[
            ("a.txt", b"hello".as_slice(), Method::Stored),
            ("dir/", b"".as_slice(), Method::Stored),
            ("dir/b.bin", &big, Method::Deflated),
        ]);
        assert!(is_zip(&z));
        let e = read_entries(&z, all, 10_000).unwrap();
        assert_eq!(e.len(), 2);
        assert_eq!(
            (e[0].name.as_str(), e[0].bytes.as_slice()),
            ("a.txt", b"hello".as_slice())
        );
        assert_eq!((e[1].name.as_str(), &e[1].bytes), ("dir/b.bin", &big));
        let only_bin = read_entries(&z, |n| n.ends_with(".bin"), 10_000).unwrap();
        assert_eq!(only_bin.len(), 1);
    }

    #[test]
    fn written_entries_read_back_whole() {
        let big: Vec<u8> = (0..2_000_000u32).map(|i| (i * 13 % 251) as u8).collect();
        let z = write_stored(&[
            ("recording.txt", b"a recording\n".as_slice()),
            ("empty", b"".as_slice()),
            ("screenshot.png", &big),
        ]);
        assert!(is_zip(&z));
        let e = read_entries(&z, all, 4_000_000).unwrap();
        let got: Vec<(&str, &[u8])> = e
            .iter()
            .map(|e| (e.name.as_str(), e.bytes.as_slice()))
            .collect();
        assert_eq!(
            got,
            [
                ("recording.txt", b"a recording\n".as_slice()),
                ("empty", b"".as_slice()),
                ("screenshot.png", big.as_slice()),
            ]
        );
        // The CRCs in the directory are each entry's own (read_entries also checks them).
        let mut at = find_end(&z).and_then(|end| le32(&z, end + 16)).unwrap() as usize;
        for (_, data) in &got {
            assert_eq!(le32(&z, at), Some(CENTRAL));
            assert_eq!(le32(&z, at + 16), Some(crc32(data)));
            at += 46 + le16(&z, at + 28).unwrap() as usize;
        }
        assert_eq!(write_stored(&[]), write_zip(&[]));
        assert_eq!(read_entries(&write_stored(&[]), all, 10).unwrap().len(), 0);
    }

    #[test]
    fn an_empty_zip_has_no_entries() {
        let z = write_zip(&[]);
        assert!(is_zip(&z));
        assert_eq!(read_entries(&z, all, 10).unwrap().len(), 0);
    }

    #[test]
    fn malformed_archives_are_errors_not_panics() {
        assert!(!is_zip(b"ATN!"));
        assert!(
            read_entries(b"", all, 10)
                .unwrap_err()
                .contains("not a zip")
        );
        assert!(read_entries("not a zip at all, honestly".as_bytes(), all, 10).is_err());

        let z = write_zip(&[("a.adf", &[9u8; 300], Method::Deflated)]);
        assert!(
            read_entries(&z, all, 299)
                .unwrap_err()
                .contains("too large")
        );
        for cut in 0..z.len() {
            let _ = read_entries(&z[..cut], all, 1000);
        }
        // a flipped byte in the stored data trips the checksum
        let mut s = write_zip(&[("a.adf", &[9u8; 300], Method::Stored)]);
        s[40] ^= 1;
        assert!(
            read_entries(&s, all, 1000)
                .unwrap_err()
                .contains("checksum")
        );
        // encrypted flag in the central directory
        let mut e = write_zip(&[("a.adf", &[9u8; 3], Method::Stored)]);
        let cd = e.len() - END_LEN - (46 + 5);
        e[cd + 8] |= 1;
        assert!(
            read_entries(&e, all, 1000)
                .unwrap_err()
                .contains("encrypted")
        );
    }

    #[test]
    fn random_bytes_never_panic() {
        let mut x = 0x2545_F491_4F6C_DD1Du64;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        let base = write_zip(&[
            ("one.adf", &[1u8; 700], Method::Deflated),
            ("two.adf", &[2u8; 90], Method::Stored),
        ]);
        for _ in 0..3000 {
            let len = (next() % 400) as usize;
            let mut junk: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            if next() % 2 == 0 {
                junk.splice(0..0, LOCAL.to_le_bytes());
            }
            let _ = read_entries(&junk, all, 1 << 20);
            let mut m = base.clone();
            for _ in 0..1 + next() % 4 {
                let at = (next() as usize) % m.len();
                m[at] = next() as u8;
            }
            let _ = read_entries(&m, all, 1 << 20);
        }
    }
}
