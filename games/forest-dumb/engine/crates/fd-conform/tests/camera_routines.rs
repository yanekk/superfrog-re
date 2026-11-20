//! The camera's engine port (`fd_core::level::camera`, plans/forest-dumb-level-1 T09) against the
//! original on the routine rig, over 10 000 states of each T36 generator, on the main program from
//! the owner's disks.
//!
//! Each check loads [`Camera`] from the state's memory, runs the engine function and stores the
//! camera back, so the rig compares every byte the original writes with what the engine wrote. The
//! engine models no scratch register: a check compares `d0` where a caller reads it (a scroll step's
//! result, which its flag step's loop tests) and no register where none does (`level3_handler` saves
//! and restores every register around `game_frame`; `camera_update` reloads `d0`, `d1` after each
//! flag step). docs/09-play-measured.md, "The camera".

use fd_conform::generators::{self, Gen};
use fd_conform::judge::{self, Cpu};
use fd_conform::routine::{RegsCompared, Rig, RigReport};
use fd_core::PORTED;
use fd_core::level::camera::{Camera, Dir, EdgeFlags};

const CASES: usize = 10_000;

const CHAR_X: u32 = 0x1e5a;
const CHAR_Y: u32 = 0x1e5e;
const CAMERA_X: u32 = 0x1e62;
const CAMERA_Y: u32 = 0x1e64;
const SCREEN_X: u32 = 0x4d76;
const SCREEN_Y: u32 = 0x4d7a;
const AT: u32 = 0x5186;
const LATCH: u32 = 0x5196;
const FRAME_FLAG: u32 = 0x519e;
const CROSSED: u32 = 0x51f8;

fn h0(o: u32) -> u32 {
    generators::h(0, o)
}

/// The camera and `frame_flag` as the original's memory holds them.
fn load(cpu: &Cpu) -> (Camera, u16) {
    let m = &cpu.mem;
    let mut edges = EdgeFlags::default();
    for i in 0..4u32 {
        edges.crossed[i as usize] = m.rb(h0(CROSSED + i)) != 0;
        edges.latch[i as usize] = m.rw(h0(LATCH + 2 * i));
        edges.at[i as usize] = (m.rw(h0(AT + 4 * i)), m.rw(h0(AT + 4 * i + 2)));
    }
    let camera = Camera {
        x: m.rw(h0(CAMERA_X)),
        y: m.rw(h0(CAMERA_Y)),
        screen_x: m.rl(h0(SCREEN_X)),
        screen_y: m.rl(h0(SCREEN_Y)),
        edges,
    };
    (camera, m.rw(h0(FRAME_FLAG)))
}

/// Store them back. A byte written with the value it held is not a change, so only what the engine
/// changed differs from the state the check started from.
fn store(cpu: &mut Cpu, c: &Camera, frame_flag: u16) {
    let m = &mut cpu.mem;
    m.ww(h0(CAMERA_X), c.x);
    m.ww(h0(CAMERA_Y), c.y);
    m.wl(h0(SCREEN_X), c.screen_x);
    m.wl(h0(SCREEN_Y), c.screen_y);
    for i in 0..4u32 {
        let e = &c.edges;
        // `st.b` writes `$ff`, `clr.l` 0. Only `camera_update`'s generator pokes these bytes, and it
        // clears them first; the program holds 0 there.
        m.wb(
            h0(CROSSED + i),
            if e.crossed[i as usize] { 0xff } else { 0 },
        );
        m.ww(h0(LATCH + 2 * i), e.latch[i as usize]);
        m.ww(h0(AT + 4 * i), e.at[i as usize].0);
        m.ww(h0(AT + 4 * i + 2), e.at[i as usize].1);
    }
    m.ww(h0(FRAME_FLAG), frame_flag);
}

/// Run `f` on the camera loaded from `cpu`, store it, and put its result in `d0` as the original
/// returns it (-1 refused, 0 not).
fn on_camera(cpu: &mut Cpu, f: impl FnOnce(&mut Camera, &mut u16) -> Option<bool>) {
    let (mut c, mut ff) = load(cpu);
    let result = f(&mut c, &mut ff);
    store(cpu, &c, ff);
    if let Some(refused) = result {
        cpu.d[0] = if refused { 0xffff_ffff } else { 0 };
    }
}

fn update(cpu: &mut Cpu) {
    let target = (cpu.mem.rl(h0(CHAR_X)), cpu.mem.rl(h0(CHAR_Y)));
    on_camera(cpu, |c, ff| {
        c.update(target, ff);
        None
    });
}

fn clamp(cpu: &mut Cpu) {
    on_camera(cpu, |c, _| {
        c.clamp();
        None
    });
}

/// A flag step enters with the distance in `d0` (x) or `d1` (y).
fn follow(cpu: &mut Cpu, dir: Dir) {
    let d = match dir {
        Dir::Right | Dir::Left => cpu.d[0],
        Dir::Up | Dir::Down => cpu.d[1],
    } as u16;
    on_camera(cpu, |c, ff| Some(c.follow(dir, d, ff)));
}

fn scroll(cpu: &mut Cpu, dir: Dir) {
    on_camera(cpu, |c, ff| Some(c.scroll(dir, ff)));
}

/// Each routine: its generator's name (T36's, as in `re/translate.toml`), entry in hunk 0, the
/// engine port's adapter and the registers its callers read.
fn routines() -> Vec<(&'static str, u32, fn(&mut Cpu), RegsCompared)> {
    let d0 = RegsCompared::d(&[0]);
    vec![
        ("camera_update", 0x4d7e, update, RegsCompared::NONE),
        ("h0_4e9a", 0x4e9a, |c| follow(c, Dir::Right), d0),
        ("h0_4ea6", 0x4ea6, |c| follow(c, Dir::Left), d0),
        ("h0_4eb4", 0x4eb4, |c| follow(c, Dir::Up), d0),
        ("h0_4ec2", 0x4ec2, |c| follow(c, Dir::Down), d0),
        ("h0_513c", 0x513c, clamp, RegsCompared::NONE),
        ("h0_51fc", 0x51fc, |c| scroll(c, Dir::Right), d0),
        ("h0_526a", 0x526a, |c| scroll(c, Dir::Left), d0),
        ("h0_52d8", 0x52d8, |c| scroll(c, Dir::Up), d0),
        ("h0_5346", 0x5346, |c| scroll(c, Dir::Down), d0),
    ]
}

#[test]
fn every_t09_routine_is_registered_and_checked_here() {
    // The routines `re/translate.toml` gives T09.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../re/translate.toml");
    let text = std::fs::read_to_string(path).unwrap();
    let field = |block: &str, key: &str| {
        block.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
    };
    let mut t09: Vec<(String, u32)> = text
        .split("[[routine]]")
        .skip(1)
        .filter(|b| field(b, "port_task").as_deref() == Some("T09"))
        .map(|b| {
            let off = field(b, "offset").unwrap();
            let off = u32::from_str_radix(off.trim_start_matches("0x"), 16).unwrap();
            (field(b, "name").unwrap(), off)
        })
        .collect();
    t09.sort();
    let mut here: Vec<(String, u32)> = routines().iter().map(|r| (r.0.to_string(), r.1)).collect();
    here.sort();
    assert_eq!(here, t09);
    for (name, off) in &t09 {
        assert!(
            PORTED
                .iter()
                .any(|p| p.name == name && p.hunk == 0 && p.offset == *off),
            "{name} is not in PORTED"
        );
    }
}

#[test]
fn every_camera_routine_matches_the_original() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let mut lines = Vec::new();
    for (name, off, port, regs) in routines() {
        let generator = generators::generator(name).unwrap();
        let rep: RigReport =
            judge::check_regs(&mut r, name, CASES, h0(off), &generator, regs, port)
                .unwrap_or_else(|f| panic!("{f}"));
        assert_eq!((rep.passed, rep.hangs, rep.faults), (CASES, 0, 0), "{name}");
        lines.push(format!("{name}: {} paths", rep.paths));
    }
    println!("{}", lines.join("\n"));
}

/// The scroll step with the latch test left out: what a port that ignored `main_loop`'s pending
/// redraw would do.
fn scroll_right_ignoring_the_latch(cpu: &mut Cpu) {
    on_camera(cpu, |c, ff| {
        c.edges.latch[0] = 0;
        Some(c.scroll(Dir::Right, ff))
    });
}

#[test]
fn a_scroll_ignoring_its_latch_fails() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let generator = generators::generator("h0_51fc").unwrap();
    let f = judge::check_regs(
        &mut r,
        "h0_51fc",
        CASES,
        h0(0x51fc),
        &generator,
        RegsCompared::d(&[0]),
        scroll_right_ignoring_the_latch,
    )
    .unwrap_err();
    assert!(f.failures > 0, "{f}");
}

#[test]
fn the_registers_named_are_compared_and_only_those() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let generator = generators::generator("h0_51fc").unwrap();
    let check = |r: &mut Rig, regs, port: fn(&mut Cpu)| {
        judge::check_regs(r, "h0_51fc", 500, h0(0x51fc), &generator, regs, port)
    };
    let scroll_right = |c: &mut Cpu| scroll(c, Dir::Right);
    // The original leaves scratch in `d1`, which the engine does not model: compared, it differs.
    let all = check(&mut r, RegsCompared::ALL, scroll_right).unwrap_err();
    assert!(all.first.contains("d1:"), "{all}");
    check(&mut r, RegsCompared::d(&[0]), scroll_right).unwrap();
    // A wrong result in `d0` fails when `d0` is named.
    let inverted = |c: &mut Cpu| {
        scroll(c, Dir::Right);
        c.d[0] = !c.d[0];
    };
    let f = check(&mut r, RegsCompared::d(&[0]), inverted).unwrap_err();
    assert!(f.first.contains("d0:"), "{f}");
}

#[test]
fn the_generators_lay_no_base_and_load_store_round_trips() {
    // A store of what was loaded changes nothing, so the rig sees only the engine's own writes.
    let base = std::sync::Arc::new(vec![0u8; 1 << 24]);
    for (name, _, _, _) in routines() {
        let generator = generators::generator(name).unwrap();
        assert!(generator.base.is_none(), "{name}");
        let mut g = Gen::new(9);
        for _ in 0..200 {
            let s = (generator.case)(&mut g);
            let base = base.clone();
            let before = judge::Mem::new(base, &s.pokes);
            let mut cpu = Cpu {
                d: s.d,
                a: s.a,
                mem: before.clone(),
            };
            on_camera(&mut cpu, |_, _| None);
            // Except the crossed flags: `camera_update`'s generator pokes them with any byte, which
            // a `bool` stores back as `$ff`; the routine clears them before reading them.
            let crossed = h0(CROSSED)..h0(CROSSED + 4);
            let changed: Vec<_> = cpu
                .mem
                .changed_since(&before)
                .into_iter()
                .filter(|(a, _)| name != "camera_update" || !crossed.contains(a))
                .collect();
            assert_eq!(changed, vec![], "{name}");
        }
    }
}
