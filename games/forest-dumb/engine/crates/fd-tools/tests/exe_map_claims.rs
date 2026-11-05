//! `docs/07-executable-map.md`'s claims against the real program, and its names against the disks.
//! What needs the running game (the variable's values) is the first conformance case's job.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use amiga_disk::hunk::{self, Hunk, HunkKind};
use fd_tools::{InventoryFile, disk_dir, extract};

const VARIABLE: u32 = 0x195c9;

struct Out(PathBuf);

impl Drop for Out {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(self.0.parent().unwrap());
    }
}

fn extracted(tag: &str) -> (Out, Vec<InventoryFile>) {
    let root = std::env::temp_dir().join(format!("fd-map-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let out = Out(root.join("build"));
    let dir = disk_dir().unwrap_or_else(|e| panic!("{e}"));
    let files = extract(&dir, &out.0).unwrap();
    (out, files)
}

fn program(out: &Out) -> (PathBuf, Vec<Hunk>) {
    let path = out.0.join("files/disk1/FOREST_DUMB_FOREVER");
    let hunks = hunk::parse(&std::fs::read(&path).unwrap()).unwrap();
    (path, hunks)
}

fn game_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn relocated_targets(hunks: &[Hunk], target: usize) -> BTreeSet<u32> {
    let mut found = BTreeSet::new();
    for h in hunks {
        for r in h.relocs.iter().filter(|r| r.target as usize == target) {
            for &o in &r.offsets {
                let b = &h.data[o as usize..o as usize + 4];
                found.insert(u32::from_be_bytes(b.try_into().unwrap()));
            }
        }
    }
    found
}

#[test]
fn the_hunks_are_as_the_map_says() {
    let (out, _) = extracted("hunks");
    let (_, hunks) = program(&out);
    let shape: Vec<(HunkKind, u32)> = hunks.iter().map(|h| (h.kind, h.size)).collect();
    assert_eq!(
        shape,
        [
            (HunkKind::Code, 0x5be68),
            (HunkKind::Data, 0x1280),
            (HunkKind::Data, 0x1830),
            (HunkKind::Bss, 0x3aa28),
            (HunkKind::Data, 0x1c94c),
        ]
    );
}

#[test]
fn the_entry_saves_the_registers_then_prompts_and_reads_the_marker() {
    let (out, _) = extracted("entry");
    let (_, hunks) = program(&out);
    assert_eq!(
        hunks[0].data[..16],
        [
            0x48, 0xe7, 0xff, 0xfe, // movem.l d0-d7/a0-a6,-(a7)
            0x4e, 0xb9, 0x00, 0x00, 0x7b, 0x70, // jsr prompt_disk_2
            0x4e, 0xb9, 0x00, 0x00, 0x8f, 0xd8, // jsr read_marker
        ]
    );
}

#[test]
fn the_variable_is_inside_hunk_4_and_the_player_counts_it() {
    let (out, _) = extracted("variable");
    let (_, hunks) = program(&out);
    let h4 = &hunks[4];
    assert!(VARIABLE < h4.size);
    // addq.b #1,<abs>, with the address relocated against hunk 4 itself.
    assert_eq!(h4.data[0x1852e..0x18530], [0x52, 0x39]);
    assert_eq!(h4.data[0x18530..0x18534], VARIABLE.to_be_bytes());
    let reloc = h4.relocs.iter().find(|r| r.target == 4).unwrap();
    assert!(reloc.offsets.contains(&0x18530));
    // mt_init: move.b #6,mt_speed, the byte before.
    assert_eq!(h4.data[0x184da..0x184de], [0x13, 0xfc, 0x00, 0x06]);
    assert_eq!(h4.data[0x184de..0x184e2], (VARIABLE - 1).to_be_bytes());
}

#[test]
fn the_custom_chip_references_number_what_the_map_says() {
    let (out, _) = extracted("chips");
    let (_, hunks) = program(&out);
    let count = |h: &Hunk| {
        h.data
            .chunks_exact(2)
            .zip(h.data[2..].chunks_exact(2))
            .filter(|(a, b)| a[0] == 0 && a[1] == 0xdf && (b[0] == 0xf0 || b[0] == 0xf1))
            .count()
    };
    assert_eq!((count(&hunks[0]), count(&hunks[4])), (379, 24));
}

/// DF0:ST's routine and HD_ID: what T03 left for this task.
#[test]
fn nothing_relocated_reaches_the_save_routine_or_the_hd_id_name() {
    let (out, _) = extracted("save");
    let (_, hunks) = program(&out);
    let into_h0 = relocated_targets(&hunks, 0);
    let inside: Vec<_> = into_h0
        .iter()
        .filter(|&&t| (0x92d8..0x93b4).contains(&t) || t == 0x91f8)
        .collect();
    assert!(inside.is_empty(), "{inside:x?}");
    // The check can see a reference: the one the save routine's own name has.
    assert!(into_h0.contains(&0x918c));
}

#[test]
fn the_file_table_is_the_data_disk_exactly() {
    let (_out, files) = extracted("table");
    let doc = std::fs::read_to_string(game_dir().join("docs/07-executable-map.md")).unwrap();
    let table: BTreeSet<String> = doc
        .lines()
        .skip_while(|l| !l.starts_with("| File | Disk | When |"))
        .skip(2)
        .take_while(|l| l.starts_with('|'))
        .inspect(|l| assert_eq!(l.split('|').nth(2), Some(" 2 "), "{l}"))
        .filter_map(|l| l.split('`').nth(1))
        .map(String::from)
        .collect();
    let disk2: BTreeSet<String> = files
        .iter()
        .filter(|f| f.disk == 2)
        .map(|f| f.name.clone())
        .collect();
    assert_eq!(table, disk2);
}

/// Also proves `symbols.toml` loads: the lister refuses a bad name, overlap or out-of-range offset.
#[test]
fn the_listing_regenerates_and_decodes_hunk_4() {
    let (out, _) = extracted("listing");
    let (exe, _) = program(&out);
    let game = game_dir();
    let python = game.join("../../venv/bin/python");
    let lst = out.0.join("re");
    let run = Command::new(&python)
        .arg(game.join("tools/re_listing.py"))
        .arg("--exe")
        .arg(&exe)
        .arg("--out")
        .arg(&lst)
        .output()
        .unwrap_or_else(|e| panic!("{}: {e}", python.display()));
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let text = std::fs::read_to_string(lst.join("forest_dumb_forever.lst")).unwrap();
    for wanted in ["\nmain_loop:", "\nentry:", "addq.b   #$1, mt_counter"] {
        assert!(text.contains(wanted), "{wanted}");
    }
    // T40: the sweep resyncs at every instruction call_tree.py's flow reaches, so these call
    // targets, once inside instructions decoded out of step after data, start lines of their own.
    for start in ["h0+$2c7e", "h0+$3698", "h0+$5820", "h0+$62b6", "h0+$94ba"] {
        assert!(text.contains(&format!("\n{start}:\n")), "{start} has no label");
    }
    // A 68000 instruction never starts at an odd address. An odd-sized data symbol leaves the sweep
    // on one until the next symbol, so everything up to there decodes as noise and still lists.
    let odd: Vec<&str> = text
        .lines()
        .filter(|l| l.get(8..10) == Some("  "))
        .filter(|l| {
            let at = u32::from_str_radix(&l[..8], 16);
            at.is_ok_and(|a| a < 0x5be68 && a & 1 == 1)
        })
        .collect();
    assert!(odd.is_empty(), "{} lines of hunk 0 start odd, first {:?}", odd.len(), odd.first());
}
