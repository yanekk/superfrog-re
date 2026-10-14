//! Finds inputs that play a level to its exit with every object live, and prints them as an input
//! script: how the `enemies-w6-*` completion cases' scripts are written (T40).
//!
//! `cargo run --release --example plan_exit -- <level> [--via col,row | --stomp <update> | --hurt <update> | --coins N]... [--frames N]
//! [--prefix script.txt] [--no-exit] [--out script.txt]` (`--no-exit`: stop at the last `--via`;
//! `--out`: the script so far, rewritten after each chunk) (a script this printed: its inputs are played first, up to its closing `-` line)
//!
//! The plan is made on the engine and played on the original as it goes: from the original's
//! memory a beam search over ten joystick inputs, each held 4 frames, looks `HORIZON` frames
//! ahead toward the next `--via` cell, then the exit (marker 54), scoring each end by a distance
//! field flooded through the map's open cells (so it routes round walls), less for hearts lost.
//! Its first `COMMIT` frames are played on the original, once the whole path has kept the
//! original's frog alive; else the next-best path is tried. The original lags in a busy level and the engine
//! free-running does not, so only the original's outcome counts, and each chunk starts from its
//! memory again.
//!
//! `--coins N` (T103): take coins (markers 8, 9) until `N` are still needed (`frog+$5c`, h0+$832;
//! 0 opens the exit lock), steering by a field flooded from every coin still in the original's
//! map at the chunk's start, a coin taken worth more than any distance; `--skip c0,r0,c1,r1`
//! leaves the coins in that box out; `--ground m,...` lets the field climb from cells with those
//! markers (platform objects: W4's crumbling ledges are 27).
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::h0;
use sf_core::{Game, Start};

const LEVEL_STATE: u32 = h0(0xa06);
const HEARTS: u32 = h0(0x830);
const HORIZON: u32 = 30;
const HOLD: u32 = 4;
const COMMIT: usize = 48;
const BEAM: usize = 24;
/// A committed path must reach the ground at least this many frames in (T103).
const GROUNDED: u32 = 56;

/// The frog's middle, in map pixels a row up: `/ 16` gives the map cell of its lower half (the
/// map's pixel rows start one row above its first; the frog stands with its feet at y + `$10`).
fn frog(st: &sf_core::State) -> (i32, i32) {
    let w = |o| st.w(FROG + o) as i16 as i32;
    (w(f::X) + 0x17, w(f::Y) - 0x10)
}

/// The cost to `goal` from every cell, along the moves the frog can make (T103). Markers `$3a`
/// and up are ground and walls (`level/collision.rs`); a secret passage (`$26`) is open. A move
/// sideways or down through open cells costs 1, up into an open cell only within `JUMP` rows of
/// ground below it (in its column or the next; 20 above a bounce pad, marker 5; `ground`
/// markers count as ground: a crumbling ledge, an object the map does not show), so a shaft
/// taller than a jump is one-way; into or out of a wall 1 000 (a gate opens to its switch);
/// into a deadly cell (6) 40.
fn flood(body: &[u16], w: usize, h: usize, goal: &[(usize, usize)], ground: &[u16]) -> Vec<u32> {
    flood_dir(body, w, h, goal, ground, false)
}

/// [`flood`], or with `forward` the cost from `goal` to every cell instead.
fn flood_dir(body: &[u16], w: usize, h: usize, goal: &[(usize, usize)], ground: &[u16], forward: bool) -> Vec<u32> {
    const JUMP: usize = 6;
    let m = |x: usize, y: usize| body[y * w + x] >> 10;
    let wall = |x: usize, y: usize| m(x, y) >= 0x3a;
    let floor = |x: usize, y: usize| m(x, y) >= 0x3a || ground.contains(&m(x, y));
    // The cost of the move from (px, py) into (x, y).
    let cost = |px: usize, py: usize, x: usize, y: usize| -> u32 {
        if wall(x, y) || wall(px, py) {
            1000
        } else if m(x, y) == 6 {
            40
        } else {
            1
        }
    };
    // A bounce pad (marker 5) throws the frog this many rows.
    const PAD: usize = 20;
    // Rows from each cell down to the first floor in its column, and whether that is a pad.
    let mut above = vec![(usize::MAX / 2, false); w * h];
    for x in 0..w {
        let mut d = (usize::MAX / 2, false);
        for y in (0..h).rev() {
            let m = body[y * w + x] >> 10;
            d = if m == 5 { (0, true) } else if floor(x, y) { (0, false) } else { (d.0 + 1, d.1) };
            above[y * w + x] = d;
        }
    }
    let climbable = |x: usize, y: usize| {
        // A solid cell beside it is a wall, not ground.
        [x.wrapping_sub(1), x, x + 1].iter().any(|&c| {
            c < w && {
                let (k, pad) = above[y * w + c];
                k >= 1 && k <= if pad { PAD } else { JUMP }
            }
        })
    };
    let mut d = vec![u32::MAX; w * h];
    let mut q = std::collections::BinaryHeap::new();
    for &(x, y) in goal {
        d[y * w + x] = 0;
        q.push(std::cmp::Reverse((0u32, x, y)));
    }
    // Backwards from the goal: `(px, py)` is a cell a move from which reaches `(x, y)`.
    while let Some(std::cmp::Reverse((here, x, y))) = q.pop() {
        if here > d[y * w + x] {
            continue;
        }
        // Backwards, the cells a move from which reaches (x, y); forwards, those (x, y) reaches.
        let mut near = vec![(x.wrapping_sub(1), y), (x + 1, y)];
        if forward {
            near.push((x, y + 1));
            if y > 0 && (wall(x, y - 1) || climbable(x, y - 1)) {
                near.push((x, y - 1));
            }
        } else {
            near.push((x, y.wrapping_sub(1)));
            if wall(x, y) || climbable(x, y) {
                near.push((x, y + 1));
            }
        }
        for (px, py) in near {
            if px < w && py < h {
                let step = if forward { cost(x, y, px, py) } else { cost(px, py, x, y) };
                let nd = here + step;
                if nd < d[py * w + px] {
                    d[py * w + px] = nd;
                    q.push(std::cmp::Reverse((nd, px, py)));
                }
            }
        }
    }
    d
}

/// A step on the way: a cell to reach, an object (by its update routine, h0 offset) to stomp or
/// kill, or one to be hurt by.
#[derive(Debug, Clone, Copy)]
enum Via {
    Cell(usize, usize),
    Stomp(u32),
    Hurt(u32),
    Coins(u16),
}

/// The map's markers as the original's memory has them now, in the file's layout (marker in
/// the top six bits); a `ground` marker (a platform object's) kept from the file where memory
/// has cleared it.
fn map_now(st: &sf_core::State, file: &[u16], ground: &[u16]) -> Vec<u16> {
    let row0 = sf_core::layout::h1(0x218e0 + 0x4c0);
    (0..84u32 * 300)
        .map(|k| {
            let m = st.w(row0 + (k / 300) * 0x260 + 2 * (k % 300)) & 0x3f;
            let f = file[k as usize] >> 10;
            (if m == 0 && ground.contains(&f) { f } else { m }) << 10
        })
        .collect()
}

/// `frog+$5c` (h0+$832): coins still needed before the exit lock (marker 29) goes.
const COINS_LEFT: u32 = h0(0x832);

/// The map's cells in memory (`map_rows[1]`, h1): a cell's marker is its low 6 bits.
fn coin_cells(st: &sf_core::State) -> Vec<(usize, usize)> {
    let row0 = sf_core::layout::h1(0x218e0 + 0x4c0);
    let mut v = Vec::new();
    for r in 0..84u32 {
        for c in 0..300u32 {
            if matches!(st.w(row0 + r * 0x260 + 2 * c) & 0x3f, 8 | 9) {
                v.push((c as usize, r as usize));
            }
        }
    }
    v
}

const SLOTS: u32 = 0x1ba6c;

/// Whether a slot runs what a killed or stunned enemy runs (h0+$23b34, $23bee) or a reward's arc
/// (h0+$23e6c).
fn killed(l: impl Fn(u32) -> u32) -> bool {
    (0..20).any(|k| {
        let s = h0(SLOTS + k * 0x7c);
        l(s + 0x40) != 0 && [0x23b34, 0x23bee, 0x23e6c].map(h0).contains(&l(s + 4))
    })
}

/// The position of the first live slot running update `u` (an h0 offset).
fn nearest(l: impl Fn(u32) -> u32, w: impl Fn(u32) -> u16, u: u32) -> Option<(i32, i32)> {
    (0..20).map(|k| h0(SLOTS + k * 0x7c)).find(|&s| l(s + 0x40) != 0 && l(s + 4) == h0(u)).map(|s| {
        (w(s + 8) as i16 as i32, w(s + 0xa) as i16 as i32)
    })
}

/// The field at a cell, or (a cell inside a wall) one more than its nearest open neighbour's;
/// 100 000 when none is open.
fn dist(field: &[u32], cx: usize, cy: usize) -> i64 {
    let (w, h) = (300usize, field.len() / 300);
    let at = |x: usize, y: usize| if x < w && y < h { field[y * w + x] } else { u32::MAX };
    let mut d = at(cx, cy);
    for (x, y) in [(cx.wrapping_sub(1), cy), (cx + 1, cy), (cx, cy.wrapping_sub(1)), (cx, cy + 1)] {
        d = d.min(at(x, y).saturating_add(1));
    }
    if d == u32::MAX { 100_000 } else { d as i64 }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let id: sf_core::LevelId = args[0].parse().unwrap();
    let mut via: Vec<Via> = Vec::new();
    let mut budget = 12_000usize;
    let mut prefix: Vec<FrameInput> = Vec::new();
    let mut no_exit = false;
    let mut out: Option<String> = None;
    // Boxes (c0, r0, c1, r1) whose coins `--coins` leaves alone: a room the frog cannot climb
    // back out of (T103: W4-1's lower rooms, under the shaft at 125,35).
    let mut ground: Vec<u16> = Vec::new();
    let mut reach_only = false;
    let mut skip_boxes: Vec<(usize, usize, usize, usize)> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--via" => {
                let (c, r) = args[i + 1].split_once(',').unwrap();
                via.push(Via::Cell(c.parse().unwrap(), r.parse().unwrap()));
            }
            "--stomp" => via.push(Via::Stomp(u32::from_str_radix(&args[i + 1], 16).unwrap())),
            "--hurt" => via.push(Via::Hurt(u32::from_str_radix(&args[i + 1], 16).unwrap())),
            "--coins" => via.push(Via::Coins(args[i + 1].parse().unwrap())),
            "--reach-only" => {
                reach_only = true;
                i -= 1;
            }
            "--ground" => ground = args[i + 1].split(',').map(|x| x.parse().unwrap()).collect(),
            "--skip" => {
                let v: Vec<usize> = args[i + 1].split(',').map(|x| x.parse().unwrap()).collect();
                skip_boxes.push((v[0], v[1], v[2], v[3]));
            }
            "--no-exit" => {
                no_exit = true;
                i -= 1;
            }
            "--out" => out = Some(args[i + 1].clone()),
            "--frames" => budget = args[i + 1].parse().unwrap(),
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
            a => panic!("{a}?"),
        }
        i += 2;
    }
    let data = sf_conform::game_data();
    let t = &data.levels.iter().find(|l| *l.0 == id).expect("level").1.t7mp;
    let (w, h) = (t.width as usize, t.height as usize);
    let exits: Vec<(usize, usize)> = (0..w * h)
        .filter(|&k| t.body[k] >> 10 == 54)
        .map(|k| (k % w, k / w))
        .collect();
    // Each chunk's field, flooded from the original's map as it is then (T103): a switch opens
    // its gate in memory (W4-2's way to the exit is the switch at 222,4).
    let mut goals: Vec<Vec<u32>> = vec![Vec::new(); via.len() + 1];
    let mut goal = 0;

    let mut synced = sync::reach(&SyncPoint::AtLevel(id), &InputScript::idle(), budget as u32 + 100)
        .unwrap_or_else(|e| panic!("{e}"));
    let amiga = &mut synced.amiga;
    let display = sf_conform::display_data();
    let acts = [
        Joy::NONE,
        Joy::RIGHT,
        Joy::LEFT,
        Joy::UP,
        Joy::RIGHT | Joy::UP,
        Joy::LEFT | Joy::UP,
        Joy::DOWN,
    ];
    let mut inputs: Vec<FrameInput> = Vec::new();
    for &x in &prefix {
        sync::step_inserting(amiga, x).unwrap();
        inputs.push(x);
    }
    let peek_w = |a: &sf_oracle::Amiga, addr: u32| {
        let b = a.peek(addr, 2);
        u16::from_be_bytes([b[0], b[1]])
    };
    // Each chunk kept: the original before it, the inputs and goal then, and which candidate it
    // took; a chunk with no candidate left takes the one before back and tries its next.
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let mut history: Vec<(sf_oracle::Snapshot, usize, usize, usize)> = Vec::new();
    let mut skip = 0usize;
    let mut backtracks = 0;
    while inputs.len() < budget {
        let mem = sync::core_memory(amiga.memory());
        let start = Game::with_display(display.clone(), Start::Transplant(mem));
        let lives = start.state().w(FROG + f::LIVES);
        let target = via.get(goal).copied();
        let body = map_now(start.state(), &t.body, &ground);
        let exit_field = flood(&body, w, h, &exits, &ground);
        let (fx, fy) = frog(start.state());
        let here = [((fx.max(0) / 16) as usize, (fy.max(0) / 16) as usize)];
        let ahead = flood_dir(&body, w, h, &here, &ground, true);
        // The switches the frog can reach: one opens a gate on the way (W4-2's exit is behind
        // the gate of the switch at 222,4).
        let switches: Vec<(usize, usize)> = (0..w * h)
            .filter(|&k| (48..=51).contains(&(body[k] >> 10)) && ahead[k] < 1000)
            .map(|k| (k % w, k / w))
            .collect();
        goals[goal] = match target {
            Some(Via::Cell(x, y)) => flood(&body, w, h, &[(x, y)], &ground),
            None if dist(&exit_field, here[0].0, here[0].1) >= 1000 && !switches.is_empty() => {
                eprintln!("  the exit is out of reach: to {} switches", switches.len());
                flood(&body, w, h, &switches, &ground)
            }
            None => exit_field.clone(),
            _ => Vec::new(),
        };
        if let Some(Via::Coins(_)) = target {
            let back = flood(&body, w, h, &here, &ground);
            let coins: Vec<(usize, usize)> = coin_cells(start.state())
                .into_iter()
                // A coin the frog cannot come back from (only through a wall), with the exit
                // out of reach from it too, is a trap (W4-1's rooms under the shaft at 125,35).
                .filter(|&(c, r)| back[r * w + c] < 1000 || exit_field[r * w + c] < 1000)
                // And one the frog cannot reach yet (behind a gate) is left for later.
                .filter(|&(c, r)| ahead[r * w + c] < 1000)
                .filter(|&(c, r)| !skip_boxes.iter().any(|&(c0, r0, c1, r1)| (c0..=c1).contains(&c) && (r0..=r1).contains(&r)))
                .collect();
            eprintln!("  {} coins in reach", coins.len());
            if reach_only {
                // The map, `+` where the exit is in reach, `-` where the frog can go and come
                // back from, `v` where it can go only,
                // `#` a wall, `5` a pad.
                for r in 0..h {
                    let row: String = (0..w)
                        .map(|c| match body[r * w + c] >> 10 {
                            _ if exit_field[r * w + c] < 1000 => '+',
                            m if m >= 0x3a => '#',
                            _ if back[r * w + c] < 1000 && ahead[r * w + c] < 1000 => '-',
                            _ if ahead[r * w + c] < 1000 => 'v',
                            5 => '5',
                            _ => '.',
                        })
                        .collect();
                    eprintln!("{r:3} {row}");
                }
                return;
            }
            // None left in reach: a switch the frog can reach opens its gate (W4-3's start area
            // opens only so); with no switch either, every coin.
            let coins = if coins.is_empty() {
                eprintln!("  {} switches in reach", switches.len());
                if switches.is_empty() { coin_cells(start.state()) } else { switches.clone() }
            } else {
                coins
            };
            goals[goal] = flood(&body, w, h, &coins, &ground);
        }
        let field = &goals[goal];
        let hearts = start.state().w(HEARTS);
        let score = |st: &sf_core::State| -> i64 {
            let (x, y) = frog(st);
            match target {
                Some(Via::Stomp(u)) => {
                    if killed(|a| st.l(a)) {
                        return i64::MIN / 4;
                    }
                    // Onto the object from above: the frog's centre over its top middle.
                    nearest(|a| st.l(a), |a| st.w(a), u).map_or(100_000, |(ox, oy)| {
                        ((x - ox - 16).abs() + 2 * (y - (oy - 32)).abs()) as i64
                    }) - 400 * st.w(HEARTS) as i64
                }
                Some(Via::Hurt(u)) => {
                    if st.w(HEARTS) < hearts {
                        return i64::MIN / 4;
                    }
                    nearest(|a| st.l(a), |a| st.w(a), u)
                        .map_or(100_000, |(ox, oy)| ((x - ox - 16).abs() + (y - (oy - 8)).abs()) as i64)
                }
                _ => {
                    let (cx, cy) = ((x.max(0) / 16) as usize, (y.max(0) / 16) as usize);
                    let d = dist(field, cx, cy);
                    let coins = if matches!(target, Some(Via::Coins(_))) { 4000 * st.w(COINS_LEFT) as i64 } else { 0 };
                    coins + d * 16 + (16 - (x % 16 - 8).abs()) as i64 / 4 - 400 * st.w(HEARTS) as i64
                }
            }
        };
        let mut beam: Vec<(Game, Vec<FrameInput>)> = vec![(start, Vec::new())];
        let mut alive = Vec::new();
        // Paths on the ground at some depth past `GROUNDED` frames, and their scores (T103).
        let mut grounded: Vec<(i64, Game, Vec<FrameInput>)> = Vec::new();
        for depth in 0..HORIZON {
            // Each beam entry's children, expanded on every core.
            let jobs: Vec<(&Game, &Vec<FrameInput>, Joy)> =
                beam.iter().flat_map(|(b, p)| acts.iter().map(move |&a| (b, p, a))).collect();
            let chunk = jobs.len().div_ceil(threads).max(1);
            let score = &score;
            let inputs = &inputs;
            let mut next: Vec<(i64, Game, Vec<FrameInput>)> = std::thread::scope(|sc| {
                let hs: Vec<_> = jobs
                    .chunks(chunk)
                    .map(|js| {
                        sc.spawn(move || {
                            js.iter()
                                .filter_map(|&(b, path, a)| {
                                    let mut n = b.clone();
                                    let mut p = path.clone();
                                    for _ in 0..HOLD {
                                        let x = FrameInput::joy(a);
                                        p.push(x);
                                        // A path the engine panics on is dropped, its inputs
                                        // kept for a look (T103: a bob blit past the buffers).
                                        let stepped = std::panic::catch_unwind(
                                            std::panic::AssertUnwindSafe(|| n.step(x)),
                                        );
                                        if stepped.is_err() {
                                            let _ = std::fs::write(
                                                "/tmp/plan_exit-panic.txt",
                                                script(&inputs.iter().chain(&p).copied().collect::<Vec<_>>()),
                                            );
                                            return None;
                                        }
                                        if n.state().w(FROG + f::LIVES) != lives
                                            || n.state().w(FROG + f::DYING) != 0
                                        {
                                            return None;
                                        }
                                    }
                                    let s = if n.state().l(LEVEL_STATE) == 1 {
                                        i64::MIN / 2
                                    } else {
                                        score(n.state())
                                    };
                                    Some((s, n, p))
                                })
                                .collect::<Vec<_>>()
                        })
                    })
                    .collect();
                hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
            });
            next.sort_by_key(|x| x.0);
            let mut seen = std::collections::BTreeSet::new();
            beam = next
                .into_iter()
                .filter(|(_, n, _)| {
                    let w = |o| n.state().w(FROG + o);
                    seen.insert((w(f::X) / 3, w(f::Y) / 3, w(f::VY), w(f::ON_GROUND)))
                })
                .take(BEAM)
                .map(|(_, n, p)| (n, p))
                .collect();
            if beam.is_empty() {
                // Every path dies within the horizon: the shorter ones that were still alive.
                beam = alive;
                break;
            }
            if (depth + 1) * HOLD >= GROUNDED {
                for (n, p) in &beam {
                    if n.state().w(FROG + f::ON_GROUND) != 0 || n.state().w(FROG + f::ON_PLATFORM) != 0 {
                        grounded.push((score(n.state()), n.clone(), p.clone()));
                    }
                }
            }
            alive = beam.clone();
        }
        // Play the best path's first frames on the original; the next best if that loses a life.
        let snap = amiga.snapshot();
        let mut taken = None;
        // Only paths that reach the ground past `GROUNDED` frames, best first (T103): one still
        // in the air at its end can be falling into a pit nothing past the horizon escapes, as
        // W4-1's are. The level's drop-in is the exception: the frog lands only around frame 150.
        grounded.sort_by_key(|x| x.0);
        // None grounded (the frog on what neither flag shows): the beam's ends after all.
        let order: Vec<(&Game, &Vec<FrameInput>)> = if inputs.len() < 200 || grounded.is_empty() {
            beam.iter().map(|(g, p)| (g, p)).collect()
        } else {
            grounded.iter().map(|(_, g, p)| (g, p)).collect()
        };
        for (_, p) in order.into_iter().skip(skip).take(8) {
            // The whole path must keep the frog alive on the original, not only what is kept.
            let n = p.len().min(COMMIT);
            let mut ok = true;
            for (k, &x) in p.iter().enumerate() {
                sync::step_inserting(amiga, x).unwrap();
                if peek_w(amiga, FROG + f::LIVES) != lives || peek_w(amiga, FROG + f::DYING) != 0 {
                    eprintln!(
                        "  a path dies {k} frames in on the original, frog at {},{}",
                        peek_w(amiga, FROG + f::X),
                        peek_w(amiga, FROG + f::Y)
                    );
                    ok = false;
                    break;
                }
            }
            amiga.restore(&snap);
            if ok {
                for &x in &p[..n] {
                    sync::step_inserting(amiga, x).unwrap();
                }
                taken = Some(p[..n].to_vec());
                break;
            }
        }
        let Some(p) = taken else {
            eprintln!("frame {}: every path loses a life on the original", inputs.len());
            backtracks += 1;
            // Three candidates a chunk at most, then the chunk before it.
            let mut popped = history.pop();
            while popped.as_ref().is_some_and(|h| h.3 + 1 >= 3) {
                popped = history.pop();
            }
            let Some((s, len, g, sk)) = popped.filter(|_| backtracks <= 400) else {
                break;
            };
            amiga.restore(&s);
            inputs.truncate(len);
            goal = g;
            skip = sk + 1;
            eprintln!("  back to frame {len}, candidate {skip}");
            continue;
        };
        history.push((snap, inputs.len(), goal, skip));
        skip = 0;
        inputs.extend(p);
        if let Some(o) = &out {
            std::fs::write(o, script(&inputs)).unwrap();
        }
        let o = sync::core_memory(amiga.memory());
        let st = sf_core::State::transplant(&o);
        let (x, y) = frog(&st);
        let d = match via.get(goal) {
            Some(Via::Stomp(_)) => {
                if killed(|a| st.l(a)) { 0 } else { 99 }
            }
            Some(Via::Hurt(_)) => {
                if st.w(HEARTS) < hearts { 0 } else { 99 }
            }
            Some(Via::Coins(n)) => {
                if st.w(COINS_LEFT) <= *n { 0 } else { 99 }
            }
            _ => dist(&goals[goal], (x.max(0) / 16) as usize, (y.max(0) / 16) as usize),
        };
        eprintln!(
            "frame {}: frog {},{} (cell {},{}) goal {goal} distance {d} hearts {} coins needed {} clock {:x}",
            inputs.len(),
            x,
            y,
            x / 16,
            y / 16,
            st.w(HEARTS),
            st.w(COINS_LEFT),
            st.l(h0(0x876))
        );
        if goal < via.len() && d <= 2 {
            eprintln!("via {:?} at frame {}", via[goal], inputs.len());
            goal += 1;
            if no_exit && goal == via.len() {
                break;
            }
        }
        if st.l(LEVEL_STATE) == 1 {
            eprintln!("exit at frame {}", inputs.len());
            break;
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
