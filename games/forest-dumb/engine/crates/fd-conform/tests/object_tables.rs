//! Every object value of the six levels has a group the map page draws it by (plan
//! forest-dumb-map-page T01, DESIGN §2.1). The groups are the `[[value]]` rows of every
//! `re/objects-*.toml`: W1-1's own table, `objects-more.toml` for the values W1-1 lacks, and any
//! per-world table a later plan adds. The code dispatches on the value alone (docs/10), so a value
//! two tables put in different groups is a contradiction, never one table winning. A value in no
//! table is named with its cell rather than guessed at (DESIGN §1, stance).
//!
//! The tables are read by a small parser here: the crate admits no `toml` dependency. The real
//! maps need the disks: absent, that test fails naming where it looked.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use fd_assets::{AssetError, read_level};

/// Each value's group and the table that gave it.
type Groups = BTreeMap<u8, (String, String)>;

/// The `value` and `group` of each `[[value]]` row of one table's text.
fn rows(file: &str, text: &str) -> Result<Vec<(u8, String)>, String> {
    let mut out = Vec::new();
    for (i, block) in text.split("[[value]]").enumerate().skip(1) {
        let (mut value, mut group) = (None, None);
        for line in block.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some((key, val)) = line.split_once('=') else {
                continue;
            };
            let val = val.trim();
            match key.trim() {
                "value" => {
                    let v = match val.strip_prefix("0x") {
                        Some(hex) => u8::from_str_radix(hex, 16),
                        None => val.parse(),
                    };
                    value = Some(v.map_err(|e| format!("{file}: row {i}: value {val}: {e}"))?);
                }
                "group" => group = Some(val.trim_matches('"').to_string()),
                _ => {}
            }
        }
        match (value, group) {
            (Some(v), Some(g)) if !g.is_empty() => out.push((v, g)),
            _ => return Err(format!("{file}: row {i} lacks a value or a group")),
        }
    }
    Ok(out)
}

/// One map of value to group from every table, in the order given. A value listed twice with
/// different groups fails, naming both files; the same group twice is agreement.
fn merge(tables: &[(String, String)]) -> Result<Groups, String> {
    let mut groups = Groups::new();
    for (file, text) in tables {
        for (v, g) in rows(file, text)? {
            if let Some((had, from)) = groups.get(&v) {
                if *had != g {
                    return Err(format!(
                        "value ${v:02x}: group `{had}` in {from}, `{g}` in {file}"
                    ));
                }
                continue;
            }
            groups.insert(v, (g, file.clone()));
        }
    }
    Ok(groups)
}

/// Every cell of `objects` (column by column, 96 a column) over its first `columns` columns holding
/// a non-zero value no table groups, as `<level>:<col>,<row> holds $vv`.
fn ungrouped(level: &str, objects: &[u8], columns: usize, groups: &Groups) -> Vec<String> {
    let mut out = Vec::new();
    for c in 0..columns {
        for r in 0..96 {
            let v = objects[c * 96 + r];
            if v != 0 && !groups.contains_key(&v) {
                out.push(format!("{level}:{c},{r} holds ${v:02x}"));
            }
        }
    }
    out
}

fn re_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../re")
}

/// Every `re/objects-*.toml`, by file name, so the order (and an error naming two) is stable.
fn tables_on_disk() -> Vec<(String, String)> {
    tables_in(&re_dir())
}

/// Every `objects-*.toml` in `dir`, by file name, each named as `re/<file>`.
fn tables_in(dir: &Path) -> Vec<(String, String)> {
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("objects-") && n.ends_with(".toml"))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|n| {
            let text = std::fs::read_to_string(dir.join(&n)).unwrap_or_else(|e| panic!("{n}: {e}"));
            (format!("re/{n}"), text)
        })
        .collect()
}

fn files(name: &str) -> Result<Vec<u8>, AssetError> {
    let dir = fd_tools::disk_dir().map_err(|e| AssetError::missing(name, e))?;
    fd_tools::game_file(&dir, &format!("disk2/{name}")).map_err(|e| AssetError::missing(name, e))
}

#[test]
fn every_value_in_the_six_object_maps_has_a_group() {
    let tables = tables_on_disk();
    let read: Vec<&str> = tables.iter().map(|(n, _)| n.as_str()).collect();
    for want in ["re/objects-W1-1.toml", "re/objects-more.toml"] {
        assert!(read.contains(&want), "{want} not read; read {read:?}");
    }
    let groups = merge(&tables).unwrap_or_else(|e| panic!("{e}"));
    let mut missing = Vec::new();
    for world in 1..=6u8 {
        let level = read_level(world, &files).unwrap_or_else(|e| panic!("{e}"));
        // The used columns are the collision map's: 240, W6-1 320 (fd-assets README, Gotcha).
        let used = level.collision.columns as usize;
        missing.extend(ungrouped(
            &format!("W{world}-1"),
            &level.objects.cells,
            used,
            &groups,
        ));
    }
    assert!(
        missing.is_empty(),
        "{} cells hold a value no re/objects-*.toml groups, first: {:?}",
        missing.len(),
        &missing[..missing.len().min(10)]
    );
}

/// The flame, `$34`, is the one value of worlds 3 to 6 W1-1 lacks (counted 2026-11-20).
#[test]
fn the_flame_is_in_worlds_3_to_6_and_grouped_by_objects_more() {
    let groups = merge(&tables_on_disk()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        groups.get(&0x34),
        Some(&("flame".to_string(), "re/objects-more.toml".to_string()))
    );
    let mut counts = Vec::new();
    for world in 1..=6u8 {
        let level = read_level(world, &files).unwrap_or_else(|e| panic!("{e}"));
        let used = level.collision.columns as usize * 96;
        counts.push(
            level.objects.cells[..used]
                .iter()
                .filter(|&&v| v == 0x34)
                .count(),
        );
    }
    assert_eq!(counts, [0, 0, 31, 10, 25, 17]);
}

#[test]
fn an_unknown_value_is_named_with_its_cell() {
    let groups = merge(&[(
        "made-up.toml".into(),
        "[[value]]\nvalue = 0x20\ngroup = \"item\"\n".into(),
    )])
    .unwrap();
    let mut objects = vec![0u8; 3 * 96];
    objects[96 + 5] = 0x20;
    objects[2 * 96 + 7] = 0x77;
    assert_eq!(
        ungrouped("W9-1", &objects, 3, &groups),
        ["W9-1:2,7 holds $77"]
    );
    // Past the used columns nothing is looked at.
    assert!(ungrouped("W9-1", &objects, 2, &groups).is_empty());
}

#[test]
fn two_tables_grouping_one_value_differently_fail_naming_both() {
    let a = (
        "re/objects-W1-1.toml".to_string(),
        "[[value]]\nvalue = 0x34\ngroup = \"flame\"\n".to_string(),
    );
    let b = (
        "re/objects-W2-1.toml".to_string(),
        "[[value]]\nvalue = 52 # decimal\ngroup = \"bonus\"\n".to_string(),
    );
    let err = merge(&[a.clone(), b]).unwrap_err();
    assert!(
        err.contains("$34")
            && err.contains("re/objects-W1-1.toml")
            && err.contains("re/objects-W2-1.toml"),
        "{err}"
    );
    // The same group in two tables agrees.
    let c = ("re/objects-more.toml".to_string(), a.1.clone());
    assert_eq!(merge(&[a, c]).unwrap().len(), 1);
}

#[test]
fn a_row_without_a_group_fails_naming_its_file() {
    let err = merge(&[(
        "re/objects-x.toml".into(),
        "[[value]]\nvalue = 0x34\n".into(),
    )])
    .unwrap_err();
    assert!(err.contains("re/objects-x.toml"), "{err}");
}

/// Every `objects-*.toml` a directory holds is read, a per-world one a later plan adds included,
/// and nothing else; a per-world table contradicting another fails naming both.
#[test]
fn every_objects_table_in_re_is_read() {
    let dir = std::env::temp_dir().join(format!("fd-object-tables-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let row = |g: &str| format!("[[value]]\nvalue = 0x34\ngroup = \"{g}\"\n");
    for (n, text) in [
        ("objects-W1-1.toml", row("flame")),
        ("objects-W2-1.toml", row("bonus")),
        ("objects-more.toml", row("flame")),
        ("symbols.toml", row("ignored")),
        ("objects-old.txt", row("ignored")),
    ] {
        std::fs::write(dir.join(n), text).unwrap();
    }
    let tables = tables_in(&dir);
    std::fs::remove_dir_all(&dir).unwrap();
    let names: Vec<&str> = tables.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        [
            "re/objects-W1-1.toml",
            "re/objects-W2-1.toml",
            "re/objects-more.toml"
        ]
    );
    let err = merge(&tables).unwrap_err();
    assert!(err.contains("re/objects-W2-1.toml"), "{err}");
}
