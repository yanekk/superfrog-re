//! The `camera` generators T36 keeps (plans/forest-dumb-level-1 T36, DESIGN §4.3), on the main
//! program from the owner's disks.
//!
//! The rig has no instruction trace, so the branches are shown through a witness: a literal model of
//! each routine, written from the listing, which counts which way each conditional branch went.
//! Every model must match the original on all 2 000 of its generator's states (registers, bytes
//! changed); on a state where the two agree, the model took the original's branches, since each
//! branch here changes what the routine writes or returns. A branch is named by the address of its
//! instruction, as `h0+$x` in the listing (`tools/translate.py`'s flow extraction).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use fd_conform::generators::{self, Gen, camera, h};
use fd_conform::judge::{self, Cpu};
use fd_conform::routine::{Rig, RigReport};

const CASES: usize = 2000;

thread_local! {
    /// Per branch address: how many times it went each way, [not taken, taken].
    static COV: RefCell<BTreeMap<u32, [usize; 2]>> = RefCell::new(BTreeMap::new());
}

/// Record that the branch at `h0+$at` was `taken` or not, and return `taken`.
fn br(at: u32, taken: bool) -> bool {
    COV.with(|c| c.borrow_mut().entry(at).or_default()[taken as usize] += 1);
    taken
}

/// N after `cmpi.w #imm, x` (or `tst.w x` with `imm` 0).
fn minus(x: u16, imm: u16) -> bool {
    x.wrapping_sub(imm) & 0x8000 != 0
}

fn set_low_word(r: &mut u32, w: u16) {
    *r = (*r & 0xffff_0000) | w as u32;
}

const FRAME_FLAG: u32 = 0x519e;

/// One direction's scroll step and flag step, as laid out in the listing.
struct Dir {
    coord: u32,
    up: bool,
    latch: u32,
    flag: u32,
    /// Where the camera stops: `$17cf`/`$53f` going up, 0 going down.
    limit: u16,
    /// The scroll step's three branches: boundary, latch, clamp.
    branches: [u32; 3],
    /// The flag step: its entry register (0 or 1), the bias it subtracts, whether it inverts, and its
    /// loop's two branches (`bne` out, `dbra` back).
    reg: usize,
    bias: u32,
    not: bool,
    loop_branches: [u32; 2],
}

const RIGHT: Dir = Dir {
    coord: camera::CAMERA_X,
    up: true,
    latch: 0x5196,
    flag: 0x51f8,
    limit: 0x17cf,
    branches: [0x5212, 0x5228, 0x523c],
    reg: 0,
    bias: 0x93,
    not: false,
    loop_branches: [0x51ac, 0x51b0],
};
const LEFT: Dir = Dir {
    coord: camera::CAMERA_X,
    up: false,
    latch: 0x5198,
    flag: 0x51f9,
    limit: 0,
    branches: [0x5280, 0x5296, 0x52aa],
    reg: 0,
    bias: 0x79,
    not: true,
    loop_branches: [0x51c2, 0x51c6],
};
const UP: Dir = Dir {
    coord: camera::CAMERA_Y,
    up: false,
    latch: 0x519a,
    flag: 0x51fa,
    limit: 0,
    branches: [0x52ee, 0x5304, 0x5318],
    reg: 1,
    bias: 0x47,
    not: true,
    loop_branches: [0x51d8, 0x51dc],
};
const DOWN: Dir = Dir {
    coord: camera::CAMERA_Y,
    up: true,
    latch: 0x519c,
    flag: 0x51fb,
    limit: 0x53f,
    branches: [0x535c, 0x5372, 0x5386],
    reg: 1,
    bias: 0x59,
    not: false,
    loop_branches: [0x51ee, 0x51f2],
};

/// `h0_51fc`, `h0_526a`, `h0_52d8`, `h0_5346`: one pixel, flag and latch on a 16-pixel boundary.
fn scroll(cpu: &mut Cpu, d: &Dir) {
    let at = h(0, d.coord);
    let c = cpu.mem.rw(at);
    let step = |v: u16| {
        if d.up {
            v.wrapping_add(1)
        } else {
            v.wrapping_sub(1)
        }
    };
    set_low_word(&mut cpu.d[0], step(c) & 0xfff0);
    set_low_word(&mut cpu.d[1], c & 0xfff0);
    if !br(d.branches[0], step(c) & 0xfff0 != c & 0xfff0) {
        cpu.mem.ww(at, step(c));
        cpu.d[0] = 0;
        return;
    }
    if br(d.branches[1], cpu.mem.rw(h(0, d.latch)) != 0) {
        cpu.d[0] = 0xffff_ffff;
        return;
    }
    let n = step(c);
    cpu.mem.ww(at, n);
    // Up: `cmpi.w #limit; bmi` keeps it. Down: `cmpi.w #0; bpl` keeps it.
    let keep = if d.up {
        minus(n, d.limit)
    } else {
        !minus(n, 0)
    };
    if !br(d.branches[2], keep) {
        cpu.mem.ww(at, d.limit);
    }
    cpu.mem.wb(h(0, d.flag), 0xff);
    cpu.mem.ww(h(0, d.latch), 0xffff);
    cpu.mem.ww(h(0, FRAME_FLAG), 0xffff);
    cpu.d[0] = 0;
}

/// `h0_4e9a`..`h0_4ec2`: scroll steps until one returns non-zero or the count runs out.
fn flag_step(cpu: &mut Cpu, d: &Dir) {
    let mut d7 = cpu.d[d.reg].wrapping_sub(d.bias);
    if d.not {
        d7 = !d7;
    }
    d7 = d7.wrapping_sub(1);
    loop {
        cpu.d[7] = d7;
        scroll(cpu, d);
        if br(d.loop_branches[0], cpu.d[0] != 0) {
            return;
        }
        let w = (d7 as u16).wrapping_sub(1);
        set_low_word(&mut d7, w);
        cpu.d[7] = d7;
        if !br(d.loop_branches[1], w != 0xffff) {
            return;
        }
    }
}

/// `camera_update`.
fn camera_update(cpu: &mut Cpu) {
    cpu.mem.wl(h(0, 0x51f8), 0);
    let edges: [(&Dir, bool, u16, u32); 4] = [
        (&RIGHT, false, 0x94, 0x4dac),
        (&LEFT, true, 0x78, 0x4dd0),
        (&DOWN, false, 0x5a, 0x4df4),
        (&UP, true, 0x46, 0x4e18),
    ];
    for (i, (dir, call_if_minus, edge, at)) in edges.into_iter().enumerate() {
        cpu.d[0] = cpu.mem.rl(h(0, camera::CHAR_X));
        cpu.d[1] = cpu.mem.rl(h(0, camera::CHAR_Y));
        let w0 = (cpu.d[0] as u16).wrapping_sub(cpu.mem.rw(h(0, camera::CAMERA_X)));
        let w1 = (cpu.d[1] as u16).wrapping_sub(cpu.mem.rw(h(0, camera::CAMERA_Y)));
        set_low_word(&mut cpu.d[0], w0);
        set_low_word(&mut cpu.d[1], w1);
        if i == 0 {
            cpu.mem.wl(h(0, 0x4d76), cpu.d[0]);
            cpu.mem.wl(h(0, 0x4d7a), cpu.d[1]);
        }
        let v = if dir.reg == 0 { w0 } else { w1 };
        // bmi skips the call for right and down; bpl skips it for left and up.
        let m = minus(v, edge);
        let skip = if call_if_minus { !m } else { m };
        if !br(at, skip) {
            flag_step(cpu, dir);
        }
    }
    for (i, at) in [0x4e26u32, 0x4e44, 0x4e62, 0x4e80].into_iter().enumerate() {
        if !br(at, cpu.mem.rb(h(0, 0x51f8 + i as u32)) == 0) {
            let x = cpu.mem.rw(h(0, camera::CAMERA_X));
            let y = cpu.mem.rw(h(0, camera::CAMERA_Y));
            cpu.mem.ww(h(0, 0x5186 + 4 * i as u32), x);
            cpu.mem.ww(h(0, 0x5188 + 4 * i as u32), y);
        }
    }
}

/// `h0_513c` with its x limit as a parameter, so a wrong one can be shown failing.
fn clamp_to(cpu: &mut Cpu, max_x: u16) {
    let (cx, cy) = (h(0, camera::CAMERA_X), h(0, camera::CAMERA_Y));
    if !br(0x5144, minus(cpu.mem.rw(cy), 0x53f)) {
        cpu.mem.ww(cy, 0x53f);
    }
    if !br(0x5156, !minus(cpu.mem.rw(cy), 0)) {
        cpu.mem.ww(cy, 0);
    }
    if !br(0x5168, minus(cpu.mem.rw(cx), max_x)) {
        cpu.mem.ww(cx, max_x);
    }
    if !br(0x517a, !minus(cpu.mem.rw(cx), 0)) {
        cpu.mem.ww(cx, 0);
    }
}

fn clamp(cpu: &mut Cpu) {
    clamp_to(cpu, 0x17cf);
}

fn clamp_wrong(cpu: &mut Cpu) {
    clamp_to(cpu, 0x17ce);
}

/// Each routine: its generator's name, entry offset in hunk 0, witness, and the branches in its
/// listing (its own and its callees').
fn routines() -> Vec<(&'static str, u32, fn(&mut Cpu), Vec<u32>)> {
    let scroll_br = |d: &Dir| d.branches.to_vec();
    let step_br = |d: &Dir| [d.loop_branches.to_vec(), d.branches.to_vec()].concat();
    let mut all = vec![
        0x4dac, 0x4dd0, 0x4df4, 0x4e18, 0x4e26, 0x4e44, 0x4e62, 0x4e80,
    ];
    for d in [&RIGHT, &LEFT, &UP, &DOWN] {
        all.extend(step_br(d));
    }
    vec![
        ("camera_update", 0x4d7e, camera_update as fn(&mut Cpu), all),
        ("h0_4e9a", 0x4e9a, |c| flag_step(c, &RIGHT), step_br(&RIGHT)),
        ("h0_4ea6", 0x4ea6, |c| flag_step(c, &LEFT), step_br(&LEFT)),
        ("h0_4eb4", 0x4eb4, |c| flag_step(c, &UP), step_br(&UP)),
        ("h0_4ec2", 0x4ec2, |c| flag_step(c, &DOWN), step_br(&DOWN)),
        (
            "h0_513c",
            0x513c,
            clamp,
            vec![0x5144, 0x5156, 0x5168, 0x517a],
        ),
        ("h0_51fc", 0x51fc, |c| scroll(c, &RIGHT), scroll_br(&RIGHT)),
        ("h0_526a", 0x526a, |c| scroll(c, &LEFT), scroll_br(&LEFT)),
        ("h0_52d8", 0x52d8, |c| scroll(c, &UP), scroll_br(&UP)),
        ("h0_5346", 0x5346, |c| scroll(c, &DOWN), scroll_br(&DOWN)),
    ]
}

/// Every `camera` entry of `re/translate.toml` T36 keeps, by name.
fn t36_entries() -> BTreeSet<String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../re/translate.toml");
    // Each `[[routine]]` table's `name` and `gen_task` lines, read by hand: fd-conform takes no
    // toml crate.
    let text = std::fs::read_to_string(path).unwrap();
    let field = |block: &str, key: &str| {
        block.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
    };
    text.split("[[routine]]")
        .skip(1)
        .filter(|b| field(b, "gen_task").as_deref() == Some("T36"))
        .map(|b| field(b, "name").unwrap())
        .collect()
}

#[test]
fn every_t36_entry_has_a_generator_and_a_witness() {
    let names: BTreeSet<String> = routines().iter().map(|r| r.0.to_string()).collect();
    assert_eq!(names, t36_entries());
    for n in &names {
        assert!(generators::generator(n).is_some(), "{n} is not registered");
    }
}

#[test]
fn every_generator_takes_every_branch_both_ways_on_the_original() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let mut lines = Vec::new();
    for (name, off, witness, branches) in routines() {
        COV.with(|c| c.borrow_mut().clear());
        let generator = generators::generator(name).unwrap();
        let rep: RigReport = judge::check(&mut r, name, CASES, h(0, off), &generator, witness)
            .unwrap_or_else(|f| panic!("the witness of {name} differs from the original: {f}"));
        assert_eq!((rep.passed, rep.hangs, rep.faults), (CASES, 0, 0), "{name}");
        let cov = COV.with(|c| c.borrow().clone());
        let seen: BTreeSet<u32> = cov.keys().copied().collect();
        assert_eq!(
            seen,
            branches.iter().copied().collect(),
            "{name}: the witness's branches are not the listing's"
        );
        for b in &branches {
            let [not, taken] = cov[b];
            assert!(
                not > 0 && taken > 0,
                "{name}: the branch at h0+${b:x} went one way only ({not} not taken, {taken} taken)"
            );
        }
        let least = branches
            .iter()
            .map(|b| cov[b][0].min(cov[b][1]))
            .min()
            .unwrap();
        lines.push(format!(
            "{name}: {} paths, {} branches, each way at least {least} times",
            rep.paths,
            branches.len()
        ));
    }
    // What docs/11-translation.md records; `--nocapture` prints it.
    println!("{}", lines.join("\n"));
}

#[test]
fn a_witness_with_the_wrong_x_limit_fails() {
    // The states reach the clamp's limit exactly: one off is caught.
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let generator = generators::generator("h0_513c").unwrap();
    let f = judge::check(
        &mut r,
        "h0_513c",
        CASES,
        h(0, 0x513c),
        &generator,
        clamp_wrong,
    )
    .unwrap_err();
    assert!(f.failures > 0, "{f}");
}

/// The bytes each routine reads, as `h0` offsets and sizes.
fn reads(name: &str) -> Vec<(u32, u32)> {
    use camera::*;
    let latch = |i: usize| (LATCHES[i], 2);
    match name {
        "camera_update" => {
            let mut v = vec![
                (CHAR_X, 4),
                (CHAR_Y, 4),
                (CAMERA_X, 2),
                (CAMERA_Y, 2),
                (FLAGS, 4),
            ];
            v.extend((0..4).map(latch));
            v
        }
        "h0_4e9a" | "h0_51fc" => vec![(CAMERA_X, 2), latch(0)],
        "h0_4ea6" | "h0_526a" => vec![(CAMERA_X, 2), latch(1)],
        "h0_4eb4" | "h0_52d8" => vec![(CAMERA_Y, 2), latch(2)],
        "h0_4ec2" | "h0_5346" => vec![(CAMERA_Y, 2), latch(3)],
        "h0_513c" => vec![(CAMERA_X, 2), (CAMERA_Y, 2)],
        _ => panic!("{name}"),
    }
}

#[test]
fn no_generator_writes_outside_what_its_routine_reads() {
    for (name, _, _, _) in routines() {
        let generator = generators::generator(name).unwrap();
        assert!(generator.base.is_none(), "{name} lays a base");
        let allowed: BTreeSet<u32> = reads(name)
            .into_iter()
            .flat_map(|(o, n)| (0..n).map(move |i| h(0, o + i)))
            .collect();
        let mut g = Gen::new(0x7e57);
        for _ in 0..CASES {
            let s = (generator.case)(&mut g);
            assert!(s.hw_reads.is_empty(), "{name} sets a hardware read");
            for (a, bytes) in &s.pokes {
                for i in 0..bytes.len() as u32 {
                    assert!(
                        allowed.contains(&(a + i)),
                        "{name} writes ${:06x}, which its routine does not read",
                        a + i
                    );
                }
            }
        }
    }
}
