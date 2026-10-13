//! Finds inputs that take the frog to map cells, on the engine from a level's start with the
//! enemies frozen (as the item cases are), and prints them as an input script: how the `items-*`
//! cases' scripts are written.
//!
//! `cargo run --release --example plan -- <level> <col,row>... [--prefix <script>] [--hold <n>]`
//!
//! The targets are reached in turn, after the prefix script's inputs. A target is reached when
//! its marker changes (taken) or the frog's box covers it; a beam search over ten joystick inputs,
//! each held `n` frames (4), steers toward it and drops any path where the frog dies.
use sf_conform::freeze::Freeze;
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::h1;
use sf_core::{Game, Start};

/// The map's row 0 (`map_rows[1]`, h1), and a row's length in bytes.
const ROW0: u32 = 0x218e0 + 0x4c0;
const ROW_BYTES: u32 = 0x260;

/// The marker of map cell `col,row`.
fn marker(g: &Game, col: u32, row: u32) -> u16 {
    let addr = h1(ROW0) + row * ROW_BYTES + 2 * col;
    g.state().w(addr) & 0x3f
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
                targets.push((c.parse::<u32>().unwrap(), r.parse::<u32>().unwrap()));
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
        let start_marker = marker(&g, tc, tr);
        let (tx, ty) = ((tc * 16 + 8) as i32, (tr * 16 + 8) as i32);
        let reached = |g: &Game| {
            let (x, y) = frog(g);
            if start_marker != 0 {
                marker(g, tc, tr) != start_marker
            } else {
                (x - tx).abs() < 10 && (y - ty).abs() < 20
            }
        };
        let score = |g: &Game| {
            let (x, y) = frog(g);
            (x - tx).abs() + 2 * (y - ty).abs()
        };
        let mut beam: Vec<(Game, Vec<FrameInput>)> = vec![(g.clone(), Vec::new())];
        let mut done = None;
        'search: for step in 0..250 {
            let mut next: Vec<(i32, Game, Vec<FrameInput>)> = Vec::new();
            for (b, path) in &beam {
                for &a in &acts {
                    let mut n = b.clone();
                    let mut p = path.clone();
                    let mut dead = false;
                    for _ in 0..hold {
                        let x = FrameInput::joy(a);
                        n.step(x);
                        p.push(x);
                        if n.state().w(FROG + f::LIVES) != lives
                            || n.state().w(FROG + f::DYING) != 0
                        {
                            dead = true;
                            break;
                        }
                        if reached(&n) {
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
                .take(48)
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
