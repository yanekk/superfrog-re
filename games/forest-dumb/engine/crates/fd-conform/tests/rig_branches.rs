//! `Rig::coverage` (plans/forest-dumb-level-1 T60, DESIGN §4.3): every conditional branch of a
//! routine and its callees, and over how many of a generator's states it went each way, measured
//! on the original by the routine runner's trace. The rules run on code assembled by hand into empty
//! memory; T36's and T37's generators run on the main program from the owner's disks.

use std::collections::BTreeSet;
use std::time::Instant;

use fd_conform::generators::{self, Generator, Setup, camera, h};
use fd_conform::routine::{Branch, Rig};

const CODE: u32 = 0x2000;

/// `tst.l d0; beq.s +4; moveq #1,d2; bra.s +6; tst.l d1; bne.s +2; moveq #2,d2; tst.l d3;
/// bmi.s +2; moveq #3,d4; rts`: branches at `$2002` and `$2010` in sequence, `$200a` inside the
/// first's taken arm.
const NESTED: [u16; 11] = [
    0x4a80, 0x6704, 0x7401, 0x6006, 0x4a81, 0x6602, 0x7402, 0x4a83, 0x6b02, 0x7803, 0x4e75,
];

fn rig() -> Rig {
    let mut mem = vec![0u8; 0x10_0000];
    let bytes: Vec<u8> = NESTED.iter().flat_map(|w| w.to_be_bytes()).collect();
    mem[CODE as usize..CODE as usize + bytes.len()].copy_from_slice(&bytes);
    let mut r = Rig::from_memory(mem);
    r.set_max_instr(1_000);
    r
}

/// d0 never zero, so the inner branch never runs.
fn d0_set(g: &mut generators::Gen) -> Setup {
    let mut s = Setup::random(g);
    s.d[0] |= 1;
    s
}

#[test]
fn a_branch_no_state_reaches_is_listed_with_both_counts_zero() {
    let mut r = rig();
    let generator = Generator {
        base: None,
        case: d0_set,
    };
    let c = r.coverage("nested", 100, CODE, &generator);
    assert_eq!(c.report.passed, 100);
    assert_eq!(c.branches.len(), 3, "{:?}", c.branches);
    assert_eq!(
        c.branches[0],
        Branch {
            at: 0x2002,
            taken: 0,
            not_taken: 100
        }
    );
    assert_eq!(
        c.branches[1],
        Branch {
            at: 0x200a,
            taken: 0,
            not_taken: 0
        }
    );
    assert!(c.branches[2].covered(), "{:?}", c.branches[2]);
    let uncovered: Vec<u32> = c.uncovered().map(|b| b.at).collect();
    assert_eq!(uncovered, vec![0x2002, 0x200a]);
    // The report is the survey's, and coverage leaves check and survey as they were.
    let before = r.survey("nested", 100, CODE, &generator);
    assert_eq!(c.report, before);
    assert_eq!(r.survey("nested", 100, CODE, &generator), before);
    // No reads: the routine reads only registers (the stack is left out).
    assert!(c.reads.is_empty(), "{:?}", c.reads);
}

// ---------------------------------------------------------------------------------------------
// On the main program, from the owner's disks
// ---------------------------------------------------------------------------------------------

const CASES: usize = 2000;

/// The 28 branches of `camera_update`'s flow listing, callees included, as `tests/gen_camera.rs`
/// lists them (h0 offsets), and as `docs/11-translation.md`'s T36 table counts them.
const CAMERA_UPDATE: [u32; 28] = [
    0x4dac, 0x4dd0, 0x4df4, 0x4e18, 0x4e26, 0x4e44, 0x4e62, 0x4e80, // its own
    0x51ac, 0x51b0, 0x5212, 0x5228, 0x523c, // right: flag step loop, scroll
    0x51c2, 0x51c6, 0x5280, 0x5296, 0x52aa, // left
    0x51d8, 0x51dc, 0x52ee, 0x5304, 0x5318, // up
    0x51ee, 0x51f2, 0x535c, 0x5372, 0x5386, // down
];

#[test]
fn camera_update_covers_t36s_28_branches_both_ways() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let generator = generators::generator("camera_update").unwrap();
    let c = r.coverage("camera_update", CASES, h(0, 0x4d7e), &generator);
    assert_eq!(
        (c.report.passed, c.report.hangs, c.report.faults),
        (CASES, 0, 0)
    );
    let mut want: Vec<u32> = CAMERA_UPDATE.iter().map(|&o| h(0, o)).collect();
    want.sort();
    let got: Vec<u32> = c.branches.iter().map(|b| b.at).collect();
    assert_eq!(got, want, "the branches are not T36's");
    let uncovered: Vec<String> = c.uncovered().map(|b| format!("{b:?}")).collect();
    assert!(uncovered.is_empty(), "uncovered: {uncovered:?}");
    // What it reads is what T36 found by hand (`tests/gen_camera.rs`, `reads`).
    use camera::*;
    let mut spans = vec![
        (CHAR_X, 4),
        (CHAR_Y, 4),
        (CAMERA_X, 2),
        (CAMERA_Y, 2),
        (FLAGS, 4),
    ];
    spans.extend(LATCHES.iter().map(|&l| (l, 2)));
    let want: BTreeSet<u32> = spans
        .into_iter()
        .flat_map(|(o, n)| (0..n).map(move |i| h(0, o + i)))
        .collect();
    assert_eq!(c.reads, want);
    let least = c
        .branches
        .iter()
        .map(|b| b.taken.min(b.not_taken))
        .min()
        .unwrap();
    println!(
        "camera_update: {} paths, {} branches, each way on at least {least} states, {} bytes read",
        c.report.paths,
        c.branches.len(),
        c.reads.len()
    );
}

#[test]
fn h0_29ca_leaves_its_blitter_wait_uncovered() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let generator = generators::generator("h0_29ca").unwrap();
    let c = r.coverage("h0_29ca", CASES, h(0, 0x29ca), &generator);
    assert_eq!(c.report.hangs, 0, "{:?}", c.report);
    let wait = h(0, 0x2a1c);
    let uncovered: Vec<Branch> = c.uncovered().copied().collect();
    assert_eq!(uncovered.len(), 1, "{:?}", c.branches);
    let b = uncovered[0];
    assert_eq!((b.at, b.taken), (wait, 0), "{b:?}");
    assert!(b.not_taken > 0, "{b:?}");
    // The blitter wait polls DMACONR, so the hardware register is among the reads.
    assert!(c.reads.contains(&0xdf_f002), "{:?}", c.reads);
    println!("h0_29ca: {:?}", c.branches);
}

/// The trace's time cost, for `toolkit/docs/architecture/verification.md`: a survey (recording off)
/// against coverage (on) over the same states. Measured, not asserted; run with `--ignored
/// --nocapture` (a release build gives the figures the doc records).
#[test]
#[ignore]
fn measure_the_cost_of_recording() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    for (name, off) in [("camera_update", 0x4d7e), ("h0_29ca", 0x29ca)] {
        let generator = generators::generator(name).unwrap();
        let entry = h(0, off);
        // Once each first, so neither pays for a cold start.
        r.survey(name, CASES, entry, &generator);
        r.coverage(name, CASES, entry, &generator);
        let t = Instant::now();
        let rep = r.survey(name, CASES, entry, &generator);
        let off_t = t.elapsed();
        let t = Instant::now();
        let c = r.coverage(name, CASES, entry, &generator);
        let on_t = t.elapsed();
        assert_eq!(rep, c.report);
        println!(
            "{name}: {CASES} states, off {:.1} ms, on {:.1} ms ({:.2}x)",
            off_t.as_secs_f64() * 1e3,
            on_t.as_secs_f64() * 1e3,
            on_t.as_secs_f64() / off_t.as_secs_f64()
        );
    }
}
