//! The import on the owner's real disks: fails loudly, naming where it looked, when they are absent.

use std::path::{Path, PathBuf};

use amiga_disk::ofs::OfsImage;
use amiga_disk::sha1::sha1_hex;
use fd_tools::{InventoryFile, disk_dir, extract, inventory_text, zip_paths};

/// A scratch `build` folder, removed when dropped: what is extracted is the owner's.
struct Out(PathBuf);

impl Out {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!("fd-real-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        Out(root.join("build"))
    }
}

impl Drop for Out {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
    }
}

fn dir() -> PathBuf {
    disk_dir().unwrap_or_else(|e| panic!("{e}"))
}

fn on_disk(files: &[InventoryFile], disk: u8) -> Vec<&InventoryFile> {
    files.iter().filter(|f| f.disk == disk).collect()
}

fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let path = e.unwrap().path();
        if path.is_dir() {
            walk(&path, found);
        } else {
            found.push(path);
        }
    }
}

#[test]
fn the_known_facts_hold() {
    let dir = dir();
    let out = Out::new("facts");
    let files = extract(&dir, &out.0).unwrap();

    for n in 1..=2 {
        let adf = std::fs::read(out.0.join(format!("disk{n}.adf"))).unwrap();
        assert_eq!(OfsImage::parse(&adf).unwrap().volume_name(), "FOREVER");
    }
    let (one, two) = (on_disk(&files, 1), on_disk(&files, 2));
    assert_eq!(one.len(), 12, "disk 1's file count");

    let main = one
        .iter()
        .find(|f| f.name == "FOREST_DUMB_FOREVER")
        .expect("the main program");
    assert_eq!(main.size, 521_640);
    assert!(one.iter().any(|f| f.name == "s/startup-sequence"));

    assert!(
        two.iter().any(|f| f.name == "!EAGLE_SOFT!"),
        "the marker is on disk 2"
    );
    assert!(!one.iter().any(|f| f.name == "!EAGLE_SOFT!"));
    // The 48 data files are all of disk 2 but its marker.
    assert_eq!(two.iter().filter(|f| f.name != "!EAGLE_SOFT!").count(), 48);
    // Thirty of them are the six worlds' level files: Gen and Mapa with their .Mapa, MapaBobow.
    let level = |f: &&&InventoryFile| f.name.starts_with("_Mapa") || f.name.starts_with("_Gen");
    assert_eq!(two.iter().filter(level).count(), 30);
}

#[test]
fn every_entry_is_written_with_its_recorded_bytes_and_nothing_else() {
    let dir = dir();
    let out = Out::new("bytes");
    let files = extract(&dir, &out.0).unwrap();
    for f in &files {
        let path = out.0.join(format!("files/disk{}/{}", f.disk, f.name));
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert_eq!(bytes.len() as u32, f.size, "{}", f.name);
        assert_eq!(sha1_hex(&bytes), f.sha1, "{}", f.name);
    }
    let mut found = Vec::new();
    walk(&out.0, &mut found);
    assert_eq!(
        found.len(),
        files.len() + 2,
        "every file and the two ADFs: {found:?}"
    );
}

#[test]
fn extraction_is_repeatable_and_leaves_the_zips_alone() {
    let dir = dir();
    let sums = || -> Vec<String> {
        let zips = zip_paths(&dir).unwrap();
        zips.iter()
            .map(|z| sha1_hex(&std::fs::read(z).unwrap()))
            .collect()
    };
    let before = sums();
    let (a, b) = (Out::new("again-a"), Out::new("again-b"));
    let first = extract(&dir, &a.0).unwrap();
    assert_eq!(first, extract(&dir, &b.0).unwrap());
    // Over the top of an earlier run, which is the whole recovery from a crash (DESIGN §2.10).
    assert_eq!(first, extract(&dir, &a.0).unwrap());
    assert_eq!(before, sums());
}

#[test]
fn a_bad_second_disk_leaves_nothing_behind() {
    let dir = dir();
    let [one, _] = zip_paths(&dir).unwrap();
    let scratch = Out::new("partial");
    let holding = scratch.0.parent().unwrap();
    std::fs::create_dir_all(holding).unwrap();
    std::fs::copy(&one, holding.join(one.file_name().unwrap())).unwrap();
    std::fs::write(holding.join("x (Disk 2 of 2).zip"), b"not a zip").unwrap();
    assert!(extract(holding, &scratch.0).is_err());
    assert!(!scratch.0.exists());
}

#[test]
fn the_committed_inventory_is_what_the_disks_hold() {
    let dir = dir();
    let out = Out::new("inventory");
    let files = extract(&dir, &out.0).unwrap();
    let committed = include_str!("../../../../docs/04-file-inventory.txt");
    assert_eq!(inventory_text(&dir, &files).unwrap(), committed);
}
