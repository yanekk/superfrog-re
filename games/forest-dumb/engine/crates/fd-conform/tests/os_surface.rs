//! T01 (plans/forest-dumb-translator): the OS survey on the original. The run from power-on
//! through level 1's load and world 2's load takes about 25 s optimised, 56 s in a test build; it
//! is in the full set.

use std::path::Path;

use fd_conform::os_surface::{Options, committed_path, run, run_in};

/// `docs/07-executable-map.md`'s file routines and where each calls dos.library: `jsr -$1e(a5)`,
/// `jsr -$2a(a5)` and `jsr -$24(a5)`.
const FILE_ROUTINES: [(&str, [&str; 3]); 3] = [
    ("load_file", ["h0+$8eec", "h0+$8f1e", "h0+$8f2e"]),
    ("load_file_raw", ["h0+$8f80", "h0+$8fb2", "h0+$8fc2"]),
    ("read_marker", ["h0+$9000", "h0+$9032", "h0+$9042"]),
];

/// `save_scores_unreferenced`'s Open, which docs/07 finds unreachable.
const SAVE_SCORES_OPEN: &str = "h0+$9456";

#[test]
#[ignore = "full: boots the original to level 1 and world 2's load, about 56 s"]
fn the_survey_is_the_committed_file_and_finds_the_file_routines() {
    let opts = Options::default();
    let r = run(&opts).unwrap_or_else(|e| panic!("{e}"));
    let text = r.toml(opts.frames, &opts.worlds);
    let path = committed_path();
    let committed =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    // The same run gives the same bytes, so a difference is a changed program, emulator or tool:
    // regenerate with `fd-conform os-surface` and read the diff.
    assert!(
        text == committed,
        "{} is stale; the run gave:\n{text}",
        path.display()
    );
    assert!(r.undecoded.is_empty(), "{:?}", r.undecoded);
    for (call, k) in [("Open", 0), ("Read", 1), ("Close", 2)] {
        let found = r.calls_named(call);
        let dos: Vec<&String> = found
            .iter()
            .filter(|(l, _)| l == "dos.library")
            .flat_map(|(_, s)| s)
            .collect();
        for (routine, sites) in FILE_ROUTINES {
            assert!(
                dos.contains(&&sites[k].to_string()),
                "{routine}'s {call} at {} not in {found:?}",
                sites[k]
            );
        }
        assert!(
            found.iter().all(|(l, _)| !l.contains('|')),
            "{call} left ambiguous: {found:?}"
        );
    }
    assert!(
        !r.calls
            .values()
            .any(|g| g.sites.iter().any(|s| s == SAVE_SCORES_OPEN)),
        "save_scores_unreferenced ran"
    );
}

#[test]
fn a_missing_kickstart_names_the_path() {
    let e = run_in(
        Path::new("/nonexistent/kick.rom"),
        Path::new("/nonexistent"),
        &Options::default(),
    )
    .unwrap_err();
    assert!(e.contains("/nonexistent/kick.rom"), "{e}");
}

#[test]
fn a_world_out_of_range_is_refused() {
    let opts = Options {
        worlds: vec![1, 7],
        ..Options::default()
    };
    let e = run_in(
        Path::new("/nonexistent/kick.rom"),
        Path::new("/nonexistent"),
        &opts,
    )
    .unwrap_err();
    assert!(e.contains("1 to 6"), "{e}");
}
