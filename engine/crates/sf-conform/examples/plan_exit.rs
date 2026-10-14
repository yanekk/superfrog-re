//! Finds inputs that play a level to its exit with every object live, and prints them as an input
//! script: how the `enemies-w6-*` completion cases' scripts are written (T40).
//!
//! `cargo run --release --example plan_exit -- <level> [--via col,row | --stomp <update> | --hurt <update> | --coins N]...
//! [--frames N] [--prefix script.txt] [--no-exit] [--out script.txt]` (`--no-exit`: stop at the last `--via`;
//! `--out`: the script so far, rewritten after each chunk) (a script this printed: its inputs are played first, up to its closing `-` line)
//!
//! The coin phase (T104): a W6 exit stays shut until `coins_left_for_exit` is 0, so before the
//! exit the frog takes coins, steering by a field flooded from every coin left in the live map
//! (markers 8 and 9; a taken coin's marker is cleared), each coin taken worth more than any
//! distance. `--coins N` is a step that ends when N coins are still needed (to take coins before a
//! switch's `--via`); one ending at 0 is added before the exit unless given. A coin the frog makes
//! no headway toward for `STALL` chunks is passed over until no other is left.
//!
//! The plan is made on the engine and played on the original as it goes: from the original's
//! memory a beam search over ten joystick inputs, each held 4 frames, looks `HORIZON` frames
//! ahead toward the next `--via` cell, then the exit (marker 54), scoring each end by a distance
//! field flooded through the map's open cells (so it routes round walls), less for hearts lost.
//! Its first `COMMIT` frames are played on the original, once the whole path has kept the
//! original's frog alive; else the next-best path is tried. The original lags in a busy level and the engine
//! free-running does not, so only the original's outcome counts, and each chunk starts from its
//! memory again.
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
/// Chunks without a coin taken or the distance shortened before the nearest coin is passed over.
const STALL: u32 = 5;
/// The map's row 0 (`map_rows[1]`, h1), and a row's length in bytes: the live markers.
const ROW0: u32 = 0x218e0 + 0x4c0;
const ROW_BYTES: u32 = 0x260;

/// The cells whose live marker is a coin's (8, 9), less `skipped`.
fn coins(st: &sf_core::State, skipped: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut v = Vec::new();
    for r in 0..84 {
        for c in 0..300 {
            let m = st.w(sf_core::layout::h1(ROW0) + r * ROW_BYTES + 2 * c) & 0x3f;
            if matches!(m, 8 | 9) && !skipped.contains(&(c as usize, r as usize)) {
                v.push((c as usize, r as usize));
            }
        }
    }
    v
}

/// The frog's middle, in map pixels a row up: `/ 16` gives the map cell of its lower half (the
/// map's pixel rows start one row above its first; the frog stands with its feet at y + `$10`).
fn frog(st: &sf_core::State) -> (i32, i32) {
    let w = |o| st.w(FROG + o) as i16 as i32;
    (w(f::X) + 0x17, w(f::Y) - 0x10)
}

/// The cost to `goal` from every cell: a step through an open cell 1, through a solid one
/// (`marker & $18 == $18`) 60, a deadly one (6) 40, with the map as the level's file has it.
fn flood(body: &[u16], w: usize, h: usize, goal: &[(usize, usize)]) -> Vec<u32> {
    // Solid cells cost 60 (a switch's gate is solid until it opens), deadly ones 40.
    let cost = |x: usize, y: usize| match body[y * w + x] >> 10 {
        m if m & 0x18 == 0x18 => 60,
        6 => 40,
        _ => 1,
    };
    let mut d = vec![u32::MAX; w * h];
    let mut q = std::collections::BinaryHeap::new();
    for &(x, y) in goal {
        d[y * w + x] = 0;
        q.push(std::cmp::Reverse((0u32, x, y)));
    }
    while let Some(std::cmp::Reverse((here, x, y))) = q.pop() {
        if here > d[y * w + x] {
            continue;
        }
        let near = [(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1)), (x, y + 1)];
        for (nx, ny) in near {
            if nx < w && ny < h {
                let nd = here + cost(nx, ny);
                if nd < d[ny * w + nx] {
                    d[ny * w + nx] = nd;
                    q.push(std::cmp::Reverse((nd, nx, ny)));
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
    /// Coins taken until this many are still needed.
    Coins(u16),
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
    if !no_exit && !via.iter().any(|v| matches!(v, Via::Coins(0))) {
        via.push(Via::Coins(0));
    }
    let data = sf_conform::game_data();
    let t = &data.levels.iter().find(|l| *l.0 == id).expect("level").1.t7mp;
    let (w, h) = (t.width as usize, t.height as usize);
    let exits: Vec<(usize, usize)> = (0..w * h)
        .filter(|&k| t.body[k] >> 10 == 54)
        .map(|k| (k % w, k / w))
        .collect();
    let mut goals: Vec<Vec<u32>> = via
        .iter()
        .map(|v| match *v {
            Via::Cell(x, y) => flood(&t.body, w, h, &[(x, y)]),
            _ => Vec::new(),
        })
        .collect();
    goals.push(flood(&t.body, w, h, &exits));
    let mut goal = 0;
    // The coin phase's passed-over coins, its stall count and best distance so far.
    let mut skipped: Vec<(usize, usize)> = Vec::new();
    let (mut stall, mut best_d) = (0u32, i64::MAX);

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
    let mut history: Vec<(sf_oracle::Snapshot, usize, usize, usize)> = Vec::new();
    let mut skip = 0usize;
    let mut backtracks = 0;
    while inputs.len() < budget {
        let mem = sync::core_memory(amiga.memory());
        let start = Game::with_display(display.clone(), Start::Transplant(mem));
        let lives = start.state().w(FROG + f::LIVES);
        let target = via.get(goal).copied();
        let needed = start.state().w(FROG + f::COINS_LEFT);
        if let Some(Via::Coins(_)) = target {
            let mut left = coins(start.state(), &skipped);
            if left.is_empty() && !skipped.is_empty() {
                eprintln!("  every coin left was passed over: trying them again");
                skipped.clear();
                left = coins(start.state(), &skipped);
            }
            goals[goal] = flood(&t.body, w, h, &left);
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
                    // A coin taken is worth more than any distance (the field still has it).
                    let taken = needed.saturating_sub(st.w(FROG + f::COINS_LEFT)) as i64;
                    d * 16 + (16 - (x % 16 - 8).abs()) as i64 / 4 - 400 * st.w(HEARTS) as i64
                        - 8000 * taken
                }
            }
        };
        let mut beam: Vec<(Game, Vec<FrameInput>)> = vec![(start, Vec::new())];
        let mut alive = Vec::new();
        for _ in 0..HORIZON {
            let mut next: Vec<(i64, Game, Vec<FrameInput>)> = Vec::new();
            for (b, path) in &beam {
                for &a in &acts {
                    let mut n = b.clone();
                    let mut p = path.clone();
                    let mut dead = false;
                    for _ in 0..HOLD {
                        let x = FrameInput::joy(a);
                        n.step(x);
                        p.push(x);
                        if n.state().w(FROG + f::LIVES) != lives || n.state().w(FROG + f::DYING) != 0 {
                            dead = true;
                            break;
                        }
                    }
                    if !dead {
                        let s = if n.state().l(LEVEL_STATE) == 1 { i64::MIN / 2 } else { score(n.state()) };
                        next.push((s, n, p));
                    }
                }
            }
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
            alive = beam.clone();
        }
        // Play the best path's first frames on the original; the next best if that loses a life.
        let snap = amiga.snapshot();
        let mut taken = None;
        // Paths that end on the ground first: one still falling may be falling to its death just
        // past the horizon.
        let mut order: Vec<&(Game, Vec<FrameInput>)> = beam.iter().collect();
        order.sort_by_key(|(g, _)| g.state().w(FROG + f::ON_GROUND) == 0);
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
            let Some((s, len, g, sk)) = popped.filter(|_| backtracks <= 60) else {
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
            Some(&Via::Coins(n)) => {
                let left = st.w(FROG + f::COINS_LEFT);
                let d = dist(&goals[goal], (x.max(0) / 16) as usize, (y.max(0) / 16) as usize);
                if left < needed || d < best_d {
                    (stall, best_d) = (0, if left < needed { i64::MAX } else { d });
                } else {
                    stall += 1;
                }
                if stall >= STALL {
                    // No headway: pass over the coin nearest the frog by the map's paths.
                    let here = flood(&t.body, w, h, &[((x.max(0) / 16) as usize, (y.max(0) / 16) as usize)]);
                    if let Some(&c) = coins(&st, &skipped).iter().min_by_key(|&&(c, r)| here[r * w + c]) {
                        eprintln!("  passing over the coin at {},{}", c.0, c.1);
                        skipped.push(c);
                    }
                    (stall, best_d) = (0, i64::MAX);
                }
                eprintln!("  coins needed {left}");
                if left <= n { 0 } else { 99 }
            }
            _ => dist(&goals[goal], (x.max(0) / 16) as usize, (y.max(0) / 16) as usize),
        };
        eprintln!(
            "frame {}: frog {},{} (cell {},{}) goal {goal} distance {d} hearts {}",
            inputs.len(),
            x,
            y,
            x / 16,
            y / 16,
            st.w(HEARTS)
        );
        if goal < via.len() && d <= 2 {
            eprintln!("via {:?} at frame {}", via[goal], inputs.len());
            goal += 1;
            skipped.clear();
            (stall, best_d) = (0, i64::MAX);
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
