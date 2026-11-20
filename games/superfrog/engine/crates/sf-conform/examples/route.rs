//! `plan` for enemy cases (T41): the frog is steered to map cells with every object the engine
//! has a kind for live, by its steps to the target through open cells (all but markers `$38`–`$3f`,
//! gravity aside), so a route follows the maze. A target is the cell the frog's feet should be in,
//! reached within a column across and two rows up or down. Prints the input script.
//!
//! `cargo run --release --example route -- <level> <col,row | wait,N>... [--prefix <script>] [--hold <n>]`
use sf_conform::freeze::Freeze;
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::{Game, Start};

/// The map cell the frog's feet are in (`+$02` is about its feet's y).
fn feet(g: &Game) -> (i32, i32) {
    let (x, _) = frog(g);
    let y = g.state().w(FROG + f::Y) as i16 as i32;
    (x.div_euclid(16), (y - 8).div_euclid(16))
}

/// Steps from each cell to (`tc`, `tr`) through open cells, four ways (`u32::MAX`: none).
fn distances(open: &[bool], w: usize, h: usize, tc: usize, tr: usize) -> Vec<u32> {
    let mut d = vec![u32::MAX; w * h];
    let mut q = std::collections::VecDeque::new();
    d[tr * w + tc] = 0;
    q.push_back((tc, tr));
    while let Some((c, r)) = q.pop_front() {
        let here = d[r * w + c];
        let mut go = |c: usize, r: usize| {
            if open[r * w + c] && d[r * w + c] == u32::MAX {
                d[r * w + c] = here + 1;
                q.push_back((c, r));
            }
        };
        if c > 0 { go(c - 1, r); }
        if c + 1 < w { go(c + 1, r); }
        if r > 0 { go(c, r - 1); }
        if r + 1 < h { go(c, r + 1); }
    }
    d
}

/// The frog's centre, in map pixels.
fn frog(g: &Game) -> (i32, i32) {
    let w = |o| g.state().w(FROG + o) as i16 as i32;
    (w(f::X) + 0x17, w(f::Y) + 0x2a)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let id = &args[0];
    let mut targets = Vec::new();
    let mut prefix = InputScript::idle();
    let mut hold = 4u32;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--prefix" => {
                prefix = InputScript::load(&sf_conform::case::script_path(&args[i + 1])).unwrap();
                i += 2;
            }
            "--hold" => {
                hold = args[i + 1].parse().unwrap();
                i += 2;
            }
            t => {
                let (c, r) = t.split_once(',').unwrap();
                if c == "wait" {
                    targets.push((u32::MAX, r.parse::<u32>().unwrap()));
                } else {
                    targets.push((c.parse::<u32>().unwrap(), r.parse::<u32>().unwrap()));
                }
                i += 1;
            }
        }
    }
    let synced = sync::reach_frozen(
        &SyncPoint::AtLevel(id.parse().unwrap()),
        &InputScript::idle(),
        20000,
        Freeze::Enemies,
    )
    .unwrap();
    let display = sf_conform::display_data();
    let mut g = Game::with_display(display.clone(), Start::Transplant(synced.memory));
    let mut inputs: Vec<FrameInput> = Vec::new();
    // the prefix: run until its last change
    let pre_end = prefix
        .to_oracle()
        .canonical()
        .lines()
        .filter_map(|l| l.split(' ').next()?.trim().parse::<u32>().ok())
        .max()
        .map_or(0, |m| m + 1);
    for fr in 0..pre_end {
        let x = prefix.input_at(fr);
        g.step(x);
        inputs.push(x);
    }
    let lives = g.state().w(FROG + f::LIVES);
    // Open cells: all but solid ground (markers `$38`–`$3f`).
    let t7mp = &sf_conform::game_data().levels[&id.parse().unwrap()].t7mp;
    let (width, height) = (t7mp.width as usize, t7mp.height as usize);
    let open: Vec<bool> = (0..height)
        .flat_map(|r| (0..width).map(move |c| (r, c)))
        .map(|(r, c)| (t7mp.cell(c as u16, r as u16) >> 10) < 0x38)
        .collect();
    let acts = [
        Joy::NONE,
        Joy::RIGHT,
        Joy::LEFT,
        Joy::UP,
        Joy::RIGHT | Joy::UP,
        Joy::LEFT | Joy::UP,
        Joy::DOWN,
        Joy::FIRE,
        Joy::RIGHT | Joy::FIRE,
        Joy::LEFT | Joy::FIRE,
    ];
    for &(tc, tr) in &targets {
        // `wait,N`: nothing held for N frames.
        if tc == u32::MAX {
            for _ in 0..tr {
                let x = FrameInput::joy(Joy::NONE);
                g.step(x);
                inputs.push(x);
            }
            continue;
        }
        let dist = distances(&open, width, height, tc as usize, tr as usize);
        // The frog's feet within a column across and two rows up or down of the target cell.
        let reached = |g: &Game, _was: i32| {
            let (c, r) = feet(g);
            (c - tc as i32).abs() <= 1 && (r - tr as i32).abs() <= 2
        };
        // Steps through open cells to the target (gravity aside), then pixels across.
        let score = |g: &Game| {
            let (c, r) = feet(g);
            let d = if (0..width as i32).contains(&c) && (0..height as i32).contains(&r) {
                dist[r as usize * width + c as usize]
            } else {
                u32::MAX / 64
            };
            (d.min(u32::MAX / 64) as i32) * 16 + (frog(g).0 - (tc * 16 + 8) as i32).abs()
        };
        let mut beam: Vec<(Game, Vec<FrameInput>)> = vec![(g.clone(), Vec::new())];
        let mut done = None;
        'search: for step in 0..std::env::var("ROUTE_STEPS").map_or(250, |v| v.parse().unwrap()) {
            let mut next: Vec<(i32, Game, Vec<FrameInput>)> = Vec::new();
            for (b, path) in &beam {
                for &a in &acts {
                    let mut n = b.clone();
                    let mut p = path.clone();
                    let mut dead = false;
                    for _ in 0..hold {
                        let was = frog(&n).0;
                        let x = FrameInput::joy(a);
                        n.step(x);
                        p.push(x);
                        if n.state().w(FROG + f::LIVES) != lives
                            || n.state().w(FROG + f::DYING) != 0
                        {
                            dead = true;
                            break;
                        }
                        if reached(&n, was) {
                            done = Some((n, p));
                            break 'search;
                        }
                    }
                    if !dead {
                        next.push((score(&n), n, p));
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
                .take(std::env::var("ROUTE_BEAM").map_or(24, |v| v.parse().unwrap()))
                .map(|(_, n, p)| (n, p))
                .collect();
            if step % 25 == 0 {
                eprintln!(
                    "target {tc},{tr} step {step}: best {:?} frog {:?}",
                    beam.first().map(|b| score(&b.0)),
                    beam.first().map(|b| frog(&b.0))
                );
            }
            if beam.is_empty() {
                break;
            }
        }
        let Some((n, p)) = done else {
            eprintln!("target {tc},{tr}: not reached");
            break;
        };
        eprintln!(
            "target {tc},{tr}: reached at frame {}",
            inputs.len() + p.len()
        );
        inputs.extend(p);
        g = n;
    }
    // the script
    let mut last = None;
    for (fr, x) in inputs.iter().enumerate() {
        if Some(*x) != last {
            println!("{fr}: {}", InputScript::format_input(*x));
            last = Some(*x);
        }
    }
}
