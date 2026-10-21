//! Experiment (not plan code): a whole-level Go-Explore on the engine, no map, no hints. An archive
//! of cells keyed by (frog cell / CELL, coins left / BAND, switches pressed, wings), each holding the
//! best state reaching it (most hearts, then fewest frames) and its inputs. Each round picks cells,
//! weighted to the less tried and the further on in coins, and plays random sticky joystick bursts
//! from them; the first state that leaves the level alive is the route.
//!
//! `cargo run --release -p sf-plan --example goexp -- <level> [--threads N] [--minutes M]
//!  [--seed S] [--out route.txt] [--follow script.txt]`
//!
//! `--follow`: seed the archive with every state along a committed script (a teacher route), the
//! way route repair would start from it; the search then explores from those states as well.

use std::collections::HashMap;
use std::time::Instant;

use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::h0;
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::{Game, LevelId, Start};
use sf_plan::core::map::pressed;
use sf_plan::core::{Facts, facts, frog_cell, landed};

const LEVEL_STATE: u32 = h0(0xa06);
const HEARTS: u32 = h0(0x830);
const COINS_LEFT: u32 = h0(0x832);

const CELL: u16 = 2;
const BAND: u16 = 4;

type Key = (u16, u16, u16, u32, bool);

struct Entry {
    game: Game,
    inputs: Vec<u8>,
    hearts: u16,
    chosen: u32,
    seen: u32,
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn key(fa: &Facts, g: &Game) -> Key {
    let st = g.state();
    let (c, r) = frog_cell(st);
    (
        c / CELL,
        r / CELL,
        // No coins left is its own band: from there the exit is the goal (`exit_pull`).
        match st.w(COINS_LEFT) {
            0 => 0,
            n => 1 + (n - 1) / BAND,
        },
        pressed(fa, st),
        st.w(FROG + f::FLIGHT) != 0,
    )
}

const ACTS: [Joy; 10] = [
    Joy::NONE,
    Joy::RIGHT,
    Joy::LEFT,
    Joy::UP,
    Joy(Joy::RIGHT.0 | Joy::UP.0),
    Joy(Joy::LEFT.0 | Joy::UP.0),
    Joy::DOWN,
    Joy::FIRE,
    Joy(Joy::RIGHT.0 | Joy::FIRE.0),
    Joy(Joy::LEFT.0 | Joy::FIRE.0),
];

enum Out {
    Cell(Key, Game, Vec<u8>, u16),
    Exit(Vec<u8>),
}

/// One burst from `g`: up to 16 holds of a random action (sticky), 4–32 frames each; reports the
/// state after each hold, or the exit.
fn burst(fa: &Facts, g0: &Game, inputs0: &[u8], rng: &mut Rng, frames: &mut u64) -> Vec<Out> {
    let mut g = g0.clone();
    let mut inputs = inputs0.to_vec();
    let st = g.state();
    let lives = st.w(FROG + f::LIVES);
    let hearts0 = st.w(HEARTS);
    let mut out = Vec::new();
    let holds = 1 + rng.below(16);
    let mut a = ACTS[rng.below(ACTS.len())];
    for _ in 0..holds {
        if rng.unit() > 0.6 {
            a = ACTS[rng.below(ACTS.len())];
        }
        let n = [4, 8, 16, 32][rng.below(4)];
        for _ in 0..n {
            g.step_unseen(FrameInput::joy(a));
            *frames += 1;
            inputs.push(a.0);
            let st = g.state();
            if (st.w(FROG + f::LIVES) as i16) < lives as i16 || st.w(FROG + f::DYING) != 0 {
                return out;
            }
            if st.w(HEARTS) < hearts0 {
                return out;
            }
            if st.l(LEVEL_STATE) != 0xffff_fffe {
                out.push(Out::Exit(inputs));
                return out;
            }
        }
        if inputs.len() > 30_000 {
            return out;
        }
        let st = g.state();
        out.push(Out::Cell(key(fa, &g), g.clone(), inputs.clone(), st.w(HEARTS)));
    }
    out
}

/// `--guide`: (coarse col, coarse row, coins band) → the furthest point along the guide path there.
static GUIDE: std::sync::OnceLock<HashMap<(u16, u16, u16), u32>> = std::sync::OnceLock::new();

/// The guide path: a route played on the original from the level's sync point, the frog's cell
/// and coins left every 8 frames, as `GUIDE`'s map (each point also marks its coarse neighbours).
fn guide_from(id: LevelId, path: &str) -> HashMap<(u16, u16, u16), u32> {
    let script = InputScript::load(std::path::Path::new(path)).unwrap();
    let n = script.lines.last().map_or(0, |l| l.0);
    let mut amiga = sync::reach(&SyncPoint::AtLevel(id), &InputScript::idle(), n + 200).unwrap().amiga;
    let mut m = HashMap::new();
    let mut i = 0u32;
    for k in 0..n {
        sync::step_inserting(&mut amiga, script.input_at(k)).unwrap();
        if k % 8 != 0 || k < 20 {
            continue;
        }
        let st = sf_core::State::transplant(&sync::core_memory(amiga.memory()));
        if st.l(LEVEL_STATE) != 0xffff_fffe {
            break;
        }
        let (c, r) = frog_cell(&st);
        let band = match st.w(COINS_LEFT) {
            0 => 0,
            n => 1 + (n - 1) / BAND,
        };
        let (cc, rr) = (c / CELL, r / CELL);
        for dc in [0, 1, 2] {
            for dr in [0, 1, 2] {
                let e = m.entry(((cc + dc).saturating_sub(1), (rr + dr).saturating_sub(1), band)).or_insert(i);
                *e = (*e).max(i);
            }
        }
        i += 1;
    }
    eprintln!("guide: {i} points from {path}");
    m
}

/// What one search found: the shortest exit's inputs from `start`, and its stats.
struct Found {
    exit: Option<Vec<u8>>,
    cells: usize,
    frames: u64,
    secs: f64,
}

/// The Go-Explore loop from `start`. `teacher`: inputs to replay from `start` first, every 8th
/// state seeding the archive (route repair's old route, or a committed route). `polish`: seconds
/// to keep exploring after the first exit, keeping the shortest.
#[allow(clippy::too_many_arguments)]
fn search(fa: &Facts, start: &Game, teacher: Option<&[u8]>, secs: f64, polish: f64, seed: u64, threads: usize, quiet: bool) -> Found {
    let t0 = Instant::now();
    let mut archive: HashMap<Key, Entry> = HashMap::new();
    let mut order: Vec<Key> = Vec::new();
    let put = |archive: &mut HashMap<Key, Entry>, order: &mut Vec<Key>, k: Key, g: Game, inp: Vec<u8>, h: u16| {
        match archive.get_mut(&k) {
            Some(e) => {
                e.seen += 1;
                if (h, std::cmp::Reverse(inp.len())) > (e.hearts, std::cmp::Reverse(e.inputs.len())) {
                    e.game = g;
                    e.inputs = inp;
                    e.hearts = h;
                    e.chosen = 0;
                }
            }
            None => {
                archive.insert(k, Entry { game: g, inputs: inp, hearts: h, chosen: 0, seen: 1 });
                order.push(k);
            }
        }
    };
    let hearts = start.state().w(HEARTS);
    put(&mut archive, &mut order, key(fa, start), start.clone(), Vec::new(), hearts);
    let mut best: Option<Vec<u8>> = None;
    if let Some(tch) = teacher {
        let mut t = start.clone();
        let lives = t.state().w(FROG + f::LIVES);
        let mut inp = Vec::new();
        for (k, &j) in tch.iter().enumerate() {
            t.step_unseen(FrameInput::joy(Joy(j)));
            inp.push(j);
            let st = t.state();
            if st.w(FROG + f::DYING) != 0 || (st.w(FROG + f::LIVES) as i16) < lives as i16 {
                break;
            }
            if st.l(LEVEL_STATE) != 0xffff_fffe {
                best = Some(inp.clone());
                break;
            }
            if k % 8 == 7 {
                put(&mut archive, &mut order, key(fa, &t), t.clone(), inp.clone(), st.w(HEARTS));
            }
        }
        if !quiet {
            eprintln!("  teacher: {} cells seeded, exits: {}", order.len(), best.is_some());
        }
    }
    let mut rng = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    let mut frames = 0u64;
    let mut best_band = u16::MAX;
    let mut best_exit_d = f64::MAX;
    let mut first_exit: Option<f64> = best.as_ref().map(|_| 0.0);
    let per_thread = 8;
    loop {
        let el = t0.elapsed().as_secs_f64();
        if el > secs || first_exit.is_some_and(|t| el > t + polish) {
            break;
        }
        let minband = order.iter().map(|k| k.2).min().unwrap();
        if minband < best_band {
            best_band = minband;
            if !quiet {
                eprintln!("  {el:6.1}s cells {} coins left <= {} frames {frames}", order.len(), minband * BAND);
            }
        }
        // With the coins met, cells nearer an exit (in cells, as the crow flies) are likelier.
        let exit_dist = |k: &Key| -> f64 {
            let (c, r) = ((k.0 * CELL) as f64, (k.1 * CELL) as f64);
            fa.exits
                .iter()
                .map(|x| (c - x.0 as f64).abs() + (r - x.1 as f64).abs())
                .fold(f64::MAX, f64::min)
        };
        if minband == 0 && !quiet {
            let d = order.iter().filter(|k| k.2 == 0).map(exit_dist).fold(f64::MAX, f64::min);
            if d < best_exit_d {
                best_exit_d = d;
                eprintln!("  {el:6.1}s coins met, nearest exit {d:.0} cells, cells {}", order.len());
            }
        }
        let guide_max = GUIDE.get().map_or(0, |gm| order.iter().filter_map(|k| gm.get(&(k.0, k.1, k.2))).copied().max().unwrap_or(0));
        let weights: Vec<f64> = order
            .iter()
            .map(|k| {
                let e = &archive[k];
                let novelty = 1.0 / (1.0 + e.chosen as f64).sqrt() + 0.5 / (1.0 + e.seen as f64).sqrt();
                let pull = if k.2 == 0 { 4.0 * (-exit_dist(k) / 12.0).exp() + 0.2 } else { 1.0 };
                // A guide path: cells further along it are likelier, cells off it rare.
                let guide = match GUIDE.get() {
                    Some(gm) => match gm.get(&(k.0, k.1, k.2)) {
                        Some(&p) => 3.0 * (-((guide_max - p) as f64) / 20.0).exp(),
                        None => 0.15,
                    },
                    None => 1.0,
                };
                novelty * pull * guide * (-(0.8 * (k.2 - minband) as f64)).exp()
            })
            .collect();
        let total: f64 = weights.iter().sum();
        let picks: Vec<(Key, u64)> = (0..threads * per_thread)
            .map(|_| {
                let mut x = rng.unit() * total;
                let mut i = 0;
                while i + 1 < weights.len() && x >= weights[i] {
                    x -= weights[i];
                    i += 1;
                }
                (order[i], rng.next())
            })
            .collect();
        for (k, _) in &picks {
            archive.get_mut(k).unwrap().chosen += 1;
        }
        let arch = &archive;
        let results: Vec<(Vec<Out>, u64)> = std::thread::scope(|s| {
            let hs: Vec<_> = picks
                .chunks(per_thread)
                .map(|chunk| {
                    s.spawn(move || {
                        let mut res = Vec::new();
                        let mut fr = 0u64;
                        for (k, sd) in chunk {
                            let e = &arch[k];
                            res.extend(burst(fa, &e.game, &e.inputs, &mut Rng(*sd | 1), &mut fr));
                        }
                        (res, fr)
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        for (res, fr) in results {
            frames += fr;
            for o in res {
                match o {
                    Out::Cell(k, g, inp, h) => put(&mut archive, &mut order, k, g, inp, h),
                    Out::Exit(inp) => {
                        if best.as_ref().is_none_or(|b| inp.len() < b.len()) {
                            if !quiet {
                                eprintln!("  {:6.1}s exit, route {} frames", t0.elapsed().as_secs_f64(), inp.len());
                            }
                            best = Some(inp);
                            first_exit.get_or_insert(t0.elapsed().as_secs_f64());
                        }
                    }
                }
            }
        }
    }
    Found { exit: best, cells: order.len(), frames, secs: t0.elapsed().as_secs_f64() }
}

fn write_script(path: &str, inputs: &[u8], timing: &str) {
    let mut s = format!("# timing: {timing} (goexp experiment)\n");
    let mut last = None;
    for (fr, &j) in inputs.iter().enumerate() {
        if Some(j) != last {
            s += &format!("{fr}: {}\n", InputScript::format_input(FrameInput::joy(Joy(j))));
            last = Some(j);
        }
    }
    s += &format!("{}: -\n", inputs.len());
    std::fs::write(path, s).unwrap();
}

/// A script's joystick bytes from frame `from` to its end.
fn load_joys(path: &str, from: u32) -> Vec<u8> {
    let script = InputScript::load(std::path::Path::new(path)).unwrap();
    let n = script.lines.last().map_or(0, |l| l.0);
    (from..n).map(|k| script.input_at(k).joy.0).collect()
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut opt = |name: &str| -> Option<String> {
        let k = args.iter().position(|a| a == name)?;
        let v = args[k + 1].clone();
        args.drain(k..k + 2);
        Some(v)
    };
    let threads: usize = opt("--threads").map_or(10, |v| v.parse().unwrap());
    let minutes: f64 = opt("--minutes").map_or(30.0, |v| v.parse().unwrap());
    let polish: f64 = opt("--polish").map_or(0.0, |v| v.parse().unwrap());
    let seed: u64 = opt("--seed").map_or(1, |v| v.parse().unwrap());
    let out_path = opt("--out");
    let follow = opt("--follow");
    let guide = opt("--guide");
    let original = opt("--original").is_some();
    let id: LevelId = args[0].parse().unwrap();
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    let t0 = Instant::now();
    let budget = minutes * 60.0;
    // The search starts once the frog has landed past the level-start freeze (as map_stats).
    let mut g = Game::with_display(display.clone(), Start::Level(id));
    let mut prefix: Vec<u8> = Vec::new();
    while !(landed(g.state()) && g.frame() > 150) && g.frame() < 400 {
        g.step_unseen(FrameInput::default());
        prefix.push(0);
    }
    let fa = facts(&g, &data.levels[&id], display.rom());
    eprintln!("{id}: coins needed {}, switches {}", g.state().w(COINS_LEFT), fa.switches.len());
    let teacher: Option<Vec<u8>> = follow.as_ref().map(|p| load_joys(p, prefix.len() as u32));
    if let Some(p) = &guide {
        GUIDE.set(guide_from(id, p)).unwrap();
    }
    if !original {
        let r = search(&fa, &g, teacher.as_deref(), budget, polish, seed, threads, false);
        match r.exit {
            Some(e) => {
                let mut all = prefix.clone();
                all.extend(e);
                println!("{id}: EXIT route {} frames, {:.0}s, cells {}, engine frames {}", all.len(), r.secs, r.cells, r.frames);
                if let Some(p) = &out_path {
                    write_script(p, &all, "engine");
                }
            }
            None => println!("{id}: TIMEOUT {:.0}s, cells {}, engine frames {}", r.secs, r.cells, r.frames),
        }
        return;
    }
    // Original mode: plan on the engine from the original's own state, play the plan on the
    // original until their frogs part (or it dies), keep what agreed less a margin, and plan again
    // from there with the rest of the old plan as the teacher.
    let mut amiga = sync::reach(&SyncPoint::AtLevel(id), &InputScript::idle(), 40_000).unwrap().amiga;
    for &j in &prefix {
        sync::step_inserting(&mut amiga, FrameInput::joy(Joy(j))).unwrap();
    }
    let mut route: Vec<u8> = prefix.clone();
    let mut plan_teacher = teacher;
    let (mut oracle_frames, mut engine_frames, mut repairs) = (0u64, 0u64, 0u32);
    let mut seed = seed;
    loop {
        let left = budget - t0.elapsed().as_secs_f64();
        if left <= 0.0 {
            println!("{id}: TIMEOUT original mode, route kept {} frames, repairs {repairs}", route.len());
            break;
        }
        let start = Game::with_display(display.clone(), Start::Transplant(sync::core_memory(amiga.memory())));
        let r = search(&fa, &start, plan_teacher.as_deref(), left, polish, seed, threads, true);
        engine_frames += r.frames;
        seed = seed.wrapping_add(1);
        let Some(plan) = r.exit else {
            println!("{id}: TIMEOUT original mode in search, route kept {} frames, repairs {repairs}", route.len());
            break;
        };
        // Lockstep on the original and the engine.
        let snap = amiga.snapshot();
        let mut eng = start.clone();
        let lives = start.state().w(FROG + f::LIVES);
        let mut agreed = 0usize;
        let mut exited = false;
        for (k, &j) in plan.iter().enumerate() {
            let x = FrameInput::joy(Joy(j));
            sync::step_inserting(&mut amiga, x).unwrap();
            oracle_frames += 1;
            eng.step_unseen(x);
            let o = sf_core::State::transplant(&sync::core_memory(amiga.memory()));
            let (co, ce) = (frog_cell(&o), frog_cell(eng.state()));
            if o.w(FROG + f::DYING) != 0 || (o.w(FROG + f::LIVES) as i16) < lives as i16 || sync::in_wall_loop(&amiga) {
                break;
            }
            if co.0.abs_diff(ce.0) > 1 || co.1.abs_diff(ce.1) > 1 {
                break;
            }
            agreed = k + 1;
            if o.l(LEVEL_STATE) != 0xffff_fffe {
                exited = true;
                break;
            }
        }
        if exited {
            route.extend_from_slice(&plan[..agreed]);
            println!(
                "{id}: EXIT on the original, route {} frames, {:.0}s, repairs {repairs}, engine frames {engine_frames}, oracle frames {oracle_frames}",
                route.len(),
                t0.elapsed().as_secs_f64()
            );
            if let Some(p) = &out_path {
                write_script(p, &route, "original");
            }
            break;
        }
        let keep = agreed.saturating_sub(16);
        amiga.restore(&snap);
        for &j in &plan[..keep] {
            sync::step_inserting(&mut amiga, FrameInput::joy(Joy(j))).unwrap();
            oracle_frames += 1;
        }
        route.extend_from_slice(&plan[..keep]);
        plan_teacher = Some(plan[keep..].to_vec());
        repairs += 1;
        eprintln!(
            "  {:6.1}s repair {repairs}: plan {} frames, agreed {agreed}, kept {keep}, route now {} (search {:.0}s, cells {})",
            t0.elapsed().as_secs_f64(),
            plan.len(),
            route.len(),
            r.secs,
            r.cells
        );
    }
}

