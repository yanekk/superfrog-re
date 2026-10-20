//! Scratch (bug-recorder T00, deleted before the task ends): DESIGN §2.9. Inputs recorded per
//! step from the unpaced engine (from the (b) start, `spike_start`), retimed onto the original's
//! frames, fed to it; how far the original's run stays on the recorded one, and the faithful
//! runner's verdict over the whole stretch.
//!
//! `spike_retime <chain|slot|w43> <frame|pass> [legs N]`
#![allow(dead_code)]

#[path = "spike_start.rs"]
mod start;

use sf_assets::LevelId;
use sf_conform::{InputScript, sync};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::{AmigaMemory, LAYOUT, Ram, h0};
use sf_core::{Game, LoadEnd, PassClock, Rng, Scene};
use sf_oracle::Amiga;

const VBL_COUNT: u32 = h0(0x9d8);

/// What the route is judged by, each pass end.
const WATCH: [(&str, u32, u32); 8] = [
    ("frog_x", h0(0x7d6), 2),
    ("frog_y", h0(0x7d8), 2),
    ("score", h0(0x834), 4),
    ("frog_lives", h0(0x82e), 2),
    ("frog_hearts", h0(0x830), 2),
    ("world", h0(0x40da), 4),
    ("level", h0(0x40de), 4),
    ("level_state", h0(0xa06), 4),
];

fn read(get: &dyn Fn(u32) -> u8, a: u32, n: u32) -> u32 {
    (0..n).fold(0, |v, k| v << 8 | get(a + k) as u32)
}

fn watch(get: &dyn Fn(u32) -> u8) -> [u32; 8] {
    WATCH.map(|(_, a, n)| read(get, a, n))
}

struct Rec {
    inputs: Vec<FrameInput>,
    /// Passes ended before step n (cumulative; one a frame outside a level).
    before: Vec<u64>,
    load: Vec<Option<LoadEnd>>,
    /// The watched state at the end of each step.
    state: Vec<[u32; 8]>,
    scene: Vec<Scene>,
}

fn record(mut g: Game, steps: u32, mut input: impl FnMut(u32, &Game) -> FrameInput) -> Rec {
    let mut r = Rec { inputs: vec![], before: vec![], load: vec![], state: vec![], scene: vec![] };
    let mut c = 0u64;
    for n in 0..steps {
        let i = input(n, &g);
        r.before.push(c);
        let out = g.step(i);
        c += g.passes_ended() as u64;
        r.inputs.push(i);
        r.load.push(out.load);
        r.state.push(watch(&|a| g.state().b(a)));
        r.scene.push(g.scene());
    }
    r
}

fn script_of(lines: &[(u32, &str)]) -> InputScript {
    let text: String = lines.iter().map(|(f, t)| format!("{f}: {t}\n")).collect();
    InputScript::parse(&text).unwrap()
}

/// The original's load over, as the runner decides it.
fn load_over(a: &Amiga, end: LoadEnd, seen: &mut bool) -> bool {
    let m = sync::core_memory(a.memory());
    let chain = sync::main_chain_deep(&a.cpu(), &m, usize::MAX);
    match end {
        LoadEnd::Reached(x) => chain.iter().take(4).any(|&c| c == x),
        LoadEnd::Left(x) => {
            let inside = chain.contains(&x);
            *seen |= inside;
            *seen && !inside
        }
    }
}

struct Fed {
    /// The original's input each frame.
    frames: Vec<FrameInput>,
    /// (original frame, engine step it was fed) per frame.
    of_step: Vec<u32>,
    first_pass_diff: Option<String>,
    first_step_diff: Option<String>,
    max_dx: (u32, u32, u64),
    load_frames: Vec<(u32, u32)>,
    passes_o: u64,
    seen: Vec<[u32; 8]>,
}

/// Feed `rec` to the original, one engine step per original frame (`by_pass` false) or the
/// engine's pass k's steps over the original's pass k's frames, the last held while it lags.
fn feed(a: &mut Amiga, rec: &Rec, by_pass: bool, legs: Option<u64>, g0: &Game) -> Fed {
    let _ = (legs, g0);
    let mut fed = Fed {
        frames: vec![],
        of_step: vec![],
        first_pass_diff: None,
        first_step_diff: None,
        max_dx: (0, 0, 0),
        load_frames: vec![],
        passes_o: 0,
        seen: vec![],
    };
    let steps = rec.inputs.len() as u32;
    let is_level = |sc: Scene| matches!(sc, Scene::Level | Scene::Bonus(_));
    let is_slot = |sc: Scene| matches!(sc, Scene::Slot { .. });
    // The engine's anchors: (step after which the event has happened, kind 0 level frame 0, 1 slot).
    let mut anchors: Vec<(u32, u8)> = Vec::new();
    for k in 1..steps as usize {
        if is_level(rec.scene[k]) && !is_level(rec.scene[k - 1]) {
            anchors.push((k as u32, 0));
        }
        if is_slot(rec.scene[k]) && !is_slot(rec.scene[k - 1]) {
            anchors.push((k as u32, 1));
        }
    }
    eprintln!("engine anchors: {anchors:?}");
    let use_anchors = std::env::var("SPIKE_NO_ANCHOR").is_err();
    let mut next = 0usize;
    let mut clock = PassClock::default();
    // The engine step the original's current pass began on, and frames into it.
    let mut pass_first = 0u32;
    let mut j = 0u32;
    let mut n = 0u32;
    let mut disks = sync::Disks::at(&sync::core_memory(a.memory()));
    let mut was = (true, false);
    let mut held = 0u32;
    let mut cop: Option<u32> = None;
    let use_loads = std::env::var("SPIKE_NO_LOADS").is_err();
    while n < steps {
        let pending = use_anchors && next < anchors.len() && anchors[next].0 + 1 == n;
        let step_for_frame = if pending {
            n - 1
        } else if by_pass && is_level(rec.scene[n as usize]) {
            // Pass k's steps over the original's pass k: the engine's pass begun at `pass_first`
            // ends at the first step with passes_ended > 0 from there.
            let end = (pass_first as usize..steps as usize)
                .find(|&k| rec.before.get(k + 1).is_some_and(|&c| c > rec.before[k]))
                .unwrap_or(steps as usize - 1) as u32;
            (pass_first + j).min(end)
        } else {
            n
        };
        let input = if pending { FrameInput::NONE } else { rec.inputs[step_for_frame as usize] };
        a.step(input).unwrap();
        cop = sync::cop1lc(cop, &a.cpu_writes());
        disks.serve(a);
        fed.frames.push(input);
        fed.of_step.push(step_for_frame);
        // The engine loaded at once on this step: the original's load runs on, nothing pressed;
        // the frames it spends under a screen it times itself are play (the runner's rule), so the
        // engine steps that many on.
        let mut under = 0;
        if !pending && use_loads && let Some(end) = rec.load[step_for_frame as usize] {
            let mut seen = false;
            let mut k = 0;
            while !load_over(a, end, &mut seen) {
                a.step(FrameInput::NONE).unwrap();
                cop = sync::cop1lc(cop, &a.cpu_writes());
                disks.serve(a);
                fed.frames.push(FrameInput::NONE);
                fed.of_step.push(step_for_frame);
                if cop.is_some_and(sf_conform::timed_screen) {
                    under += 1;
                }
                k += 1;
                assert!(k < 20_000);
            }
            eprintln!("load at step {step_for_frame}: original {k} frames, {under} under a timed screen");
        }
        let m = sync::core_memory(a.memory());
        let cpu = a.cpu();
        let vbl = (m.byte(VBL_COUNT) as u16) << 8 | m.byte(VBL_COUNT + 1) as u16;
        let ended = clock.frame(sync::in_level_wait(&cpu, &m), sync::mid_pass(&cpu, &m), vbl) as u64;
        let now = (start_in_level(&m), sync::main_chain(&cpu, &m).contains(&sync::IN_SLOT_MACHINE));
        let rose = if now.0 && !was.0 {
            Some(0u8)
        } else if now.1 && !was.1 {
            Some(1u8)
        } else {
            None
        };
        was = now;
        let ours_frame = watch(&|x| m.byte(x));
        fed.seen.push(ours_frame);
        if let Some(kind) = rose.filter(|_| use_anchors)
            && let Some(i) = (next..anchors.len()).find(|&i| anchors[i].1 == kind)
        {
            let s = anchors[i].0;
            eprintln!(
                "anchor {}: original frame {} (held {held}); engine step {s}, {} was being fed",
                if kind == 0 { "level frame 0" } else { "slot" },
                fed.frames.len() - 1,
                step_for_frame
            );
            fed.load_frames.push((fed.frames.len() as u32 - 1, s));
            next = i + 1;
            n = s + 1;
            pass_first = n;
            j = 0;
            held = 0;
            clock = PassClock::default();
            continue;
        }
        if pending {
            held += 1;
            assert!(held < 5000, "the original never met the engine's anchor at step {}", n - 1);
            continue;
        }
        if by_pass && is_level(rec.scene[n as usize]) {
            if ended > 0 {
                // The original ended its pass: the engine's next pass.
                let end = (pass_first as usize..steps as usize)
                    .find(|&k| rec.before.get(k + 1).is_some_and(|&c| c > rec.before[k]))
                    .unwrap_or(steps as usize - 1) as u32;
                pass_first = end + 1;
                j = 0;
                n = pass_first;
            } else {
                j += 1;
                n = step_for_frame + 1;
            }
        } else {
            n += 1 + under;
            pass_first = n;
            j = 0;
        }
        let e = rec.state[step_for_frame as usize];
        if !now.0 {
            continue;
        }
        if ended > 0 {
            let dx = ours_frame[0].abs_diff(e[0]) + ours_frame[1].abs_diff(e[1]);
            if dx > fed.max_dx.0 {
                fed.max_dx = (dx, fed.frames.len() as u32 - 1, step_for_frame as u64);
            }
            if fed.first_pass_diff.is_none()
                && let Some(k) = (2..8).find(|&k| e[k] != ours_frame[k])
            {
                fed.first_pass_diff = Some(format!(
                    "original frame {} (engine step {step_for_frame}): {} original {:x} engine {:x}",
                    fed.frames.len() - 1, WATCH[k].0, ours_frame[k], e[k]
                ));
            }
        }
        if fed.first_pass_diff.as_deref().is_none_or(|d| !d.starts_with("frog"))
            && (e[0] != ours_frame[0] || e[1] != ours_frame[1])
        {
            eprintln!("first frog difference: original frame {} engine step {step_for_frame}: original ({:x},{:x}) engine ({:x},{:x})", fed.frames.len() - 1, ours_frame[0], ours_frame[1], e[0], e[1]);
            fed.first_pass_diff = Some(format!("frog at {}", fed.frames.len() - 1));
        }
        if fed.first_step_diff.is_none()
            && let Some(k) = (0..8).find(|&k| e[k] != ours_frame[k])
        {
            fed.first_step_diff = Some(format!(
                "original frame {} (engine step {step_for_frame}): {} original {:x} engine {:x}",
                fed.frames.len() - 1, WATCH[k].0, ours_frame[k], e[k]
            ));
        }
    }
    fed
}

/// The fallback (DESIGN §2.9): every `leg` passes of the original, the engine is re-synced from
/// the original's memory (at a pass end) and both play the recorded inputs, one a frame, for
/// the leg; at the leg's end (the original's next pass end after it) the watched variables are
/// compared, the engine having stepped as many frames, unpaced.
fn legs(snap: &sf_oracle::Snapshot, rec: &Rec, leg: u64, span: u32) {
    let mut a = start::oracle(60_000);
    a.restore(snap);
    let display = sf_conform::display_data();
    let mut clock = PassClock::default();
    let (mut bad, mut legs_run, mut worst) = (0u32, 0u32, (0u32, 0u32, String::new()));
    let mut first_bad: Option<String> = None;
    let mut f = 0u32;
    let mut engine: Option<(Game, u64)> = None;
    let mut passes = 0u64;
    while f < span {
        let m = sync::core_memory(a.memory());
        let cpu = a.cpu();
        let at_end = start_in_level(&m) && sync::in_level_wait(&cpu, &m) && !sync::mid_pass(&cpu, &m);
        if at_end {
            if let Some((g, from)) = &engine
                && passes >= from + leg
            {
                let ours = watch(&|x| g.state().b(x));
                let theirs = watch(&|x| m.byte(x));
                legs_run += 1;
                let dx = ours[0].abs_diff(theirs[0]) + ours[1].abs_diff(theirs[1]);
                let other = (2..8).find(|&k| ours[k] != theirs[k]);
                if dx > 0 || other.is_some() {
                    bad += 1;
                    let what = format!(
                        "leg ending frame {f}: frog dx+dy {dx}{}",
                        other.map_or(String::new(), |k| format!(", {} original {:x} engine {:x}", WATCH[k].0, theirs[k], ours[k]))
                    );
                    if first_bad.is_none() {
                        first_bad = Some(what.clone());
                    }
                    if dx > worst.0 {
                        worst = (dx, f, what);
                    }
                }
                engine = None;
            }
            if engine.is_none() {
                let mut g = Game::with_display(display.clone(), sf_core::Start::Transplant(m.clone()));
                g.set_blank_merged(None);
                engine = Some((g, passes));
            }
        }
        let input = rec.inputs[f.min(rec.inputs.len() as u32 - 1) as usize];
        a.step(input).unwrap();
        if let Some((g, _)) = engine.as_mut() {
            g.step(input);
        }
        let m = sync::core_memory(a.memory());
        let cpu = a.cpu();
        let vbl = (m.byte(VBL_COUNT) as u16) << 8 | m.byte(VBL_COUNT + 1) as u16;
        passes += clock.frame(sync::in_level_wait(&cpu, &m), sync::mid_pass(&cpu, &m), vbl) as u64;
        f += 1;
    }
    println!(
        "legs of {leg} passes over {span} frames: {legs_run} legs, {bad} differ; first {:?}; worst {}",
        first_bad, worst.2
    );
}

fn start_in_level(m: &AmigaMemory) -> bool {
    let w = |a: u32| (m.byte(a) as u32) << 8 | m.byte(a + 1) as u32;
    w(h0(0x9d6)) != 0 && ((w(h0(0xa06)) << 16 | w(h0(0xa08))) as i32) < 0
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let by_pass = a.get(1).map(|s| s == "pass").unwrap_or(false);
    let (id, lives, mode, exit): (LevelId, &str, u16, bool) = match a[0].as_str() {
        "chain" | "slot" => ("W1-1".parse().unwrap(), "5", 1, true),
        "w43" => ("W4-3".parse().unwrap(), "3", 1, false),
        "run" => ("W1-1".parse().unwrap(), "5", 1, false),
        _ => panic!(),
    };
    if exit {
        unsafe { std::env::set_var("SPIKE_EXIT", "1") };
    }
    // The (b) start: the original at frame 0 with the engine's leftovers written in.
    let want = (match lives { "3" => 0, "5" => 1, _ => 2 }, mode);
    let d = (0u16, 1u16);
    let (_, _, entry) = start::route(id);
    let text = start::original_script(entry, d, want);
    let tag = format!("spike-retime-{}", a[0]);
    std::fs::write(sf_conform::case::script_path(&tag), &text).unwrap();
    let mut amiga = start::oracle(40_000);
    start::original_to(&mut amiga, id, &InputScript::parse(&text).unwrap(), false);
    let mut b = start::engine_b(id, d, want, Rng::default());
    for (addr, bytes) in start::pokes() {
        amiga.write_memory(addr, &bytes);
        b.poke(addr, &bytes);
    }
    for name in ["rng_state", "opt_flash", "password_buf", "sfx_chan0", "kbd_toggle", "title_blink", "title_menu", "p41_channels", "exit_sprite_left", "exit_sprite_right", "exit_sprite_copper"] {
        let d = LAYOUT.iter().find(|d| d.name == name).unwrap();
        let bytes: Vec<u8> = (0..d.width.len()).map(|k| b.state().b(d.addr + k)).collect();
        amiga.write_memory(d.addr, &bytes);
    }
    let snap = amiga.snapshot();
    let frame = start::plant(&tag, &snap);

    // The recording: the engine unpaced, as the app plays.
    let rec = match a[0].as_str() {
        "chain" => {
            let s = script_of(&[(70, "F"), (74, "-")]);
            record(b.clone(), 1150, |n, _| s.input_at(n))
        }
        "slot" => {
            let s = script_of(&[(60, "U"), (70, "U F"), (74, "-")]);
            let gamble = InputScript::load(&sf_conform::case::script_path("slot-gamble")).unwrap();
            let l: u32 = std::env::var("SPIKE_SLOT_L").ok().and_then(|v| v.parse().ok()).unwrap_or(496);
            let mut s0: Option<u32> = None;
            let mut done: Option<u32> = None;
            let steps: u32 = std::env::var("SPIKE_STEPS").ok().and_then(|v| v.parse().ok()).unwrap_or(2600);
            let r = record(b.clone(), steps, |n, g| {
                if s0.is_none() && matches!(g.scene(), Scene::Slot { .. }) {
                    s0 = Some(n);
                    eprintln!("slot entered at step {n}");
                }
                if done.is_none() && s0.is_some() && g.progress().level == 2 && g.scene() == Scene::Level {
                    done = Some(n);
                    eprintln!("W1-2 at step {n}");
                }
                match s0 {
                    None => s.input_at(n),
                    Some(s0) => gamble.input_at((n - s0) + l),
                }
            });
            r
        }
        "run" => {
            let s = InputScript::load(&sf_conform::case::script_path("frog-run-right")).unwrap();
            record(b.clone(), 2000, |n, _| s.input_at(n))
        }
        _ => {
            let s = InputScript::load(&sf_conform::case::script_path("complete-w4-3")).unwrap();
            let steps: u32 = std::env::var("SPIKE_STEPS").ok().and_then(|v| v.parse().ok()).unwrap_or(9200);
            record(b.clone(), steps, |n, _| s.input_at(n))
        }
    };
    let last = rec.state.last().unwrap();
    println!(
        "recording: {} steps, {} passes; ends {:?} score {} lives {} world {} level {}",
        rec.inputs.len(), rec.before.last().unwrap(), rec.scene.last().unwrap(), last[2], last[3], last[5], last[6]
    );
    if let Some(i) = a.iter().position(|x| x == "legs") {
        let span: u32 = a.get(i + 2).map_or(3000, |v| v.parse().unwrap());
        for leg in a[i + 1].split(',').map(|v| v.parse::<u64>().unwrap()) {
            legs(&snap, &rec, leg, span);
        }
        return;
    }
    // Feed it.
    let mut fresh = start::oracle(60_000);
    fresh.restore(&snap);
    let fed = feed(&mut fresh, &rec, by_pass, None, &b);
    let m = sync::core_memory(fresh.memory());
    let o = watch(&|x| m.byte(x));
    println!(
        "fed {} by {}: {} original frames, anchors met {:?}; ends score {} lives {} world {} level {} state {:x}",
        a[0], if by_pass { "pass" } else { "frame" }, fed.frames.len(), fed.load_frames, o[2], o[3], o[5], o[6], o[7]
    );
    println!("  first milestone diff by pass: {:?}", fed.first_pass_diff);
    println!("  first watched diff by step fed: {:?}", fed.first_step_diff);
    if let Some(d) = &fed.first_step_diff {
        let f: usize = d.split_whitespace().nth(2).unwrap().parse().unwrap();
        for k in f.saturating_sub(4)..(f + 4).min(fed.frames.len()) {
            let s = fed.of_step[k] as usize;
            println!("    of {k} step {s}: original {:x?} engine {:x?} scene {:?}", fed.seen.get(k), rec.state[s], rec.scene[s]);
        }
    }
    println!("  largest frog |dx|+|dy| at a pass end: {} (original frame {}, engine step {})", fed.max_dx.0, fed.max_dx.1, fed.max_dx.2);
    // The faithful runner over the whole stretch, on the retimed script, the (b) engine paced.
    let mut lines: Vec<(u32, FrameInput)> = Vec::new();
    for (f, i) in fed.frames.iter().enumerate() {
        if lines.last().map(|l| l.1) != Some(*i) {
            lines.push((f as u32, *i));
        }
    }
    let script = InputScript { lines };
    let c = start::case(&tag, frame, script, rec.inputs.len() as u32);
    let t0 = std::time::Instant::now();
    let mut lastp = None;
    let r = sf_conform::run_with(&c, |f, game| {
        if f == 0 {
            *game = b.clone();
        }
        lastp = Some((game.progress(), game.scene()));
    });
    match r {
        Ok(o) => println!(
            "faithful: GREEN ({:.0?}) late {} resynced {} longest mid-pass {:?}; last {lastp:?}",
            t0.elapsed(), o.late, o.resynced.len(), o.longest_mid_pass
        ),
        Err(d) => println!("faithful: DIVERGES ({:.0?}); last {lastp:?}\n{d}", t0.elapsed()),
    }
    let _ = Joy::NONE;
    let _: Option<&AmigaMemory> = None;
}
