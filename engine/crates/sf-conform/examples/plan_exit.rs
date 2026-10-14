//! Finds inputs that play a level to its exit with every object live, and prints them as an input
//! script: how the `enemies-w6-*` completion cases' scripts are written (T40).
//!
//! `cargo run --release --example plan_exit -- <level> [--via col,row | --stomp <update> | --hurt <update> | --coins N]...
//! [--frames N] [--prefix script.txt] [--no-exit] [--out script.txt]` (`--no-exit`: stop at the last `--via`;
//! `--out`: the script so far, rewritten after each chunk) (a script this printed: its inputs are played first, up to its closing `-` line)
//!
//! The coin phase (T104): a W6 exit stays shut until `coins_left_for_exit` is 0, so before the
//! exit the frog takes coins, steering by a field flooded over the live map from every coin left
//! (markers 8 and 9; a taken coin's marker is cleared) and every switch not yet pressed (a
//! vertical pair of markers 48–51, cleared when pressed: it opens the gate whose single cell has
//! its number), each coin taken or switch pressed worth more than any distance. `--coins N` is a step that ends when N coins are still needed (to take coins before a
//! switch's `--via`); one ending at 0 is added before the exit unless given. A coin the frog makes
//! no headway toward for `STALL` chunks is passed over for `PASSED` chunks, or until no other is left. The field knows
//! nothing of gravity or bounce pads, so each frame the original's frog spends in a cell makes
//! ending there `VISIT` worse until the next coin or switch: a stuck frog wanders off to find the
//! way up.
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
/// Chunks without a coin taken or the distance shortened by two cells before the nearest coin, and
/// every coin within four cells of it, is passed over (a jump's height wobbles the distance).
const STALL: u32 = 8;
/// Chunks a passed-over coin stays out of the field.
const PASSED: usize = 12;
/// What a frame already spent in a cell adds to ending a path there, in the score's units (16 a
/// cell of distance).
const VISIT: i64 = 4;
/// Paths tried on the original itself when every engine path dies there.
const ORACLE_TRIES: usize = 48;

/// xorshift64: the original's own search's mutations.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}
/// The map's row 0 (`map_rows[1]`, h1), and a row's length in bytes: the live markers.
const ROW0: u32 = 0x218e0 + 0x4c0;
const ROW_BYTES: u32 = 0x260;

/// The live map, as the level file has its cells (marker in the top six bits): an opened gate is
/// open here, where the file still has it solid.
fn live_body(st: &sf_core::State) -> Vec<u16> {
    let mut v = Vec::with_capacity(300 * 84);
    for r in 0..84 {
        for c in 0..300 {
            v.push((st.w(sf_core::layout::h1(ROW0) + r * ROW_BYTES + 2 * c) & 0x3f) << 10);
        }
    }
    v
}

fn live_marker(st: &sf_core::State, c: usize, r: usize) -> u16 {
    st.w(sf_core::layout::h1(ROW0) + r as u32 * ROW_BYTES + 2 * c as u32) & 0x3f
}

/// The cells of the switches not yet pressed: markers 48–51 with the same one above or below.
fn switches(st: &sf_core::State) -> Vec<(usize, usize)> {
    let mut v = Vec::new();
    for r in 0..84 {
        for c in 0..300 {
            let m = live_marker(st, c, r);
            let pair = (r > 0 && live_marker(st, c, r - 1) == m) || (r < 83 && live_marker(st, c, r + 1) == m);
            if (48..=51).contains(&m) && pair {
                v.push((c, r));
            }
        }
    }
    v
}

/// How many of `cells` (switch cells) no longer hold a switch: pressed.
fn pressed(st: &sf_core::State, cells: &[(usize, usize)]) -> usize {
    cells.iter().filter(|&&(c, r)| !(48..=51).contains(&live_marker(st, c, r))).count()
}

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
        62 => 1,
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

/// Cells a jump rises from where the frog stands (115 pixels measured with UP held), and from a
/// bounce pad (markers 4, 5; a guess).
const JUMP: u32 = 7;
const PAD_JUMP: u32 = 16;

/// [`flood`] with gravity: a step up is only taken within a jump of a cell the frog can stand on
/// (an open cell over a solid one or a floor, in this column or the next) or in a lift's shaft,
/// sideways and down always. Where
/// that leaves a cell no way to `goal`, the plain flood's cost plus 50 000.
fn climb(body: &[u16], w: usize, h: usize, goal: &[(usize, usize)]) -> Vec<u32> {
    // Marker 62 is a floor the frog passes up through: stacked over a lift cell (44, which raises
    // a frog standing in it a row a frame) it is a lift's shaft.
    let solid = |x: usize, y: usize| body[y * w + x] >> 10 & 0x18 == 0x18 && body[y * w + x] >> 10 != 62;
    let lift = |x: usize, y: usize| matches!(body[y * w + x] >> 10, 44 | 62);
    // How far a step up into each cell is from the nearest ground below it, a pad's lift taken off.
    let mut reach = vec![u32::MAX; w * h];
    for x in 0..w {
        let mut below = u32::MAX;
        for y in (0..h).rev() {
            if solid(x, y) {
                below = u32::MAX;
                continue;
            }
            if y + 1 < h && (solid(x, y + 1) || body[(y + 1) * w + x] >> 10 == 62) {
                below = 0;
            } else if below != u32::MAX {
                below += 1;
            }
            let pad = matches!(body[y * w + x] >> 10, 4 | 5);
            let lift = if pad { below.saturating_sub(PAD_JUMP - JUMP) } else { below };
            reach[y * w + x] = lift;
        }
    }
    // Pads lift every cell above them in the column, not only their own.
    for x in 0..w {
        let mut pad_at: Option<usize> = None;
        for y in (0..h).rev() {
            if solid(x, y) {
                pad_at = None;
                continue;
            }
            if matches!(body[y * w + x] >> 10, 4 | 5) {
                pad_at = Some(y);
            }
            if let Some(py) = pad_at {
                let k = &mut reach[y * w + x];
                *k = (*k).min(((py - y) as u32).saturating_sub(PAD_JUMP - JUMP));
            }
        }
    }
    let up_ok = |x: usize, y: usize| {
        let near = [x.wrapping_sub(1), x, x + 1];
        near.iter().any(|&c| c < w && reach[y * w + c] <= JUMP)
            || solid(x, y)
            || lift(x, y)
            || (y + 1 < h && lift(x, y + 1))
    };
    // A wall dearer than in the plain flood: a long way round beats climbing through one (W6-2's
    // shaft), and a closed gate is still crossed when nothing else reaches the goal. A deadly cell
    // is never crossed (W6-2's bounce pads under spikes).
    let cost = |x: usize, y: usize| match body[y * w + x] >> 10 {
        62 => 1,
        m if m & 0x18 == 0x18 => 400,
        6 => u32::MAX,
        _ => 1,
    };
    let mut d = vec![u32::MAX; w * h];
    let mut q = std::collections::BinaryHeap::new();
    for &(x, y) in goal {
        d[y * w + x] = 0;
        q.push(std::cmp::Reverse((0u32, x, y)));
    }
    // Backwards from the goals: a cell before (x, y) is beside it, above it (a fall), or below
    // it when the step up into (x, y) is in a jump's reach.
    while let Some(std::cmp::Reverse((here, x, y))) = q.pop() {
        if here > d[y * w + x] {
            continue;
        }
        let mut near = vec![(x.wrapping_sub(1), y), (x + 1, y), (x, y.wrapping_sub(1))];
        if up_ok(x, y) {
            near.push((x, y + 1));
        }
        for (nx, ny) in near {
            if nx < w && ny < h && cost(nx, ny) != u32::MAX {
                let nd = here + cost(nx, ny);
                if nd < d[ny * w + nx] {
                    d[ny * w + nx] = nd;
                    q.push(std::cmp::Reverse((nd, nx, ny)));
                }
            }
        }
    }
    let plain = flood(body, w, h, goal);
    d.iter()
        .zip(&plain)
        .map(|(&g, &p)| if g != u32::MAX { g } else { p.saturating_add(50_000) })
        .collect()
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
    // Each step's field, flooded over the live map at each chunk.
    let mut goals: Vec<Vec<u32>> = vec![Vec::new(); via.len() + 1];
    let mut goal = 0;
    // The coin phase's passed-over coins, its stall count and best distance so far.
    // Passed-over coins and switches, each with the chunk it comes back at.
    let mut passed: Vec<((usize, usize), usize)> = Vec::new();
    let mut chunk = 0usize;
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    // Frames the original's frog spent in each cell since the last coin or switch.
    let mut visits: std::collections::HashMap<(i32, i32), i64> = Default::default();
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
        chunk += 1;
        passed.retain(|&(_, until)| until > chunk);
        let skipped: Vec<(usize, usize)> = passed.iter().map(|&(c, _)| c).collect();
        let mem = sync::core_memory(amiga.memory());
        let start = Game::with_display(display.clone(), Start::Transplant(mem));
        let lives = start.state().w(FROG + f::LIVES);
        let target = via.get(goal).copied();
        let needed = start.state().w(FROG + f::COINS_LEFT);
        let live = live_body(start.state());
        let sw = switches(start.state());
        match target {
            Some(Via::Coins(_)) => {
                let mut left = coins(start.state(), &skipped);
                if left.is_empty() && !skipped.is_empty() {
                    eprintln!("  every coin left was passed over: trying them again");
                    passed.clear();
                    left = coins(start.state(), &[]);
                }
                left.extend(sw.iter().filter(|c| !passed.iter().any(|(p, _)| p == *c)));
                goals[goal] = climb(&live, w, h, &left);
            }
            Some(Via::Cell(x, y)) => goals[goal] = climb(&live, w, h, &[(x, y)]),
            None => goals[goal] = climb(&live, w, h, &exits),
            _ => {}
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
                    // A coin taken or a switch pressed is worth more than any distance (the field
                    // still has it).
                    let mut taken = needed.saturating_sub(st.w(FROG + f::COINS_LEFT)) as i64;
                    if let Some(Via::Coins(_)) = target {
                        taken += pressed(st, &sw) as i64;
                    }
                    let been = if let Some(Via::Coins(_)) = target {
                        visits.get(&(x / 16, y / 16)).copied().unwrap_or(0) * VISIT
                    } else {
                        0
                    };
                    d * 16 + (16 - (x % 16 - 8).abs()) as i64 / 4 - 400 * st.w(HEARTS) as i64
                        - 8000 * taken
                        + been
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
                        n.step_unseen(x);
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
        // The cells the original's frog is in over what is kept, counted once `score` is done.
        let mut been_in: Vec<(i32, i32)> = Vec::new();
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
                    let (fx, fy) = (
                        peek_w(amiga, FROG + f::X) as i16 as i32 + 0x17,
                        peek_w(amiga, FROG + f::Y) as i16 as i32 - 0x10,
                    );
                    been_in.push((fx / 16, fy / 16));
                }
                taken = Some(p[..n].to_vec());
                break;
            }
        }
        if taken.is_none() {
            // Every engine path dies on the original (its lag shortens jumps): search on the
            // original itself, the beam's paths with holds changed at random, scored from its
            // memory at the path's end.
            let mut best: Option<(i64, Vec<FrameInput>)> = None;
            for k in 0..ORACLE_TRIES {
                let mut p: Vec<FrameInput> = beam.get(k % beam.len().max(1)).map_or(Vec::new(), |b| b.1.clone());
                p.resize((HORIZON * HOLD) as usize, FrameInput::default());
                for h in 0..HORIZON as usize {
                    if rng.next() % 10 < 3 {
                        let a = FrameInput::joy(acts[(rng.next() % acts.len() as u64) as usize]);
                        p[h * HOLD as usize..(h + 1) * HOLD as usize].fill(a);
                    }
                }
                let mut ok = true;
                for &x in &p {
                    sync::step_inserting(amiga, x).unwrap();
                    if peek_w(amiga, FROG + f::LIVES) != lives || peek_w(amiga, FROG + f::DYING) != 0 {
                        ok = false;
                        break;
                    }
                }
                if ok {
                    let st = sf_core::State::transplant(&sync::core_memory(amiga.memory()));
                    let sc = if st.l(LEVEL_STATE) == 1 { i64::MIN / 2 } else { score(&st) };
                    if best.as_ref().is_none_or(|b| sc < b.0) {
                        best = Some((sc, p));
                    }
                }
                amiga.restore(&snap);
            }
            if let Some((sc, p)) = best {
                eprintln!("  the original's own search: a path scoring {sc}");
                let n = p.len().min(COMMIT);
                for &x in &p[..n] {
                    sync::step_inserting(amiga, x).unwrap();
                    let (fx, fy) = (
                        peek_w(amiga, FROG + f::X) as i16 as i32 + 0x17,
                        peek_w(amiga, FROG + f::Y) as i16 as i32 - 0x10,
                    );
                    been_in.push((fx / 16, fy / 16));
                }
                taken = Some(p[..n].to_vec());
            }
        }
        for c in been_in {
            *visits.entry(c).or_insert(0) += 1;
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
                let pushed = pressed(&st, &sw) > 0;
                if left < needed || pushed {
                    visits.clear();
                }
                if pushed {
                    eprintln!("  a switch pressed");
                }
                let d = dist(&goals[goal], (x.max(0) / 16) as usize, (y.max(0) / 16) as usize);
                if left < needed || pushed || d < best_d.saturating_sub(1) {
                    (stall, best_d) = (0, if left < needed || pushed { i64::MAX } else { d });
                } else {
                    stall += 1;
                }
                if stall >= STALL {
                    // No headway: pass over the coin nearest the frog by the map's paths.
                    let here = climb(&live_body(&st), w, h, &[((x.max(0) / 16) as usize, (y.max(0) / 16) as usize)]);
                    let mut left = coins(&st, &skipped);
                    left.extend(switches(&st).into_iter().filter(|c| !skipped.contains(c)));
                    if let Some(&(nc, nr)) = left.iter().min_by_key(|&&(c, r)| here[r * w + c]) {
                        for &(c, r) in &left {
                            if c.abs_diff(nc) <= 4 && r.abs_diff(nr) <= 4 {
                                eprintln!("  passing over the coin at {c},{r}");
                                passed.push(((c, r), chunk + PASSED));
                            }
                        }
                    }
                    (stall, best_d) = (0, i64::MAX);
                }
                eprintln!("  coins needed {left}, distance {d}, stall {stall}");
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
            passed.clear();
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
