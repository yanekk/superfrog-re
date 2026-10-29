//! The bug zips the app wrote before the zip code moved to the toolkit (toolkit-extract T03) still
//! read through `zip::read_entries`, and writing their entries back with `zip::write_stored` gives
//! the same bytes: the moved reader and writer are the ones that wrote them. The zips are the
//! owner's, in the main checkout's git-ignored `bugs/` (their screenshot is disk-derived); none
//! there fails the test with where it looked, as a missing disk does.

use sf_disk::zip;

#[test]
fn every_bug_zip_reads_back_and_rewrites_byte_equal() {
    let dir = sf_conform::disks::main_checkout()
        .unwrap_or_else(|e| panic!("{e}"))
        .join("bugs");
    let mut zips: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("no bug zips: cannot read {}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "zip"))
        .collect();
    zips.sort();
    assert!(!zips.is_empty(), "no bug zips in {}", dir.display());
    for path in &zips {
        let bytes = std::fs::read(path).unwrap();
        let entries = zip::read_entries(&bytes, |_| true, 1 << 26)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            ["recording.txt", "screenshot.png"],
            "{}",
            path.display()
        );
        let files: Vec<(&str, &[u8])> = entries
            .iter()
            .map(|e| (e.name.as_str(), e.bytes.as_slice()))
            .collect();
        assert!(
            zip::write_stored(&files) == bytes,
            "{}: rewriting its entries changed its bytes",
            path.display()
        );
    }
}
