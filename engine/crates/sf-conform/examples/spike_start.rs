//! Scratch (bug-recorder T00, deleted before the task ends): which way of starting a level with
//! the Start Level dialog's values gives the original's state (DESIGN D2).
//!
//! `spike_start key` checks the planted-snapshot key against an existing AtFrame case's.
//! `spike_start start <level> <lives 3|5|7> <mode 0|1|2> [frames] [script]`:
//! the original by power-on, options set, the code typed; the engine (b) the same way unseen,
//! (a) `Start::Level` + pokes. Diffs at frame 0, then the faithful runner over `frames` frames of
//! `script` (default `frog-run-right`) from the original's frame 0 with each engine swapped in.
#![allow(dead_code)]

use sf_assets::LevelId;
use sf_conform::{Alone, Case, Compare, InputScript, Mode, SyncPoint, display_data, sync};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::{AmigaMemory, LAYOUT, Ram, VarDef, h0, presentation};
use sf_core::{Game, Rng, Scene, Start};
use sf_oracle::{Amiga, OracleConfig, Snapshot};

const LOGIC_RUNNING: u32 = h0(0x9d6);
const LEVEL_STATE: u32 = h0(0xa06);
const WORLD: u32 = h0(0x40da);
const LEVEL: u32 = h0(0x40de);
const OPT_LIVES: u32 = h0(0x9e4);
const OPT_MODE: u32 = h0(0x9e6);
const RNG: u32 = h0(0x160d4);

fn word(m: &AmigaMemory, a: u32) -> u32 {
    (m.byte(a) as u32) << 8 | m.byte(a + 1) as u32
}
fn long(m: &AmigaMemory, a: u32) -> u32 {
    word(m, a) << 16 | word(m, a + 2)
}
fn in_level(m: &AmigaMemory) -> bool {
    word(m, LOGIC_RUNNING) != 0 && (long(m, LEVEL_STATE) as i32) < 0
}

/// (world as the original numbers it, level, password entry).
pub fn route(id: LevelId) -> (u32, u32, u32) {
    use sf_assets::World;
    match id.world {
        World::Bonus => (7, 1, 24),
        w => {
            let n = World::ALL.iter().position(|&x| x == w).unwrap() as u32;
            (n + 1, id.level as u32, 4 * n + id.level as u32 - 1)
        }
    }
}

/// Presses on the options screen from cursor LIVES: lives from `d_lives` to `lives`, down, mode
/// from `d_mode` to `mode`, down twice to LEVEL CODE. (frame offset, token) pairs, and the end.
fn option_presses(t0: u32, d: (u16, u16), want: (u16, u16)) -> (Vec<(u32, &'static str)>, u32) {
    let mut v = Vec::new();
    let mut t = t0;
    let mut change = |v: &mut Vec<(u32, &'static str)>, t: &mut u32, from: u16, to: u16| {
        let tok = if to > from { "R" } else { "L" };
        for _ in 0..from.abs_diff(to) {
            v.push((*t, tok));
            v.push((*t + 10, "-"));
            *t += 40;
        }
    };
    change(&mut v, &mut t, d.0, want.0);
    v.push((t, "D"));
    v.push((t + 10, "-"));
    t += 40;
    change(&mut v, &mut t, d.1, want.1);
    for _ in 0..2 {
        v.push((t, "D"));
        v.push((t + 10, "-"));
        t += 40;
    }
    (v, t)
}

/// The original's menu script: as `sync::drive_to`'s MENU, the options changed on the way.
pub fn original_script(entry: u32, d: (u16, u16), want: (u16, u16)) -> String {
    let mut s = String::new();
    for (t, k) in [(1000, "M"), (1005, "-"), (4600, "F"), (4605, "-"), (4650, "D"), (4655, "-"), (4700, "F"), (4705, "-")] {
        s += &format!("{t}: {k}\n");
    }
    let (presses, mut t) = option_presses(4800, d, want);
    for (f, k) in presses {
        s += &format!("{f}: {k}\n");
    }
    t += 60;
    for k in 0..6 {
        s += &format!("{t}: C{entry}.{k}\n{}: -\n", t + 6);
        t += 16;
    }
    s += &format!("{}: F\n{}: -\n", t + 40, t + 45);
    s += &format!("{}: F\n{}: -\n", t + 140, t + 145);
    s += &format!("{}: F\n{}: -\n", t + 200, t + 205);
    s
}

pub fn oracle(frames: u32) -> Amiga {
    let cfg = OracleConfig::locate(frames).unwrap().with_drives(1);
    Amiga::power_on(&cfg).unwrap()
}

/// From power-on to `id`'s frame 0 by `script`, each disk asked for put in. Returns the frames.
pub fn original_to(a: &mut Amiga, id: LevelId, script: &InputScript, show_opts: bool) -> u32 {
    let (w, l, _) = route(id);
    let mut disks: Option<sync::Disks> = None;
    for f in 0..20_000 {
        a.step(script.input_at(f)).unwrap();
        let m = sync::core_memory(a.memory());
        if show_opts && (f == 4790 || f == 5200) {
            eprintln!("  original f{f}: opt_lives {} opt_game_mode {}", word(&m, OPT_LIVES), word(&m, OPT_MODE));
        }
        if f == 5200 && std::env::var("SPIKE_POKE_HARD").is_ok() {
            a.write_memory(OPT_MODE, &[0, 2]);
        }
        let d = disks.get_or_insert_with(|| sync::Disks::at(&m));
        d.serve(a);
        if in_level(&m) && long(&m, WORLD) == w && long(&m, LEVEL) == l {
            return f + 1;
        }
    }
    panic!("{id} not reached");
}

/// The engine's (b): power-on, the options set and the code typed as a player would, unseen.
pub fn engine_b(id: LevelId, d: (u16, u16), want: (u16, u16), seed: Rng) -> Game {
    let display = display_data();
    let (_, _, entry) = route(id);
    let mut g = Game::with_display(display.clone(), Start::PowerOn { seed });
    let joy = |j: Joy| FrameInput { joy: j, ..FrameInput::NONE };
    let wait = |g: &mut Game| {
        for _ in 0..20_000 {
            if g.title_waits() {
                return;
            }
            g.step(FrameInput::NONE);
        }
        panic!("title never waits");
    };
    wait(&mut g);
    g.step(joy(Joy::FIRE));
    wait(&mut g);
    let mut script: Vec<(u32, FrameInput)> = vec![(0, joy(Joy::DOWN)), (5, FrameInput::NONE), (50, joy(Joy::FIRE)), (55, FrameInput::NONE)];
    let (presses, mut t) = option_presses(150, d, want);
    for (f, k) in presses {
        let i = match k {
            "R" => joy(Joy::RIGHT),
            "L" => joy(Joy::LEFT),
            "D" => joy(Joy::DOWN),
            _ => FrameInput::NONE,
        };
        script.push((f, i));
    }
    t += 60;
    let table = sf_core::front::password::table(&display.front);
    let _ = table;
    for k in 0..6 {
        let digit = display.front.h0_b(0x24dc + 10 * entry + k);
        let raw = if digit == b'0' { 0x0a } else { digit - b'0' };
        script.push((t, FrameInput { keys: sf_core::input::KeySet::key(raw), ..FrameInput::NONE }));
        script.push((t + 6, FrameInput::NONE));
        t += 16;
    }
    t += 40;
    let hard_at = t - 100;
    script.push((t, joy(Joy::FIRE)));
    script.push((t + 5, FrameInput::NONE));
    let mut held = FrameInput::NONE;
    for f in 0..t + 10 {
        if f == hard_at && std::env::var("SPIKE_POKE_HARD").is_ok() {
            g.poke(OPT_MODE, &[0, 2]);
        }
        if let Some(&(_, i)) = script.iter().find(|(at, _)| *at == f) {
            held = i;
        }
        g.step(held);
    }
    wait(&mut g);
    g.step(joy(Joy::FIRE));
    wait(&mut g);
    g.step(joy(Joy::FIRE));
    let mut n = 0;
    while !matches!(g.scene(), Scene::Level | Scene::Bonus(_)) {
        g.step(FrameInput::NONE);
        n += 1;
        assert!(n < 20_000);
    }
    g
}

/// (a): `Start::Level`, the options' values poked where `frog_new_game` and the options screen
/// leave them.
pub fn engine_a(id: LevelId, want: (u16, u16)) -> Game {
    let mut g = Game::with_display(display_data(), Start::Level(id));
    g.poke(OPT_LIVES, &want.0.to_be_bytes());
    g.poke(OPT_MODE, &want.1.to_be_bytes());
    let lives = 2 * want.0 + 2;
    g.poke(h0(0x82e), &lives.to_be_bytes());
    let v: [u16; 4] = match want.1 {
        0 => [0xfffb, 6, 2, 0xfffe],
        1 => [0xfffa, 4, 1, 0xfffe],
        _ => [0xfffa, 4, 0, 0xffff],
    };
    for (k, w) in v.iter().enumerate() {
        g.poke(h0(0x86e) + 2 * k as u32, &w.to_be_bytes());
    }
    g
}

/// The dialog's other four values, the same on every side.
pub fn pokes() -> Vec<(u32, Vec<u8>)> {
    let pts: u32 = std::env::var("SPIKE_POINTS").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let hearts: u16 = std::env::var("SPIKE_HEARTS").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let spud: u16 = std::env::var("SPIKE_SPUD").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let wings: u16 = std::env::var("SPIKE_WINGS").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut v = Vec::new();
    if pts != 0 {
        v.push((h0(0x834), pts.to_be_bytes().to_vec()));
        v.push((h0(0x880), ((pts / 200_000 + 1) * 200_000).to_be_bytes().to_vec()));
    }
    if std::env::var("SPIKE_EXIT").is_ok() {
        v.push((LEVEL_STATE, 1u32.to_be_bytes().to_vec()));
    }
    if hearts != 0 {
        v.push((h0(0x830), hearts.to_be_bytes().to_vec()));
    }
    if spud != 0 {
        v.push((h0(0x852), spud.to_be_bytes().to_vec()));
    }
    if wings != 0 {
        v.push((h0(0x866), wings.to_be_bytes().to_vec()));
    }
    v
}

pub fn diffs(theirs: &AmigaMemory, game: &Game) -> Vec<(&'static str, usize, u32, u32, bool)> {
    let pieces: Vec<VarDef> = LAYOUT.iter().flat_map(|d| d.pieces()).collect();
    let mut out: Vec<(&'static str, usize, u32, u32, bool)> = Vec::new();
    for (p, (id, ours)) in pieces.iter().zip(game.probe()) {
        let t = p.read(theirs as &dyn Ram);
        if t != ours {
            match out.last_mut() {
                Some(l) if l.0 == p.name => l.1 += 1,
                _ => out.push((p.name, 1, t, ours, presentation(id).is_some())),
            }
        }
    }
    out
}

/// `sync::cache_key` as the crate computes it (pub(crate) there).
pub fn cache_key(drives: u8, what: &str) -> String {
    let cfg = OracleConfig::locate(1).unwrap();
    let file = |p: &std::path::PathBuf| sf_disk::sha1_hex(&std::fs::read(p).unwrap());
    let mut text = format!("sync format 1\nvamiga {}\nkickstart {}\n", sf_oracle::BUILD, file(&cfg.kickstart));
    for d in &cfg.disks {
        text += &format!("disk {}\n", file(d));
    }
    if drives != 1 {
        text += &format!("drives {drives}\n");
    }
    text += what;
    sf_disk::sha1_hex(text.as_bytes())
}

fn cache_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").unwrap()).join(".cache/superfrog-re/sync")
}

/// The key `SyncPoint::AtFrame { script: name, frame, df1: false }` reads its snapshot under.
pub fn at_frame_key(name: &str, frame: u32) -> String {
    let s = InputScript::load(&sf_conform::case::script_path(name)).unwrap();
    cache_key(1, &format!("frame {frame} script\n{}", s.to_oracle().canonical()))
}

/// Plant `snap` where `AtFrame { script: name, frame }` finds it.
pub fn plant(name: &str, snap: &Snapshot) -> u32 {
    let key = at_frame_key(name, snap.frame);
    let mut b = b"SFSYNC\0\0".to_vec();
    b.extend(key.as_bytes());
    b.extend(snap.frame.to_le_bytes());
    b.push(snap.input.joy.bits());
    b.extend(snap.input.keys.0.to_le_bytes());
    b.extend(&snap.bytes);
    std::fs::create_dir_all(cache_dir()).unwrap();
    std::fs::write(cache_dir().join(format!("{key}.sync")), b).unwrap();
    snap.frame
}

pub fn case(name: &str, frame: u32, script: InputScript, frames: u32) -> Case {
    Case {
        name: format!("spike {name}"),
        sync: SyncPoint::AtFrame { script: name.to_string(), frame, df1: false },
        script,
        frames,
        compare: Compare { vars: true, picture: true, paula: true, render: false },
        exclude: Vec::new(),
        only_vars: None,
        resync: true,
        lag_limit: None,
        mode: Mode::Faithful,
        alone: Alone::Off,
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match a[0].as_str() {
        "key" => {
            let k = at_frame_key("boot-to-title", 4410);
            println!("{k} exists: {}", cache_dir().join(format!("{k}.sync")).exists());
        }
        "start" => start(&a[1..]),
        // The same script from the existing sync point, the engine transplanted there.
        "plain" => {
            let id: LevelId = a[1].parse().unwrap();
            let mut c = case("x", 0, InputScript::load(&sf_conform::case::script_path(&a[2])).unwrap(), a[3].parse().unwrap());
            c.sync = SyncPoint::AtLevel(id);
            match sf_conform::run(&c) {
                Ok(o) => println!("{id} plain {}: GREEN late {} resynced {}", a[2], o.late, o.resynced.len()),
                Err(d) => println!("{id} plain {}: DIVERGES\n{d}", a[2]),
            }
        }
        _ => panic!("key | start"),
    }
}

fn start(a: &[String]) {
    let id: LevelId = a[0].parse().unwrap();
    let lives: u16 = match a[1].as_str() {
        "3" => 0,
        "5" => 1,
        "7" => 2,
        _ => panic!("lives 3|5|7"),
    };
    let mode: u16 = a[2].parse().unwrap();
    let frames: u32 = a.get(3).map_or(2000, |v| v.parse().unwrap());
    let play = a.get(4).cloned().unwrap_or("frog-run-right".into());
    let want = (lives, mode);
    let d = (0u16, 1u16);
    let (_, _, entry) = route(id);
    let text = original_script(entry, d, want);
    let tag = format!("spike-start-{id}-{}-{mode}", a[1]);
    let path = sf_conform::case::script_path(&tag);
    std::fs::write(&path, &text).unwrap();
    let script = InputScript::parse(&text).unwrap();
    let t0 = std::time::Instant::now();
    let mut amiga = oracle(30_000);
    let n = original_to(&mut amiga, id, &script, true);
    let theirs0 = sync::core_memory(amiga.memory());
    eprintln!(
        "{id} lives {} mode {mode}: original frame 0 at {n} ({:.0?}); opt_lives {} opt_game_mode {} frog_lives {}",
        a[1], t0.elapsed(), word(&theirs0, OPT_LIVES), word(&theirs0, OPT_MODE), word(&theirs0, h0(0x82e))
    );
    let mut b = engine_b(id, d, want, Rng::default());
    let mut ga = engine_a(id, want);
    for (addr, bytes) in pokes() {
        amiga.write_memory(addr, &bytes);
        b.poke(addr, &bytes);
        ga.poke(addr, &bytes);
    }
    // The original takes the engine's random state (the product's direction); (a) takes (b)'s so
    // both compare against one original.
    let rng: Vec<u8> = (0..8).map(|k| b.state().b(RNG + k)).collect();
    amiga.write_memory(RNG, &rng);
    ga.poke(RNG, &rng);
    // What the title's time decides (flow.rs TITLE_TIME): the engine's into the original, as the
    // random state; (a) given (b)'s too.
    if std::env::var("SPIKE_NO_TITLE").is_err() {
        let extra = std::env::var("SPIKE_COPY").unwrap_or_default();
        let names: Vec<&str> = ["opt_flash", "password_buf", "sfx_chan0", "kbd_toggle", "title_blink", "title_menu"]
            .into_iter()
            .chain(extra.split(',').filter(|s| !s.is_empty()))
            .collect();
        for name in names {
            let d = LAYOUT.iter().find(|d| d.name == name).unwrap();
            let bytes: Vec<u8> = (0..d.width.len()).map(|k| b.state().b(d.addr + k)).collect();
            amiga.write_memory(d.addr, &bytes);
            ga.poke(d.addr, &bytes);
        }
    }
    let theirs = sync::core_memory(amiga.memory());
    for (name, g) in [("b", &b), ("a", &ga)] {
        let d = diffs(&theirs, g);
        let gp: Vec<_> = d.iter().filter(|v| !v.4).collect();
        println!("{id} L{} M{mode} ({name}) frame 0: {} vars differ, {} gameplay:", a[1], d.len(), gp.len());
        for v in &d {
            println!("    {}{} ({}): original {:x}, engine {:x}", if v.4 { "[pres] " } else { "" }, v.0, v.1, v.2, v.3);
        }
    }
    if frames == 0 {
        return;
    }
    let frame = plant(&tag, &amiga.snapshot());
    let play_script = InputScript::load(&sf_conform::case::script_path(&play)).unwrap();
    for (name, g) in [("b", b), ("a", ga)] {
        let c = case(&tag, frame, play_script.clone(), frames);
        let t0 = std::time::Instant::now();
        let mut last = None;
        let r = sf_conform::run_with(&c, |f, game| {
            if f == 0 {
                *game = g.clone();
            }
            last = Some((game.progress(), game.scene()));
        });
        println!("  last: {last:?}");
        match r {
            Ok(o) => println!(
                "{id} L{} M{mode} ({name}) {play} {frames} frames: GREEN ({:.0?}) late {} resynced {} straddled {}",
                a[1], t0.elapsed(), o.late, o.resynced.len(), o.straddled.len()
            ),
            Err(dv) => println!("{id} L{} M{mode} ({name}) {play} {frames} frames: DIVERGES ({:.0?})\n{dv}", a[1], t0.elapsed()),
        }
    }
}
