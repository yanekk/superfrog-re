//! The `objects` generators T54 keeps (plans/forest-dumb-level-1 T54): the creature slots' movement
//! and their sort by y, on the main program from the owner's disks. Each runs 2 000 states on the
//! original with no hang, fault or unmodelled blit, takes every conditional branch of its flow
//! (callees included) both ways, and pokes nothing its routine does not read.
//!
//! The routine oracle has no instruction trace, so a branch's ways are found as in
//! `tests/gen_character.rs`: the `Bcc.w` at B is detoured through a stub where one way traps
//! (`illegal`) and the other `jmp`s back to where the original would have gone. A trap means some
//! state went that way. No branch here is unreachable.

use std::collections::BTreeSet;

use fd_conform::generators::{
    Generator, Setup, h,
    objects_move::{self, COVER, Cover},
};
use fd_conform::routine::{Rig, RigReport};

const CASES: usize = 2_000;
/// Free memory below hunk 0 (based at `$c077d8`), within a `bra.w` of every branch here (the
/// farthest, `h0+$3716`, is `$4eee` past it). `tests/gen_character.rs` uses it the same way.
const STUB: u32 = 0xc0_6000;
/// The states a detoured run tries before the full [`CASES`].
const QUICK: usize = 200;

#[derive(Clone, Copy, Debug)]
enum Way {
    Taken,
    NotTaken,
}

fn rig() -> Rig {
    Rig::new().unwrap_or_else(|e| panic!("{e}"))
}

fn generator(name: &str) -> Generator {
    fd_conform::generators::generator(name).unwrap_or_else(|| panic!("{name} is not registered"))
}

fn word(m: &[u8], a: u32) -> u16 {
    u16::from_be_bytes([m[a as usize], m[a as usize + 1]])
}

fn put(m: &mut [u8], a: u32, words: &[u16]) {
    for (i, w) in words.iter().enumerate() {
        let at = a as usize + 2 * i;
        m[at..at + 2].copy_from_slice(&w.to_be_bytes());
    }
}

/// A `Bcc.w` (not `bra`/`bsr`, not a byte displacement) or a `DBcc`.
fn is_branch(op: u16) -> bool {
    (op & 0xf000 == 0x6000 && op & 0x0e00 != 0 && op & 0xff == 0) || op & 0xf0f8 == 0x50c8
}

/// The `T54` routines of `re/translate.toml`, by name, read by hand: `fd-conform` has no toml parser.
fn t54_routines() -> BTreeSet<String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../re/translate.toml");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut out = BTreeSet::new();
    for entry in text.split("[[routine]]").skip(1) {
        let field = |k: &str| {
            entry.lines().find_map(|l| {
                let (key, v) = l.split_once('=')?;
                (key.trim() == k).then(|| v.trim().trim_matches('"').to_string())
            })
        };
        if field("gen_task").as_deref() == Some("T54") {
            assert_eq!(field("subsystem").as_deref(), Some("objects"));
            out.insert(field("name").expect("a routine has a name"));
        }
    }
    out
}

#[test]
fn every_t54_routine_has_a_generator_and_a_cover_entry() {
    let want = t54_routines();
    assert_eq!(
        want.len(),
        11,
        "translate.toml's T54 share changed: {want:?}"
    );
    let registered: BTreeSet<String> = objects_move::ALL
        .iter()
        .map(|(n, _)| n.to_string())
        .collect();
    let covered: BTreeSet<String> = COVER.iter().map(|c| c.name.to_string()).collect();
    assert_eq!(registered, want);
    assert_eq!(covered, want);
    for n in &want {
        assert!(
            fd_conform::generators::generator(n).is_some(),
            "{n} not in MODULES"
        );
    }
}

fn survey(r: &mut Rig, c: &Cover, g: &Generator) -> RigReport {
    r.survey(c.name, CASES, h(0, c.entry), g)
}

#[test]
fn each_generator_runs_2000_states_with_no_hang_or_fault() {
    let mut r = rig();
    let mut line = Vec::new();
    for c in COVER {
        let rep = survey(&mut r, c, &generator(c.name));
        assert_eq!(
            (rep.passed, rep.hangs, rep.faults, rep.unmodelled_blits),
            (CASES, 0, 0, 0),
            "{}: {rep:?}",
            c.name
        );
        line.push(format!("{} {}", c.name, rep.paths));
    }
    // The paths recorded in docs/11-translation.md.
    eprintln!("paths: {}", line.join(", "));
}

/// `base` with the branch at `b` detoured through [`STUB`] so that `way` traps.
fn detoured(base: &[u8], b: u32, way: Way) -> Vec<u8> {
    let mut m = base.to_vec();
    let (op, disp) = (word(&m, b), word(&m, b + 2));
    let target = (b + 2).wrapping_add(disp as i16 as i32 as u32);
    let fall = b + 4;
    let trap = [0x4afc, 0x4e71, 0x4e71];
    let jmp = |a: u32| [0x4ef9, (a >> 16) as u16, a as u16];
    let (not_taken, taken) = match way {
        Way::Taken => (jmp(fall), trap),
        Way::NotTaken => (trap, jmp(target)),
    };
    put(&mut m, STUB, &[op, 8]);
    put(&mut m, STUB + 4, &not_taken);
    put(&mut m, STUB + 10, &taken);
    let to_stub = STUB.wrapping_sub(b + 2) as i32;
    assert!(
        (-0x8000..0x8000).contains(&to_stub),
        "{b:x} is out of a bra.w's reach of the stub"
    );
    put(&mut m, b, &[0x6000, to_stub as u16]);
    m
}

#[test]
fn every_branch_is_taken_both_ways() {
    let base = rig().memory_for(&Generator {
        base: None,
        case: Setup::random,
    });
    let mut wrong = Vec::new();
    for c in COVER {
        let g = generator(c.name);
        let mut seen = BTreeSet::new();
        for &off in c.branches {
            let b = h(0, off);
            assert!(
                is_branch(word(&base, b)),
                "{} h0+${off:x}: ${:04x} is not a Bcc.w or DBcc",
                c.name,
                word(&base, b)
            );
            assert!(seen.insert(off), "{} lists h0+${off:x} twice", c.name);
            for way in [Way::Taken, Way::NotTaken] {
                let mut r = Rig::from_memory(detoured(&base, b, way));
                // The states are a seeded sequence, so the first `QUICK` are the first of the
                // full run's: a way one of them goes is a way the generator's states go. Only a
                // way none of them goes costs the full 2 000.
                let mut rep = r.survey(c.name, QUICK, h(0, c.entry), &g);
                if rep.faults == 0 {
                    rep = survey(&mut r, c, &g);
                }
                assert_eq!(rep.hangs, 0, "{} h0+${off:x} {way:?}: {rep:?}", c.name);
                if rep.faults == 0 {
                    wrong.push(format!(
                        "{} h0+${off:x} {way:?}: no state goes this way",
                        c.name
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn no_generator_pokes_outside_what_its_routine_reads() {
    for c in COVER {
        let g = generator(c.name);
        assert!(g.base.is_none(), "{}", c.name);
        let allowed: Vec<std::ops::Range<u32>> = c
            .reads
            .iter()
            .map(|v| {
                let (a, n) = v.span();
                a..a + n
            })
            .collect();
        let inside = |a: u32| allowed.iter().any(|r| r.contains(&a));
        let mut rng = fd_conform::routine::Gen::new(1);
        for _ in 0..CASES {
            let s = (g.case)(&mut rng);
            assert!(s.hw_reads.is_empty(), "{} sets a hardware read", c.name);
            for (a, bytes) in &s.pokes {
                for i in 0..bytes.len() as u32 {
                    assert!(inside(a + i), "{}: pokes ${:06x}", c.name, a + i);
                }
            }
        }
    }
}
