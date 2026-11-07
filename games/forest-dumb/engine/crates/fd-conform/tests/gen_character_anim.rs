//! The second `character` share's generators (plans/forest-dumb-level-1 T53) on the main program from
//! the owner's disks: each runs 2 000 states on the original with no hang or fault, takes every
//! conditional branch of its routine both ways (or the branch is named unreachable with why), and
//! changes nothing its routine does not read.
//!
//! As in `gen_character.rs`, a branch's ways are found by detouring it through a stub whose one end
//! traps (`illegal`) and whose other `jmp`s back to where the original would have gone. A `Bcc.w` in a
//! `bra.w`'s reach of the stub is detoured alone. Any other (`read_joystick`'s `Bcc.b`, and its
//! `Bcc.w`s, too far from free memory) is detoured with the instruction before it: the two become a
//! `jmp` to the stub, which runs a copy of that instruction, then the branch as a `Bcc.w`.

use std::collections::BTreeSet;

use fd_conform::generators::{
    Generator, Setup,
    character::Way,
    character_anim::{COVER, Cover, JOY1DAT, Var},
    h,
};
use fd_conform::routine::{Rig, RigReport};

const CASES: usize = 2_000;
/// Free memory below hunk 0 (based at `$c077d8`), within a `bra.w` of every branch up to `h0+$6800`.
const STUB: u32 = 0xc0_6000;
const NOP: u16 = 0x4e71;

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

/// A `Bcc` (not `bra`/`bsr`) with a byte or word displacement.
fn is_bcc(op: u16) -> bool {
    op & 0xf000 == 0x6000 && op & 0x0e00 != 0 && op & 0xff != 0xff
}

/// The `T53` routines of `re/translate.toml`, by name, read by hand: `fd-conform` has no toml parser
/// (Cargo.toml says why).
fn t53_routines() -> BTreeSet<String> {
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
        if field("gen_task").as_deref() == Some("T53") {
            out.insert(field("name").expect("a routine has a name"));
        }
    }
    out
}

#[test]
fn every_t53_routine_has_a_generator_and_a_cover_entry() {
    let want = t53_routines();
    assert_eq!(
        want.len(),
        6,
        "translate.toml's T53 share changed: {want:?}"
    );
    let registered: BTreeSet<String> = fd_conform::generators::character_anim::ALL
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

/// The program's memory with the branch at `b` detoured so that `way` traps. `lead` is the length of
/// the instruction before it to move along, or 0.
fn detoured(base: &[u8], b: u32, lead: u32, way: Way) -> Vec<u8> {
    let mut m = base.to_vec();
    let op = word(&m, b);
    let (cc, target, fall) = if op & 0xff == 0 {
        let disp = word(&m, b + 2) as i16 as i32 as u32;
        (op, (b + 2).wrapping_add(disp), b + 4)
    } else {
        let disp = op as u8 as i8 as i32 as u32;
        (op & 0xff00, (b + 2).wrapping_add(disp), b + 2)
    };
    let trap = [0x4afc, NOP, NOP];
    let jmp = |a: u32| [0x4ef9, (a >> 16) as u16, a as u16];
    let (not_taken, taken) = match way {
        Way::Taken => (jmp(fall), trap),
        Way::NotTaken => (trap, jmp(target)),
    };
    // The stub: the moved instruction, then the branch as a Bcc.w to its own +10.
    let start = b - lead;
    let moved = m[start as usize..b as usize].to_vec();
    m[STUB as usize..(STUB + lead) as usize].copy_from_slice(&moved);
    let at = STUB + lead;
    put(&mut m, at, &[cc, 8]);
    put(&mut m, at + 4, &not_taken);
    put(&mut m, at + 10, &taken);
    if lead == 0 {
        assert_eq!(
            op & 0xff,
            0,
            "h0 branch at ${b:x}: a Bcc.b needs its lead moved"
        );
        let to_stub = STUB.wrapping_sub(b + 2) as i32;
        assert!(
            (-0x8000..0x8000).contains(&to_stub),
            "${b:x} is out of a bra.w's reach of the stub: give it a lead"
        );
        put(&mut m, b, &[0x6000, to_stub as u16]);
    } else {
        let len = fall - start;
        assert!(
            len >= 6,
            "${b:x}: the lead and branch are {len} bytes, too few for a jmp"
        );
        put(&mut m, start, &jmp(STUB));
        let pad = vec![NOP; ((len - 6) / 2) as usize];
        put(&mut m, start + 6, &pad);
    }
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
        for &(off, lead) in c.branches {
            let b = h(0, off);
            assert!(
                is_bcc(word(&base, b)),
                "{} h0+${off:x}: ${:04x} is not a Bcc",
                c.name,
                word(&base, b)
            );
            assert!(seen.insert(off), "{} lists h0+${off:x} twice", c.name);
            for way in [Way::Taken, Way::NotTaken] {
                let mut r = Rig::from_memory(detoured(&base, b, lead, way));
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

/// A detoured run that does not trap runs as the original: the same report as the plain program.
/// Guards the moved-instruction detour, which the `Bcc.w`-alone one in `gen_character.rs` never needed.
#[test]
fn a_detour_that_does_not_trap_changes_nothing() {
    let base = rig().memory_for(&Generator {
        base: None,
        case: Setup::random,
    });
    let c = COVER.iter().find(|c| c.name == "read_joystick").unwrap();
    let plain = survey(&mut Rig::from_memory(base.to_vec()), c);
    for &(off, lead) in c.branches {
        for way in [Way::Taken, Way::NotTaken] {
            let rep = survey(
                &mut Rig::from_memory(detoured(&base, h(0, off), lead, way)),
                c,
            );
            // Each trapping state is a fault; every other state passes as on the plain program.
            assert_eq!(
                rep.passed + rep.faults,
                plain.passed,
                "h0+${off:x} {way:?}: {rep:?}"
            );
        }
    }
}

#[test]
fn no_generator_pokes_outside_what_its_routine_reads() {
    for c in COVER {
        let g = generator(c.name);
        assert!(g.base.is_none(), "{}", c.name);
        let span = |v: &Var| {
            let (a, n) = v.span();
            a..a + n
        };
        let mem: Vec<_> = c
            .reads
            .iter()
            .filter(|v| **v != Var::Joy1dat)
            .map(span)
            .collect();
        let joy = c.reads.contains(&Var::Joy1dat);
        let mut rng = fd_conform::routine::Gen::new(1);
        for _ in 0..CASES {
            let s = (g.case)(&mut rng);
            for (a, bytes) in &s.pokes {
                for i in 0..bytes.len() as u32 {
                    assert!(
                        mem.iter().any(|r| r.contains(&(a + i))),
                        "{}: pokes ${:06x}",
                        c.name,
                        a + i
                    );
                }
            }
            for &(a, _) in &s.hw_reads {
                assert!(
                    joy && (JOY1DAT..JOY1DAT + 2).contains(&a),
                    "{}: makes ${a:06x} read",
                    c.name
                );
            }
        }
    }
}
