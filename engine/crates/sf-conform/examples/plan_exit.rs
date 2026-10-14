//! Finds inputs that play a level to its exit with every object live, and prints them as an input
//! script: how the `enemies-w6-*` completion cases' scripts are written (T40).
//!
//! `cargo run --release --example plan_exit -- <level> [--via col,row]... [--frames N]
//! [--prefix script.txt]` (a script this printed: its inputs are played first, up to its closing `-` line)
//!
//! The plan is made on the engine and played on the original as it goes: from the original's
//! memory a beam search over ten joystick inputs, each held 4 frames, looks `HORIZON` frames
//! ahead toward the next `--via` cell, then the exit (marker 54), scoring each end by a distance
//! field flooded through the map's open cells (so it routes round walls), less for hearts lost.
//! Its first `COMMIT` frames are played on the original; a chunk that costs the original a life
//! is taken back and the next-best path tried. The original lags in a busy level and the engine
//! free-running does not, so only the original's outcome counts, and each chunk starts from its
//! memory again.
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::{h0, h1};
use sf_core::{Game, Start};

/// The map's row 0 (`map_rows[1]`, h1), and a row's length in bytes.
const ROW0: u32 = 0x218e0 + 0x4c0;
const ROW_BYTES: u32 = 0x260;
const LEVEL_STATE: u32 = h0(0xa06);
const HEARTS: u32 = h0(0x830);
const HORIZON: u32 = 30;
const HOLD: u32 = 4;
const COMMIT: usize = 48;
const BEAM: usize = 24;

/// The frog's centre, in map pixels.
fn frog(st: &sf_core::State) -> (i32, i32) {
    let w = |o| st.w(FROG + o) as i16 as i32;
    (w(f::X) + 0x17, w(f::Y) + 0x1a)
}

/// The cost to `goal` from every cell: a step through an open cell 1, through a solid one
/// (`marker & $18 == $18`) 60, with the map as the level's file has it.
fn flood(body: &[u16], w: usize, h: usize, goal: &[(usize, usize)]) -> Vec<u32> {
    // Solid cells cost 25: a switch's gate is solid until it opens.
    let cost = |x: usize, y: usize| if body[y * w + x] >> 10 & 0x18 == 0x18 { 60 } else { 1 };
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
    let mut via: Vec<(usize, usize)> = Vec::new();
    let mut budget = 12_000usize;
    let mut prefix: Vec<FrameInput> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--via" => {
                let (c, r) = args[i + 1].split_once(',').unwrap();
                via.push((c.parse().unwrap(), r.parse().unwrap()));
            }
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
    let mut goals: Vec<Vec<u32>> = via.iter().map(|&v| flood(&t.body, w, h, &[v])).collect();
    goals.push(flood(&t.body, w, h, &exits));
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
    while inputs.len() < budget {
        let mem = sync::core_memory(amiga.memory());
        let start = Game::with_display(display.clone(), Start::Transplant(mem));
        let lives = start.state().w(FROG + f::LIVES);
        let field = &goals[goal];
        let score = |st: &sf_core::State| -> i64 {
            let (x, y) = frog(st);
            let (cx, cy) = ((x.max(0) / 16) as usize, (y.max(0) / 16) as usize);
            let d = dist(field, cx, cy);
            d * 16 + (16 - (x % 16 - 8).abs()) as i64 / 4 - 400 * st.w(HEARTS) as i64
        };
        let mut beam: Vec<(Game, Vec<FrameInput>)> = vec![(start, Vec::new())];
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
                break;
            }
        }
        // Play the best path's first frames on the original; the next best if that loses a life.
        let snap = amiga.snapshot();
        let mut taken = None;
        for (_, p) in beam.iter().take(8) {
            let n = p.len().min(COMMIT);
            let mut ok = true;
            for &x in &p[..n] {
                sync::step_inserting(amiga, x).unwrap();
                if peek_w(amiga, FROG + f::LIVES) != lives || peek_w(amiga, FROG + f::DYING) != 0 {
                    ok = false;
                    break;
                }
            }
            if ok {
                taken = Some(p[..n].to_vec());
                break;
            }
            amiga.restore(&snap);
        }
        let Some(p) = taken else {
            eprintln!("frame {}: every path loses a life on the original", inputs.len());
            break;
        };
        inputs.extend(p);
        let o = sync::core_memory(amiga.memory());
        let st = sf_core::State::transplant(&o);
        let (x, y) = frog(&st);
        let d = dist(&goals[goal], (x.max(0) / 16) as usize, (y.max(0) / 16) as usize);
        eprintln!(
            "frame {}: frog {},{} (cell {},{}) goal {goal} distance {d} hearts {}",
            inputs.len(),
            x,
            y,
            x / 16,
            y / 16,
            st.w(HEARTS)
        );
        if goal < via.len() && d <= 1 {
            goal += 1;
        }
        if st.l(LEVEL_STATE) == 1 {
            eprintln!("exit at frame {}", inputs.len());
            break;
        }
    }
    let _ = h1(ROW0 + ROW_BYTES);
    let mut last = None;
    for (fr, x) in inputs.iter().enumerate() {
        if Some(*x) != last {
            println!("{fr}: {}", InputScript::format_input(*x));
            last = Some(*x);
        }
    }
    println!("{}: -", inputs.len());
}
