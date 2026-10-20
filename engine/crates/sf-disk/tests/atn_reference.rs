//! `decompress` against the emulated original (`tools/atn.py`, run in the worktree's venv) on every
//! chunk of the owner's four disks, plus the three stragglers and malformed input (DESIGN §2.4).
//!
//! The Python side runs from `sf_conform::disks::python_decompress` (a pure crate's tests may not
//! spawn processes or touch files), through a temp dir it deletes before returning.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use sf_conform::disks::{PyDecoded, python_decompress};
use sf_disk::{AtnError, Chunk, DiskSet, DiskSlot, Offered, carve, decompress, identify};

const STRAGGLERS: [&str; 3] = ["d1_c16", "d2_c39", "d4_c16"];

fn real_set() -> &'static DiskSet {
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

fn id(c: &Chunk) -> String {
    format!("d{}_c{:02}", c.disk.number(), c.index)
}

fn chunks() -> Vec<Chunk> {
    DiskSlot::ALL
        .iter()
        .flat_map(|&s| carve(real_set().disk(s)))
        .collect()
}

/// The chunk with the disk sector after its carved end appended, so the trailer is whole.
fn extended(c: &Chunk) -> Vec<u8> {
    let adf = &real_set().disk(c.disk).bytes;
    let start = c.offset as usize;
    adf[start..(start + c.raw.len() + 512).min(adf.len())].to_vec()
}

/// The emulated decompressor's output for every chunk (and straggler `_ext`), once per binary.
fn reference() -> &'static BTreeMap<String, PyDecoded> {
    static REF: OnceLock<BTreeMap<String, PyDecoded>> = OnceLock::new();
    REF.get_or_init(|| {
        let set = real_set();
        let adfs = DiskSlot::ALL.map(|s| set.disk(s).bytes.as_slice());
        python_decompress(adfs).unwrap_or_else(|e| panic!("{e}"))
    })
}

#[test]
fn every_chunk_the_emulator_decodes_decodes_identically() {
    let r = reference();
    let chunks = chunks();
    assert_eq!(chunks.len(), 106);
    let mut same = 0;
    for c in &chunks {
        let name = id(c);
        let PyDecoded {
            ok,
            decomp_size: dsz,
            ..
        } = r[&name];
        assert_eq!(dsz, c.decomp_size, "{name}");
        if !ok {
            assert!(
                STRAGGLERS.contains(&name.as_str()),
                "{name} failed in Python"
            );
            continue;
        }
        let got = decompress(&c.raw).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert!(got == r[&name].data, "{name}: differs from tools/atn.py");
        same += 1;
    }
    assert_eq!(same, 103);
}

/// The three chunks the emulation harness fails on: their trailer runs past the carved sector.
/// Carved, Rust refuses them as truncated; with the next sector appended, both decoders succeed
/// and agree, and `d4_c16` is a level.
#[test]
fn the_stragglers_need_the_next_sector() {
    let r = reference();
    let chunks = chunks();
    for name in STRAGGLERS {
        let c = chunks.iter().find(|c| id(c) == name).unwrap();
        assert!(c.raw.len() < c.comp_size as usize + 46, "{name}");
        assert_eq!(decompress(&c.raw), Err(AtnError::Truncated), "{name}");
        let ext = format!("{name}_ext");
        assert!(r[&ext].ok && r[&ext].decomp_size == c.decomp_size, "{ext}");
        let got = decompress(&extended(c)).unwrap_or_else(|e| panic!("{ext}: {e:?}"));
        assert_eq!(got.len(), c.decomp_size as usize, "{ext}");
        assert!(got == r[&ext].data, "{ext}: differs from tools/atn.py");
    }
    let level = decompress(&extended(
        chunks.iter().find(|c| id(c) == "d4_c16").unwrap(),
    ))
    .unwrap();
    let tags = t7mp_tags(&level);
    let tag = |t: &str| {
        tags.iter()
            .find(|r| r.0 == t)
            .unwrap_or_else(|| panic!("{t}: {tags:?}"))
    };
    assert_eq!(tags[0].0, "T7MP", "{tags:?}");
    assert_eq!(tag("XBLK").1, 300u32.to_be_bytes());
    assert_eq!(tag("YBLK").1, 84u32.to_be_bytes());
    assert_eq!(tag("BODY").1.len(), 300 * 84 * 2);
}

/// `[tag][u32 len][payload]` records, as `parse` in `tools/t7mp.py` reads them.
fn t7mp_tags(data: &[u8]) -> Vec<(String, &[u8])> {
    let mut tags = Vec::new();
    let mut at = 0;
    while at + 8 <= data.len() {
        let len = u32::from_be_bytes(data[at + 4..at + 8].try_into().unwrap()) as usize;
        if at + 8 + len > data.len() {
            break;
        }
        let tag = String::from_utf8_lossy(&data[at..at + 4]).into_owned();
        tags.push((tag, &data[at + 8..at + 8 + len]));
        at += 8 + len;
    }
    tags
}

fn smallest_good_chunk() -> Chunk {
    chunks()
        .into_iter()
        .filter(|c| c.raw.len() >= c.comp_size as usize + 46)
        .min_by_key(|c| c.decomp_size)
        .unwrap()
}

#[test]
fn malformed_input_returns_its_error() {
    let c = smallest_good_chunk();
    assert_eq!(decompress(&[]), Err(AtnError::BadMagic));
    assert_eq!(decompress(b"ATN"), Err(AtnError::BadMagic));
    assert_eq!(decompress(b"ATN!\0\0"), Err(AtnError::Truncated));
    let mut wrong = c.raw.clone();
    wrong[3] = b'?';
    assert_eq!(decompress(&wrong), Err(AtnError::BadMagic));
    for cut in [12, c.comp_size as usize, c.comp_size as usize + 45] {
        assert_eq!(
            decompress(&c.raw[..cut]),
            Err(AtnError::Truncated),
            "cut at {cut}"
        );
    }
    let real = decompress(&c.raw).unwrap();
    for lie in [
        c.decomp_size + 1,
        c.decomp_size + 1000,
        c.decomp_size - 1,
        1,
        0,
    ] {
        let mut b = c.raw.clone();
        b[4..8].copy_from_slice(&lie.to_be_bytes());
        let want = AtnError::LengthMismatch {
            expected: lie,
            got: real.len() as u32,
        };
        assert_eq!(decompress(&b), Err(want), "decomp_size {lie}");
    }
    let mut huge = c.raw.clone();
    huge[4..8].copy_from_slice(&u32::MAX.to_be_bytes());
    assert_eq!(
        decompress(&huge),
        Err(AtnError::TooLarge {
            decomp_size: u32::MAX
        })
    );
}
