//! Translator T13, stage 1's gate (DESIGN §2.6): the whole census, every kind the program uses, each
//! translated alone and compared with Moira's step in every column (regs, ccr, mem, cycles) on 2 000
//! generated states a site, seeded apart from T06 to T12's own runs. It wraps the tool, not a copy of
//! it: extract the disks, export the whole program with Ghidra (`pcode_export.py --whole`), then
//! `insn_check.py`, which builds `fd-judge` optimised with the census module and judges in shards.
//! About 20 s from a warm build (the export 9 s), a few minutes cold, so `#[ignore]`d for full. The
//! reading is in `docs/14-translator.md`; the report `build/translator/insn.md` names any failing kind.

use std::path::PathBuf;
use std::process::Command;

/// The gate's seed. Any text no earlier task's run used makes fresh states; this one is the seed the
/// reading in docs/14-translator.md was taken with, so the test and the reading agree.
const SEED: &str = "insn-gate-T13";
const STATES: &str = "2000";

fn game() -> PathBuf {
    // Canonical: fd_tools::extract refuses an output path holding `..`.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap()
}

fn python(args: &[&str]) -> String {
    let out = Command::new("python3")
        .arg("-I")
        .args(args)
        .current_dir(game())
        .output()
        .expect("python3 runs");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{args:?} exited {}:\n{stdout}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    stdout
}

/// The number after `"key":` in a one-line JSON summary.
fn field(summary: &str, key: &str) -> u64 {
    let pat = format!("\"{key}\":");
    let at = summary
        .find(&pat)
        .unwrap_or_else(|| panic!("{key} in {summary}"))
        + pat.len();
    let digits: String = summary[at..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits
        .parse()
        .unwrap_or_else(|_| panic!("{key} in {summary}"))
}

#[test]
#[ignore = "full: Ghidra's export and the whole census on Moira, 2 000 states a site, about 20 s warm"]
fn every_census_kind_passes_every_column_on_fresh_states() {
    let disks = fd_tools::disk_dir().unwrap_or_else(|e| panic!("{e}"));
    fd_tools::extract(&disks, &game().join("build")).unwrap_or_else(|e| panic!("{e}"));
    let export = python(&["tools/pcode_export.py", "--whole"]);
    // T03's whole export; a different count means the census below is not the one recorded.
    assert_eq!(
        field(export.lines().last().unwrap(), "instructions"),
        6684,
        "{export}"
    );

    let out = python(&[
        "tools/insn_check.py",
        "--states",
        STATES,
        "--columns",
        "regs,ccr,mem,cycles",
        "--seed",
        SEED,
    ]);
    let s = out.lines().last().expect("a summary line");
    assert_eq!(field(s, "kinds"), 290, "{s}");
    assert_eq!(
        field(s, "failing_kinds"),
        0,
        "{s}\nsee games/forest-dumb/build/translator/insn.md"
    );
    assert_eq!(field(s, "fail"), 0, "{s}");
    for col in ["regs", "ccr", "mem", "cycles", "stop"] {
        assert_eq!(field(s, col), 0, "{col}: {s}");
    }
    // Every kind's cycles counted (a "not-counted" kind would hide its cycle failures, FINDINGS).
    let json = std::fs::read_to_string(game().join("build/translator/insn.json")).unwrap();
    assert!(!json.contains("not-counted"), "a kind's cycles not counted");
    assert!(field(s, "skipped") * 50 < field(s, "states"), "{s}");
    // And every kind judged on a quarter of its states at least: the total above would hide one kind
    // skipped whole, which then "passes" on nothing. The original's exceptions skip at most about half
    // of a kind's states (`move.l (d8,An,Xn),(An)`, 1 051 of 2 000, from a random odd index).
    let kinds = per_kind(&json);
    assert_eq!(kinds.len(), 290);
    for (kind, states, pass) in kinds {
        assert!(
            pass * 4 >= states,
            "{kind}: judged on {pass} of {states} states"
        );
    }
}

/// `(kind, states, pass)` for each record of `insn.json`, a pretty-printed list of flat-keyed kinds.
fn per_kind(json: &str) -> Vec<(String, u64, u64)> {
    let num = |rec: &str, key: &str| -> u64 {
        let pat = format!("\"{key}\":");
        let at = rec.find(&pat).unwrap_or_else(|| panic!("{key} in {rec}")) + pat.len();
        let digits: String = rec[at..]
            .trim_start()
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        digits.parse().unwrap_or_else(|_| panic!("{key} in {rec}"))
    };
    json.split("\"kind\":")
        .skip(1)
        .map(|rec| {
            let name = rec.trim_start().trim_start_matches('"');
            let name = name[..name.find('"').unwrap()].to_string();
            (name, num(rec, "states"), num(rec, "pass"))
        })
        .collect()
}
