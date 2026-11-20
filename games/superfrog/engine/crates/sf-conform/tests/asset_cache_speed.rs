//! T09 done-when: decoding the asset cache is faster than a fresh import (unpack + decode) of the
//! owner's disks. Here, not in sf-assets, because a pure crate's tests may not read the clock
//! (DESIGN §3.1). Both times are printed; run with `--nocapture` to see them.

use std::time::Instant;

use sf_assets::{cache_decode, cache_encode, decode, unpack};
use sf_disk::{Offered, identify};

#[test]
fn decoding_the_cache_is_faster_than_a_fresh_import() {
    let files: Vec<Offered> = sf_conform::disks::read_disk_files()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|(name, bytes)| Offered { name, bytes })
        .collect();
    let set = identify(&files).unwrap_or_else(|e| panic!("{e:?}"));

    let t = Instant::now();
    let fresh = decode(&unpack(&set).unwrap()).unwrap();
    let import = t.elapsed();
    let bytes = cache_encode(&fresh, "test-build");
    let t = Instant::now();
    let cached = cache_decode(&bytes).unwrap();
    let load = t.elapsed();

    assert!(cached == fresh);
    println!(
        "fresh import {import:?}, cache decode {load:?}, cache {} bytes",
        bytes.len()
    );
    assert!(
        load < import,
        "cache decode {load:?} >= fresh import {import:?}"
    );
}
