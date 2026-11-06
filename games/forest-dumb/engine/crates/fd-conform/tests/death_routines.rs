//! What ends a life or the game (`fd_core::character::death`, plans/forest-dumb-level-1 T23, T70)
//! against the original on the routine rig, over 10 000 states of each T39 generator (T38's for
//! `char_hit_check`, creatures and their stomp included since T24), on the main program from the
//! owner's disks.
//!
//! Each check loads the variables the routines touch into a `State`, runs the engine function on
//! it and stores them back, so the rig compares every byte the original writes with what the
//! engine wrote, and `char_hit_check`'s custom-chip writes (the hurt sound). No register is compared: `level3_handler` saves and restores every register round
//! `game_frame`, and none of these leaves a result a caller reads (docs/09-play-measured.md).

use fd_conform::generators::{self, Gen, Generator};
use fd_conform::judge::{self, Cpu};
use fd_conform::routine::{RegsCompared, Rig, RigReport};
use fd_core::character::Play;
use fd_core::level::redraw::CreatureSlot;
use fd_core::{Scratch, State, PORTED};

const CASES: usize = 10_000;

const LAST_KEY: u32 = 0x1f78;
const PAUSED: u32 = 0x2100;
const PAUSE_DOWN: u32 = 0x2102;
const ESC_DOWN: u32 = 0x21b0;
const GIVE_UP: u32 = 0x21b2;
const TICK: u32 = 0x20ac;
const EXIT_TOUCHED: u32 = 0x2696;
const COUNTDOWN: u32 = 0x6a80;
const LIVES: u32 = 0x6a84;
const ITEMS_LEFT: u32 = 0x6a88;
const CHEAT: u32 = 0x780c;
const CLXDAT_COPY: u32 = 0x1fe6;
const HITS_TAKEN: u32 = 0x3b26;
const DYING: u32 = 0x4b62;
const FALLING: u32 = 0x4ed0;
const UNHURT_T: u32 = 0x9b04;
const RUN_LEFT: u32 = 0x4710;
const RUN_RIGHT: u32 = 0x4714;
const RISING: u32 = 0x4ecf;
const JUMPED: u32 = 0x4109;
const JUMP_T: u32 = 0x410a;
/// `creature_active`: five flag bytes; `creature_slots`, eight bytes a slot.
const SLOT_FLAGS: u32 = 0x67da;
const SLOTS: u32 = 0x67ec;
const CHAR_X: u32 = 0x1e5a;
const CHAR_Y: u32 = 0x1e5e;
const SCORE: u32 = 0x2a66;
const SCORE_ADD: u32 = 0x2a6a;
const JUMP_3B28: u32 = 0x3b28;
/// CLXDAT, as the CPU reads it.
const CLXDAT: u32 = 0xdf_f00e;

fn h0(o: u32) -> u32 {
    generators::h(0, o)
}

/// The variables these routines touch, from the original's memory, in a level's state.
fn load(cpu: &Cpu) -> State {
    let m = &cpu.mem;
    let mut s = State::level_1();
    s.keys.last = m.rb(h0(LAST_KEY));
    s.keys.pause_down = m.rw(h0(PAUSE_DOWN));
    s.keys.esc_down = m.rw(h0(ESC_DOWN));
    s.keys.cheat = m.rw(h0(CHEAT));
    s.paused = m.rw(h0(PAUSED));
    s.give_up = m.rw(h0(GIVE_UP));
    s.countdown_tick = m.rl(h0(TICK));
    s.level_exit_touched = m.rw(h0(EXIT_TOUCHED));
    s.countdown = m.rl(h0(COUNTDOWN));
    s.lives = m.rw(h0(LIVES));
    s.items_left = m.rw(h0(ITEMS_LEFT));
    s.clxdat_copy = m.rw(h0(CLXDAT_COPY));
    s.hits_taken = m.rw(h0(HITS_TAKEN));
    s.motion.dying = m.rw(h0(DYING));
    s.motion.falling = m.rb(h0(FALLING));
    s.motion.unhurt_t = m.rb(h0(UNHURT_T));
    s.motion.run_left = m.rl(h0(RUN_LEFT));
    s.motion.run_right = m.rl(h0(RUN_RIGHT));
    s.motion.rising = m.rb(h0(RISING));
    s.motion.jumped = m.rb(h0(JUMPED));
    s.motion.jump_t = m.rb(h0(JUMP_T));
    s.motion.jump_3b28 = m.rw(h0(JUMP_3B28));
    s.char_x = m.rl(h0(CHAR_X));
    s.char_y = m.rl(h0(CHAR_Y));
    s.score = m.rl(h0(SCORE));
    s.collect.score_add = m.rl(h0(SCORE_ADD));
    let c = &mut s.edges.creatures;
    for i in 0..5u32 {
        c.active[i as usize] = m.rb(h0(SLOT_FLAGS + i));
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

/// Store them back. A byte written with the value it held is not a change, so only what the engine
/// changed differs from the state the check started from.
fn store(cpu: &mut Cpu, s: &State) {
    let m = &mut cpu.mem;
    m.wb(h0(LAST_KEY), s.keys.last);
    m.ww(h0(PAUSE_DOWN), s.keys.pause_down);
    m.ww(h0(ESC_DOWN), s.keys.esc_down);
    m.ww(h0(CHEAT), s.keys.cheat);
    m.ww(h0(PAUSED), s.paused);
    m.ww(h0(GIVE_UP), s.give_up);
    m.wl(h0(TICK), s.countdown_tick);
    m.ww(h0(EXIT_TOUCHED), s.level_exit_touched);
    m.wl(h0(COUNTDOWN), s.countdown);
    m.ww(h0(LIVES), s.lives);
    m.ww(h0(ITEMS_LEFT), s.items_left);
    m.ww(h0(CLXDAT_COPY), s.clxdat_copy);
    m.ww(h0(HITS_TAKEN), s.hits_taken);
    m.ww(h0(DYING), s.motion.dying);
    m.wb(h0(FALLING), s.motion.falling);
    m.wb(h0(UNHURT_T), s.motion.unhurt_t);
    m.wl(h0(RUN_LEFT), s.motion.run_left);
    m.wl(h0(RUN_RIGHT), s.motion.run_right);
    m.wb(h0(RISING), s.motion.rising);
    m.wb(h0(JUMPED), s.motion.jumped);
    m.wb(h0(JUMP_T), s.motion.jump_t);
    m.ww(h0(JUMP_3B28), s.motion.jump_3b28);
    m.wl(h0(CHAR_X), s.char_x);
    m.wl(h0(CHAR_Y), s.char_y);
    m.wl(h0(SCORE), s.score);
    m.wl(h0(SCORE_ADD), s.collect.score_add);
    let c = &s.edges.creatures;
    for i in 0..5u32 {
        m.wb(h0(SLOT_FLAGS + i), c.active[i as usize]);
        let v = c.slots[i as usize];
        let a = h0(SLOTS + 8 * i);
        m.ww(a, v.x);
        m.ww(a + 2, v.y);
        m.ww(a + 4, v.kind);
        m.wb(a + 6, v.facing);
        m.wb(a + 7, v.phase);
    }
}

/// Run `f` on the state loaded from `cpu` and store it.
fn on_play(cpu: &mut Cpu, f: impl FnOnce(&mut Play)) {
    let mut s = load(cpu);
    f(&mut Play { state: &mut s });
    store(cpu, &s);
}

fn countdown_step(cpu: &mut Cpu) {
    on_play(cpu, |p| p.countdown_step());
}

fn play_keys(cpu: &mut Cpu) {
    on_play(cpu, |p| p.play_keys());
}

fn help_key(cpu: &mut Cpu) {
    on_play(cpu, |p| p.help_key());
}

fn esc_key(cpu: &mut Cpu) {
    on_play(cpu, |p| p.esc_key());
}

fn lives_minus_one(cpu: &mut Cpu) {
    on_play(cpu, |p| p.lives_minus_one());
}

/// CLXDAT as the case reads it, the engine run with a scratch on the case's beam, and its
/// custom-chip writes handed to the rig after its memory.
fn char_hit_check(cpu: &mut Cpu) {
    let clxdat = cpu.mem.rw(CLXDAT);
    let mut scratch = Scratch {
        beam: cpu.mem.beam.get() as u8,
        ..Scratch::default()
    };
    on_play(cpu, |p| p.char_hit_check(clxdat, &mut scratch));
    cpu.mem.hw_writes.extend(scratch.hw);
}

/// The generator each routine runs on: T39's, and T38's for `char_hit_check`, its creature slots
/// active or not, so the stomp (T24) is held too.
fn generator(name: &str) -> Generator {
    generators::generator(name).unwrap()
}

/// Each routine: its generator's name (T39's, as in `re/translate.toml`), entry in hunk 0 and the
/// engine port's adapter.
fn routines() -> Vec<(&'static str, u32, fn(&mut Cpu))> {
    vec![
        ("countdown_step", 0x20b0, countdown_step),
        ("play_keys", 0x2104, play_keys),
        ("help_key", 0x2146, help_key),
        ("esc_key", 0x2176, esc_key),
        ("lives_minus_one", 0x69d6, lives_minus_one),
        ("char_hit_check", 0x3b2a, char_hit_check),
    ]
}

#[test]
fn every_death_routine_is_registered_and_checked_here() {
    // The routines `re/translate.toml` gives T23, and `help_key`, which `play_keys` runs whole (T26's
    // there; T23 ports it with its caller).
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../re/translate.toml");
    let text = std::fs::read_to_string(path).unwrap();
    let field = |block: &str, key: &str| {
        block.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
    };
    let mut t23: Vec<(String, u32)> = text
        .split("[[routine]]")
        .skip(1)
        .filter(|b| {
            field(b, "port_task").as_deref() == Some("T23")
                || field(b, "name").as_deref() == Some("help_key")
                // T70 ports its contact half; translate.toml gives it T24, which keeps the stomp.
                || field(b, "name").as_deref() == Some("char_hit_check")
        })
        .map(|b| {
            let off = field(b, "offset").unwrap();
            let off = u32::from_str_radix(off.trim_start_matches("0x"), 16).unwrap();
            (field(b, "name").unwrap(), off)
        })
        .collect();
    t23.sort();
    let mut here: Vec<(String, u32)> = routines().iter().map(|r| (r.0.to_string(), r.1)).collect();
    here.sort();
    assert_eq!(here, t23);
    for (name, off) in &t23 {
        assert!(
            PORTED
                .iter()
                .any(|p| p.name == name && p.hunk == 0 && p.offset == *off),
            "{name} is not in PORTED"
        );
    }
}

#[test]
fn every_death_routine_matches_the_original() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let mut lines = Vec::new();
    for (name, off, port) in routines() {
        let generator = generator(name);
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
        assert_eq!((rep.passed, rep.hangs, rep.faults), (CASES, 0, 0), "{name}");
        lines.push(format!("{name}: {} paths", rep.paths));
    }
    println!("{}", lines.join("\n"));
}

/// `lives_minus_one` without its test for none left: what a port that let lives wrap to 99 would do.
fn lives_minus_one_below_zero(cpu: &mut Cpu) {
    on_play(cpu, |p| {
        if p.state.lives == 0 {
            p.state.lives = 0x99;
        } else {
            p.lives_minus_one();
        }
    });
}

#[test]
fn losing_a_life_with_none_left_fails() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let generator = generators::generator("lives_minus_one").unwrap();
    let f = judge::check_regs(
        &mut r,
        "lives_minus_one",
        CASES,
        h0(0x69d6),
        &generator,
        RegsCompared::NONE,
        lives_minus_one_below_zero,
    )
    .unwrap_err();
    assert!(f.failures > 0, "{f}");
}

#[test]
fn the_generators_lay_no_base_and_load_store_round_trips() {
    // A store of what was loaded changes nothing, so the rig sees only the engine's own writes.
    let base = std::sync::Arc::new(vec![0u8; 1 << 24]);
    for (name, _, _) in routines() {
        let generator = generator(name);
        assert!(generator.base.is_none(), "{name}");
        let mut g = Gen::new(9);
        for _ in 0..200 {
            let s = (generator.case)(&mut g);
            let before = judge::Mem::new(base.clone(), &s.pokes);
            let mut cpu = Cpu {
                d: s.d,
                a: s.a,
                mem: before.clone(),
            };
            on_play(&mut cpu, |_| {});
            assert_eq!(cpu.mem.changed_since(&before), vec![], "{name}");
        }
    }
}

/// `char_hit_check` counting contact during the grace after a life (`char_unhurt_t` not 0): what a
/// port that missed the `tst.b h0+$9b04` would do.
fn hit_check_without_grace(cpu: &mut Cpu) {
    let clxdat = cpu.mem.rw(CLXDAT);
    let mut scratch = Scratch::default();
    on_play(cpu, |p| {
        let t = p.state.motion.unhurt_t;
        p.state.motion.unhurt_t = 0;
        p.char_hit_check(clxdat, &mut scratch);
        p.state.motion.unhurt_t = t;
    });
    cpu.mem.hw_writes.extend(scratch.hw);
}

#[test]
fn counting_contact_during_the_grace_fails() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let f = judge::check_regs(
        &mut r,
        "char_hit_check",
        CASES,
        h0(0x3b2a),
        &generator("char_hit_check"),
        RegsCompared::NONE,
        hit_check_without_grace,
    )
    .unwrap_err();
    assert!(f.failures > 0, "{f}");
}

/// `char_hit_check` stomping a creature already stomped (kind 7): what a port that missed the
/// `cmpi.w #7, $4(a0)` would do (T24).
fn hit_check_stomping_the_stomped(cpu: &mut Cpu) {
    let clxdat = cpu.mem.rw(CLXDAT);
    let mut scratch = Scratch::default();
    on_play(cpu, |p| {
        let c = &mut p.state.edges.creatures;
        let stomped: Vec<usize> = (0..5).filter(|&i| c.slots[i].kind == 7).collect();
        for &i in &stomped {
            c.slots[i].kind = 0;
        }
        p.char_hit_check(clxdat, &mut scratch);
        let c = &mut p.state.edges.creatures;
        for &i in &stomped {
            if c.slots[i].kind == 0 {
                c.slots[i].kind = 7;
            }
        }
    });
    cpu.mem.hw_writes.extend(scratch.hw);
}

#[test]
fn stomping_a_stomped_creature_fails() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let f = judge::check_regs(
        &mut r,
        "char_hit_check",
        CASES,
        h0(0x3b2a),
        &generator("char_hit_check"),
        RegsCompared::NONE,
        hit_check_stomping_the_stomped,
    )
    .unwrap_err();
    assert!(f.failures > 0, "{f}");
}
