//! What `translated_whole.rs` and `translated_short.rs` share (translator T40, DESIGN §2.10): the
//! translated app played as a person plays it, from launch, by key presses, its state equal to a
//! headless run every frame.
//!
//! A headless run of the translated program (`fd_conform`'s `TranslatedEngine`, level 1 mounted, as
//! the app runs it) decides the inputs as it goes, [`plan`]: fire at the disk 2 prompt, fire held
//! ten steps in twenty on the title and the SCENA 1 screen until level 1 begins, the one route
//! (`fd-plan/tests/routes/W1-1.txt`) from the step after, and, for the whole run, fire likewise on
//! world 2's SCENA screen after its load until play goes on, then [`WORLD_2_PLAY`] steps of play.
//! The program is deterministic, so the app fed the same inputs as key presses must step through
//! the same states: [`play_in_the_app`] runs it and holds every step's state hash to the plan's.
#![allow(dead_code)]

use std::collections::BTreeMap;

use amiga_input::{FrameInput, Joy};
use fd_conform::FrameEngine;
use fd_conform::translated_engine::TranslatedEngine;
use fd_e2e::{App, Captured, Image, LaunchOpts};
use fd_translated::TranslatedGame;

use crate::common;

/// Chip RAM's size, where slow RAM starts in the engine's memory.
const CHIP: usize = 512 << 10;
/// Where slow RAM starts in the A500's address space.
const SLOW_BASE: usize = 0xC0_0000;

/// Fire at the disk 2 prompt, as `translated_app.rs` presses it: steps 20 to 29.
const PROMPT: std::ops::Range<u64> = 20..30;
/// Where the title is up after it (from about step 184, T37), and the first Start press.
pub const TITLE: u64 = 300;
/// World 2's load and SCENA screen: fire is first pressed this many steps after the level's end.
const AFTER_THE_END: u64 = 50;
/// Steps of world 2's play after its first pass.
pub const WORLD_2_PLAY: u64 = 300;
/// Seatbelt: the steps waiting for a screen to take fire, title or SCENA 2, before the plan fails.
const WAIT_LIMIT: u64 = 3_000;
/// Steps on the clock at the run's end, so the window presents and its shot has a frame.
pub const PACED: u64 = 60;

/// Fire held ten steps in twenty from step `from` on.
fn pulse(n: u64, from: u64) -> bool {
    n >= from && (n - from) % 20 < 10
}

/// A variable of `fd_core::LAYOUT` in RAM as the engine holds it, chip then slow.
pub fn var(m: &[u8], name: &str) -> u32 {
    let v = fd_core::layout_var(name).expect("in LAYOUT");
    let at = fd_core::HUNK_BASES[v.hunk as usize] + v.offset;
    (0..v.width.bytes() as u32).fold(0, |acc, k| {
        let a = ((at + k) & 0xFF_FFFF) as usize;
        let i = match a.checked_sub(SLOW_BASE) {
            Some(s) => CHIP + s,
            None => a,
        };
        acc << 8 | m[i] as u32
    })
}

pub fn require_tree() {
    assert!(
        TranslatedGame::generated(),
        "no translated program built in: run `python3 -I games/forest-dumb/tools/translate.py --build`"
    );
}

/// What the headless run did, step `n` (from 1) at index `n - 1`.
pub struct Plan {
    pub inputs: Vec<FrameInput>,
    /// `fd_app::memory_hash` after each step, as the app's `hashes.txt` writes it.
    pub hashes: Vec<String>,
    /// The step level 1 begins on (`fd_app::in_level_1` first true), as the app reports it.
    pub begins: u64,
    /// The route's steps played.
    pub route: u64,
    /// The step `world_index` became 1: the level's end (`next_world`).
    pub end: Option<u64>,
    /// The first step of world 2's play: `irq_count` counting again after the SCENA screen.
    pub world_2: Option<u64>,
    /// `lives` after each step of the route.
    pub lives: Vec<u32>,
    /// The pictures of the last [`PACED`] steps, as the app shows them, by step.
    pub pictures: BTreeMap<u64, Image>,
}

impl Plan {
    pub fn steps(&self) -> u64 {
        self.inputs.len() as u64
    }
}

/// The headless run's picture as the app shows it.
fn picture(e: &TranslatedEngine) -> Image {
    let p = fd_app::canvas(&FrameEngine::picture(e).expect("a frame has run"));
    let mut rgba = vec![0; p.width as usize * p.height as usize * 4];
    fd_core::render::rgba(&p, &mut rgba);
    // As a window shot holds it (`common::picture`).
    for px in rgba.chunks_exact_mut(4) {
        px[3] = 255;
    }
    Image {
        width: p.width as u32,
        height: p.height as u32,
        rgba,
    }
}

/// The headless run: through the title into level 1, `route_steps` of the one route, and with
/// `world_2` on to world 2's play for [`WORLD_2_PLAY`] steps.
pub fn plan(route_steps: usize, world_2: bool) -> Plan {
    require_tree();
    let route = common::route();
    assert!(
        route_steps <= route.len(),
        "the route has {} steps",
        route.len()
    );
    let disks = fd_conform::disks::load().unwrap_or_else(|e| panic!("{e}"));
    let mut e = TranslatedEngine::power_on(&disks).unwrap_or_else(|e| panic!("{e}"));
    e.mount_level_1(common::data());
    let mut p = Plan {
        inputs: Vec::new(),
        hashes: Vec::new(),
        begins: 0,
        route: route_steps as u64,
        end: None,
        world_2: None,
        lives: Vec::new(),
        pictures: BTreeMap::new(),
    };
    let mut kept = Vec::new();
    let mut step = |e: &mut TranslatedEngine, p: &mut Plan, i: FrameInput| -> Vec<u8> {
        e.frame(i);
        let n = p.inputs.len() as u64 + 1;
        assert_eq!(e.stopped(), None, "the headless run stopped at step {n}");
        p.inputs.push(i);
        let m = e.ram().expect("a frame has run");
        p.hashes.push(format!(
            "{:016x}",
            fd_app::memory_hash(&m[..CHIP], &m[CHIP..])
        ));
        kept.push((n, picture(e)));
        if kept.len() > PACED as usize {
            kept.remove(0);
        }
        m.to_vec()
    };
    let fire = FrameInput::joy(Joy::FIRE);
    // The prompt, the title and SCENA 1, until play begins.
    loop {
        let n = p.inputs.len() as u64 + 1;
        assert!(n < TITLE + WAIT_LIMIT, "level 1 never began");
        let i = if PROMPT.contains(&n) || pulse(n, TITLE) {
            fire
        } else {
            FrameInput::NONE
        };
        let m = step(&mut e, &mut p, i);
        if fd_app::in_level_1(&m[..CHIP], &m[CHIP..]) {
            p.begins = n;
            break;
        }
    }
    for &i in &route[..route_steps] {
        let m = step(&mut e, &mut p, i);
        p.lives.push(var(&m, "lives"));
        if p.end.is_none() && var(&m, "world_index") != 0 {
            p.end = Some(p.inputs.len() as u64);
        }
    }
    if world_2 {
        let end = p.end.expect("the route ends the level");
        let mut irq = None;
        loop {
            let n = p.inputs.len() as u64 + 1;
            assert!(n < end + WAIT_LIMIT, "world 2's play never began");
            let i = if pulse(n, end + AFTER_THE_END) {
                fire
            } else {
                FrameInput::NONE
            };
            let m = step(&mut e, &mut p, i);
            assert_eq!(var(&m, "world_index"), 1, "step {n}: in world 2");
            // `irq_count` stands still from the level's end through world 2's load and SCENA screen.
            let count = var(&m, "irq_count");
            if *irq.get_or_insert(count) != count {
                p.world_2 = Some(n);
                break;
            }
        }
        for _ in 1..WORLD_2_PLAY {
            step(&mut e, &mut p, FrameInput::NONE);
        }
    }
    p.pictures = kept.into_iter().collect();
    p
}

/// The host key each joystick bit is on in the app's default keys, as `common::key_lines`.
const KEYS: [(Joy, &str); 5] = [
    (Joy::UP, "Space"),
    (Joy::DOWN, "ArrowDown"),
    (Joy::LEFT, "ArrowLeft"),
    (Joy::RIGHT, "ArrowRight"),
    (Joy::FIRE, "AltLeft"),
];

/// `inputs` (step `n` at `n - 1`) as script lines on the default keys: a key goes down or up
/// where its bit changes, landing before the step that reads it.
pub fn key_lines(inputs: &[FrameInput]) -> String {
    let mut out = String::new();
    let mut held = Joy::NONE;
    for (k, j) in inputs.iter().map(|i| i.joy).chain([Joy::NONE]).enumerate() {
        for (bit, key) in KEYS {
            let (was, is) = (held.0 & bit.0 != 0, j.0 & bit.0 != 0);
            if was != is {
                let how = if is { "down" } else { "up" };
                out.push_str(&format!("{}: {how} {key}\n", k + 1));
            }
        }
        held = j;
    }
    out
}

/// The app's sound by step: `audio.wav`'s samples cut at `audio-frames.txt`'s counts.
pub fn sound(c: &Captured) -> BTreeMap<u64, Vec<[i16; 2]>> {
    let wav = std::fs::read(&c.audio_wav).expect("audio.wav");
    // The capture's WAV is the canonical 44-byte header (amiga_app::capture::wav), then the data.
    assert_eq!(&wav[..4], b"RIFF");
    let mut samples = wav[44..].chunks_exact(4).map(|s| {
        [
            i16::from_le_bytes([s[0], s[1]]),
            i16::from_le_bytes([s[2], s[3]]),
        ]
    });
    let frames = std::fs::read_to_string(c.dir.join("audio-frames.txt")).expect("audio-frames.txt");
    frames
        .lines()
        .map(|l| {
            let mut w = l.split(' ').map(|v| v.parse::<u64>().unwrap());
            let n = w.next().unwrap();
            let count: u64 = w.sum();
            (n, samples.by_ref().take(count as usize).collect())
        })
        .collect()
}

/// Whether any step from `from` to `to` is heard.
pub fn heard(s: &BTreeMap<u64, Vec<[i16; 2]>>, from: u64, to: u64) -> bool {
    s.range(from..=to)
        .any(|(_, v)| v.iter().any(|&[l, r]| l != 0 || r != 0))
}

/// The plan played in the real app by key presses, unpaced but for its last [`PACED`] steps, a
/// shot of the window half-way through them; every step's input and state held to the plan's,
/// the shot to its picture. The app's sound by step ([`sound`]), read before the app's home goes.
pub fn play_in_the_app(p: &Plan, timeout_s: u64) -> BTreeMap<u64, Vec<[i16; 2]>> {
    let steps = p.steps();
    let shot = steps - PACED / 2;
    let mut app = App::launch_translated(LaunchOpts {
        max_frames: steps,
        timeout_s,
        extra_args: vec!["--capture-hashes".into()],
        ..Default::default()
    });
    let c = app.run_script_unread(&format!(
        "fast-until {}\n{shot}: shot\n{}",
        steps - PACED,
        key_lines(&p.inputs)
    ));
    assert!(c.exit.success(), "{}", c.stderr);
    assert!(
        c.stderr
            .contains("fd-app: playing the translated program from power-on"),
        "{}",
        c.stderr
    );
    assert!(!c.stderr.contains("stopped at step"), "{}", c.stderr);
    assert!(
        c.stderr
            .contains(&format!("fd-app: level 1 begins at step {}\n", p.begins)),
        "level 1 begins at step {} headless: {}",
        p.begins,
        c.stderr
    );
    let fed = common::inputs(&c);
    let hashes = common::hashes(&c);
    assert_eq!(
        hashes.len() as u64,
        steps + 1,
        "a hash for the start and every step"
    );
    for n in 1..=steps {
        let i = fed.get(&n).copied().unwrap_or(FrameInput::NONE);
        assert_eq!(i, p.inputs[n as usize - 1], "the input step {n} saw");
        assert_eq!(
            hashes[n as usize],
            p.hashes[n as usize - 1],
            "the state after step {n}"
        );
    }
    let (w, h) = (fd_app::CANVAS.2 as u32, fd_app::CANVAS.3 as u32);
    let (frame, window) = common::shown(&c, shot, w, h);
    let want = p
        .pictures
        .get(&frame)
        .unwrap_or_else(|| panic!("the shot at step {shot} shows frame {frame}, not a paced one"));
    assert!(
        window == *want,
        "the window at step {shot} shows frame {frame} as headless"
    );
    let s = sound(&c);
    assert_eq!(s.len() as u64, steps, "a line of audio-frames.txt a step");
    s
}
