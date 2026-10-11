//! The asset cache on the owner's real `GameData`: it round-trips, is deterministic, refuses every
//! kind of damage; and `decide` over every launch situation. Its speed against a fresh import is
//! measured in sf-conform/tests/asset_cache_speed.rs (the clock is shell-side, DESIGN §3.1).

mod common;

use common::{data, disks};
use sf_assets::*;
use sf_disk::{DiskSlot, Offered};

fn cache() -> Vec<u8> {
    cache_encode(data(), "test-build")
}

/// Where the body starts: magic, format, build string, body length, hash.
fn body_start(build: &str) -> usize {
    8 + 4 + 4 + build.len() + 8 + 32
}

#[test]
fn the_real_game_data_round_trips() {
    let bytes = cache();
    assert!(cache_decode(&bytes).unwrap() == *data());
}

#[test]
fn encoding_is_deterministic() {
    assert_eq!(cache(), cache());
    // A clone (maps rebuilt) and another build string change only the header.
    let a = cache_encode(&data().clone(), "a");
    let b = cache_encode(data(), "another-build");
    assert_eq!(a[body_start("a")..], b[body_start("another-build")..]);
}

#[test]
fn every_kind_of_damage_is_detected() {
    let good = cache();
    let start = body_start("test-build");

    for at in [start, start + 1, (start + good.len()) / 2, good.len() - 1] {
        let mut b = good.clone();
        b[at] ^= 0x01;
        assert_eq!(
            cache_decode(&b),
            Err(CacheError::Corrupt),
            "flipped byte at {at}"
        );
    }
    // The hash itself damaged.
    let mut b = good.clone();
    b[start - 1] ^= 0x80;
    assert_eq!(cache_decode(&b), Err(CacheError::Corrupt));
    // Bytes after the body.
    let mut b = good.clone();
    b.push(0);
    assert_eq!(cache_decode(&b), Err(CacheError::Corrupt));

    for len in [1, 7, 8, 11, 12, start - 1, start, good.len() - 1] {
        assert_eq!(
            cache_decode(&good[..len]),
            Err(CacheError::Truncated),
            "cut to {len} bytes"
        );
    }

    let mut b = good.clone();
    b[8..12].copy_from_slice(&(CACHE_FORMAT + 1).to_be_bytes());
    assert_eq!(
        cache_decode(&b),
        Err(CacheError::OtherFormat(CACHE_FORMAT + 1))
    );

    let mut b = good.clone();
    b[0] = b'X';
    assert_eq!(cache_decode(&b), Err(CacheError::BadMagic));
    assert_eq!(cache_decode(b""), Err(CacheError::BadMagic));
    assert_eq!(cache_decode(b"T7MP"), Err(CacheError::BadMagic));
}

#[test]
fn an_empty_game_data_round_trips() {
    let empty = GameData::default();
    assert_eq!(cache_decode(&cache_encode(&empty, "")).unwrap(), empty);
}

fn real_disks() -> Vec<Offered> {
    DiskSlot::ALL
        .iter()
        .map(|s| Offered {
            name: format!("disk{}.adf", s.number()),
            bytes: disks().disk(*s).bytes.clone(),
        })
        .collect()
}

fn reason(a: &LaunchAction) -> &str {
    match a {
        LaunchAction::UseCache => "",
        LaunchAction::Reimport { reason } | LaunchAction::AskForDisks { reason } => reason,
    }
}

/// A sentence a player can read: capitalised, full stop, no code in it.
fn assert_human(s: &str) {
    assert!(s.chars().next().unwrap().is_uppercase(), "{s}");
    assert!(s.ends_with('.'), "{s}");
    for code in [
        "::",
        "{",
        "}",
        "Err(",
        "Some(",
        "None",
        "BadMagic",
        "Corrupt",
        "Truncated",
        "OtherFormat",
    ] {
        assert!(!s.contains(code), "`{code}` in {s}");
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Expect {
    Use,
    Reimport,
    Ask,
}

#[test]
fn decide_covers_every_combination() {
    let good = real_disks();
    let mut changed = good.clone();
    changed[2].bytes[1000] ^= 1;
    let mut missing = good.clone();
    missing.remove(3);

    let caches: [(&str, Option<Result<(), CacheError>>); 6] = [
        ("good", Some(Ok(()))),
        ("none", None),
        ("bad magic", Some(Err(CacheError::BadMagic))),
        ("other format", Some(Err(CacheError::OtherFormat(0)))),
        ("corrupt", Some(Err(CacheError::Corrupt))),
        ("truncated", Some(Err(CacheError::Truncated))),
    ];
    let disk_sets: [(&str, Option<Vec<Offered>>); 5] = [
        ("good", Some(good)),
        ("none remembered", None),
        ("one changed", Some(changed)),
        ("one gone", Some(missing)),
        ("all gone", Some(Vec::new())),
    ];
    for (cn, cache) in &caches {
        for (dn, disks) in &disk_sets {
            let expect = match (*cn, *dn) {
                ("good", _) => Expect::Use,
                (_, "good") => Expect::Reimport,
                _ => Expect::Ask,
            };
            let a = decide(&LaunchFacts {
                cache: *cache,
                remembered_disks: disks.clone(),
            });
            let got = match a {
                LaunchAction::UseCache => Expect::Use,
                LaunchAction::Reimport { .. } => Expect::Reimport,
                LaunchAction::AskForDisks { .. } => Expect::Ask,
            };
            assert_eq!(got, expect, "cache {cn}, disks {dn}: {a:?}");
            if got != Expect::Use {
                assert_human(reason(&a));
            }
        }
    }
}

#[test]
fn reasons_say_what_happened() {
    let a = decide(&LaunchFacts {
        cache: Some(Err(CacheError::OtherFormat(0))),
        remembered_disks: Some(real_disks()),
    });
    println!("{}", reason(&a));
    assert!(reason(&a).contains("another version"));
    let mut missing = real_disks();
    missing.remove(3);
    let a = decide(&LaunchFacts {
        cache: None,
        remembered_disks: Some(missing),
    });
    println!("{}", reason(&a));
    assert!(reason(&a).contains("not been imported"));
    assert!(
        reason(&a).contains("Disk 4"),
        "names the missing disk: {}",
        reason(&a)
    );
    let a = decide(&LaunchFacts {
        cache: None,
        remembered_disks: None,
    });
    println!("{}", reason(&a));
}
