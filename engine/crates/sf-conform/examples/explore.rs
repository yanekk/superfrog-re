//! Plays a level to its exit by exploring, for the levels `plan_exit`'s map field cannot route
//! (T115): W3's pipes (springs, markers 39–43), gates opened by a far switch, water and slopes,
//! which the field reads as walls or open air. Prints the inputs as an input script.
//!
//! `cargo run --release --example explore -- <level> [--prefix script.txt] [--out script.txt]
//! [--frames N] [--to col,row]... [--coins N] [--iters N] [--hearts N]`
//!
//! Each step starts from the original's memory (its inputs replayed from power-on, as
//! `plan_exit`'s chunks do) and explores on the engine: an archive of the frog's cells (with the
//! coins still needed and the switches left), each holding the shortest way there found; a cell
//! picked at random, the less tried the likelier, is played on from with a burst of random
//! joystick holds (4 frames each), and every new cell (or a known one reached sooner) is kept.
//! A way that makes progress (a coin taken, the next `--to` cell reached, with no coins left the
//! exit) ends the search: it is played on the original, kept as far as the original's frog
//! follows the engine's (within a cell) and stays alive, and the next step starts there. The
//! original lags in a busy level and the engine free-running does not, so only the original's
//! outcome counts. `--hearts N`: ways that leave fewer hearts than N are not kept (default 1).
//! `--coins N`: stop taking coins at N still needed (default 0). `--reward` (T168): the goal is
//! instead an enemy stomped (h0+$23bee) and touched again into a reward (h0+$23e6c), the archive
//! keeping apart the cells reached with a stomped enemy out.
//! `--snap <file>` (T205): start from a snapshot `whole_game` saved at the leg's start in the whole
//! game instead of the level's `AtLevel` sync point.
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::h0;
use sf_core::{Game, Start};
use std::collections::HashMap;

const LEVEL_STATE: u32 = h0(0xa06);
const HEARTS: u32 = h0(0x830);
/// `frog+$5c` (h0+$832): coins still needed before the exit lock goes.
const COINS_LEFT: u32 = h0(0x832);
const HOLD: usize = 4;

/// The frog's cell, as `plan_exit` measures it (its middle, a row up).
fn cell(st: &sf_core::State) -> (i32, i32) {
    let w = |o| st.w(FROG + o) as i16 as i32;
    ((w(f::X) + 0x17).max(0) / 16, (w(f::Y) - 0x10).max(0) / 16)
}

/// Switches not yet pulled (markers 48–51 still in the map).
fn switches_left(st: &sf_core::State) -> usize {
    let row0 = sf_core::layout::h1(0x218e0 + 0x4c0);
    (0..84u32 * 300)
        .filter(|&k| (48..=51).contains(&(st.w(row0 + (k / 300) * 0x260 + 2 * (k % 300)) & 0x3f)))
        .count()
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
}

#[derive(Clone, Copy, PartialEq)]
enum Goal {
    Cell(i32, i32),
    Coins(u16),
    Reward,
}

/// Whether a live slot runs update `u` (an h0 offset).
fn runs(st: &sf_core::State, u: u32) -> bool {
    (0..20).any(|k| {
        let s = h0(0x1ba6c + k * 0x7c);
        st.l(s + 0x40) != 0 && st.l(s + 4) == h0(u)
    })
}

/// What a way found: its inputs, and the engine's frog cell after each.
struct Way {
    inputs: Vec<FrameInput>,
    cells: Vec<(i32, i32)>,
}

impl Way {
    /// Frames the frog has been in its last cell, in 32s up to 8: a frog held still (stunned
    /// after a hurt, W3-2 4861: some 130 frames) makes progress only so.
    fn still(&self) -> usize {
        let Some(&last) = self.cells.last() else {
            return 0;
        };
        (self.cells.iter().rev().take_while(|&&c| c == last).count() / 32).min(8)
    }
}

/// Whether `st` has made the progress `goal` asks for, from `coins` needed at the start.
fn done(st: &sf_core::State, goal: Goal, coins: u16) -> bool {
    match goal {
        Goal::Cell(c, r) => {
            let (x, y) = cell(st);
            (x - c).abs() <= 1 && (y - r).abs() <= 1
        }
        Goal::Reward => runs(st, 0x23e6c),
        Goal::Coins(n) if coins > n => st.w(COINS_LEFT) < coins,
        // No coins left to take: the exit.
        Goal::Coins(_) => st.l(LEVEL_STATE) != 0xffff_fffe,
    }
}

/// Frames the frog must then live through, idle, for a way to count: one ending in a fall onto
/// spikes is no way on (W3-2: 117,39 over the spikes at row 41).
const SAFE: usize = 40;

/// Whether the frog in `g` lives through [`SAFE`] idle frames (the exit ends the level first).
fn safe(g: &Game, lives: u16) -> bool {
    let mut g = g.clone();
    for _ in 0..SAFE {
        let st = g.state();
        if st.l(LEVEL_STATE) != 0xffff_fffe {
            return true;
        }
        if (st.w(FROG + f::LIVES) as i16) < lives as i16 || st.w(FROG + f::DYING) != 0 {
            return false;
        }
        g.step_unseen(FrameInput::default());
    }
    let st = g.state();
    st.w(FROG + f::LIVES) as i16 >= lives as i16 && st.w(FROG + f::DYING) == 0
}

/// Explores from `start` until a way makes progress, or `iters` bursts are spent.
/// `dead`: where earlier steps ended with no way on (cell, coins needed), which a way may not
/// end in.
fn explore(
    start: &Game,
    goal: Goal,
    iters: usize,
    min_hearts: u16,
    seed: u64,
    dead: &[((i32, i32), u16)],
) -> Option<Way> {
    let acts = [
        Joy::NONE,
        Joy::RIGHT,
        Joy::LEFT,
        Joy::UP,
        Joy::RIGHT | Joy::UP,
        Joy::LEFT | Joy::UP,
        Joy::DOWN,
        Joy::FIRE,
    ];
    let s0 = start.state();
    let lives = s0.w(FROG + f::LIVES);
    let coins = s0.w(COINS_LEFT);
    let mut rng = Rng(seed | 1);
    // Key: cell, coins needed, switches left (with `--reward`, twice that and one more while a
    // stomped enemy is out), frames still there ([`Way::still`]). Value: (the game there, its
    // way, times picked).
    type Key = ((i32, i32), u16, usize, usize);
    let mut archive: HashMap<Key, (Game, Way, u32)> = HashMap::new();
    let reward = goal == Goal::Reward;
    let key = |st: &sf_core::State, w: &Way| {
        let sw = if reward { 2 * switches_left(st) + runs(st, 0x23bee) as usize } else { switches_left(st) };
        (cell(st), st.w(COINS_LEFT), sw, w.still())
    };
    let none = Way {
        inputs: vec![],
        cells: vec![],
    };
    archive.insert(key(s0, &none), (start.clone(), none, 0));
    let mut keys: Vec<Key> = archive.keys().copied().collect();
    // How often a burst ended in each key's cell.
    let mut visits: HashMap<Key, u32> = HashMap::new();
    for _ in 0..iters {
        // The less tried and the less often come back to, the likelier: the best of four draws.
        let k = (0..4)
            .map(|_| keys[rng.below(keys.len())])
            .min_by_key(|k| archive[k].2 + visits.get(k).copied().unwrap_or(0) / 4)
            .unwrap();
        let entry = archive.get_mut(&k).unwrap();
        entry.2 += 1;
        let mut g = entry.0.clone();
        let mut inputs = entry.1.inputs.clone();
        let mut cells = entry.1.cells.clone();
        let holds = 1 + rng.below(12);
        'burst: for _ in 0..holds {
            let a = acts[rng.below(acts.len())];
            for _ in 0..HOLD {
                let x = FrameInput {
                    joy: a,
                    ..FrameInput::default()
                };
                g.step_unseen(x);
                inputs.push(x);
                let st = g.state();
                cells.push(cell(st));
                if (st.w(FROG + f::LIVES) as i16) < lives as i16
                    || st.w(FROG + f::DYING) != 0
                    || st.w(HEARTS) < min_hearts
                {
                    break 'burst;
                }
                if done(st, goal, coins)
                    && !dead.contains(&(cell(st), st.w(COINS_LEFT)))
                    && safe(&g, lives)
                {
                    return Some(Way { inputs, cells });
                }
                let ended = st.l(LEVEL_STATE) != 0xffff_fffe;
                if ended {
                    break 'burst;
                }
            }
            let st = g.state();
            let way = Way {
                inputs: inputs.clone(),
                cells: cells.clone(),
            };
            let kk = key(st, &way);
            *visits.entry(kk).or_insert(0) += 1;
            let better = archive.get(&kk).is_none_or(|e| {
                (st.w(HEARTS), std::cmp::Reverse(inputs.len()))
                    > (e.0.state().w(HEARTS), std::cmp::Reverse(e.1.inputs.len()))
            });
            if better {
                if !archive.contains_key(&kk) {
                    keys.push(kk);
                }
                archive.insert(kk, (g.clone(), way, 0));
            }
        }
    }
    eprintln!("  no way in {iters} bursts: {} cells", keys.len());
    // Where it got: the cells reached, by switches left, as boxes, when asked (`EXPLORE_MAP`).
    if let Ok(p) = std::env::var("EXPLORE_MAP") {
        let mut s = String::new();
        for &((c, r), n, sw, _) in &keys {
            s += &format!("{c} {r} {n} {sw}\n");
        }
        std::fs::write(format!("{p}.{seed:x}"), s).unwrap();
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let id: sf_core::LevelId = args[0].parse().unwrap();
    let (mut prefix, mut out, mut budget) = (Vec::new(), None, 20_000usize);
    let (mut goals, mut iters, mut min_hearts, mut coins_to) = (Vec::new(), 4000usize, 1u16, 0u16);
    let mut reward = false;
    let mut snap: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--prefix" => {
                let p = InputScript::load(std::path::Path::new(&args[i + 1])).unwrap();
                let n: usize = std::fs::read_to_string(&args[i + 1])
                    .unwrap()
                    .lines()
                    .filter_map(|l| l.split(':').next()?.trim().parse::<usize>().ok())
                    .max()
                    .unwrap_or(0);
                prefix = (0..n as u32).map(|k| p.input_at(k)).collect();
            }
            "--out" => out = Some(args[i + 1].clone()),
            "--snap" => snap = Some(args[i + 1].clone()),
            "--frames" => budget = args[i + 1].parse().unwrap(),
            "--iters" => iters = args[i + 1].parse().unwrap(),
            "--hearts" => min_hearts = args[i + 1].parse().unwrap(),
            "--coins" => coins_to = args[i + 1].parse().unwrap(),
            "--reward" => {
                reward = true;
                i -= 1;
            }
            "--to" => {
                let (c, r) = args[i + 1].split_once(',').unwrap();
                goals.push(Goal::Cell(c.parse().unwrap(), r.parse().unwrap()));
            }
            a => panic!("{a}?"),
        }
        i += 2;
    }
    goals.push(if reward { Goal::Reward } else { Goal::Coins(coins_to) });
    // From the leg's start in the whole game (`--snap`, a snapshot `whole_game` saved), else the
    // level's own sync point.
    let mut amiga = match &snap {
        Some(p) => sync::at_snapshot_file(std::path::Path::new(p), budget as u32 + 100),
        None => sync::reach(&SyncPoint::AtLevel(id), &InputScript::idle(), budget as u32 + 100)
            .map(|s| s.amiga),
    }
    .unwrap_or_else(|e| panic!("{e}"));
    let amiga = &mut amiga;
    let display = sf_conform::display_data();
    let mut inputs: Vec<FrameInput> = Vec::new();
    for &x in &prefix {
        sync::step_inserting(amiga, x).unwrap();
        inputs.push(x);
    }
    let peek_w = |a: &sf_oracle::Amiga, addr: u32| {
        let b = a.peek(addr, 2);
        u16::from_be_bytes([b[0], b[1]])
    };
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .min(6);
    let mut goal = 0;
    let mut seed = 0x9e37_79b9_7f4a_7c15u64;
    let mut fails = 0;
    // Each step kept: the original before it, and the inputs' length then.
    let mut history: Vec<(sf_oracle::Snapshot, usize)> = Vec::new();
    let mut dead: Vec<((i32, i32), u16)> = Vec::new();
    while inputs.len() < budget && goal < goals.len() {
        let start = Game::with_display(
            display.clone(),
            Start::Transplant(sync::core_memory(amiga.memory())),
        );
        let lives = start.state().w(FROG + f::LIVES);
        // Several explorations at once, each its own seed: the shortest way found.
        let found: Vec<Way> = std::thread::scope(|s| {
            let hs: Vec<_> = (0..threads as u64)
                .map(|t| {
                    let start = start.clone();
                    let g = goals[goal];
                    let sd = seed.wrapping_add(t.wrapping_mul(0x2545_f491_4f6c_dd1d));
                    let dead = &dead;
                    s.spawn(move || explore(&start, g, iters, min_hearts, sd, dead))
                })
                .collect();
            hs.into_iter().filter_map(|h| h.join().unwrap()).collect()
        });
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let Some(way) = found.into_iter().min_by_key(|w| w.inputs.len()) else {
            fails += 1;
            eprintln!("frame {}: no way found ({fails})", inputs.len());
            // Back to the step before: the last one kept may have left the frog nowhere to go,
            // and no way may end there again.
            let st = sf_core::State::transplant(&sync::core_memory(amiga.memory()));
            dead.push((cell(&st), st.w(COINS_LEFT)));
            if let Some((s, len)) = history.pop() {
                amiga.restore(&s);
                inputs.truncate(len);
                eprintln!("  back to frame {len}");
            }
            if fails >= 4 {
                break;
            }
            continue;
        };
        // On the original: as far as its frog follows the engine's and lives.
        let snap = amiga.snapshot();
        let before = inputs.len();
        let mut kept = 0;
        for (k, &x) in way.inputs.iter().enumerate() {
            sync::step_inserting(amiga, x).unwrap();
            let st = sf_core::State::transplant(&sync::core_memory(amiga.memory()));
            let (c, r) = cell(&st);
            let (ec, er) = way.cells[k];
            if (peek_w(amiga, FROG + f::LIVES) as i16) < lives as i16
                || peek_w(amiga, FROG + f::DYING) != 0
                || sync::in_wall_loop(amiga)
            {
                break;
            }
            if (c - ec).abs() > 1 || (r - er).abs() > 1 {
                break;
            }
            kept = k + 1;
            if st.l(LEVEL_STATE) != 0xffff_fffe && inputs.len() + kept > 10 {
                break;
            }
        }
        // Back off a little from where they parted (the original may be on its way to a fall).
        let keep = if kept == way.inputs.len() {
            kept
        } else {
            kept.saturating_sub(16)
        };
        amiga.restore(&snap);
        for &x in &way.inputs[..keep] {
            sync::step_inserting(amiga, x).unwrap();
        }
        inputs.extend_from_slice(&way.inputs[..keep]);
        if keep == 0 {
            fails += 1;
            eprintln!(
                "frame {}: the original parted at once ({fails})",
                inputs.len()
            );
            if fails >= 6 {
                break;
            }
            continue;
        }
        history.push((snap, before));
        // Only a way kept whole made its progress; a part kept and then no way on, again and
        // again, looped for ever (T205: W1-4's top left, W2-4 at 227,14).
        if keep == way.inputs.len() {
            fails = 0;
        }
        if let Some(o) = &out {
            std::fs::write(o, script(&inputs)).unwrap();
        }
        let st = sf_core::State::transplant(&sync::core_memory(amiga.memory()));
        let (c, r) = cell(&st);
        eprintln!(
            "frame {}: kept {keep}/{} cell {c},{r} hearts {} coins needed {} switches {} clock {:x}",
            inputs.len(),
            way.inputs.len(),
            st.w(HEARTS),
            st.w(COINS_LEFT),
            switches_left(&st),
            st.l(h0(0x876))
        );
        if st.l(LEVEL_STATE) != 0xffff_fffe {
            eprintln!("exit at frame {}", inputs.len());
            break;
        }
        match goals[goal] {
            Goal::Cell(gc, gr) if (c - gc).abs() <= 1 && (r - gr).abs() <= 1 => {
                eprintln!("at {gc},{gr}");
                goal += 1;
            }
            Goal::Reward if runs(&st, 0x23e6c) => {
                eprintln!("reward at frame {}", inputs.len());
                break;
            }
            _ => {}
        }
    }
    print!("{}", script(&inputs));
}

/// `inputs` as a script: a line at each change, and a closing `-` line.
fn script(inputs: &[FrameInput]) -> String {
    let mut s = String::new();
    let mut last = None;
    for (fr, x) in inputs.iter().enumerate() {
        if Some(*x) != last {
            s += &format!("{fr}: {}\n", InputScript::format_input(*x));
            last = Some(*x);
        }
    }
    s += &format!("{}: -\n", inputs.len());
    s
}
