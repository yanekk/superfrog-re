//! `identify` and `carve` on the owner's four real disks (found by `sf_conform::disks`; a missing
//! disk fails the test with where it looked). Variants of the real files are built in memory.

mod common;

use common::zipwrite::{Method, write_zip};
use sf_disk::{
    ADF_SIZE, DiskSet, DiskSlot, ImportError, Offered, carve, identify, known_sha1, sha1_hex, zip,
};

fn real_files() -> Vec<Offered> {
    sf_conform::disks::read_disk_files()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|(name, bytes)| Offered { name, bytes })
        .collect()
}

fn real_set() -> DiskSet {
    identify(&real_files()).unwrap_or_else(|e| panic!("{e:?}"))
}

fn raw(set: &DiskSet, slot: DiskSlot) -> Offered {
    Offered {
        name: format!("disk{}.adf", slot.number()),
        bytes: set.disk(slot).bytes.clone(),
    }
}

fn all_raw(set: &DiskSet) -> Vec<Offered> {
    DiskSlot::ALL.map(|s| raw(set, s)).to_vec()
}

#[test]
fn the_four_real_files_identify() {
    let files = real_files();
    assert_eq!(files.len(), 4, "{files:?}");
    let set = identify(&files).unwrap_or_else(|e| panic!("{e:?}"));
    for slot in DiskSlot::ALL {
        let d = set.disk(slot);
        assert_eq!(d.slot, slot);
        assert_eq!(d.bytes.len(), ADF_SIZE);
        assert_eq!(sha1_hex(&d.bytes), known_sha1(slot));
        assert!(
            d.source.contains(&format!("Disk {} of 4", slot.number())),
            "{}",
            d.source
        );
    }
}

#[test]
fn the_same_four_as_raw_adfs_identify_in_any_order() {
    let set = real_set();
    let mut files = all_raw(&set);
    files.reverse();
    assert_eq!(
        identify(&files).unwrap(),
        DiskSet {
            disks: set.disks.clone().map(|mut d| {
                d.source = format!("disk{}.adf", d.slot.number());
                d
            })
        }
    );
}

#[test]
fn zips_with_two_disks_junk_and_folders_are_handled() {
    let set = real_set();
    let b = |s: DiskSlot| set.disk(s).bytes.as_slice();
    let two = write_zip(&[
        ("Superfrog/Disk 1.adf", b(DiskSlot::D1), Method::Deflated),
        ("Superfrog/Disk 2.ADF", b(DiskSlot::D2), Method::Stored),
    ]);
    let junk = write_zip(&[
        (
            "readme.txt",
            "crack intro by CSL".as_bytes(),
            Method::Deflated,
        ),
        ("nested/deeper/sf3.adf", b(DiskSlot::D3), Method::Deflated),
        (
            "__MACOSX/nested/deeper/._sf3.adf",
            b"resource fork".as_slice(),
            Method::Stored,
        ),
        ("nested/", b"".as_slice(), Method::Stored),
    ]);
    let files = [
        Offered {
            name: "one-and-two.zip".into(),
            bytes: two,
        },
        Offered {
            name: "three.zip".into(),
            bytes: junk,
        },
        raw(&set, DiskSlot::D4),
    ];
    let got = identify(&files).unwrap_or_else(|e| panic!("{e:?}"));
    for slot in DiskSlot::ALL {
        assert_eq!(got.disk(slot).bytes, set.disk(slot).bytes, "{slot}");
    }
    assert_eq!(got.disk(DiskSlot::D2).source, "one-and-two.zip");
    assert_eq!(got.disk(DiskSlot::D3).source, "three.zip");
}

#[test]
fn one_disk_missing() {
    let set = real_set();
    let mut files = all_raw(&set);
    files.remove(2);
    assert_eq!(
        identify(&files).unwrap_err(),
        vec![ImportError::Missing(DiskSlot::D3)]
    );
}

#[test]
fn a_truncated_adf_is_damaged() {
    let set = real_set();
    let mut files = all_raw(&set);
    files[1].bytes.truncate(ADF_SIZE - 512);
    let e = identify(&files).unwrap_err();
    assert!(
        matches!(&e[0], ImportError::Damaged { name, .. } if name == "disk2.adf"),
        "{e:?}"
    );
    assert_eq!(e[1], ImportError::Missing(DiskSlot::D2));
    assert_eq!(e.len(), 2);
}

#[test]
fn a_flipped_byte_is_unknown_with_its_sha1() {
    let set = real_set();
    let mut files = all_raw(&set);
    files[3].bytes[123_456] ^= 0x10;
    let want = sha1_hex(&files[3].bytes);
    assert_eq!(
        identify(&files).unwrap_err(),
        vec![
            ImportError::Unknown {
                name: "disk4.adf".into(),
                sha1: want
            },
            ImportError::Missing(DiskSlot::D4),
        ]
    );
}

#[test]
fn the_same_disk_twice_is_a_duplicate() {
    let set = real_set();
    let mut files = real_files();
    files.extend(all_raw(&set).into_iter().filter(|f| f.name == "disk1.adf"));
    files.push(raw(&set, DiskSlot::D1)); // a third copy is still one message
    assert_eq!(
        identify(&files).unwrap_err(),
        vec![ImportError::Duplicate(DiskSlot::D1)]
    );
}

#[test]
fn a_zip_that_is_not_a_zip_is_damaged_never_a_panic() {
    let set = real_set();
    let mut files = all_raw(&set);
    files[0] = Offered {
        name: "Disk 1.zip".into(),
        bytes: set.disk(DiskSlot::D1).bytes[..4096].to_vec(),
    };
    let e = identify(&files).unwrap_err();
    assert!(
        matches!(&e[0], ImportError::Damaged { name, .. } if name == "Disk 1.zip"),
        "{e:?}"
    );
    assert!(e[0].to_string().contains("Disk 1.zip"));

    // the real zip, cut and corrupted at random places, and random bytes named .zip
    let real = real_files()
        .into_iter()
        .find(|f| f.name.ends_with(".zip"))
        .expect("a real zip");
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    for _ in 0..40 {
        let mut bytes = real.bytes.clone();
        match next() % 3 {
            0 => bytes.truncate((next() as usize) % bytes.len()),
            1 => {
                for _ in 0..1 + next() % 8 {
                    let at = (next() as usize) % bytes.len();
                    bytes[at] = next() as u8;
                }
            }
            _ => bytes = (0..next() % 5000).map(|_| next() as u8).collect(),
        }
        let offered = [Offered {
            name: "fuzzed.zip".into(),
            bytes,
        }];
        let e = identify(&offered).unwrap_err();
        assert!(e.iter().any(|e| matches!(e, ImportError::Missing(_))));
        let _ = zip::read_entries(&offered[0].bytes, |_| true, 4 * ADF_SIZE);
    }
}

#[test]
fn carve_matches_the_chunk_inventory() {
    let set = real_set();
    let inventory = include_str!("../../../../docs/05-chunk-inventory.txt");
    let want: Vec<(String, u16, u32, u32)> = inventory
        .lines()
        .filter(|l| l.starts_with('d') && !l.starts_with("disk"))
        .map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            (
                f[0].to_string(),
                f[1].parse().unwrap(),
                f[2].parse().unwrap(),
                f[3].parse().unwrap(),
            )
        })
        .collect();
    let got: Vec<(String, u16, u32, u32)> = DiskSlot::ALL
        .iter()
        .flat_map(|&s| carve(set.disk(s)))
        .map(|c| {
            (
                format!("d{}", c.disk.number()),
                c.index,
                c.decomp_size,
                c.comp_size,
            )
        })
        .collect();
    assert_eq!(got.len(), 106);
    assert_eq!(got, want);
}

#[test]
fn carve_matches_the_python_carver() {
    let set = real_set();
    for slot in DiskSlot::ALL {
        let adf = set.disk(slot);
        let want = sf_conform::disks::python_carve(&adf.bytes).unwrap_or_else(|e| panic!("{e}"));
        let got: Vec<_> = carve(adf)
            .into_iter()
            .map(|c| {
                (
                    c.offset,
                    c.decomp_size,
                    c.comp_size,
                    c.raw.len(),
                    sha1_hex(&c.raw),
                )
            })
            .collect();
        assert!(!got.is_empty());
        assert_eq!(got, want, "{slot}");
    }
}
