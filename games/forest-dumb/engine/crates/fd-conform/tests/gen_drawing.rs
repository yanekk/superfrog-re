//! The `drawing` generators (plans/forest-dumb-level-1 T37) on the original, from the owner's disks:
//! each runs 2 000 states with no hang, takes every conditional branch of its routine's unit both
//! ways (or only the way [`BRANCHES`] names, with why), and writes nothing its routine does not read.
//!
//! The routine oracle has no instruction trace, so a branch's direction is measured by flipping it:
//! the program is run per state as it is, with the branch made always taken (its condition set to
//! `bra`) and with it made never taken (a `nop`, or a `bra.b` over a `.w`'s displacement). A state whose run equals the always
//! run took it at every pass; equal to the never run, at none; equal to neither, both ways within
//! the state (a loop); equal to both, it never reached the branch. "Equal" is the same instruction
//! count, bytes written and custom-chip writes, or the same failure.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use amiga_oracle::{AmigaMemory, CallError, Cpu68k, Gen, Regs};
use fd_conform::generators::{self, Generator, Setup, drawing, h};
use fd_conform::routine::{Rig, STACK, STACK_AREA};

const CASES: usize = 2000;

/// How a conditional branch may go on the routine oracle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Way {
    /// Both ways, on some state.
    Both,
    /// Never taken on any state, with why no state can take it.
    NotTaken(&'static str),
}

use Way::{Both, NotTaken};

/// Blitter busy never reads set: the oracle's blitter finishes each blit as it starts, and a
/// state that set DMACONR bit 6 would wait for ever (a hang, not a path).
const BLIT_WAIT: Way = NotTaken("blitter wait: DMACONR bit 6 never reads set on the oracle");

/// Every conditional branch (`Bcc`, not `dbra`) in each routine's unit, by h0 offset, from the
/// listing decoded by flow (`tools/call_tree.py`'s decoder). A unit is the routine's own flow and its
/// callees less `split_at` (`re/translate.toml`). The `dbra`s left out count fixed loops (four
/// sprites of four rows, five bitplanes, 32 colours, three raster lines) and go both ways on every
/// state that reaches them.
const BRANCHES: &[(&str, &[(u32, Way)])] = &[
    (
        "h0_21c0",
        &[
            (0x21c6, Both),
            (0x21d4, Both),
            (0x223a, BLIT_WAIT),
            (0x22aa, BLIT_WAIT),
            (0x22ec, Both),
            (0x2318, BLIT_WAIT),
            (0x235e, BLIT_WAIT),
            (0x2386, Both),
            (
                0x23aa,
                NotTaken(
                    "raster wait: VHPOSR steps by one on every read, so two reads never match",
                ),
            ),
            (0x23e2, Both),
            (0x3abc, BLIT_WAIT),
        ],
    ),
    ("h0_29ca", &[(0x2a1c, BLIT_WAIT)]),
    ("h0_2c7e", &[(0x2c96, Both), (0x2cde, Both)]),
    ("h0_2d4c", &[(0x2d8e, Both), (0x2dd0, Both)]),
    ("h0_3ab4", &[(0x3abc, BLIT_WAIT)]),
    (
        "char_sprite_position",
        &[(0x5474, Both), (0x549e, Both), (0x54b4, Both)],
    ),
    (
        "h0_5820",
        &[
            (0x5836, Both),
            (0x586c, Both),
            (0x5880, Both),
            (0x58e6, Both),
            (0x5910, BLIT_WAIT),
            (0x5970, Both),
            (0x5984, Both),
            (0x59ec, Both),
            (0x5a16, BLIT_WAIT),
            (0x5a78, Both),
            (0x5a8c, Both),
            (0x5aee, BLIT_WAIT),
        ],
    ),
    (
        "h0_5e44",
        &[
            (0x5ea6, BLIT_WAIT),
            (0x5f16, BLIT_WAIT),
            (0x5f5c, BLIT_WAIT),
            (0x5f8a, Both),
            (0x5fb2, Both),
            (0x5fde, BLIT_WAIT),
            (0x6024, BLIT_WAIT),
            (0x606a, BLIT_WAIT),
            (0x609a, Both),
            (0x60a2, Both),
            (0x60aa, Both),
            (0x60ba, Both),
            (0x60e6, Both),
            (0x610a, Both),
            (0x6136, BLIT_WAIT),
            (0x617c, BLIT_WAIT),
            (0x61c2, BLIT_WAIT),
        ],
    ),
    (
        "h0_62b6",
        &[
            (0x62cc, Both),
            (0x62f2, Both),
            (0x62fe, Both),
            (0x6336, Both),
            (0x6356, Both),
            (0x6360, Both),
            (0x636a, Both),
            (0x6374, Both),
            (0x6394, Both),
            (0x63ac, Both),
            (0x63bc, Both),
            (0x63d8, Both),
            (0x64f0, Both),
            (0x64fa, Both),
            (0x6502, Both),
            (0x666e, Both),
            (0x6752, BLIT_WAIT),
        ],
    ),
    (
        "h0_94ba",
        &[
            (0x94c0, Both),
            (0x94ec, Both),
            (0x955e, Both),
            (0x956e, Both),
            (0x9588, Both),
            (0x9596, Both),
            (0x95b0, Both),
            (0x95c0, Both),
        ],
    ),
    (
        "h0_94fe",
        &[
            (0x955e, Both),
            (0x956e, Both),
            (0x9588, Both),
            (0x9596, Both),
            (0x95b0, Both),
            (0x95c0, Both),
        ],
    ),
    (
        "h0_9550",
        &[
            (0x955e, Both),
            (0x956e, Both),
            (0x9588, Both),
            (0x9596, Both),
            (0x95b0, Both),
            (0x95c0, Both),
        ],
    ),
    ("h0_9754", &[(0x9818, Both)]),
    (
        "char_sprite_copy",
        &[(0x9b18, Both), (0x9c1c, Both), (0x9c24, Both)],
    ),
];

/// `re/translate.toml`'s `drawing` entries as (name, hunk, offset), read by hand: fd-conform takes
/// no toml crate (its Cargo.toml).
fn drawing_entries() -> Vec<(String, usize, u32)> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../re/translate.toml");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut out = Vec::new();
    for block in text.split("[[routine]]").skip(1) {
        let field = |k: &str| {
            block.lines().find_map(|l| {
                let (key, v) = l.split_once('=')?;
                (key.trim() == k).then(|| v.trim().trim_matches('"').to_string())
            })
        };
        if field("subsystem").as_deref() != Some("drawing") {
            continue;
        }
        let off = field("offset").unwrap();
        let off = u32::from_str_radix(off.trim_start_matches("0x"), 16).unwrap();
        out.push((
            field("name").unwrap(),
            field("hunk").unwrap().parse().unwrap(),
            off,
        ));
    }
    out
}

/// The rig's seed for `name` (`routine.rs`), so these are the states the survey and the judge run.
fn seed(name: &str) -> u64 {
    name.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
        (h ^ b as u64).wrapping_mul(0x100_0000_01b3)
    })
}

fn gen_of(name: &str) -> Generator {
    generators::generator(name).unwrap_or_else(|| panic!("no generator `{name}`"))
}

#[test]
fn every_drawing_entry_has_its_generator_and_its_lists() {
    let entries = drawing_entries();
    assert_eq!(entries.len(), 14, "{entries:?}");
    let names: Vec<&str> = entries.iter().map(|e| e.0.as_str()).collect();
    for n in &names {
        gen_of(n);
        assert!(
            drawing::READS.iter().any(|(r, _)| r == n),
            "{n}: no READS entry"
        );
        assert!(
            BRANCHES.iter().any(|(r, _)| r == n),
            "{n}: no BRANCHES entry"
        );
    }
    let mine: Vec<&str> = drawing::ALL.iter().map(|(n, _)| *n).collect();
    assert_eq!(mine.len(), names.len());
    assert!(mine.iter().all(|n| names.contains(n)), "{mine:?}");
}

#[test]
fn each_generator_runs_2000_states_on_the_original_with_no_hang() {
    let mut rig = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    for (name, hunk, off) in drawing_entries() {
        let r = rig.survey(&name, CASES, h(hunk, off), &gen_of(&name));
        // The line docs/11-translation.md records.
        println!(
            "{name}: {} returned, {} paths, {} hangs, {} faults, {} unmodelled blits",
            r.passed, r.paths, r.hangs, r.faults, r.unmodelled_blits
        );
        assert_eq!(r.hangs, 0, "{name}: {r:?}");
        assert_eq!(r.unmodelled_blits, 0, "{name}: {r:?}");
        assert!(
            r.passed >= CASES * 9 / 10,
            "{name} returned on too few states: {r:?}"
        );
    }
}

#[test]
fn no_generator_writes_outside_what_its_routine_reads() {
    let rig = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let none = rig.memory_for(&Generator {
        base: None,
        case: Setup::random,
    });
    for (name, ranges) in drawing::READS {
        let inside = |a: u32| STACK_AREA.contains(&a) || ranges.iter().any(|r| r.contains(&a));
        let g = gen_of(name);
        let mem = rig.memory_for(&g);
        let changed = (0..mem.len()).find(|&a| mem[a] != none[a] && !inside(a as u32));
        assert_eq!(
            changed,
            None,
            "{name}: its base writes ${:06x}",
            changed.unwrap_or(0)
        );
        let mut rng = Gen::new(seed(name));
        for case in 0..CASES {
            let s = (g.case)(&mut rng);
            for (a, b) in &s.pokes {
                for i in 0..b.len() as u32 {
                    assert!(inside(a + i), "{name} case {case}: pokes ${:06x}", a + i);
                }
            }
            for &(a, _) in &s.hw_reads {
                assert!(inside(a), "{name} case {case}: sets hardware byte ${a:06x}");
            }
        }
    }
}

/// One run's result, hashed (instruction count, bytes written, custom-chip writes, or the failure),
/// and the instructions it ran.
fn outcome(cpu: &mut Cpu68k, entry: u32, s: &Setup, max_instr: u64) -> (u64, u64) {
    let mut regs = Regs {
        d: s.d,
        a: s.a,
        ..Regs::default()
    };
    regs.a[7] = STACK;
    let mut hs = DefaultHasher::new();
    let mut ran = max_instr;
    match cpu.call(entry, regs, max_instr) {
        Ok(r) => {
            ran = r.instr;
            r.instr.hash(&mut hs);
            r.writes
                .iter()
                .filter(|w| !STACK_AREA.contains(&w.0))
                .for_each(|w| w.hash(&mut hs));
            r.hw_writes.hash(&mut hs);
        }
        Err(CallError::Timeout) => "hang".hash(&mut hs),
        Err(e) => format!("{e:?}").hash(&mut hs),
    }
    (hs.finish(), ran)
}

/// Per state, the run's [`outcome`] with the program as the cpu's memory holds it.
fn outcomes(cpu: &mut Cpu68k, entry: u32, states: &[Setup], max_instr: u64) -> Vec<(u64, u64)> {
    states
        .iter()
        .map(|s| {
            let saved: Vec<(u32, Vec<u8>)> = s
                .pokes
                .iter()
                .map(|(a, b)| (*a, cpu.read(*a, b.len()).to_vec()))
                .collect();
            for (a, b) in &s.pokes {
                cpu.write(*a, b);
            }
            for &(a, v) in &s.hw_reads {
                cpu.set_hardware_read(a, v);
            }
            let o = outcome(cpu, entry, s, max_instr);
            for (a, b) in saved.iter().rev() {
                cpu.write(*a, b);
            }
            for &(a, _) in &s.hw_reads {
                cpu.set_hardware_read(a, 0);
            }
            o
        })
        .collect()
}

/// The `Bcc` at `a`: its first word made `bra` (always), and never: a `nop` for a `.b`, a `bra.b`
/// over the displacement word for a `.w`. Either way one instruction, as the branch was, so a
/// never-taken branch runs the same instruction count.
fn flips(cpu: &Cpu68k, a: u32) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let w = u16::from_be_bytes(cpu.read(a, 2).try_into().unwrap());
    let cond = (w >> 8) & 0xf;
    assert!(w >> 12 == 6 && cond >= 2, "${a:06x}: ${w:04x} is not a Bcc");
    let len = if w & 0xff == 0 { 4 } else { 2 };
    let orig = cpu.read(a, len).to_vec();
    let mut always = orig.clone();
    always[0] = 0x60;
    let never = if len == 2 {
        vec![0x4e, 0x71]
    } else {
        vec![0x60, 0x02, orig[2], orig[3]]
    };
    (orig, always, never)
}

/// On how many of `cases` states of `g` the routine at `entry` took each branch at `ats`, and on how
/// many it did not.
fn directions(
    rig: &Rig,
    name: &str,
    entry: u32,
    g: &Generator,
    cases: usize,
    ats: &[u32],
) -> Vec<(usize, usize)> {
    let mut cpu = Cpu68k::new(AmigaMemory {
        chip: vec![],
        slow: vec![],
    });
    cpu.memory_mut().copy_from_slice(&rig.memory_for(g));
    cpu.set_hardware(true);
    let mut rng = Gen::new(seed(name));
    let states: Vec<Setup> = (0..cases).map(|_| (g.case)(&mut rng)).collect();
    let base = outcomes(&mut cpu, entry, &states, fd_conform::routine::MAX_INSTR);
    // A flip that makes a wait loop for ever would run to MAX_INSTR on every state; any run
    // twice as long as the longest original one has already differed from it.
    let limit = 2 * base.iter().map(|o| o.1).max().unwrap() + 100;
    ats.iter()
        .map(|&a| {
            let (orig, always, never) = flips(&cpu, a);
            cpu.write(a, &always);
            let ta = outcomes(&mut cpu, entry, &states, limit);
            cpu.write(a, &never);
            let tn = outcomes(&mut cpu, entry, &states, limit);
            cpu.write(a, &orig);
            let (mut taken, mut not) = (0, 0);
            for i in 0..cases {
                match (base[i].0 == ta[i].0, base[i].0 == tn[i].0) {
                    (true, false) => taken += 1,
                    (false, true) => not += 1,
                    (false, false) => {
                        taken += 1;
                        not += 1;
                    }
                    (true, true) => {}
                }
            }
            (taken, not)
        })
        .collect()
}

/// h0_9754 with camera_y 0: `(0 ^ $ff) + $2f` is never below $f0, so its `bmi` is never taken,
/// and with camera_y $80 always.
fn camera_y_at(y: u16) -> Setup {
    let mut s = Setup::default();
    s.w(h(0, 0x1e64), y);
    s
}

#[test]
fn the_flips_see_a_branch_that_goes_one_way() {
    let rig = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let at = [h(0, 0x9818)];
    let entry = h(0, 0x9754);
    let top = Generator {
        base: None,
        case: |_| camera_y_at(0),
    };
    assert_eq!(directions(&rig, "top", entry, &top, 20, &at), [(0, 20)]);
    let low = Generator {
        base: None,
        case: |_| camera_y_at(0x80),
    };
    assert_eq!(directions(&rig, "low", entry, &low, 20, &at), [(20, 0)]);
    // A branch the routine never reaches is neither.
    let wait = [h(0, 0x3abc)];
    assert_eq!(directions(&rig, "top", entry, &top, 20, &wait), [(0, 0)]);
    // The drawing generator takes it both ways.
    let both = directions(&rig, "h0_9754", entry, &gen_of("h0_9754"), 200, &at);
    assert!(both[0].0 > 0 && both[0].1 > 0, "{both:?}");
}

#[test]
fn each_branch_goes_both_ways_or_is_named() {
    let rig = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let entries = drawing_entries();
    let mut wrong = Vec::new();
    for (name, branches) in BRANCHES {
        let &(_, hunk, off) = entries.iter().find(|e| e.0 == *name).unwrap();
        let ats: Vec<u32> = branches.iter().map(|b| h(0, b.0)).collect();
        let seen = directions(&rig, name, h(hunk, off), &gen_of(name), CASES, &ats);
        for (&(at, way), (taken, not)) in branches.iter().zip(seen) {
            // The lines docs/11-translation.md records.
            println!("{name} h0+${at:x}: taken on {taken} states, not taken on {not}");
            let ok = match way {
                Both => taken > 0 && not > 0,
                NotTaken(_) => taken == 0 && not > 0,
            };
            if !ok {
                wrong.push(format!(
                    "{name} h0+${at:x} ({way:?}): taken {taken}, not {not}"
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
