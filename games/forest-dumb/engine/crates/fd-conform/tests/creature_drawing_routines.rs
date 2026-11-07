//! T52's engine ports (`fd_core::render::objects::Creatures`, plans/forest-dumb-level-1 T52) against
//! the original on the routine rig, over 10 000 states of each T37 generator, on the main program from
//! the owner's disks; and the creatures they draw on the play screen against the original's.
//!
//! Each check loads the drawing's state (`CreatureDraw`, the slots and their flags, `camera_y`) from
//! the state's memory, runs the engine function with a [`DrawIo`] over that memory (a blit turned
//! into the original's custom-chip writes, in order, which the judge's blitter runs into memory),
//! and stores what the engine changed back, so the rig compares every byte and every custom-chip
//! write. No register is compared: `game_frame` reloads what it reads after `h0_2c7e` and `h0_2d4c`,
//! and their loops reload theirs after `h0_5e44` and `h0_5820`.

use fd_conform::generators::{self, h};
use fd_conform::judge::{self, Cpu};
use fd_conform::routine::{RegsCompared, Rig, RigReport};
use fd_core::level::redraw::{CreatureSlot, CreatureSlots};
use fd_core::render::objects::{CreatureBlit, CreatureDraw, Creatures, DrawIo};
use fd_core::{PORTED, State};

const CASES: usize = 10_000;

const CAMERA_Y: u32 = 0x1e64;
const RESTORE_INDEX: u32 = 0x2c7a;
const CUT_INDEX: u32 = 0x2c7c;
const X: u32 = 0x5818;
const Y: u32 = 0x581a;
const CURRENT: u32 = 0x581c;
const RESTORING: u32 = 0x581e;
const ACTIVE: u32 = 0x67da;
const SAVED: u32 = 0x67e0;
const DRAWN: u32 = 0x67e6;
const SLOTS: u32 = 0x67ec;

/// `h0+o`, wrapping: a wild slot's offset can be any long.
fn h0(o: u32) -> u32 {
    h(0, 0).wrapping_add(o)
}

/// The drawing's memory, seen through [`DrawIo`].
struct CpuIo<'a>(&'a mut Cpu);

impl DrawIo for CpuIo<'_> {
    fn rb(&self, a: u32) -> u8 {
        self.0.mem.rb(h0(a))
    }
    fn blit(&mut self, b: CreatureBlit) {
        let m = &mut self.0.mem;
        match b {
            CreatureBlit::CutSetup => {
                m.ww(0xdf_f066, 0x28);
                m.ww(0xdf_f064, 0x22);
                m.ww(0xdf_f062, 0x22);
                m.ww(0xdf_f060, 0x28);
                m.wl(0xdf_f044, 0xffff_0000);
            }
            CreatureBlit::Restore { src, dst, first } => {
                m.wl(0xdf_f054, dst);
                m.wl(0xdf_f050, src);
                if first {
                    m.ww(0xdf_f066, 0x2c);
                    m.ww(0xdf_f064, 0);
                    m.wl(0xdf_f044, 0xffff_ffff);
                    m.ww(0xdf_f040, 0x09f0);
                    m.ww(0xdf_f042, 0);
                }
                m.ww(0xdf_f058, 0x1401);
            }
            CreatureBlit::Cut {
                mask,
                picture,
                dst,
                con0,
                con1,
                size,
            } => {
                m.wl(0xdf_f054, dst);
                m.wl(0xdf_f050, mask);
                m.wl(0xdf_f04c, picture);
                m.wl(0xdf_f048, dst);
                m.ww(0xdf_f040, con0);
                m.ww(0xdf_f042, con1);
                m.ww(0xdf_f058, size);
            }
        }
    }
}

/// The state the drawing uses, as the original's memory holds it.
fn load(cpu: &Cpu) -> State {
    let m = &cpu.mem;
    let mut s = State::level_1();
    s.camera.y = m.rw(h0(CAMERA_Y));
    s.creature_draw = CreatureDraw {
        restore_index: m.rw(h0(RESTORE_INDEX)),
        cut_index: m.rw(h0(CUT_INDEX)),
        x: m.rw(h0(X)),
        y: m.rw(h0(Y)),
        restoring: m.rw(h0(RESTORING)),
        saved: std::array::from_fn(|i| m.rb(h0(SAVED + i as u32))),
        drawn: std::array::from_fn(|i| m.rb(h0(DRAWN + i as u32))),
    };
    let c = &mut s.edges.creatures;
    c.current = m.rw(h0(CURRENT));
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
    let d = &s.creature_draw;
    w(CAMERA_Y, &s.camera.y.to_be_bytes());
    w(RESTORE_INDEX, &d.restore_index.to_be_bytes());
    w(CUT_INDEX, &d.cut_index.to_be_bytes());
    w(X, &d.x.to_be_bytes());
    w(Y, &d.y.to_be_bytes());
    w(RESTORING, &d.restoring.to_be_bytes());
    w(SAVED, &d.saved);
    w(DRAWN, &d.drawn);
    let c = &s.edges.creatures;
    w(CURRENT, &c.current.to_be_bytes());
    w(ACTIVE, &c.active);
    for i in 0..5u32 {
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

/// Run `f` on the drawing over the state loaded from `cpu`, then store it.
fn on_creatures(cpu: &mut Cpu, f: impl FnOnce(&mut Creatures)) {
    let before = load(cpu);
    let mut s = before.clone();
    let camera_y = s.camera.y;
    f(&mut Creatures {
        draw: &mut s.creature_draw,
        slots: &mut s.edges.creatures,
        camera_y,
        io: &mut CpuIo(cpu),
    });
    store(cpu, &before, &s);
}

fn restore_all(cpu: &mut Cpu) {
    on_creatures(cpu, |c| c.restore_all());
}

fn restore(cpu: &mut Cpu) {
    on_creatures(cpu, |c| c.restore());
}

fn cut_all(cpu: &mut Cpu) {
    on_creatures(cpu, |c| c.cut_all());
}

fn cut(cpu: &mut Cpu) {
    on_creatures(cpu, |c| c.cut());
}

/// Each routine: its generator's name (T37's, as in `re/translate.toml`), entry in hunk 0 and the
/// engine port's adapter.
fn routines() -> Vec<(&'static str, u32, fn(&mut Cpu))> {
    vec![
        ("h0_2c7e", 0x2c7e, restore_all),
        ("h0_5e44", 0x5e44, restore),
        ("h0_2d4c", 0x2d4c, cut_all),
        ("h0_5820", 0x5820, cut),
    ]
}

#[test]
fn every_t52_routine_is_registered_and_checked_here() {
    // The routines `re/translate.toml` gives T52.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../re/translate.toml");
    let text = std::fs::read_to_string(path).unwrap();
    let field = |block: &str, key: &str| {
        block.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
    };
    let mut t52: Vec<(String, u32)> = text
        .split("[[routine]]")
        .skip(1)
        .filter(|b| field(b, "port_task").as_deref() == Some("T52"))
        .map(|b| {
            let off = field(b, "offset").unwrap();
            let off = u32::from_str_radix(off.trim_start_matches("0x"), 16).unwrap();
            (field(b, "name").unwrap(), off)
        })
        .collect();
    t52.sort();
    let mut here: Vec<(String, u32)> = routines().iter().map(|r| (r.0.to_string(), r.1)).collect();
    here.sort();
    assert_eq!(here, t52);
    for (name, off) in &t52 {
        assert!(
            PORTED
                .iter()
                .any(|p| p.name == name && p.hunk == 0 && p.offset == *off),
            "{name} is not in PORTED"
        );
    }
}

#[test]
fn every_t52_routine_matches_the_original() {
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
        // A creature kind of 8 or more (and, for `h0_5820`, a slot past the five) reads its frame
        // table's pointer from code past h0+$3a18's eight; the original takes an exception there,
        // which T37 counted on 55 and 32 of 2 000 states. Counted, never compared.
        assert_eq!((rep.hangs, rep.unmodelled_blits), (0, 0), "{name}");
        assert_eq!(rep.passed + rep.faults, CASES, "{name}");
        assert!(rep.faults < CASES / 20, "{name}: {} faults", rep.faults);
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

/// The blits a port starts, kept to be changed before they reach memory.
struct Recorded(Vec<CreatureBlit>, fd_conform::judge::Mem);

impl DrawIo for Recorded {
    fn rb(&self, a: u32) -> u8 {
        self.1.rb(h0(a))
    }
    fn blit(&mut self, b: CreatureBlit) {
        self.0.push(b);
    }
}

/// Run `f` as [`on_creatures`] does, with `change` applied to its blits before they are written.
fn on_changed(
    cpu: &mut Cpu,
    f: impl FnOnce(&mut Creatures),
    change: impl Fn(&mut Vec<CreatureBlit>),
) {
    let before = load(cpu);
    let mut s = before.clone();
    let camera_y = s.camera.y;
    let mut rec = Recorded(Vec::new(), cpu.mem.clone());
    f(&mut Creatures {
        draw: &mut s.creature_draw,
        slots: &mut s.edges.creatures,
        camera_y,
        io: &mut rec,
    });
    change(&mut rec.0);
    for b in rec.0 {
        CpuIo(cpu).blit(b);
    }
    store(cpu, &before, &s);
}

#[test]
fn the_rig_catches_a_restore_that_never_wraps_the_screen() {
    // `cmpa.l #h3+$e998, a0; bmi` skipped: a creature on the screen's last band restores below it.
    let f = fails("h0_5e44", 0x5e44, |cpu| {
        on_changed(
            cpu,
            |c| c.restore(),
            |bs| {
                // A row below the first that lands above it has wrapped; undo the wrap.
                let top = match bs.first() {
                    Some(CreatureBlit::Restore { dst, .. }) => *dst,
                    _ => return,
                };
                for b in bs.iter_mut().skip(3) {
                    if let CreatureBlit::Restore { dst, .. } = b
                        && *dst < top
                    {
                        *dst += 0xe600;
                    }
                }
            },
        );
    });
    assert!(!f.is_empty());
}

#[test]
fn the_rig_catches_a_cut_split_one_line_off() {
    // `cmpi.l #$1f, d0; beq` read as always adding one: at a low byte of `$e0` the first part would
    // be 32 lines.
    let f = fails("h0_5820", 0x5820, |cpu| {
        let y = cpu.mem.rw(h0(Y));
        on_changed(
            cpu,
            |c| c.cut(),
            |bs| {
                if y & 0xff == 0xe0
                    && let Some(CreatureBlit::Cut { size, .. }) = bs.first_mut()
                {
                    *size = 0x2803;
                }
            },
        );
    });
    assert!(!f.is_empty());
}

// ---------------------------------------------------------------------------------------------
// In play, against the original
// ---------------------------------------------------------------------------------------------

#[test]
fn the_drawing_state_at_the_sync_point_is_the_originals() {
    use fd_conform::Live;
    use fd_conform::sync::reach_level_1;
    use fd_core::Mem;
    let am = reach_level_1(0).unwrap_or_else(|e| panic!("{e}"));
    let m = Live(&am);
    let word = |o: u32| u16::from_be_bytes(m.read(0, o, 2).try_into().unwrap());
    let ours = State::level_1().creature_draw;
    let theirs = CreatureDraw {
        restore_index: word(RESTORE_INDEX),
        cut_index: word(CUT_INDEX),
        x: word(X),
        y: word(Y),
        restoring: word(RESTORING),
        saved: m.read(0, SAVED, 5).try_into().unwrap(),
        drawn: m.read(0, DRAWN, 5).try_into().unwrap(),
    };
    assert_eq!(ours, theirs);
}

/// A transplanted game's drawing state and play screen, the creatures restored with the slots of
/// `then` and cut with the slots of `now`, as `game_frame` runs them around the creatures' moves.
fn drawn(
    g: &fd_core::Game,
    then: &CreatureSlots,
    now: &CreatureSlots,
) -> (fd_core::render::playfield::Screen, usize) {
    use fd_core::level::redraw::LevelImage;
    use fd_core::render::objects::PlayDraw;
    let data = g.data();
    let image = LevelImage::new(&data.level);
    let mut screen = g.display().screen.clone();
    let mut draw = g.state().creature_draw;
    let camera_y = g.state().camera.y;
    let mut cuts = 0;
    let mut io = PlayDraw {
        image: &image,
        program: &data.program,
        screen: &mut screen,
        data: &data.display,
    };
    let mut slots = then.clone();
    draw.saved = slots.active;
    slots.active = draw.drawn;
    Creatures {
        draw: &mut draw,
        slots: &mut slots,
        camera_y,
        io: &mut io,
    }
    .restore_all();
    let mut slots = now.clone();
    Creatures {
        draw: &mut draw,
        slots: &mut slots,
        camera_y,
        io: &mut io,
    }
    .cut_all();
    cuts += slots.active.iter().filter(|&&f| f != 0).count();
    (screen, cuts)
}

/// Running right from the sync point, jumping onto the first creatures and back left: the T10 review's
/// script (`drawing_routines.rs`), by pass from the sync point.
fn input(f: u32) -> amiga_oracle::FrameInput {
    use amiga_oracle::{FrameInput, Joy};
    let script = [
        (0, FrameInput::NONE),
        (40, FrameInput::joy(Joy::RIGHT)),
        (200, FrameInput::joy(Joy::RIGHT | Joy::UP)),
        (215, FrameInput::joy(Joy::RIGHT)),
        (260, FrameInput::joy(Joy::LEFT | Joy::UP)),
        (275, FrameInput::joy(Joy::LEFT)),
        (400, FrameInput::joy(Joy::LEFT | Joy::UP)),
        (415, FrameInput::joy(Joy::LEFT)),
    ];
    script
        .iter()
        .rev()
        .find(|(at, _)| *at <= f)
        .map_or(FrameInput::NONE, |(_, i)| *i)
}

const PASSES: u32 = 520;

/// The original's RAM where each pass ends: `level3_handler`'s `INTREQ=$4010` after `game_frame`
/// returns (docs/09, "Where a pass ends"). A step's own end falls inside the pass on most moving
/// frames, before the cuts, which come last.
fn watch_pass_end(am: &mut amiga_oracle::Amiga) {
    am.watch_write(Some((0x9c, 0x4010)));
}

#[test]
fn the_creatures_on_the_play_screen_are_the_originals() {
    // The picture check (T52): from the original where each pass ends, the engine runs the
    // foreground's edge redraws (T55), restores the creatures the original drew and cuts them where
    // the original's next pass moved them (creature movement is T24's). Every play screen byte the
    // creature drawing changes must equal the original's where that next pass ends.
    use fd_conform::sync::reach_level_1;
    use fd_conform::{OracleMem, game_data};
    use fd_core::render::playfield::{SCREEN, SCREEN_BYTES};
    use fd_core::{Game, Mem, Start, Transplant};
    let mut am = reach_level_1(PASSES + 1).unwrap_or_else(|e| panic!("{e}"));
    watch_pass_end(&mut am);
    let data = game_data().unwrap_or_else(|e| panic!("{e}"));
    let (mut written, mut cuts, mut differ, mut paired) = (0usize, 0usize, 0usize, 0);
    let mut then: Option<Game> = None;
    for f in 0..PASSES {
        am.step(input(f)).unwrap_or_else(|e| panic!("{e}"));
        // A step that sees no pass end, or two (docs/09), breaks the pairing until the next.
        let Some(m) = am.watched_memory().filter(|_| am.frames_run() == 1) else {
            then = None;
            continue;
        };
        let mut now = Game::new(data.clone(), Start::Level1);
        now.transplant(&OracleMem(&m));
        if let Some(mut then) = then.take()
            && now.state().irq_count == then.state().irq_count.wrapping_add(1)
        {
            paired += 1;
            let theirs = OracleMem(&m).read(3, SCREEN, SCREEN_BYTES as usize);
            then.foreground();
            let (screen, n) = drawn(
                &then,
                &then.state().edges.creatures,
                &now.state().edges.creatures,
            );
            cuts += n;
            let before = &then.display().screen.bytes;
            for (i, (&v, &was)) in screen.bytes.iter().zip(before).enumerate() {
                if v != was {
                    written += 1;
                    differ += (theirs[i] != v) as usize;
                }
            }
        }
        then = Some(now);
    }
    println!(
        "{paired} passes paired, {cuts} creature cuts, {written} bytes written, {differ} differ"
    );
    assert!(paired > PASSES as usize / 2, "{paired} passes paired");
    assert!(cuts > 100, "{cuts} creature cuts: no creature in view");
    assert!(written > 10_000, "{written} bytes written");
    assert_eq!(differ, 0, "{differ} of {written} play screen bytes differ");
}
#[test]
fn game_frame_restores_and_cuts_the_creatures() {
    // The rig proves the routines, not the calls: `Game::step` must restore the creatures the last
    // pass drew, cut the active ones and keep their flags as drawn.
    use fd_conform::sync::reach_level_1;
    use fd_conform::{Live, game_data};
    use fd_core::{Game, Start, Transplant};
    let mut am = reach_level_1(PASSES + 1).unwrap_or_else(|e| panic!("{e}"));
    let data = game_data().unwrap_or_else(|e| panic!("{e}"));
    let mut checked = 0;
    for f in 0..PASSES {
        am.step(input(f)).unwrap_or_else(|e| panic!("{e}"));
        let mut g = Game::new(data.clone(), Start::Level1);
        g.transplant(&Masked(Live(&am), MECHANICS.to_vec()));
        let creatures = g.state().edges.creatures.clone();
        let items = g.state().items_left;
        if creatures.active.iter().all(|&f| f == 0) {
            continue;
        }
        // What the step should draw: the foreground, then the restore with the slots as they were
        // and the cut with the slots the step moved and sorted (T24).
        let mut want = g.clone();
        want.foreground();
        let undrawn = want.display().screen.clone();
        // A case freezing creatures in the original skips the drawing (owner, 2026-11-06): its step
        // leaves the screen as the foreground does.
        let mut frozen = g.clone();
        frozen.freeze_as_a_case();
        frozen.step(amiga_oracle::FrameInput::NONE);
        g.step(amiga_oracle::FrameInput::NONE);
        let s = g.state();
        let (want, _) = drawn(&want, &creatures, &s.edges.creatures);
        // A creature written back (T22), an item taken, or a spring or an animated cell the pass
        // itself started (T25) redraws more than this test models.
        if s.edges.creatures.active != creatures.active
            || s.items_left != items
            || s.spring.timer != 0
            || s.edges.cells.used.iter().flatten().any(|&u| u != 0)
        {
            continue;
        }
        assert_eq!(s.creature_draw.drawn, s.edges.creatures.active, "pass {f}");
        assert_eq!(
            (s.creature_draw.restore_index, s.creature_draw.cut_index),
            (5, 5)
        );
        assert!(
            g.display().screen.bytes == want.bytes,
            "pass {f}: game_frame's drawing differs from the ports run in its order"
        );
        assert!(
            frozen.display().screen.bytes == undrawn.bytes,
            "pass {f}: a frozen case drew the creatures"
        );
        // Beyond the drawing's own variables, which it leaves as they were, and what T25's frozen
        // routines keep (`level::FROZEN_IN_A_CASE`).
        let mut fs = frozen.state().clone();
        fs.creature_draw = s.creature_draw;
        fs.edges.creatures.current = s.edges.creatures.current;
        fs.edges.cells = s.edges.cells.clone();
        fs.spring = s.spring;
        assert!(
            &fs == s,
            "pass {f}: the skip changed more than the drawing's state"
        );
        checked += 1;
    }
    assert!(checked > 50, "{checked} passes with a creature checked");
}

/// The original's memory with each (hunk, offset, length) of `.1` read as 0.
struct Masked<'a>(fd_conform::Live<'a>, Vec<(u8, u32, u32)>);

impl fd_core::Mem for Masked<'_> {
    fn read(&self, hunk: u8, offset: u32, len: usize) -> Vec<u8> {
        let mut v = self.0.read(hunk, offset, len);
        for &(h, at, n) in &self.1 {
            for a in at..at + n {
                if let (true, Some(b)) = (h == hunk, v.get_mut(a.wrapping_sub(offset) as usize)) {
                    *b = 0;
                }
            }
        }
        v
    }
}

/// What T25's mechanics draw in a pass, left out so the screen shows the creatures' drawing alone:
/// the animated cells' lists (`anim_cell_used`, `h2+$1680`) and the spring (`spring_timer`).
const MECHANICS: [(u8, u32, u32); 2] = [(2, 0x1680, 48), (0, 0x21bc, 2)];

#[test]
fn game_frame_restores_the_creatures_the_last_pass_drew() {
    // T52 review: `h0_2c7e` runs with the flags the last cut kept (`creature_active+$c`), not the
    // live ones. A creature drawn but no longer active is restored away and not cut; restoring by
    // the live flags would leave it on the screen.
    use fd_conform::sync::reach_level_1;
    use fd_conform::{Live, game_data};
    use fd_core::Mem;
    use fd_core::{Game, Start, Transplant};
    let mut am = reach_level_1(PASSES + 1).unwrap_or_else(|e| panic!("{e}"));
    let data = game_data().unwrap_or_else(|e| panic!("{e}"));
    let mut checked = 0;
    for f in 0..PASSES {
        am.step(input(f)).unwrap_or_else(|e| panic!("{e}"));
        let Some(k) = Live(&am).read(0, ACTIVE, 5).iter().position(|&b| b != 0) else {
            continue;
        };
        let mut g = Game::new(data.clone(), Start::Level1);
        let mut zeroed = MECHANICS.to_vec();
        zeroed.push((0, ACTIVE + k as u32, 1));
        g.transplant(&Masked(Live(&am), zeroed));
        let creatures = g.state().edges.creatures.clone();
        let items = g.state().items_left;
        if g.state().creature_draw.drawn[k] == 0 {
            continue;
        }
        let mut want = g.clone();
        want.foreground();
        let undrawn = want.display().screen.clone();
        g.step(amiga_oracle::FrameInput::NONE);
        let s = g.state();
        // Cut with the slots the step moved and sorted (T24).
        let (want, _) = drawn(&want, &creatures, &s.edges.creatures);
        if s.edges.creatures.active != creatures.active
            || s.items_left != items
            || s.spring.timer != 0
            || s.edges.cells.used.iter().flatten().any(|&u| u != 0)
        {
            continue;
        }
        assert!(
            want.bytes != undrawn.bytes,
            "pass {f}: the restore changed nothing"
        );
        assert!(
            g.display().screen.bytes == want.bytes,
            "pass {f}: game_frame restored by other flags than the last pass drew"
        );
        checked += 1;
    }
    assert!(
        checked > 50,
        "{checked} passes with a creature gone checked"
    );
}
