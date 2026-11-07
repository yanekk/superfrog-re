//! T08's engine ports against the original on the rig: `char_update` and the thirteen routines
//! under it (`fd_core::character::Character`), 10 000 states each from T35's generators, registers,
//! memory and custom-chip writes compared (plans/forest-dumb-level-1 DESIGN §4;
//! `toolkit/docs/porting-routines.md` §3).
//!
//! Each adapter reads the level state the character routines use from the case's memory, runs the
//! engine with the case's registers and beam in its [`Scratch`], and gives back every character
//! variable's bytes, the scratch registers and the custom-chip writes. Tables come from the rig's
//! own memory through [`Program::memory`], so a counter a state sets to any long reads where the
//! original reads.
//!
//! Mutants, each a plausible misreading of the listing, must fail.

use std::sync::Arc;

use amiga_oracle::Regs;
use fd_conform::generators::{self, Generator, Setup, h};
use fd_conform::routine::{Effects, Rig, RigReport};
use fd_core::character::{AnimTables, Character};
use fd_core::level::collision::CollisionPlane;
use fd_core::{LAYOUT, PORTED, Program, Scratch, State, layout_var};

const CASES: usize = 10_000;

fn rig() -> Rig {
    Rig::new().unwrap_or_else(|e| panic!("{e}"))
}

fn generator(name: &str) -> Generator {
    generators::generator(name).unwrap_or_else(|| panic!("{name} is not registered"))
}

/// The LAYOUT variables the character routines read or write.
fn vars() -> Vec<&'static str> {
    let mut v = vec![
        "char_x",
        "char_y",
        "char_walk_t",
        "char_facing",
        "char_probe_1",
        "char_probe_2",
        "char_anim_ptr",
        "joy_left",
        "joy_right",
        "joy_up",
        "joy_down",
    ];
    v.extend(
        LAYOUT
            .iter()
            .map(|x| x.name)
            .skip_while(|n| *n != "char_jump_3b28"),
    );
    v
}

fn at(name: &str) -> (u32, usize) {
    let v = layout_var(name).unwrap_or_else(|| panic!("{name} is not in LAYOUT"));
    (h(v.hunk as usize, v.offset), v.width.bytes())
}

/// The case's state: each variable read from memory under the pokes.
fn state(mem: &[u8], s: &Setup) -> State {
    let mut st = State::level_1();
    for name in vars() {
        let (a, n) = at(name);
        let v = (0..n as u32).fold(0, |acc, i| acc << 8 | s.byte(mem, a + i) as u32);
        assert!(st.set(name, v), "{name}");
    }
    st
}

/// What the engine leaves: every variable's bytes, the registers, the custom-chip writes.
fn effects(st: &State, r: Scratch) -> Effects {
    let mut writes = Vec::new();
    for name in vars() {
        let (a, n) = at(name);
        let v = st.get(name).unwrap();
        for i in 0..n {
            writes.push((a + i as u32, (v >> (8 * (n - 1 - i))) as u8));
        }
    }
    Effects {
        regs: Regs {
            d: r.d,
            a: r.a,
            ..Regs::default()
        },
        writes,
        hw_writes: r.hw,
    }
}

type Run = fn(&mut Character);

/// A routine as a port: the engine's method `f` over the case.
fn port(mem: Arc<Vec<u8>>, f: Run) -> impl FnMut(&Setup) -> Effects {
    let program = Program::memory(mem.clone());
    let anim = AnimTables::from_hunk0(program.bytes()).unwrap_or_else(|e| panic!("{e}"));
    let plane = CollisionPlane::memory(mem.clone(), &program);
    move |s| {
        let mut st = state(&mem, s);
        let mut r = Scratch {
            d: s.d,
            a: s.a,
            hw: Vec::new(),
            beam: s.beam,
        };
        let mut c = Character {
            state: &mut st,
            program: &program,
            anim: &anim,
            plane: &plane,
            scratch: &mut r,
        };
        f(&mut c);
        effects(&st, r)
    }
}

/// Each T08 routine on the rig: name, entry, the engine's method.
const ROUTINES: &[(&str, u32, Run)] = &[
    ("char_update", 0x3e0c, |c| c.update()),
    ("char_step", 0x3e30, |c| c.step()),
    ("h0_3ea8", 0x3ea8, |c| c.idle()),
    ("h0_410c", 0x410c, |c| c.land_or_fall()),
    ("h0_416c", 0x416c, |c| c.fall_step()),
    ("h0_4274", 0x4274, |c| c.rise_step()),
    ("h0_4696", 0x4696, |c| c.move_x()),
    ("h0_4718", 0x4718, |c| c.left_pressed()),
    ("h0_47e2", 0x47e2, |c| {
        c.run_left_step();
    }),
    ("h0_48be", 0x48be, |c| c.right_pressed()),
    ("h0_497c", 0x497c, |c| {
        c.run_right_step();
    }),
    ("h0_4b0e", 0x4b0e, |c| c.up_pressed()),
    ("char_probe", 0x4f84, |c| c.probe()),
    ("char_probe_flags", 0x4f90, |c| c.probe_flags()),
];

/// The T08 routines of `re/translate.toml`, by name; `game_frame` is T63's (owner, 2026-11-05).
fn t08_routines() -> Vec<String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../re/translate.toml");
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let mut out = Vec::new();
    for entry in text.split("[[routine]]").skip(1) {
        let field = |k: &str| {
            entry.lines().find_map(|l| {
                let (key, v) = l.split_once('=')?;
                (key.trim() == k).then(|| v.trim().trim_matches('"').to_string())
            })
        };
        let name = field("name").expect("a routine has a name");
        if field("port_task").as_deref() == Some("T08") && name != "game_frame" {
            out.push(name);
        }
    }
    out.sort();
    out
}

#[test]
fn every_t08_routine_is_registered_and_passes_the_rig() {
    let mut names: Vec<String> = ROUTINES.iter().map(|p| p.0.to_string()).collect();
    names.sort();
    assert_eq!(names, t08_routines(), "translate.toml's T08 share changed");
    assert!(
        PORTED.iter().all(|p| p.name != "game_frame"),
        "game_frame is registered by T63 once its callees are ported"
    );

    let mut r = rig();
    let mut line = Vec::new();
    for &(name, entry, f) in ROUTINES {
        let p = PORTED
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} has no PORTED entry"));
        assert_eq!((p.hunk, p.offset), (0, entry), "{name}");
        let g = generator(name);
        let mem = r.memory_for(&g);
        let rep: RigReport = r
            .check(name, CASES, h(0, entry), &g, port(mem, f))
            .unwrap_or_else(|f| panic!("{f}"));
        assert_eq!(
            (rep.passed, rep.faults, rep.unmodelled_blits),
            (CASES - rep.hangs, 0, 0),
            "{name}: {rep:?}"
        );
        assert!(rep.passed * 10 >= CASES * 9, "{name}: {rep:?}");
        line.push(format!("{name} {} ({} hangs)", rep.paths, rep.hangs));
    }
    eprintln!("paths: {}", line.join(", "));
}

// ---------------------------------------------------------------------------------------------
// Mutants: each must fail on the rig.
// ---------------------------------------------------------------------------------------------

fn fails(name: &str, entry: u32, f: Run) -> String {
    let mut r = rig();
    let g = generator(name);
    let mem = r.memory_for(&g);
    r.check(name, CASES, h(0, entry), &g, port(mem, f))
        .map(|rep| panic!("{name}: a mutant passed {rep:?}"))
        .unwrap_err()
        .to_string()
}

#[test]
fn the_rig_catches_a_probe_point_a_line_off() {
    // The low ground point read at y + $2b, not $2c.
    let f = fails("char_probe", 0x4f84, |c| {
        c.probe();
        let x = c.state.char_x.wrapping_sub(0x10).wrapping_add(0x18);
        let w = c.plane.window(x, c.state.char_y.wrapping_add(0x2b));
        c.state.motion.samples[15] = if w >> 13 & 1 != 0 { 0xff } else { 0 };
    });
    assert!(f.contains("h0+$54f") || f.contains("char_samples"), "{f}");
}

#[test]
fn the_rig_catches_the_right_slide_reading_its_own_fast_table() {
    // Sliding right after a run of $a0 reads h0+$40cb, as the code before the overwrite says.
    let f = fails("h0_3ea8", 0x3ea8, |c| {
        let fast_right = c.state.motion.run_right.wrapping_sub(0xa0) & 0x8000_0000 == 0;
        let before = (c.state.char_x, c.state.motion.slide_t);
        c.idle();
        if fast_right && c.state.anim.ptr == c.anim.long(0x4f00) && c.state.motion.slide_t != 0 {
            let t = before.1 as u32;
            let v = c.program.byte(0x40cb + t);
            if v != 0xff {
                c.state.char_x = before.0.wrapping_add(v as u32);
            }
        }
    });
    assert!(f.contains("char_x") || f.contains("h0+$1e5"), "{f}");
}

#[test]
fn the_rig_catches_an_unsigned_run_compare() {
    // `cmpi.l #$30; bpl` read as unsigned: a counter past $8000_0030 would slide.
    let f = fails("h0_3ea8", 0x3ea8, |c| {
        if c.state.motion.run_left >= 0x8000_0030 {
            c.state.motion.run_left = 0x30;
        }
        c.idle();
    });
    assert!(!f.is_empty());
}

#[test]
fn the_rig_catches_the_run_step_left_out_of_d0() {
    let f = fails("h0_47e2", 0x47e2, |c| {
        c.run_left_step();
        c.scratch.d[0] = 0;
    });
    assert!(f.contains("d0:"), "{f}");
}

#[test]
fn the_rig_catches_a_missing_slide_sound() {
    let f = fails("h0_3ea8", 0x3ea8, |c| {
        c.idle();
        c.scratch.hw.clear();
    });
    assert!(!f.is_empty());
}
