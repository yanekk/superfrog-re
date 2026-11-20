//! T02 (translator DESIGN §2.3): the code the original runs, over level 1's cases, the power-on run
//! and the world 2 to 6 starts, reduced to `re/entries.toml`. The reduction's unit tests are in
//! `src/code_seen.rs`; this is the run on the game, about 8.5 minutes unoptimised, so it is
//! `#[ignore]`d for full.
//!
//! A fresh run must write exactly the committed file: a second run of `fd-conform code-seen` is
//! identical to the one that wrote it. When a case is added or changed and this fails, rerun
//! `cargo run --release -p fd-conform -- code-seen` and commit the new file.

use std::collections::BTreeSet;

use amiga_oracle::kickstart_path;
use fd_conform::code_seen::{
    Options, Source, collect_in, entries, funcs, parse_symbols, re_dir, render,
};
use fd_conform::sync::cache_dir;

#[test]
#[ignore = "full: every case, the power-on run and five world starts on the original, about 8.5 minutes unoptimised"]
fn a_fresh_run_writes_the_committed_entries_and_every_executed_func_is_in_them() {
    let kickstart = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    let disks = fd_tools::disk_dir().unwrap_or_else(|e| panic!("{e}"));
    let mut skipped = Vec::new();
    let (seen, hunks) = collect_in(
        &kickstart,
        &disks,
        &cache_dir(),
        &Options::default(),
        &mut skipped,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    // The power-on cases (T32) are not level cases; the power-on run below covers their way.
    assert_eq!(
        skipped,
        ["first-case", "power-on-level-1", "power-on-title"]
    );
    let runs: Vec<&str> = seen.runs.iter().map(|r| r.0.as_str()).collect();
    for w in [
        "power-on", "world-2", "world-3", "world-4", "world-5", "world-6",
    ] {
        assert!(runs.contains(&w), "{runs:?}");
    }
    assert_eq!(runs.len(), 1 + 5 + 42, "{runs:?}");

    let symbols =
        parse_symbols(&std::fs::read_to_string(re_dir().join("symbols.toml")).unwrap()).unwrap();
    let funcs = funcs(&symbols);
    let e = entries(&seen.insns, &funcs, &seen.vectors, hunks[0].0);
    let executed: BTreeSet<u32> = funcs
        .iter()
        .copied()
        .filter(|a| seen.insns.contains_key(a))
        .collect();
    for a in &executed {
        assert!(
            matches!(
                e.get(a),
                Some(Source::Symbol | Source::Trace | Source::ProgramEntry)
            ),
            "func at {a:x} ran but is not an entry"
        );
    }

    let committed = std::fs::read_to_string(re_dir().join("entries.toml")).unwrap();
    assert!(
        render(&e, &hunks) == committed,
        "re/entries.toml is not what a fresh run writes; rerun `fd-conform code-seen` and commit it"
    );
}
