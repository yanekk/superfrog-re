//! The map cache (T05, DESIGN §2.8, §2.9) on small made-up maps: no disks, milliseconds.
//!
//! Every test runs under one temp `SF_MAP_CACHE`, set before any test reads it, so none touches the
//! real cache; each test uses keys of its own.

use std::cell::Cell as Counter;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use sf_core::LevelId;
use sf_plan::cache::{self, StartKey, cache_dir, key, load_or_build};
use sf_plan::core::{Edge, MoveMap, Node};

fn dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let d = std::env::temp_dir().join(format!("sf-map-cache-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        // SAFETY: set once, inside the OnceLock, before any test reads it (every test calls `dir`
        // first); nothing in this process reads the environment through C meanwhile.
        unsafe { std::env::set_var("SF_MAP_CACHE", &d) };
        d
    })
    .clone()
}

fn path(key: &str) -> PathBuf {
    dir().join(format!("{key}.map"))
}

/// A small map whose content depends on `n`.
fn map(n: u32) -> MoveMap {
    let nodes = (0..3 + n)
        .map(|i| Node {
            cell: (i as u16, 2 * i as u16),
            wings: i % 2 == 1,
        })
        .collect::<Vec<_>>();
    let edges = (0..nodes.len() as u32 - 1)
        .map(|i| Edge {
            from: i,
            to: i + 1,
            inputs: vec![(0x10, 3 + i as u16), (0x02, n as u16)],
            frames: 3 + i + n,
            coins: vec![i as u16],
            needs_switches: n & 1,
            needs_wings: i == 0,
        })
        .collect();
    MoveMap::new(nodes, edges)
}

fn level(s: &str) -> LevelId {
    s.parse().unwrap()
}

#[test]
fn the_cache_is_the_temp_one() {
    let d = dir();
    assert_eq!(cache_dir(), d);
    assert!(
        !d.starts_with(
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap()
                .join(".cache")
        )
    );
}

#[test]
fn round_trip_is_equal() {
    dir();
    let k = "round-trip";
    let built = Counter::new(0);
    let a = load_or_build(k, false, || {
        built.set(built.get() + 1);
        map(4)
    });
    assert_eq!(a, map(4));
    assert!(path(k).is_file());
    let b = load_or_build(k, false, || panic!("the saved map is used"));
    assert_eq!(b, map(4));
    assert_eq!(built.get(), 1);
}

#[test]
fn a_truncated_file_is_rebuilt_and_rewritten() {
    dir();
    let k = "truncated";
    load_or_build(k, false, || map(5));
    let whole = std::fs::read(path(k)).unwrap();
    for cut in [0, 7, 20, whole.len() / 2, whole.len() - 1] {
        std::fs::write(path(k), &whole[..cut]).unwrap();
        let built = Counter::new(false);
        let m = load_or_build(k, false, || {
            built.set(true);
            map(5)
        });
        assert!(built.get(), "cut at {cut} was trusted");
        assert_eq!(m, map(5));
        assert_eq!(std::fs::read(path(k)).unwrap(), whole, "rewritten whole");
    }
}

#[test]
fn a_corrupt_file_is_rebuilt() {
    dir();
    let k = "corrupt";
    load_or_build(k, false, || map(6));
    let whole = std::fs::read(path(k)).unwrap();
    // A frame count flipped in the payload decodes as a map, so only the checksum catches it.
    let mut bad = whole.clone();
    *bad.last_mut().unwrap() ^= 0x01;
    assert!(cache::decode(k, &bad).is_none());
    // So does a longer file, and the magic or the format changed.
    let mut long = whole.clone();
    long.push(0);
    assert!(cache::decode(k, &long).is_none());
    let mut magic = whole.clone();
    magic[0] ^= 0xff;
    assert!(cache::decode(k, &magic).is_none());
    let mut format = whole.clone();
    format[8] ^= 0xff;
    assert!(cache::decode(k, &format).is_none());
    std::fs::write(path(k), &bad).unwrap();
    let built = Counter::new(false);
    load_or_build(k, false, || {
        built.set(true);
        map(6)
    });
    assert!(built.get());
}

#[test]
fn a_file_of_another_key_is_ignored() {
    dir();
    let here = key(level("W1-4"), StartKey::AtLevel, false);
    // Another engine build: the same level, start and wings, another build hash.
    let (stem, hash) = here.rsplit_once('.').unwrap();
    assert_eq!(hash.len(), 16);
    let other = format!(
        "{stem}.{}",
        if hash == "0000000000000000" { "1" } else { "0" }.repeat(16)
    );
    assert_ne!(other, here);
    // Written under the other build, then found at this build's name: the key inside says whose.
    std::fs::write(path(&here), cache::encode(&other, &map(7))).unwrap();
    let built = Counter::new(false);
    let m = load_or_build(&here, false, || {
        built.set(true);
        map(8)
    });
    assert!(built.get(), "another build's map was trusted");
    assert_eq!(m, map(8));
    assert_eq!(
        cache::decode(&here, &std::fs::read(path(&here)).unwrap()),
        Some(map(8))
    );
}

#[test]
fn the_key_separates_what_makes_a_map() {
    let w14 = level("W1-4");
    let keys = [
        key(w14, StartKey::AtLevel, false),
        key(w14, StartKey::AtLevel, true),
        key(level("W6-3"), StartKey::AtLevel, false),
        key(level("W6-3alt"), StartKey::AtLevel, false),
        key(w14, StartKey::App { seed: 1 }, false),
        key(w14, StartKey::App { seed: 2 }, false),
        key(w14, StartKey::snap(b"leg one"), false),
        key(w14, StartKey::snap(b"leg two"), false),
    ];
    let distinct = keys.iter().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(distinct.len(), keys.len(), "{keys:?}");
    // Stable within a build, and a plain file name.
    assert_eq!(keys[0], key(w14, StartKey::AtLevel, false));
    assert_eq!(StartKey::snap(b"leg one"), StartKey::snap(b"leg one"));
    for k in &keys {
        assert!(
            k.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.'),
            "{k}"
        );
    }
    assert!(keys[0].starts_with("W1-4.at.nowings."), "{}", keys[0]);
}

#[test]
fn no_cache_builds_and_leaves_the_file_alone() {
    dir();
    let k = "no-cache";
    load_or_build(k, false, || map(1));
    let before = std::fs::read(path(k)).unwrap();
    let built = Counter::new(false);
    let m = load_or_build(k, true, || {
        built.set(true);
        map(2)
    });
    assert!(built.get());
    assert_eq!(m, map(2));
    assert_eq!(std::fs::read(path(k)).unwrap(), before);
    // And with no file there, writes none.
    let fresh = "no-cache-fresh";
    load_or_build(fresh, true, || map(3));
    assert!(!path(fresh).exists());
}

#[test]
fn two_writers_leave_one_whole_file() {
    dir();
    let k = "two-writers";
    // Big maps, so a write takes long enough to overlap the other's and a reader's.
    let maps = [map(4000), map(4001)];
    let writing = AtomicUsize::new(maps.len());
    let reads = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for m in &maps {
            s.spawn(|| {
                for _ in 0..40 {
                    let _ = std::fs::remove_file(path(k));
                    let got = load_or_build(k, false, || m.clone());
                    assert!(maps.contains(&got));
                }
                writing.fetch_sub(1, Ordering::SeqCst);
            });
        }
        s.spawn(|| {
            // A reader sees no file or a whole one, never a part, for as long as the writers write.
            while writing.load(Ordering::SeqCst) > 0 {
                if let Ok(b) = std::fs::read(path(k)) {
                    let m = cache::decode(k, &b).expect("a part-written map");
                    assert!(maps.contains(&m));
                    reads.fetch_add(1, Ordering::SeqCst);
                }
            }
        });
    });
    assert!(
        reads.load(Ordering::SeqCst) > 0,
        "the reader never overlapped a writer"
    );
    let m = cache::decode(k, &std::fs::read(path(k)).unwrap()).expect("one whole file");
    assert!(maps.contains(&m));
    let tmp = std::fs::read_dir(dir())
        .unwrap()
        .flatten()
        .filter(|e| {
            e.file_name().to_string_lossy().starts_with(k)
                && e.file_name().to_string_lossy().ends_with(".tmp")
        })
        .count();
    assert_eq!(tmp, 0, "temp files left behind");
}

/// The core modules `src` uses: `super::m` and `crate::core::m`, grouped (`super::{a, b}`) or not.
fn core_uses(src: &str) -> Vec<String> {
    let ident = |s: &str| {
        s.chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect::<String>()
    };
    let mut uses = Vec::new();
    for pat in ["super::", "crate::core::"] {
        for (at, _) in src.match_indices(pat) {
            let rest = &src[at + pat.len()..];
            if let Some(group) = rest.strip_prefix('{') {
                let group = &group[..group.find('}').unwrap_or(group.len())];
                uses.extend(group.split(',').map(|g| ident(g.trim())));
            } else {
                uses.push(ident(rest));
            }
        }
    }
    uses.retain(|m| !m.is_empty() && m.chars().next().unwrap().is_ascii_lowercase());
    uses
}

#[test]
fn the_key_hashes_every_core_file_a_map_is_built_from() {
    let core = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/core");
    let listed = cache::CORE_FILES
        .iter()
        .map(|(n, _)| n.strip_suffix(".rs").unwrap().to_string())
        .collect::<Vec<_>>();
    assert!(listed.contains(&"map".to_string()));
    for (name, bytes) in cache::CORE_FILES {
        // The hashed bytes are the file as it is now (`include_bytes!` rebuilds on a change).
        assert_eq!(std::fs::read(core.join(name)).unwrap(), bytes, "{name}");
        for m in core_uses(std::str::from_utf8(bytes).unwrap()) {
            assert!(
                listed.contains(&m),
                "{name} uses core::{m}: add {m}.rs to cache::CORE_FILES"
            );
        }
    }
}

#[test]
fn core_uses_reads_every_form() {
    let src = "use super::facts::{Cell, live};\nuse super::{graph, moves::Macro};\n\
               let d = super::graph::shortest(m);\nuse crate::core::lookup::Word;\nsuper::Builder";
    assert_eq!(
        core_uses(src),
        ["facts", "graph", "moves", "graph", "lookup"]
    );
}
