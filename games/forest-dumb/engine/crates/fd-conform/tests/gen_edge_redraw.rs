//! The edge redraws' generators (plans/forest-dumb-level-1 T56) on the original, from the owner's
//! disks: each runs 2 000 states with no hang, takes every conditional branch of its routine's unit
//! both ways (or only the way [`BRANCHES`] names, with why), and writes nothing its routine does not
//! read.
//!
//! Branches are measured by flipping, as `gen_drawing.rs` does (T37): each state is run as the
//! program is, with the branch made always taken and made never taken; equal to the always run it
//! took the branch at every pass, to the never run at none, to neither both ways, to both it never
//! got there. One difference: `h0_3ac2` waits on VHPOSR, which the oracle steps on every read and
//! never resets, so its loop count would differ between two runs of one state by where the counter
//! happened to be. Every run here first sets the counter to a value fixed by the state's index
//! ([`SYNC`]), so a state's three runs see the same raster.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use amiga_oracle::{AmigaMemory, CallError, Cpu68k, Gen, Regs};
use fd_conform::generators::{self, Generator, Setup, edge_redraw, h};
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

/// Blitter busy never reads set: the oracle's blitter finishes each blit as it starts, and a state
/// that set DMACONR bit 6 would wait for ever (a hang, not a path).
const BLIT_WAIT: Way = NotTaken("blitter wait: DMACONR bit 6 never reads set on the oracle");
/// h0_3ac2's two branches: the raster wait (`cmpi.b #$e7,$dff006; bpl`) and the blitter wait.
const WAIT: [(u32, Way); 2] = [(0x3aca, Both), (0x3ad6, BLIT_WAIT)];

/// Every conditional branch (`Bcc`, not `dbra`) in each routine's unit, by h0 offset, from the listing
/// decoded by flow (`tools/call_tree.py`'s decoder), 2026-11-05. A unit is the routine's own flow and
/// its callees less `split_at` (`re/translate.toml`): redraw_edge_a and _b keep `h0_774` and so
/// `h0_3ac2`; _c and _d split every callee off. `h0_7cc` and `h0_6852` have no branch. The `dbra`s
/// left out count the 22-tile and 14-tile edges, the 20 and 13 scanned cells, and h0_6882's sort.
const BRANCHES: &[(&str, &[(u32, Way)])] = &[
    ("h0_774", &WAIT),
    ("h0_7cc", &[]),
    (
        "redraw_edge_a",
        &[
            (0x84c, Both),
            (0x996, Both),
            (0x9ac, Both),
            (0x9b6, Both),
            (0x9be, Both),
            (0x9ce, Both),
            (0xaf4, Both),
            (0xb06, Both),
            (0xb16, Both),
            WAIT[0],
            WAIT[1],
        ],
    ),
    ("h0_a06", &WAIT),
    (
        "redraw_edge_b",
        &[
            (0xb28, Both),
            (0xc5c, Both),
            (0xc78, Both),
            (0xc8c, Both),
            (0xc96, Both),
            (0xc9e, Both),
            (0xcae, Both),
            (0xdca, Both),
            (0xddc, Both),
            (0xdec, Both),
            WAIT[0],
            WAIT[1],
        ],
    ),
    ("h0_ce4", &WAIT),
    (
        "redraw_edge_c",
        &[
            (0xdfe, Both),
            (0xfc2, Both),
            (0xfe4, Both),
            (0xff8, Both),
            (0x1002, Both),
            (0x100a, Both),
            (0x101a, Both),
            (0x1130, Both),
            (0x1140, Both),
        ],
    ),
    ("h0_104c", &WAIT),
    (
        "redraw_edge_d",
        &[
            (0x1324, Both),
            (0x1338, Both),
            (0x1342, Both),
            (0x134a, Both),
            (0x135a, Both),
            (0x147c, Both),
            (0x148c, Both),
        ],
    ),
    ("h0_1390", &WAIT),
    ("h0_3ac2", &WAIT),
    ("h0_681c", &[(0x6828, Both), (0x6846, Both)]),
    ("h0_6852", &[]),
    (
        "h0_6882",
        &[
            (0x6894, Both),
            (0x68a8, Both),
            (0x68ba, Both),
            (0x68ce, Both),
            (0x68e0, Both),
            (0x68f4, Both),
            (0x6906, Both),
            (0x691a, Both),
            (0x693a, Both),
            (0x6944, Both),
            (0x694e, Both),
            (0x696e, Both),
            (0x697a, Both),
        ],
    ),
];

/// `re/translate.toml`'s `gen_task = "T56"` entries as (name, hunk, offset), read by hand: fd-conform
/// takes no toml crate (its Cargo.toml).
fn t56_entries() -> Vec<(String, usize, u32)> {
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
        if field("gen_task").as_deref() != Some("T56") {
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
fn every_t56_entry_has_its_generator_and_its_lists() {
    let entries = t56_entries();
    assert_eq!(entries.len(), 14, "{entries:?}");
    let names: Vec<&str> = entries.iter().map(|e| e.0.as_str()).collect();
    for n in &names {
        assert!(
            edge_redraw::ALL.iter().any(|(r, _)| r == n),
            "{n}: not in edge_redraw::ALL"
        );
        gen_of(n);
        assert!(
            edge_redraw::READS.iter().any(|(r, _)| r == n),
            "{n}: no READS entry"
        );
        assert!(
            BRANCHES.iter().any(|(r, _)| r == n),
            "{n}: no BRANCHES entry"
        );
    }
    assert_eq!(edge_redraw::ALL.len(), names.len());
    assert_eq!(BRANCHES.len(), names.len());
}

#[test]
fn each_generator_runs_2000_states_on_the_original_with_no_hang() {
    let mut rig = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    for (name, hunk, off) in t56_entries() {
        let r = rig.survey(&name, CASES, h(hunk, off), &gen_of(&name));
        // The line docs/11-translation.md records.
        println!(
            "{name}: {} returned, {} paths, {} hangs, {} faults, {} unmodelled blits",
            r.passed, r.paths, r.hangs, r.faults, r.unmodelled_blits
        );
        assert_eq!(r.hangs, 0, "{name}: {r:?}");
        assert_eq!(r.faults, 0, "{name}: {r:?}");
        assert_eq!(r.unmodelled_blits, 0, "{name}: {r:?}");
        assert_eq!(r.passed, CASES, "{name}: {r:?}");
    }
}

#[test]
fn no_generator_writes_outside_what_its_routine_reads() {
    let rig = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let none = rig.memory_for(&Generator {
        base: None,
        case: Setup::random,
    });
    for (name, ranges) in edge_redraw::READS {
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

/// Where [`SYNC_CODE`] goes: above the program and the rig's probe, below its stack.
const SYNC: u32 = 0x70_0100;
/// Read VHPOSR until it reads $ff (the oracle's counter then stands at 0 for the next read), then
/// `d1 + 1` times more: `cmpi.b #$ff,$dff006; bne.b` back; `tst.b $dff006; dbra d1` back; `rts`.
const SYNC_CODE: [u8; 22] = [
    0x0c, 0x39, 0x00, 0xff, 0x00, 0xdf, 0xf0, 0x06, 0x66, 0xf6, 0x4a, 0x39, 0x00, 0xdf, 0xf0, 0x06,
    0x51, 0xc9, 0xff, 0xf8, 0x4e, 0x75,
];

/// Set the oracle's VHPOSR counter so its next read returns `v + 1`.
fn set_beam(cpu: &mut Cpu68k, v: u8) {
    let mut regs = Regs::default();
    regs.d[1] = v as u32;
    regs.a[7] = STACK;
    cpu.call(SYNC, regs, 4_000).expect("the beam sync returns");
}

/// What the next VHPOSR read returns.
fn beam(cpu: &mut Cpu68k) -> u8 {
    // `move.b $dff006,d0; rts` at SYNC + $40.
    cpu.write(
        SYNC + 0x40,
        &[0x10, 0x39, 0x00, 0xdf, 0xf0, 0x06, 0x4e, 0x75],
    );
    let regs = Regs {
        a: [0, 0, 0, 0, 0, 0, 0, STACK],
        ..Regs::default()
    };
    cpu.call(SYNC + 0x40, regs, 10).unwrap().regs.d[0] as u8
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

/// Per state, the run's [`outcome`] with the program as the cpu's memory holds it, the raster set by
/// the state's index first.
fn outcomes(cpu: &mut Cpu68k, entry: u32, states: &[Setup], max_instr: u64) -> Vec<(u64, u64)> {
    states
        .iter()
        .enumerate()
        .map(|(i, s)| {
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
            set_beam(cpu, (i as u32).wrapping_mul(37).wrapping_add(11) as u8);
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
/// over the displacement word for a `.w`. Either way one instruction, as the branch was.
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

/// A cpu over `g`'s memory, the sync routine laid in.
fn cpu_for(rig: &Rig, g: &Generator) -> Cpu68k {
    let mut cpu = Cpu68k::new(AmigaMemory {
        chip: vec![],
        slow: vec![],
    });
    cpu.memory_mut().copy_from_slice(&rig.memory_for(g));
    cpu.memory_mut()[SYNC as usize..SYNC as usize + SYNC_CODE.len()].copy_from_slice(&SYNC_CODE);
    cpu.set_hardware(true);
    cpu
}

/// On how many of `cases` states of `g` the routine at `entry` took each branch at `ats`, and on
/// how many it did not.
fn directions(
    rig: &Rig,
    name: &str,
    entry: u32,
    g: &Generator,
    cases: usize,
    ats: &[u32],
) -> Vec<(usize, usize)> {
    let mut cpu = cpu_for(rig, g);
    let mut rng = Gen::new(seed(name));
    let states: Vec<Setup> = (0..cases).map(|_| (g.case)(&mut rng)).collect();
    let base = outcomes(&mut cpu, entry, &states, fd_conform::routine::MAX_INSTR);
    // A flip that makes a wait loop for ever would run to MAX_INSTR on every state; any run twice
    // as long as the longest original one has already differed from it.
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

#[test]
fn the_beam_sync_sets_the_raster_and_runs_repeat() {
    let rig = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let g = gen_of("h0_3ac2");
    let mut cpu = cpu_for(&rig, &g);
    for v in [0u8, 0x10, 0x66, 0xe6, 0xfe, 0xff] {
        set_beam(&mut cpu, v);
        assert_eq!(beam(&mut cpu), v.wrapping_add(1));
    }
    // The same state twice, the raster set the same: the same run. Started inside the wait's
    // window ($e7-$66) it loops; started outside it does not, and the runs differ.
    let s = Setup::default();
    let entry = h(0, 0x3ac2);
    let at = |cpu: &mut Cpu68k, v: u8| {
        set_beam(cpu, v);
        outcome(cpu, entry, &s, 10_000)
    };
    assert_eq!(at(&mut cpu, 0x10), at(&mut cpu, 0x10));
    assert_ne!(at(&mut cpu, 0x10).1, at(&mut cpu, 0x80).1);
}

#[test]
fn each_branch_goes_both_ways_or_is_named() {
    let rig = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let entries = t56_entries();
    let mut wrong = Vec::new();
    for (name, branches) in BRANCHES {
        if branches.is_empty() {
            continue;
        }
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
