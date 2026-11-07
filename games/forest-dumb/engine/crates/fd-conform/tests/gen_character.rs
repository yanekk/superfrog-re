//! The `character` routines' generators (plans/forest-dumb-level-1 T35) on the main program from the
//! owner's disks: each runs 2 000 states on the original with no hang or fault, takes every
//! conditional branch of its routine both ways (or the branch is named unreachable with why), and
//! pokes nothing its routine does not read.
//!
//! The routine oracle has no instruction trace, so a branch's ways are found by detouring it: the
//! `Bcc`/`DBcc` at B becomes `bra.w` to a stub holding the same branch, whose two ends either trap
//! (`illegal`) or `jmp` back to where the original would have gone. One way traps per run, so every
//! state that does not trap runs exactly as the original, and a trap means some state went that way.

use std::collections::BTreeSet;

use fd_conform::generators::{
    Generator, Setup,
    character::{self, COLLISION_MAP, COVER, Cover, Way},
    h,
};
use fd_conform::routine::{Rig, RigReport};

const CASES: usize = 2_000;
/// Free memory below hunk 0 (based at `$c077d8`), within a `bra.w` of every branch here.
const STUB: u32 = 0xc0_6000;

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

/// The `T35` routines of `re/translate.toml`, by name, read by hand: `fd-conform` has no toml parser
/// (Cargo.toml says why).
fn t35_routines() -> BTreeSet<String> {
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
        if field("gen_task").as_deref() == Some("T35") {
            out.insert(field("name").expect("a routine has a name"));
        }
    }
    out
}

#[test]
fn every_t35_routine_has_a_generator_and_a_cover_entry() {
    let want = t35_routines();
    assert_eq!(
        want.len(),
        14,
        "translate.toml's T35 share changed: {want:?}"
    );
    let registered: BTreeSet<String> = character::ALL.iter().map(|(n, _)| n.to_string()).collect();
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

fn survey(r: &mut Rig, c: &Cover) -> RigReport {
    r.survey(c.name, CASES, h(0, c.entry), &generator(c.name))
}

#[test]
fn each_generator_runs_2000_states_with_no_hang_or_fault() {
    let mut r = rig();
    let mut line = Vec::new();
    for c in COVER {
        let rep = survey(&mut r, c);
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

/// The program's memory with the branch at `b` detoured so that `way` traps.
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
    // The stub's branch goes to STUB+10 when taken; disp is from its own extension word.
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
fn every_branch_is_taken_both_ways_or_named() {
    let base = rig().memory_for(&Generator {
        base: None,
        case: Setup::random,
    });
    let mut wrong = Vec::new();
    for c in COVER {
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
                let rep = survey(&mut r, c);
                assert_eq!(rep.hangs, 0, "{} h0+${off:x} {way:?}: {rep:?}", c.name);
                let reached = rep.faults > 0;
                let named = c.unreachable.iter().any(|u| u.0 == off && u.1 == way);
                if reached == named {
                    wrong.push(format!(
                        "{} h0+${off:x} {way:?}: {} ({} of {CASES} states)",
                        c.name,
                        if named {
                            "named unreachable, but a state went this way"
                        } else {
                            "no state goes this way"
                        },
                        rep.faults
                    ));
                }
            }
        }
        for u in c.unreachable {
            assert!(
                seen.contains(&u.0),
                "{}: unreachable h0+${:x} is not a listed branch",
                c.name,
                u.0
            );
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

#[test]
fn no_generator_pokes_outside_what_its_routine_reads() {
    let r = rig();
    let pristine = r.memory_for(&Generator {
        base: None,
        case: Setup::random,
    });
    let map = h(3, COLLISION_MAP.0)..h(3, COLLISION_MAP.0) + COLLISION_MAP.1;
    for c in COVER {
        let g = generator(c.name);
        let mut allowed: Vec<std::ops::Range<u32>> = c
            .reads
            .iter()
            .map(|v| {
                let (a, n) = v.span();
                a..a + n
            })
            .collect();
        if c.map {
            allowed.push(map.clone());
        }
        let inside = |a: u32| allowed.iter().any(|r| r.contains(&a));
        // The base: what it changes over the program.
        let laid = r.memory_for(&g);
        assert_eq!(g.base.is_some(), c.map, "{}", c.name);
        for (a, (x, y)) in laid.iter().zip(pristine.iter()).enumerate() {
            if x != y {
                assert!(inside(a as u32), "{}: the base writes ${a:06x}", c.name);
            }
        }
        // Each case's pokes, and no hardware byte made to read anything.
        let mut rng = fd_conform::routine::Gen::new(1);
        for _ in 0..CASES {
            let s = (g.case)(&mut rng);
            assert!(s.hw_reads.is_empty(), "{}", c.name);
            for (a, bytes) in &s.pokes {
                for i in 0..bytes.len() as u32 {
                    assert!(inside(a + i), "{}: pokes ${:06x}", c.name, a + i);
                }
            }
        }
    }
}
