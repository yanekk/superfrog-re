//! T22's engine ports (`fd_core::objects::collectible`, plans/forest-dumb-level-1 T22) against the
//! original on the routine rig, over 10 000 states of each T38 and T37 generator, on the main program
//! from the owner's disks.
//!
//! Each check loads the [`State`] fields the object ports use from the state's memory, runs the
//! engine function with an [`Io`] over that memory (the object map and tile map read and written in
//! place, a blit or a sound turned into the original's custom-chip writes, in order), and stores what
//! the engine changed back, so the rig compares every byte and every custom-chip write. Registers are
//! compared where a caller reads them: `d0` after `h0_29ca`, which `h0_26ec` compares with the bonus
//! values. Every other caller reloads what it reads (`char_object_cells` reloads `char_x`, `char_y`;
//! `h0_2dd6` the slot index; `game_frame`'s next call, `play_keys`, loads its own).

use fd_conform::generators::{self, h};
use fd_conform::judge::{self, Cpu};
use fd_conform::routine::{RegsCompared, Rig, RigReport};
use fd_core::level::redraw::{Blit, CreatureSlot};
use fd_core::objects::{Collect, Io, MapObjects, Sound};
use fd_core::{PORTED, State};

const CASES: usize = 10_000;

const CHAR_X: u32 = 0x1e5a;
const CHAR_Y: u32 = 0x1e5e;
const CAMERA_X: u32 = 0x1e62;
const CAMERA_Y: u32 = 0x1e64;
const ROW: u32 = 0x2692;
const EXIT_TOUCHED: u32 = 0x2696;
const TAKEN: u32 = 0x29c6;
const SCORE: u32 = 0x2a66;
const SCORE_ADD: u32 = 0x2a6a;
const SLOT_INDEX: u32 = 0x2faa;
const ACTIVE: u32 = 0x67da;
const SLOTS: u32 = 0x67ec;
const COUNTDOWN: u32 = 0x6a80;
const ITEMS_LEFT: u32 = 0x6a88;
/// The play screen and the tiles, in hunk 3.
const SCREEN: u32 = h(3, 0x398);
const TILES: u32 = h(3, 0x1_3540);
/// The samples `item_sound` and the `$4c` bonus start, in hunk 4.
const ITEM_SAMPLE: u32 = h(4, 0x1_2acc);
const TIME_SAMPLE: u32 = h(4, 0x1_3a62);

/// `h0+o`, wrapping: a wild coordinate's offset can be any long.
fn h0(o: u32) -> u32 {
    h(0, 0).wrapping_add(o)
}

/// The object ports' memory, seen through [`Io`].
struct CpuIo<'a>(&'a mut Cpu);

impl Io for CpuIo<'_> {
    fn rb(&self, a: u32) -> u8 {
        self.0.mem.rb(h0(a))
    }
    fn wb(&mut self, a: u32, v: u8) {
        self.0.mem.wb(h0(a), v);
    }
    fn blit(&mut self, b: Blit) {
        let Blit::Cell { src, dst } = b else {
            unreachable!("the object ports start only cell blits")
        };
        // `h0_29ca`'s register writes, in its order; BLTSIZE starts the blit.
        let m = &mut self.0.mem;
        m.wl(0xdf_f054, SCREEN.wrapping_add(dst));
        m.wl(0xdf_f050, TILES.wrapping_add(src));
        m.ww(0xdf_f066, 0x2c);
        m.ww(0xdf_f064, 0);
        m.wl(0xdf_f044, 0xffff_ffff);
        m.ww(0xdf_f040, 0x09f0);
        m.ww(0xdf_f042, 0);
        m.ww(0xdf_f058, 0x1401);
    }
    fn sound(&mut self, s: Sound) {
        // DMACON off for the channel, its location, length, period and volume, DMACON on.
        let (ch, sample, len, period) = match s {
            Sound::Item => (2u32, ITEM_SAMPLE, 0x7cb, 0x165),
            Sound::Time => (3, TIME_SAMPLE, 0x1c4f, 0x1ff),
        };
        let m = &mut self.0.mem;
        let aud = 0xdf_f0a0 + 0x10 * ch;
        m.ww(0xdf_f096, 1 << ch);
        m.wl(aud, sample);
        m.ww(aud + 4, len);
        m.ww(aud + 6, period);
        m.ww(aud + 8, 0x3f);
        m.ww(0xdf_f096, 0x8200 | 1 << ch);
    }
}

/// The state the object ports use, as the original's memory holds it.
fn load(cpu: &Cpu) -> State {
    let m = &cpu.mem;
    let mut s = State::level_1();
    s.char_x = m.rl(h0(CHAR_X));
    s.char_y = m.rl(h0(CHAR_Y));
    s.camera.x = m.rw(h0(CAMERA_X));
    s.camera.y = m.rw(h0(CAMERA_Y));
    s.level_exit_touched = m.rw(h0(EXIT_TOUCHED));
    s.score = m.rl(h0(SCORE));
    s.countdown = m.rl(h0(COUNTDOWN));
    s.items_left = m.rw(h0(ITEMS_LEFT));
    s.collect = Collect {
        score_add: m.rl(h0(SCORE_ADD)),
        row: m.rl(h0(ROW)),
        taken: m.rl(h0(TAKEN)),
    };
    let c = &mut s.edges.creatures;
    c.index = m.rl(h0(SLOT_INDEX));
    for i in 0..5u32 {
        c.active[i as usize] = m.rb(h0(ACTIVE + i));
        let a = h0(SLOTS + 8 * i);
        c.slots[i as usize] = CreatureSlot {
            x: m.rw(a),
            y: m.rw(a + 2),
            kind: m.rw(a + 4),
            facing: m.rb(a + 6),
            phase: m.rb(a + 7),
        };
    }
    s
}

/// Every byte [`load`] reads, with its address, from `s`.
fn bytes(s: &State) -> Vec<(u32, u8)> {
    let mut out = Vec::new();
    let mut w = |a: u32, v: &[u8]| {
        for (k, b) in v.iter().enumerate() {
            out.push((h0(a + k as u32), *b));
        }
    };
    w(CHAR_X, &s.char_x.to_be_bytes());
    w(CHAR_Y, &s.char_y.to_be_bytes());
    w(CAMERA_X, &s.camera.x.to_be_bytes());
    w(CAMERA_Y, &s.camera.y.to_be_bytes());
    w(EXIT_TOUCHED, &s.level_exit_touched.to_be_bytes());
    w(SCORE, &s.score.to_be_bytes());
    w(COUNTDOWN, &s.countdown.to_be_bytes());
    w(ITEMS_LEFT, &s.items_left.to_be_bytes());
    w(SCORE_ADD, &s.collect.score_add.to_be_bytes());
    w(ROW, &s.collect.row.to_be_bytes());
    w(TAKEN, &s.collect.taken.to_be_bytes());
    let c = &s.edges.creatures;
    w(SLOT_INDEX, &c.index.to_be_bytes());
    for i in 0..5u32 {
        w(ACTIVE + i, &[c.active[i as usize]]);
        let v = c.slots[i as usize];
        let a = SLOTS + 8 * i;
        w(a, &v.x.to_be_bytes());
        w(a + 2, &v.y.to_be_bytes());
        w(a + 4, &v.kind.to_be_bytes());
        w(a + 6, &[v.facing, v.phase]);
    }
    out
}

/// Store what the engine changed. Only that: a blit the original sends to a wild address can land on
/// these variables, and writing back what was loaded would undo it.
fn store(cpu: &mut Cpu, before: &State, s: &State) {
    for ((a, b), (_, was)) in bytes(s).into_iter().zip(bytes(before)) {
        if b != was {
            cpu.mem.wb(a, b);
        }
    }
}

/// Run `f` on the object ports over the state loaded from `cpu`, then store it.
fn on_objects(cpu: &mut Cpu, f: impl FnOnce(&mut MapObjects)) {
    let before = load(cpu);
    let mut s = before.clone();
    f(&mut MapObjects {
        state: &mut s,
        io: &mut CpuIo(cpu),
    });
    store(cpu, &before, &s);
}

/// `h0_26ec` takes the pixel in `d0`, `d1`.
fn take(cpu: &mut Cpu) {
    let (x, y) = (cpu.d[0], cpu.d[1]);
    on_objects(cpu, |o| o.take(x, y));
}

/// `h0_29ca` takes the pixel in `d0`, `d1` and returns `object_taken` in `d0`.
fn redraw_cell(cpu: &mut Cpu) {
    let (x, y) = (cpu.d[0], cpu.d[1]);
    on_objects(cpu, |o| o.redraw_cell(x, y));
    cpu.d[0] = cpu.mem.rl(h0(TAKEN));
}

/// `h0_2e0e` takes the slot in `d0` (T38's states hold 0 to 4).
fn write_back_slot(cpu: &mut Cpu) {
    let i = cpu.d[0] as usize;
    on_objects(cpu, |o| o.write_back_slot(i));
}

/// Each routine: its generator's name (T38's and T37's, as in `re/translate.toml`), entry in hunk 0,
/// the engine port's adapter and the registers its callers read.
fn routines() -> Vec<(&'static str, u32, fn(&mut Cpu), RegsCompared)> {
    let none = RegsCompared::NONE;
    vec![
        (
            "char_object_cells",
            0x2698,
            |c| on_objects(c, |o| o.object_cells()),
            none,
        ),
        ("h0_26ec", 0x26ec, take, none),
        ("h0_29ca", 0x29ca, redraw_cell, RegsCompared::d(&[0])),
        (
            "score_add_routine",
            0x2a6e,
            |c| on_objects(c, |o| o.add_score()),
            none,
        ),
        (
            "h0_2dd6",
            0x2dd6,
            |c| on_objects(c, |o| o.write_back()),
            none,
        ),
        ("h0_2e0e", 0x2e0e, write_back_slot, none),
        (
            "countdown_plus_100",
            0x6a04,
            |c| on_objects(c, |o| o.add_time()),
            none,
        ),
        (
            "items_left_minus_one",
            0x6a4c,
            |c| on_objects(c, |o| o.count_item()),
            none,
        ),
    ]
}

#[test]
fn every_t22_routine_is_registered_and_checked_here() {
    // The routines `re/translate.toml` gives T22.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../re/translate.toml");
    let text = std::fs::read_to_string(path).unwrap();
    let field = |block: &str, key: &str| {
        block.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
    };
    let mut t22: Vec<(String, u32)> = text
        .split("[[routine]]")
        .skip(1)
        .filter(|b| field(b, "port_task").as_deref() == Some("T22"))
        .map(|b| {
            let off = field(b, "offset").unwrap();
            let off = u32::from_str_radix(off.trim_start_matches("0x"), 16).unwrap();
            (field(b, "name").unwrap(), off)
        })
        .collect();
    t22.sort();
    let mut here: Vec<(String, u32)> = routines().iter().map(|r| (r.0.to_string(), r.1)).collect();
    here.sort();
    assert_eq!(here, t22);
    for (name, off) in &t22 {
        assert!(
            PORTED
                .iter()
                .any(|p| p.name == name && p.hunk == 0 && p.offset == *off),
            "{name} is not in PORTED"
        );
    }
}

#[test]
fn every_t22_routine_matches_the_original() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let mut lines = Vec::new();
    for (name, off, port, regs) in routines() {
        let generator = generators::generator(name).unwrap();
        let rep: RigReport =
            judge::check_regs(&mut r, name, CASES, h0(off), &generator, regs, port)
                .unwrap_or_else(|f| panic!("{f}"));
        assert_eq!(
            (rep.passed, rep.hangs, rep.faults, rep.unmodelled_blits),
            (CASES, 0, 0, 0),
            "{name}"
        );
        lines.push(format!("{name}: {} paths", rep.paths));
    }
    println!("{}", lines.join("\n"));
}
