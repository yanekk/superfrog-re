//! A sync point's oracle snapshot, cached outside git so a case reaches it in seconds rather than
//! by a long boot. The key is a SHA-1 over everything that decides the state there: the file
//! format, the emulator build, the Kickstart, the disks, the drives and the way there. A key that
//! changes misses and rebuilds; a file that does not check out rebuilds too.
//!
//! A game passes its cache folder and its file magic, and builds the `what` text of its keys.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use amiga_disk::sha1::sha1;
use amiga_input::{FrameInput, Joy, KeySet};
use amiga_oracle::{Amiga, Snapshot};

/// SHA-1 over `sync format {format}`, the emulator build ([`amiga_oracle::BUILD`]), the Kickstart's
/// and each disk's digest, `drives N` unless one drive is connected, then `what`. Callers' keys
/// written before this moved to the toolkit hash this same text, so their caches are found again.
pub fn cache_key(
    format: u32,
    kickstart: &[u8],
    disks: &[&[u8]],
    drives: u8,
    what: &str,
) -> [u8; 20] {
    sha1(key_text(format, amiga_oracle::BUILD, kickstart, disks, drives, what).as_bytes())
}

/// The text [`cache_key`] hashes, with the build a parameter so a test can pin it.
pub fn key_text(
    format: u32,
    build: &str,
    kickstart: &[u8],
    disks: &[&[u8]],
    drives: u8,
    what: &str,
) -> String {
    let mut text = format!(
        "sync format {format}\nvamiga {build}\nkickstart {}\n",
        hex(&sha1(kickstart))
    );
    for d in disks {
        text += &format!("disk {}\n", hex(&sha1(d)));
    }
    // A one-drive key is what it was before drives could be connected.
    if drives != 1 {
        text += &format!("drives {drives}\n");
    }
    text + what
}

/// Lower-case hex, the form a key takes in a file name and in the file itself.
pub fn hex(key: &[u8]) -> String {
    key.iter().map(|b| format!("{b:02x}")).collect()
}

/// Where the snapshot for `key` lives under `root`.
pub fn cache_path(root: &Path, key: [u8; 20]) -> PathBuf {
    root.join(format!("{}.sync", hex(&key)))
}

/// The oracle at the sync point `key` names: `power_on`, then restored from the cache under `root`
/// when it holds a sound file for this key, else taken there by `get_there` and the snapshot
/// stored. `power_on` sets the frame cap, which a restore keeps, so it must leave room for the
/// frames to the sync point and those the caller runs after it. `A` is the toolkit's machine or a
/// game's wrapper of it.
pub fn at_snapshot<A: AsMut<Amiga>>(
    root: &Path,
    magic: &[u8],
    key: [u8; 20],
    power_on: impl FnOnce() -> Result<A, String>,
    get_there: impl FnOnce(&mut A) -> Result<(), String>,
) -> Result<A, String> {
    let path = cache_path(root, key);
    let mut amiga = power_on()?;
    match fs::read(&path).ok().and_then(|b| decode(&b, magic, key)) {
        Some(snap) => amiga.as_mut().restore(&snap),
        None => {
            get_there(&mut amiga)?;
            store(&path, &encode(magic, key, &amiga.as_mut().snapshot()));
        }
    }
    Ok(amiga)
}

/// `magic`, the key in hex, the frame, the held input, the snapshot bytes, then a SHA-1 of all
/// before it. The digest is there because the emulator is handed the bytes unchecked: a file cut
/// short or overwritten must rebuild, not reach the emulator.
pub fn encode(magic: &[u8], key: [u8; 20], s: &Snapshot) -> Vec<u8> {
    let mut b = magic.to_vec();
    b.extend(hex(&key).as_bytes());
    b.extend(s.frame.to_le_bytes());
    b.push(s.input.joy.bits());
    b.extend(s.input.keys.0.to_le_bytes());
    b.extend(&s.bytes);
    let digest = sha1(&b);
    b.extend(digest);
    b
}

/// The snapshot in a file [`encode`] wrote for this magic and key; `None` for anything else.
pub fn decode(b: &[u8], magic: &[u8], key: [u8; 20]) -> Option<Snapshot> {
    let (body, digest) = b.split_at_checked(b.len().checked_sub(20)?)?;
    if sha1(body) != digest {
        return None;
    }
    let rest = body
        .strip_prefix(magic)?
        .strip_prefix(hex(&key).as_bytes())?;
    if rest.len() < 21 {
        return None;
    }
    Some(Snapshot {
        frame: u32::from_le_bytes(rest[..4].try_into().ok()?),
        input: FrameInput {
            joy: Joy(rest[4]),
            keys: KeySet(u128::from_le_bytes(rest[5..21].try_into().ok()?)),
        },
        bytes: rest[21..].to_vec(),
    })
}

/// Write by rename, so a reader never sees half a file. A cache that cannot be written only costs
/// the next run time.
fn store(path: &Path, bytes: &[u8]) {
    if let Some(dir) = path.parent()
        && fs::create_dir_all(dir).is_ok()
    {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let tmp = path.with_extension(format!("tmp{}-{n}", std::process::id()));
        if fs::write(&tmp, bytes).is_ok() {
            let _ = fs::rename(&tmp, path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAGIC: &[u8] = b"TESTSYNC";

    fn snap() -> Snapshot {
        Snapshot {
            frame: 6301,
            input: FrameInput {
                joy: Joy(0x11),
                keys: KeySet(1 << 69),
            },
            bytes: (0..300u32).map(|i| i as u8).collect(),
        }
    }

    #[test]
    fn a_snapshot_round_trips_under_its_magic_and_key() {
        let key = [7; 20];
        assert_eq!(
            decode(&encode(MAGIC, key, &snap()), MAGIC, key),
            Some(snap())
        );
    }

    #[test]
    fn another_key_or_magic_misses() {
        let b = encode(MAGIC, [7; 20], &snap());
        assert_eq!(decode(&b, MAGIC, [8; 20]), None);
        assert_eq!(decode(&b, b"OTHERSYN", [7; 20]), None);
    }

    /// Every cut and every flipped byte misses, so none reaches the emulator.
    #[test]
    fn a_damaged_file_misses() {
        let key = [7; 20];
        let b = encode(MAGIC, key, &snap());
        for len in 0..b.len() {
            assert_eq!(decode(&b[..len], MAGIC, key), None, "cut to {len}");
        }
        for i in 0..b.len() {
            let mut c = b.clone();
            c[i] ^= 0x40;
            assert_eq!(decode(&c, MAGIC, key), None, "byte {i} flipped");
        }
    }

    #[test]
    fn the_key_text_is_the_one_callers_hashed_before_the_move() {
        let (ks, d1, d2) = (b"rom".as_slice(), b"one".as_slice(), b"two".as_slice());
        let one = key_text(1, "b", ks, &[d1, d2], 1, "x");
        assert_eq!(
            one,
            format!(
                "sync format 1\nvamiga b\nkickstart {}\ndisk {}\ndisk {}\nx",
                hex(&sha1(ks)),
                hex(&sha1(d1)),
                hex(&sha1(d2))
            )
        );
        let two = key_text(1, "b", ks, &[d1, d2], 2, "x");
        assert!(two.ends_with("drives 2\nx"), "{two}");
    }

    /// Each input that decides the state there changes the key.
    #[test]
    fn every_input_changes_the_key() {
        let k = |f, ks: &[u8], d: &[u8], dr, w| cache_key(f, ks, &[d], dr, w);
        let base = k(1, b"rom", b"disk", 1, "script a");
        for other in [
            k(2, b"rom", b"disk", 1, "script a"),
            k(1, b"rom2", b"disk", 1, "script a"),
            k(1, b"rom", b"disk2", 1, "script a"),
            k(1, b"rom", b"disk", 2, "script a"),
            k(1, b"rom", b"disk", 1, "script b"),
        ] {
            assert_ne!(base, other);
        }
        assert_ne!(
            sha1(key_text(1, "build a", b"r", &[], 1, "").as_bytes()),
            sha1(key_text(1, "build b", b"r", &[], 1, "").as_bytes())
        );
    }
}
