//! The OFS reader's first real-world check (DESIGN §2.4): the names the main program holds, found by
//! `tools/find_file_names.py` in its real bytes, against the inventory read through that reader.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use fd_tools::{InventoryFile, disk_dir, extract};

/// Names the program holds that are on neither disk, each for a reason `docs/03-boot-chain.md` gives.
/// Exact in both directions, so a new stray name fails here instead of being waved through.
const NOT_ON_THE_DISKS: [&str; 3] = [
    "DF0:ST",       // written, never read, by a routine with no caller found
    "FileName.exe", // the default in the buffer every load's real name is copied over
    "HD_ID",        // no reference found; it sits among the file-name buffers
];

struct Out(PathBuf);

impl Out {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!("fd-boot-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        Out(root.join("build"))
    }
}

impl Drop for Out {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
    }
}

fn extracted(tag: &str) -> (Out, Vec<InventoryFile>) {
    let out = Out::new(tag);
    let dir = disk_dir().unwrap_or_else(|e| panic!("{e}"));
    let files = extract(&dir, &out.0).unwrap();
    (out, files)
}

/// What the tool itself prints, so this checks the scanner T06 will use and not a copy of it.
fn scan(file: &Path) -> Vec<String> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.join("../../../../..");
    let python = root.join("venv/bin/python");
    let tool = root.join("games/forest-dumb/tools/find_file_names.py");
    let out = Command::new(&python)
        .arg(&tool)
        .arg(file)
        .output()
        .unwrap_or_else(|e| panic!("{}: {e}", python.display()));
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

/// AmigaDOS's own comparison: the device is not part of the name, and case does not count.
fn as_on_disk(name: &str) -> String {
    name.rsplit(':').next().unwrap().to_lowercase()
}

fn main_program(out: &Out) -> Vec<String> {
    scan(&out.0.join("files/disk1/FOREST_DUMB_FOREVER"))
}

#[test]
fn every_name_the_main_program_holds_is_in_the_inventory() {
    let (out, files) = extracted("agree");
    let inventory: BTreeSet<String> = files.iter().map(|f| f.name.to_lowercase()).collect();
    let stray: BTreeSet<String> = main_program(&out)
        .into_iter()
        .filter(|n| !inventory.contains(&as_on_disk(n)))
        .collect();
    assert_eq!(stray, NOT_ON_THE_DISKS.map(String::from).into());
}

#[test]
fn the_scan_finds_the_names_the_boot_depends_on() {
    let (out, _) = extracted("known");
    let names = main_program(&out);
    for known in ["_MAPA1-1.Mapa", "df0:!EAGLE_SOFT!"] {
        assert!(names.iter().any(|n| n == known), "{known} in {names:?}");
    }
}

/// The program never names itself: only the crack's and the installer's scripts do.
#[test]
fn the_main_program_is_named_by_the_scripts_not_by_itself() {
    let (out, _) = extracted("self");
    let lower = "forest_dumb_forever";
    assert!(
        !main_program(&out).iter().any(|n| as_on_disk(n) == lower),
        "the program names itself"
    );
    let disk1 = out.0.join("files/disk1");
    assert_eq!(scan(&disk1.join("s/startup-sequence")), [lower]);
    assert_eq!(scan(&disk1.join("FOREVER!")), ["FOREST_DUMB_FOREVER"]);
}

/// The other direction, which the scan's reach makes checkable: nothing on the data disk is unused.
#[test]
fn the_main_program_names_every_file_on_the_data_disk() {
    let (out, files) = extracted("data");
    let named: BTreeSet<String> = main_program(&out).iter().map(|n| as_on_disk(n)).collect();
    let unnamed: Vec<&str> = files
        .iter()
        .filter(|f| f.disk == 2 && !named.contains(&f.name.to_lowercase()))
        .map(|f| f.name.as_str())
        .collect();
    assert!(unnamed.is_empty(), "{unnamed:?}");
}
