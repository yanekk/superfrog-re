//! T25's engine ports (`fd_core::objects::spring`, `fd_core::objects::animated`, plans/forest-dumb-level-1
//! T25) against the original on the routine rig, over 10 000 states of each T38 and T37 generator, on
//! the main program from the owner's disks.
//!
//! Each check loads the [`State`] fields the mechanics use from the state's memory, runs the engine
//! function with an [`Io`] over that memory (the object map, tile map and the program's tables read in
//! place, `collision_map` written in place, a blit or a sound turned into the original's custom-chip
//! writes, in order), and stores what the engine changed back, so the rig compares every byte and
//! every custom-chip write. No register is compared: in `game_frame` each routine's next call loads
//! what it reads (`h0_62b6` after `h0_21c0`, `h0_2dd6` after it, `char_update` after `h0_24ce`).

use fd_conform::generators::{self, h};
use fd_conform::judge::{self, Cpu};
use fd_conform::routine::{RegsCompared, Rig, RigReport};
use fd_core::level::redraw::{AnimCell, Blit};
use fd_core::objects::{Io, MapObjects, Sound};
use fd_core::{PORTED, State};

const CASES: usize = 10_000;

const CHAR_X: u32 = 0x1e5a;
const CHAR_Y: u32 = 0x1e5e;
const CAMERA_X: u32 = 0x1e62;
const CAMERA_Y: u32 = 0x1e64;
const JUMP_41B6: u32 = 0x41b6;
const DYING: u32 = 0x4b62;
const RISING: u32 = 0x4ecf;
const FALLING: u32 = 0x4ed0;
const SPRING_X: u32 = 0x21b4;
const SPRING_Y: u32 = 0x21b8;
const SPRING_TIMER: u32 = 0x21bc;
const SPRING_KIND: u32 = 0x21be;
const ANIM_TICK: u32 = 0x62b2;
const WATER_CYCLE: u32 = 0x6342;
/// The animated cells' flags and entries, in hunk 2.
const ANIM_USED: u32 = h(2, 0x1680);
const ANIM_CELLS: u32 = h(2, 0x16b0);
/// The play screen, the tiles and `collision_map`, in hunk 3.
const SCREEN: u32 = h(3, 0x398);
const TILES: u32 = h(3, 0x1_3540);
const COLLISION: u32 = h(3, 0xfa50);
/// `_ELEMENTY`'s pictures and masks, and the spring's sample, in hunk 4.
const ELEMENTS: u32 = h(4, 0);
const ELEMENT_MASKS: u32 = h(4, 0x1900);
const SPRING_SAMPLE: u32 = h(4, 0xb548);

/// `h0+o`, wrapping: a wild coordinate's offset can be any long.
fn h0(o: u32) -> u32 {
    h(0, 0).wrapping_add(o)
}

/// The mechanics' memory, seen through [`Io`]. `h0_21c0` sets the blitter's modulos and masks for its
/// first tile only and only points and starts the other three, so `cells` counts them.
struct CpuIo<'a> {
    cpu: &'a mut Cpu,
    cells: u32,
}

impl Io for CpuIo<'_> {
    fn rb(&self, a: u32) -> u8 {
        self.cpu.mem.rb(h0(a))
    }
    fn wb(&mut self, a: u32, v: u8) {
        self.cpu.mem.wb(h0(a), v);
    }
    fn blit(&mut self, b: Blit) {
        // The register writes of each blit, in the original's order; BLTSIZE starts it.
        let m = &mut self.cpu.mem;
        match b {
            Blit::Cell { src, dst } => {
                m.wl(0xdf_f054, SCREEN.wrapping_add(dst));
                m.wl(0xdf_f050, TILES.wrapping_add(src));
                if self.cells == 0 {
                    m.ww(0xdf_f066, 0x2c);
                    m.ww(0xdf_f064, 0);
                    m.wl(0xdf_f044, 0xffff_ffff);
                    m.ww(0xdf_f040, 0x09f0);
                    m.ww(0xdf_f042, 0);
                }
                m.ww(0xdf_f058, 0x1401);
                self.cells += 1;
            }
            Blit::Picture { src, modulo, dst } => {
                m.wl(0xdf_f054, SCREEN.wrapping_add(dst));
                m.wl(0xdf_f050, h(4, 0).wrapping_add(src));
                m.ww(0xdf_f066, 0x2a);
                m.ww(0xdf_f064, modulo);
                m.wl(0xdf_f044, 0xffff_ffff);
                m.ww(0xdf_f040, 0x09f0);
                m.ww(0xdf_f042, 0);
                m.ww(0xdf_f058, 0x1402);
            }
            Blit::Element { frame, tile, dst } => {
                m.ww(0xdf_f066, 0x2c);
                m.ww(0xdf_f064, 0x26);
                m.ww(0xdf_f062, 0x26);
                m.ww(0xdf_f060, 0);
                m.wl(0xdf_f044, 0xffff_ffff);
                m.ww(0xdf_f040, 0x0fca);
                m.ww(0xdf_f042, 0);
                m.wl(0xdf_f054, SCREEN.wrapping_add(dst));
                m.wl(0xdf_f050, ELEMENT_MASKS.wrapping_add(2 * frame));
                m.wl(0xdf_f04c, ELEMENTS.wrapping_add(2 * frame));
                m.wl(0xdf_f048, TILES.wrapping_add(tile));
                m.ww(0xdf_f058, 0x1401);
            }
            Blit::Tile { .. } | Blit::Bob { .. } => {
                unreachable!("only the edge redraws (T55) start tile and bob blits")
            }
        }
    }
    fn sound(&mut self, s: Sound) {
        assert_eq!(s, Sound::Spring, "only h0_21c0 starts a sound here");
        // DMACON off for channel 1, its location, length, period and volume, DMACON on.
        let m = &mut self.cpu.mem;
        m.ww(0xdf_f096, 2);
        m.wl(0xdf_f0b0, SPRING_SAMPLE);
        m.ww(0xdf_f0b4, 0x9c4);
        m.ww(0xdf_f0b6, 0x1bf);
        m.ww(0xdf_f0b8, 0x3f);
        m.ww(0xdf_f096, 0x8202);
    }
    fn collision(&mut self, at: u32, v: u32) {
        self.cpu.mem.wl(COLLISION.wrapping_add(at), v);
    }
}

/// The state the mechanics use, as the original's memory holds it.
fn load(cpu: &Cpu) -> State {
    let m = &cpu.mem;
    let mut s = State::level_1();
    s.char_x = m.rl(h0(CHAR_X));
    s.char_y = m.rl(h0(CHAR_Y));
    s.camera.x = m.rw(h0(CAMERA_X));
    s.camera.y = m.rw(h0(CAMERA_Y));
    s.motion.jump_41b6 = m.rw(h0(JUMP_41B6));
    s.motion.dying = m.rw(h0(DYING));
    s.motion.rising = m.rb(h0(RISING));
    s.motion.falling = m.rb(h0(FALLING));
    s.spring.x = m.rl(h0(SPRING_X));
    s.spring.y = m.rl(h0(SPRING_Y));
    s.spring.timer = m.rw(h0(SPRING_TIMER));
    s.spring.kind = m.rw(h0(SPRING_KIND));
    s.animation.set_word(m.rw(h0(ANIM_TICK)));
    s.animation.water_cycle = m.rw(h0(WATER_CYCLE));
    for n in 0..48u32 {
        let (g, i) = ((n / 12) as usize, (n % 12) as usize);
        s.edges.cells.used[g][i] = m.rb(ANIM_USED + n);
        let e = ANIM_CELLS + 8 * n;
        s.edges.cells.cells[g][i] = AnimCell {
            x: m.rw(e),
            y: m.rw(e + 2),
            code: m.rb(e + 4),
            rest: [m.rb(e + 5), m.rb(e + 6), m.rb(e + 7)],
        };
    }
    s
}

/// Every byte [`load`] reads, with its address, from `s`.
fn bytes(s: &State) -> Vec<(u32, u8)> {
    let mut out = Vec::new();
    let mut w = |a: u32, v: &[u8]| {
        for (k, b) in v.iter().enumerate() {
            out.push((a + k as u32, *b));
        }
    };
    w(h0(CHAR_X), &s.char_x.to_be_bytes());
    w(h0(CHAR_Y), &s.char_y.to_be_bytes());
    w(h0(CAMERA_X), &s.camera.x.to_be_bytes());
    w(h0(CAMERA_Y), &s.camera.y.to_be_bytes());
    w(h0(JUMP_41B6), &s.motion.jump_41b6.to_be_bytes());
    w(h0(DYING), &s.motion.dying.to_be_bytes());
    w(h0(RISING), &[s.motion.rising]);
    w(h0(FALLING), &[s.motion.falling]);
    w(h0(SPRING_X), &s.spring.x.to_be_bytes());
    w(h0(SPRING_Y), &s.spring.y.to_be_bytes());
    w(h0(SPRING_TIMER), &s.spring.timer.to_be_bytes());
    w(h0(SPRING_KIND), &s.spring.kind.to_be_bytes());
    w(h0(ANIM_TICK), &s.animation.word().to_be_bytes());
    w(h0(WATER_CYCLE), &s.animation.water_cycle.to_be_bytes());
    for n in 0..48u32 {
        let (g, i) = ((n / 12) as usize, (n % 12) as usize);
        w(ANIM_USED + n, &[s.edges.cells.used[g][i]]);
        let c = s.edges.cells.cells[g][i];
        let e = ANIM_CELLS + 8 * n;
        w(e, &c.x.to_be_bytes());
        w(e + 2, &c.y.to_be_bytes());
        w(e + 4, &[c.code, c.rest[0], c.rest[1], c.rest[2]]);
    }
    out
}

/// Store what the engine changed. Only that: a write the original sends to a wild address can land
/// on these variables, and writing back what was loaded would undo it.
fn store(cpu: &mut Cpu, before: &State, s: &State) {
    for ((a, b), (_, was)) in bytes(s).into_iter().zip(bytes(before)) {
        if b != was {
            cpu.mem.wb(a, b);
        }
    }
}

/// Run `f` on the mechanics over the state loaded from `cpu`, then store it.
fn on_mechanics(cpu: &mut Cpu, f: impl FnOnce(&mut MapObjects)) {
    let before = load(cpu);
    let mut s = before.clone();
    f(&mut MapObjects {
        state: &mut s,
        io: &mut CpuIo { cpu, cells: 0 },
    });
    store(cpu, &before, &s);
}

fn spring_trigger(cpu: &mut Cpu) {
    on_mechanics(cpu, |o| o.spring_trigger());
}

fn spring_redraw(cpu: &mut Cpu) {
    on_mechanics(cpu, |o| o.spring_redraw());
}

fn animate(cpu: &mut Cpu) {
    on_mechanics(cpu, |o| o.animate());
}

/// Each routine: its generator's name (T38's and T37's, as in `re/translate.toml`), entry in hunk 0
/// and the engine port's adapter.
fn routines() -> Vec<(&'static str, u32, fn(&mut Cpu))> {
    vec![
        ("h0_21c0", 0x21c0, spring_redraw),
        ("h0_24ce", 0x24ce, spring_trigger),
        ("h0_62b6", 0x62b6, animate),
    ]
}

#[test]
fn every_t25_routine_is_registered_and_checked_here() {
    // The routines `re/translate.toml` gives T25.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../re/translate.toml");
    let text = std::fs::read_to_string(path).unwrap();
    let field = |block: &str, key: &str| {
        block.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
    };
    let mut t25: Vec<(String, u32)> = text
        .split("[[routine]]")
        .skip(1)
        .filter(|b| field(b, "port_task").as_deref() == Some("T25"))
        .map(|b| {
            let off = field(b, "offset").unwrap();
            let off = u32::from_str_radix(off.trim_start_matches("0x"), 16).unwrap();
            (field(b, "name").unwrap(), off)
        })
        .collect();
    t25.sort();
    let mut here: Vec<(String, u32)> = routines().iter().map(|r| (r.0.to_string(), r.1)).collect();
    here.sort();
    assert_eq!(here, t25);
    for (name, off) in &t25 {
        assert!(
            PORTED
                .iter()
                .any(|p| p.name == name && p.hunk == 0 && p.offset == *off),
            "{name} is not in PORTED"
        );
    }
}

#[test]
fn every_t25_routine_matches_the_original() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let mut lines = Vec::new();
    for (name, off, port) in routines() {
        let generator = generators::generator(name).unwrap();
        let rep: RigReport = judge::check_regs(
            &mut r,
            name,
            CASES,
            h0(off),
            &generator,
            RegsCompared::NONE,
            port,
        )
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

// ---------------------------------------------------------------------------------------------
// Mutants: each a plausible misreading of the listing, each must fail on the rig.
// ---------------------------------------------------------------------------------------------

fn fails(name: &str, entry: u32, port: fn(&mut Cpu)) -> String {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let generator = generators::generator(name).unwrap();
    judge::check_regs(
        &mut r,
        name,
        CASES,
        h0(entry),
        &generator,
        RegsCompared::NONE,
        port,
    )
    .map(|rep| panic!("{name}: a mutant passed {rep:?}"))
    .unwrap_err()
    .to_string()
}

#[test]
fn the_rig_catches_a_spring_firing_while_dying() {
    // `tst.w h0+$4b62; beq` skipped: a dying character would bounce.
    let f = fails("h0_24ce", 0x24ce, |cpu| {
        on_mechanics(cpu, |o| {
            let dying = o.state.motion.dying;
            o.state.motion.dying = 0;
            o.spring_trigger();
            o.state.motion.dying = dying;
        });
    });
    assert!(!f.is_empty());
}

#[test]
fn the_rig_catches_a_vanishing_block_solid_on_its_empty_frame() {
    // `cmpi.b #$14, d2; beq` missed: the block's cell would stay solid while it shows nothing.
    let f = fails("h0_62b6", 0x62b6, |cpu| {
        on_mechanics(cpu, |o| {
            struct Solid<'a>(&'a mut dyn Io);
            impl Io for Solid<'_> {
                fn rb(&self, a: u32) -> u8 {
                    self.0.rb(a)
                }
                fn wb(&mut self, a: u32, v: u8) {
                    self.0.wb(a, v)
                }
                fn blit(&mut self, b: Blit) {
                    self.0.blit(b)
                }
                fn sound(&mut self, s: Sound) {
                    self.0.sound(s)
                }
                fn collision(&mut self, at: u32, _: u32) {
                    self.0.collision(at, 0xffff_fff0)
                }
            }
            MapObjects {
                state: o.state,
                io: &mut Solid(o.io),
            }
            .animate();
        });
    });
    assert!(!f.is_empty());
}

#[test]
fn the_rig_catches_water_ignoring_its_cycle() {
    // `tst.w h0+$6342` misread: W1-1's water would run two frames, not four.
    let f = fails("h0_62b6", 0x62b6, |cpu| {
        on_mechanics(cpu, |o| {
            let w = o.state.animation.water_cycle;
            o.state.animation.water_cycle = 0;
            o.animate();
            o.state.animation.water_cycle = w;
        });
    });
    assert!(!f.is_empty());
}

#[test]
fn the_rig_catches_a_cell_drawn_past_an_unknown_value() {
    // The mid-list `rts` read as a skip: cells after a `$29` would be drawn this pass.
    let f = fails("h0_62b6", 0x62b6, |cpu| {
        on_mechanics(cpu, |o| {
            let a = &mut o.state.animation;
            let list = (a.phase.wrapping_add(1) & 3) as usize;
            let cells = o.state.edges.cells.clone();
            for c in o.state.edges.cells.cells[list].iter_mut() {
                if (0x29..0x30).contains(&c.code) && c.code != 0x2a {
                    c.code = 0x20;
                }
            }
            o.animate();
            for (c, was) in o.state.edges.cells.cells[list]
                .iter_mut()
                .zip(cells.cells[list])
            {
                c.code = was.code;
            }
        });
    });
    assert!(!f.is_empty());
}

#[test]
fn the_rig_catches_a0_to_a7_drawn_as_items() {
    // `cmpi.b #8, d2; bpl` read as a signed compare: `$a0 - $20` is `$80`, which it calls below 8, but
    // the original's N flag (`$80 - 8 = $78`) says not, and its list ends there.
    let f = fails("h0_62b6", 0x62b6, |cpu| {
        on_mechanics(cpu, |o| {
            let list = (o.state.animation.phase.wrapping_add(1) & 3) as usize;
            let cells = o.state.edges.cells.cells[list];
            for c in o.state.edges.cells.cells[list].iter_mut() {
                if (0xa0..0xa8).contains(&c.code) {
                    c.code -= 0x80;
                }
            }
            o.animate();
            for (c, was) in o.state.edges.cells.cells[list].iter_mut().zip(cells) {
                c.code = was.code;
            }
        });
    });
    assert!(!f.is_empty());
}

// ---------------------------------------------------------------------------------------------
// In play: the rig proves the routines, not that `game_frame` calls them.
// ---------------------------------------------------------------------------------------------

/// The engine's own state as the original's memory, with `cells` laid on the object map, `anim` as the
/// animated cells (used flags, then entries) and `vars` over `LAYOUT`'s values. The collision plane,
/// the screen and the rest read 0.
struct Placed<'a> {
    game: &'a fd_core::Game,
    cells: &'a [(u16, u16, u8)],
    anim: Option<(Vec<u8>, Vec<u8>)>,
    vars: &'a [(&'static str, u32)],
}

impl fd_core::Mem for Placed<'_> {
    fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8> {
        use fd_core::Transplant;
        use fd_core::level::redraw::OBJECT_MAP;
        match (hunk, offset, len, &self.anim) {
            (0, OBJECT_MAP, _, _) => {
                let mut map = self.game.state().object_map[..len].to_vec();
                for &(c, r, v) in self.cells {
                    map[c as usize * 0x60 + r as usize] = v;
                }
                return map;
            }
            (2, 0x1680, 48, Some((used, _))) => return used.clone(),
            (2, 0x16b0, 384, Some((_, cells))) => return cells.clone(),
            _ => {}
        }
        match fd_core::LAYOUT
            .iter()
            .find(|v| (v.hunk, v.offset, v.width.bytes()) == (hunk, offset, len))
        {
            Some(v) => {
                let value = self
                    .vars
                    .iter()
                    .find(|(n, _)| *n == v.name)
                    .map_or_else(|| self.game.probe(v), |&(_, x)| x);
                value.to_be_bytes()[4 - len..].to_vec()
            }
            None => vec![0; len],
        }
    }
}

fn new_game() -> fd_core::Game {
    use fd_core::{Game, Start};
    Game::new(
        fd_conform::game_data().unwrap_or_else(|e| panic!("{e}")),
        Start::Level1,
    )
}

#[test]
fn a_vanishing_block_in_view_makes_its_cell_solid_in_play() {
    use fd_core::Transplant;
    let game = new_game();
    // Cell (4, 4), in the first view, as list 2's first entry: the step after the start runs list 2.
    let (x, y) = (0x40u16, 0x40u16);
    let mut used = vec![0; 48];
    let mut cells = vec![0; 384];
    used[24] = 0xff;
    cells[24 * 8..24 * 8 + 5].copy_from_slice(&[0, x as u8, 0, y as u8, 0x30]);
    let mut g = game.clone();
    g.transplant(&Placed {
        game: &game,
        cells: &[(4, 4, 0x30)],
        anim: Some((used, cells)),
        vars: &[],
    });
    let at = |g: &fd_core::Game| {
        // `h0_62b6`'s offset: column byte `x >> 3`, line `y & $f0`, `$2e` bytes a line.
        let off = (x as u32 >> 3) + (y as u32 & 0xf0) * 0x2e;
        (0..4)
            .map(|k| g.plane().kept_byte(off + k))
            .collect::<Vec<_>>()
    };
    assert_eq!(at(&g), [0, 0, 0, 0]);
    g.step(amiga_oracle::FrameInput::NONE);
    assert_eq!(g.state().animation.phase, 2);
    // Tick 0, block `$30`: the table's first frame, not the empty one.
    assert_eq!(
        at(&g),
        [0xff, 0xff, 0xff, 0xf0],
        "h0_62b6 did not run in game_frame"
    );
}

#[test]
fn a_spring_under_the_falling_character_fires_in_play() {
    use fd_core::Transplant;
    let game = new_game();
    // The character falling at its start, a spring's left half under its feet.
    let (cx, cy) = (0x70u32, 0x70u32);
    let feet = ((cx + 0xe) >> 4, (cy + 0x23) >> 4);
    let mut g = game.clone();
    g.transplant(&Placed {
        game: &game,
        cells: &[(feet.0 as u16, feet.1 as u16, 0x10)],
        anim: None,
        vars: &[("char_falling", 1)],
    });
    g.step(amiga_oracle::FrameInput::NONE);
    let s = g.state();
    assert_eq!(
        (s.spring.timer, s.spring.x, s.spring.y),
        (1, cx + 0xe, cy + 0x23),
        "h0_24ce did not run in game_frame"
    );
}
