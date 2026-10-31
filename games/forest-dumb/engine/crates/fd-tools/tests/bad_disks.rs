//! What the import refuses, on zips made here: nothing derived from the real disks is needed, and
//! a refusal names the zip, says what is wrong and leaves nothing behind (DESIGN §2.10).

use std::path::{Path, PathBuf};

use amiga_disk::ADF_SIZE;
use amiga_disk::sha1::sha1_hex;
use amiga_disk::zip::write_stored;
use fd_tools::{entry_path, extract};

const ONE: &str = "Forest (Disk 1 of 2).zip";
const TWO: &str = "Forest (Disk 2 of 2).zip";

/// A folder holding the two zips, with `out` a `build` folder beside them that must stay absent.
fn setup(tag: &str, one: &[u8], two: &[u8]) -> (PathBuf, PathBuf) {
    let dir = std::env::temp_dir().join(format!("fd-bad-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(ONE), one).unwrap();
    std::fs::write(dir.join(TWO), two).unwrap();
    let out = dir.join("build");
    (dir, out)
}

fn adf_zip(size: usize) -> Vec<u8> {
    let mut adf = vec![0u8; size];
    adf[..4.min(size)].copy_from_slice(&b"DOS\0"[..4.min(size)]);
    write_stored(&[("d.adf", &adf)])
}

fn refused(tag: &str, one: &[u8], two: &[u8], blamed: &str, what: &str) {
    let (dir, out) = setup(tag, one, two);
    let e = extract(&dir, &out).unwrap_err();
    assert!(e.contains(blamed), "names the zip: {e}");
    let bad = if blamed == ONE { one } else { two };
    assert!(e.contains(&sha1_hex(bad)), "gives its checksum: {e}");
    assert!(e.contains(what), "says what is wrong: {e}");
    assert!(!out.exists(), "wrote something for {tag}");
}

#[test]
fn a_file_that_is_not_a_zip_is_reported_by_name() {
    refused(
        "garbage",
        b"not a zip at all",
        &adf_zip(ADF_SIZE),
        ONE,
        "not a zip archive",
    );
}

#[test]
fn a_zip_with_a_damaged_entry_is_reported_by_name() {
    let mut bad = write_stored(&[("d.adf", &vec![7u8; ADF_SIZE])]);
    bad[1000] ^= 0xFF;
    refused("crc", &adf_zip(ADF_SIZE), &bad, TWO, "fails its checksum");
}

#[test]
fn a_cut_short_zip_is_reported_by_name() {
    let good = adf_zip(ADF_SIZE);
    refused(
        "cut",
        &good[..good.len() / 2],
        &good,
        ONE,
        "not a zip archive",
    );
}

#[test]
fn a_truncated_adf_is_reported_by_name() {
    refused(
        "short",
        &adf_zip(ADF_SIZE),
        &adf_zip(900_000),
        TWO,
        "900000 bytes",
    );
}

#[test]
fn an_oversized_adf_is_reported_by_name() {
    let big = write_stored(&[("d.adf", &vec![0u8; ADF_SIZE + 1])]);
    refused("long", &big, &adf_zip(ADF_SIZE), ONE, "901121 bytes");
}

#[test]
fn a_zip_without_exactly_one_adf_is_reported_by_name() {
    let none = write_stored(&[("readme.txt", b"hello")]);
    refused("none", &none, &adf_zip(ADF_SIZE), ONE, "holds 0 ADFs");
    let two = write_stored(&[("a.adf", &[0u8; 4]), ("b.adf", &[0u8; 4])]);
    refused("twoadf", &adf_zip(ADF_SIZE), &two, TWO, "holds 2 ADFs");
}

#[test]
fn a_whole_size_adf_that_is_no_volume_is_reported_by_name() {
    // Disk 1 reads as garbage after the good-looking dos type, so it never gets past the root block.
    refused(
        "empty",
        &adf_zip(ADF_SIZE),
        &adf_zip(ADF_SIZE),
        ONE,
        "its ADF:",
    );
}

#[test]
fn a_missing_zip_is_named() {
    let (dir, out) = setup("missing", &adf_zip(ADF_SIZE), &adf_zip(ADF_SIZE));
    std::fs::remove_file(dir.join(TWO)).unwrap();
    let e = extract(&dir, &out).unwrap_err();
    assert!(e.contains("no zip of disk 2 of 2"), "{e}");
    assert!(!out.exists());
}

fn target_refused(dir: &Path, out: &Path) {
    let e = extract(dir, out).unwrap_err();
    assert!(e.contains("refusing to write"), "{e}");
}

#[test]
fn the_write_target_must_be_inside_a_build_folder() {
    let (dir, _) = setup("target", b"", b"");
    target_refused(&dir, &dir);
    target_refused(&dir, &dir.join("build/../elsewhere"));
    target_refused(&dir, &dir.join("rebuild"));
    assert_eq!(
        std::fs::read_dir(&dir).unwrap().count(),
        2,
        "only the two zips remain"
    );
}

#[test]
fn a_name_on_disk_cannot_leave_the_output_folder() {
    assert_eq!(
        entry_path("s/startup-sequence").unwrap(),
        Path::new("s/startup-sequence")
    );
    assert_eq!(
        entry_path("!EAGLE_SOFT!").unwrap(),
        Path::new("!EAGLE_SOFT!")
    );
    for evil in [
        "..",
        "../x",
        "s/../../x",
        "/etc/x",
        "a//b",
        ".",
        "a\\b",
        "a\0b",
        "",
    ] {
        assert!(entry_path(evil).is_err(), "{evil:?} accepted");
    }
}
