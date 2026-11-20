//! T51's engine ports against the original on the rig: the five animation frame pickers
//! (`fd_core::character::Anim`) and `read_joystick` (`fd_core::character::JoyFlags`), 10 000 states
//! each from T53's generators, registers, memory and custom-chip writes compared
//! (plans/forest-dumb-level-1 DESIGN §4; `toolkit/docs/porting-routines.md` §3).
//!
//! The engine keeps typed state, not registers. Each adapter below reads the state from the case's
//! memory, runs the engine, and gives back what the original leaves: the variables' bytes and the
//! registers the routine changes, computed from the engine's results. A register the engine got
//! wrong therefore shows as a wrong value, as it would to a caller of the original.
//!
//! Mutants, each a plausible misreading of the listing, must fail: the generators reach the
//! states that tell them apart.

use std::sync::Arc;

use amiga_oracle::Regs;
use fd_conform::generators::{self, Generator, Setup, h};
use fd_conform::routine::{Effects, Rig, RigReport};
use fd_core::character::anim::{WALK_LEFT, WALK_RIGHT};
use fd_core::character::input::LOG_FULL;
use fd_core::character::{Anim, AnimTables, InputLog, JoyFlags};
use fd_core::{PORTED, layout_var};

const CASES: usize = 10_000;
/// JOY1DAT, a hardware read.
const JOY1DAT: u32 = 0xdf_f00c;
/// `input_log`, `h0+$34c28`.
const INPUT_LOG: u32 = 0x3_4c28;

fn rig() -> Rig {
    Rig::new().unwrap_or_else(|e| panic!("{e}"))
}

fn generator(name: &str) -> Generator {
    generators::generator(name).unwrap_or_else(|| panic!("{name} is not registered"))
}

/// Where the LAYOUT variable `name` lives on the oracle, and its width in bytes.
fn at(name: &str) -> (u32, usize) {
    let v = layout_var(name).unwrap_or_else(|| panic!("{name} is not in LAYOUT"));
    (h(v.hunk as usize, v.offset), v.width.bytes())
}

/// The value of LAYOUT variable `name` in the case's memory.
fn read(mem: &[u8], s: &Setup, name: &str) -> u32 {
    let (a, n) = at(name);
    (0..n as u32).fold(0, |acc, i| acc << 8 | s.byte(mem, a + i) as u32)
}

/// `name`'s bytes holding `value`, as writes.
fn write(out: &mut Vec<(u32, u8)>, name: &str, value: u32) {
    let (a, n) = at(name);
    for i in 0..n {
        out.push((a + i as u32, (value >> (8 * (n - 1 - i))) as u8));
    }
}

/// The animation tables as the rig loaded the program: hunk 0 at its oracle base, relocated.
fn tables(mem: &[u8]) -> AnimTables {
    AnimTables::from_hunk0(&mem[h(0, 0) as usize..]).unwrap_or_else(|e| panic!("{e}"))
}

fn anim(mem: &[u8], s: &Setup) -> Anim {
    Anim {
        walk_t: read(mem, s, "char_walk_t") as u8,
        facing: read(mem, s, "char_facing") as u8,
        probe_1: read(mem, s, "char_probe_1") as u8,
        probe_2: read(mem, s, "char_probe_2") as u8,
        ptr: read(mem, s, "char_anim_ptr"),
    }
}

fn anim_writes(a: &Anim) -> Vec<(u32, u8)> {
    let mut w = Vec::new();
    write(&mut w, "char_walk_t", a.walk_t.into());
    write(&mut w, "char_facing", a.facing.into());
    write(&mut w, "char_probe_1", a.probe_1.into());
    write(&mut w, "char_probe_2", a.probe_2.into());
    write(&mut w, "char_anim_ptr", a.ptr);
    w
}

/// What a walk step leaves in registers: `d0` the table offset of the frame, `a0` its address.
fn walk_regs(s: &Setup, a: &Anim, table: u32) -> Regs {
    let mut r = Regs {
        d: s.d,
        a: s.a,
        ..Regs::default()
    };
    r.d[0] = 4 * (a.walk_t >> 3) as u32;
    r.a[0] = h(0, table).wrapping_add(r.d[0]);
    r
}

/// One picker as a port: the engine's step `f`, the table it walks (`Some`) or none.
fn picker(
    mem: Arc<Vec<u8>>,
    f: fn(&mut Anim, &AnimTables),
    walk: Option<u32>,
) -> impl FnMut(&Setup) -> Effects {
    let t = tables(&mem);
    move |s| {
        let mut a = anim(&mem, s);
        f(&mut a, &t);
        let regs = match walk {
            Some(table) => walk_regs(s, &a, table),
            None => Regs {
                d: s.d,
                a: s.a,
                ..Regs::default()
            },
        };
        Effects {
            regs,
            writes: anim_writes(&a),
            hw_writes: Vec::new(),
        }
    }
}

/// `read_joystick` as a port, the engine's read given as `f`.
fn joystick(
    mem: Arc<Vec<u8>>,
    f: fn(&mut JoyFlags, &mut InputLog, u16),
) -> impl FnMut(&Setup) -> Effects {
    move |s| {
        let hw = |a: u32| {
            s.hw_reads
                .iter()
                .rev()
                .find(|r| r.0 == a)
                .map_or(0, |r| r.1)
        };
        let dat = u16::from_be_bytes([hw(JOY1DAT), hw(JOY1DAT + 1)]);
        let mut joy = JoyFlags {
            left: read(&mem, s, "joy_left") as u16,
            right: read(&mem, s, "joy_right") as u16,
            up: read(&mem, s, "joy_up") as u16,
            down: read(&mem, s, "joy_down") as u16,
        };
        let index = read(&mem, s, "input_log_index");
        let mut log = InputLog::at(index);
        f(&mut joy, &mut log, dat);

        let mut writes = Vec::new();
        write(&mut writes, "joy_left", joy.left.into());
        write(&mut writes, "joy_right", joy.right.into());
        write(&mut writes, "joy_up", joy.up.into());
        write(&mut writes, "joy_down", joy.down.into());
        write(&mut writes, "input_log_index", log.index);
        let mut regs = Regs {
            d: s.d,
            a: s.a,
            ..Regs::default()
        };
        // `clr.l d0` before the log test, `move.l input_log_index, d1`.
        regs.d[0] = 0;
        regs.d[1] = index;
        if index != LOG_FULL {
            let byte = log.byte(index);
            writes.push((h(0, INPUT_LOG).wrapping_add(index), byte));
            regs.d[0] = byte.into();
            regs.a[0] = h(0, INPUT_LOG);
        }
        Effects {
            regs,
            writes,
            hw_writes: Vec::new(),
        }
    }
}

/// Each T51 routine: its name, entry, and the engine's port as the rig runs it.
type Port = Box<dyn FnMut(&Setup) -> Effects>;

fn ports(mem: &dyn Fn(&str) -> Arc<Vec<u8>>) -> Vec<(&'static str, u32, Port)> {
    vec![
        (
            "char_step_4be6",
            0x4be6,
            Box::new(picker(
                mem("char_step_4be6"),
                Anim::walk_left,
                Some(WALK_LEFT),
            )) as Port,
        ),
        (
            "h0_4c28",
            0x4c28,
            Box::new(picker(mem("h0_4c28"), Anim::walk_right, Some(WALK_RIGHT))),
        ),
        (
            "h0_4c6a",
            0x4c6a,
            Box::new(picker(mem("h0_4c6a"), Anim::frame_from_probes, None)),
        ),
        (
            "h0_4d32",
            0x4d32,
            Box::new(picker(mem("h0_4d32"), Anim::frame_4d32, None)),
        ),
        (
            "h0_4d54",
            0x4d54,
            Box::new(picker(mem("h0_4d54"), Anim::frame_4d54, None)),
        ),
        (
            "read_joystick",
            0x8dfc,
            Box::new(joystick(mem("read_joystick"), JoyFlags::read_joy1dat)),
        ),
    ]
}

/// The T51 routines of `re/translate.toml`, by name.
fn t51_routines() -> Vec<String> {
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
        if field("port_task").as_deref() == Some("T51") {
            out.push(field("name").expect("a routine has a name"));
        }
    }
    out.sort();
    out
}

#[test]
fn every_t51_routine_is_registered_and_passes_the_rig() {
    let mut r = rig();
    let mems = |name: &str| r.memory_for(&generator(name));
    let all = ports(&mems);
    let mut names: Vec<String> = all.iter().map(|p| p.0.to_string()).collect();
    names.sort();
    assert_eq!(names, t51_routines(), "translate.toml's T51 share changed");

    let mut line = Vec::new();
    for (name, entry, mut port) in all {
        let p = PORTED
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} has no PORTED entry"));
        assert_eq!((p.hunk, p.offset), (0, entry), "{name}");
        let rep: RigReport = r
            .check(name, CASES, h(0, entry), &generator(name), &mut port)
            .unwrap_or_else(|f| panic!("{f}"));
        assert_eq!(
            (rep.passed, rep.hangs, rep.faults, rep.unmodelled_blits),
            (CASES, 0, 0, 0),
            "{name}: {rep:?}"
        );
        line.push(format!("{name} {}", rep.paths));
    }
    eprintln!("paths: {}", line.join(", "));
}

// ---------------------------------------------------------------------------------------------
// Mutants: each must fail on the rig.
// ---------------------------------------------------------------------------------------------

/// The walk counter wrapped as "below $28" unsigned: wrong for $a8..$ff, which `bmi` keeps.
fn walk_unsigned(a: &mut Anim, t: &AnimTables) {
    let keep = a.walk_t;
    a.walk_left(t);
    if keep.wrapping_add(1) >= 0x28 {
        a.walk_t = 0;
        a.ptr = t.long(WALK_LEFT);
    }
}

/// Facing left tested in the right-facing order (`a` before `b`).
fn probes_one_order(a: &mut Anim, t: &AnimTables) {
    let left = a.facing & 0x80 != 0;
    let (p1, p2) = (a.probe_1, a.probe_2);
    if left && p1 != 1 && p2 & 3 == 3 {
        a.ptr = t.long(0x4f60);
    } else {
        a.frame_from_probes(t);
    }
}

/// Up read from bit 8 alone, as for a mouse, not bit 9 xor bit 8.
fn up_from_bit_8(j: &mut JoyFlags, log: &mut InputLog, dat: u16) {
    j.read_joy1dat(log, dat);
    j.up = (dat >> 8) & 1;
}

fn fails(name: &str, entry: u32, port: impl FnMut(&Setup) -> Effects) -> String {
    let mut r = rig();
    r.check(name, CASES, h(0, entry), &generator(name), port)
        .map(|rep| panic!("{name}: a mutant passed {rep:?}"))
        .unwrap_err()
        .to_string()
}

#[test]
fn the_rig_catches_an_unsigned_walk_compare() {
    let mem = rig().memory_for(&generator("char_step_4be6"));
    let f = fails(
        "char_step_4be6",
        0x4be6,
        picker(mem, walk_unsigned, Some(WALK_LEFT)),
    );
    assert!(f.contains("char_walk_t") || f.contains("h0+$4be4"), "{f}");
}

#[test]
fn the_rig_catches_a_wrong_walk_register() {
    let mem = rig().memory_for(&generator("h0_4c28"));
    // The frame and memory right, a0 from the other table.
    let f = fails(
        "h0_4c28",
        0x4c28,
        picker(mem, Anim::walk_right, Some(WALK_LEFT)),
    );
    assert!(f.contains("a0:"), "{f}");
}

#[test]
fn the_rig_catches_the_left_facing_probe_order() {
    let mem = rig().memory_for(&generator("h0_4c6a"));
    let f = fails("h0_4c6a", 0x4c6a, picker(mem, probes_one_order, None));
    assert!(f.contains("char_anim_ptr") || f.contains("h0+$4ed"), "{f}");
}

#[test]
fn the_rig_catches_up_read_as_a_mouse_bit() {
    let mem = rig().memory_for(&generator("read_joystick"));
    let f = fails("read_joystick", 0x8dfc, joystick(mem, up_from_bit_8));
    assert!(f.contains("joy_up") || f.contains("h0+$8df"), "{f}");
}
