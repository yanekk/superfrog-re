//! Plans a level's completion on the engine, with every object the dispatch has live (the
//! enemies freeze leaves only what nothing ports frozen), and prints it as an input script: how
//! the `enemies-w1-*` cases' scripts are written (T42).
//!
//! `cargo run --release --example complete -- <level> [--prefix <script>] [--coins <n>] [--paced] [--hold <n>] [--beam <n>] [--steps <n>]`
//!
//! From the level's start (after the prefix script), the frog heads for the nearest coin left in
//! the map (markers 8 and 9; a coin is taken when its marker is cleared), by the beam search
//! `plan` uses, until no coin is needed (`frog+$5c`, the exit's count), then for the exit (marker
//! 54) until `level_state` is set. A path where a life is lost is dropped; one where the frog is
//! hurt is kept. A coin not reached in the search's horizon is passed over for the next nearest.
//! The engine plans unpaced, and the original's lag makes its objects slower, so a long plan
//! leaves the original's route (T84 review). Plan in legs instead: `--coins n` stops after `n`
//! coins, and `--paced` starts from the engine as the original paces it through the prefix (a
//! conformance run of the prefix), so each leg starts where the original is; run it again with
//! the printed script as the next prefix until the exit is reached.
use sf_conform::freeze::Freeze;
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::{h0, h1};
use sf_core::{Game, Start};

/// The map's row 0 (`map_rows[1]`, h1), a row's length in bytes, and the level's size in cells.
const ROW0: u32 = 0x218e0 + 0x4c0;
const ROW_BYTES: u32 = 0x260;
const COLS: u32 = 300;
const ROWS: u32 = 84;
/// `level_state` (h0+$a06).
const LEVEL_STATE: u32 = h0(0xa06);

fn marker(g: &Game, col: u32, row: u32) -> u16 {
    g.state().w(h1(ROW0) + row * ROW_BYTES + 2 * col) & 0x3f
}

/// The frog's centre, in map pixels.
fn frog(g: &Game) -> (i32, i32) {
    let w = |o| g.state().w(FROG + o) as i16 as i32;
    (w(f::X) + 0x17, w(f::Y) + 0x2a)
}

fn cells_marked(g: &Game, ms: &[u16]) -> Vec<(u32, u32)> {
    let mut v = Vec::new();
    for r in 0..ROWS {
        for c in 0..COLS {
            if ms.contains(&marker(g, c, r)) {
                v.push((c, r));
            }
        }
    }
    v
}

/// Beam search from `g` until `reached`, steering by distance to (`tx`, `ty`); the inputs and the
/// game after them, or `None` within `steps` steps.
fn search(
    g: &Game,
    hold: u32,
    width: usize,
    steps: u32,
    targets: &[(i32, i32)],
    reached: &(dyn Fn(&Game) -> bool + Sync),
) -> Option<(Game, Vec<FrameInput>)> {
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
    let score = |g: &Game| {
        let (x, y) = frog(g);
        targets.iter().map(|&(tx, ty)| (x - tx).abs() + 2 * (y - ty).abs()).min().unwrap_or(0)
    };
    let mut beam: Vec<(Game, Vec<FrameInput>)> = vec![(g.clone(), Vec::new())];
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    // Given up once the best distance has not improved for `STALL` steps.
    const STALL: u32 = 40;
    let (mut best, mut since) = (i32::MAX, 0);
    for _ in 0..steps {
        // Each beam entry's children, expanded on every core.
        let jobs: Vec<(&Game, &Vec<FrameInput>, Joy)> = beam
            .iter()
            .flat_map(|(b, p)| acts.iter().map(move |&a| (b, p, a)))
            .collect();
        let chunk = jobs.len().div_ceil(threads).max(1);
        type Child = (Option<i32>, Game, Vec<FrameInput>);
        let results: Vec<Child> = std::thread::scope(|sc| {
            let hs: Vec<_> = jobs
                .chunks(chunk)
                .map(|js| {
                    sc.spawn(move || {
                        js.iter()
                            .filter_map(|&(b, path, a)| {
                                let mut n = b.clone();
                                let mut p = path.clone();
                                for _ in 0..hold {
                                    let x = FrameInput::joy(a);
                                    n.step(x);
                                    p.push(x);
                                    if n.state().w(FROG + f::LIVES) != lives
                                        || n.state().w(FROG + f::DYING) != 0
                                    {
                                        return None;
                                    }
                                    if reached(&n) {
                                        return Some((None, n, p));
                                    }
                                }
                                Some((Some(score(&n)), n, p))
                            })
                            .collect::<Vec<Child>>()
                    })
                })
                .collect();
            hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
        });
        let mut next: Vec<(i32, Game, Vec<FrameInput>)> = Vec::new();
        for (sc, n, p) in results {
            match sc {
                None => return Some((n, p)),
                Some(sc) => next.push((sc, n, p)),
            }
        }
        next.sort_by_key(|x| x.0);
        match next.first() {
            Some(n) if n.0 < best => (best, since) = (n.0, 0),
            _ => since += 1,
        }
        if since >= STALL {
            return None;
        }
        let mut seen = std::collections::BTreeSet::new();
        beam = next
            .into_iter()
            .filter(|(_, n, _)| {
                let w = |o| n.state().w(FROG + o);
                seen.insert((w(f::X) / 3, w(f::Y) / 3, w(f::VY), w(f::ON_GROUND)))
            })
            .take(width)
            .map(|(_, n, p)| (n, p))
            .collect();
        if beam.is_empty() {
            break;
        }
    }
    None
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let id = &args[0];
    let mut prefix = InputScript::idle();
    let (mut hold, mut width, mut steps) = (4u32, 48usize, 120u32);
    let (mut paced, mut max_coins) = (false, u32::MAX);
    let mut i = 1;
    while i < args.len() {
        let v = args.get(i + 1).cloned().unwrap_or_default();
        match args[i].as_str() {
            "--prefix" => prefix = InputScript::load(&sf_conform::case::script_path(&v)).unwrap(),
            "--hold" => hold = v.parse().unwrap(),
            "--beam" => width = v.parse().unwrap(),
            "--steps" => steps = v.parse().unwrap(),
            "--coins" => max_coins = v.parse().unwrap(),
            "--paced" => {
                paced = true;
                i += 1;
                continue;
            }
            a => panic!("unknown argument {a}"),
        }
        i += 2;
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
    let pre_end = prefix
        .to_oracle()
        .canonical()
        .lines()
        .filter_map(|l| l.split(' ').next()?.trim().parse::<u32>().ok())
        .max()
        .map_or(0, |m| m + 1);
    if paced && pre_end > 0 {
        // The engine as the original paces it through the prefix.
        let text = format!(
            "sync = {{ level = \"{id}\" }}\nframes = {}\nresync = true\n\
             [[exclude]]\nwhat = \"enemies\"\nnote = \"a plan's prefix\"\ntask = \"T42\"\n",
            pre_end + 1
        );
        let mut case = sf_conform::Case::parse("plan", &text).unwrap();
        case.script = prefix.clone();
        let mut at = None;
        let r = sf_conform::run_with(&case, |f, n| {
            if f == pre_end {
                at = Some(n.clone());
            }
        });
        if let Err(d) = &r {
            eprintln!("the prefix diverges from the original: {d}");
        }
        g = at.expect("the prefix's end reached");
        inputs = (0..pre_end).map(|fr| prefix.input_at(fr)).collect();
    } else {
        for fr in 0..pre_end {
            let x = prefix.input_at(fr);
            g.step(x);
            inputs.push(x);
        }
    }
    // The level's drop-in: the frog cannot move before about frame 148.
    while inputs.len() < 150 {
        g.step(FrameInput::default());
        inputs.push(FrameInput::default());
    }
    // Coins the search could not reach from where it was: left out, all tried again once every
    // other coin is taken or left out (while that took a coin).
    let mut skipped: Vec<(i32, i32)> = Vec::new();
    let mut took_since_retry = true;
    let mut taken = 0;
    while taken < max_coins {
        let left = g.state().w(FROG + 0x5c);
        if left == 0 {
            break;
        }
        let coins: Vec<(i32, i32)> = cells_marked(&g, &[8, 9])
            .into_iter()
            .map(|(c, r)| ((c * 16 + 8) as i32, (r * 16 + 8) as i32))
            .filter(|c| !skipped.contains(c))
            .collect();
        if coins.is_empty() && took_since_retry && !skipped.is_empty() {
            skipped.clear();
            took_since_retry = false;
            continue;
        }
        if coins.is_empty() {
            eprintln!("no coin left to reach from frame {}; {left} still needed", inputs.len());
            break;
        }
        // Taken, and the frog on the ground again: a leg never ends mid-jump over a pit.
        let reached =
            |n: &Game| n.state().w(FROG + 0x5c) < left && n.state().w(FROG + f::ON_GROUND) != 0;
        match search(&g, hold, width, steps, &coins, &reached) {
            Some((n, p)) => {
                inputs.extend(p);
                g = n;
                took_since_retry = true;
                taken += 1;
                eprintln!("coin: frame {}, {} needed", inputs.len(), g.state().w(FROG + 0x5c));
            }
            None => {
                // The nearest coin drew the search into a dead end: leave it out and try again.
                let (x, y) = frog(&g);
                let near = *coins
                    .iter()
                    .min_by_key(|&&(tx, ty)| (x - tx).abs() + 2 * (y - ty).abs())
                    .expect("a coin");
                skipped.push(near);
            }
        }
    }
    if g.state().w(FROG + 0x5c) != 0 {
        print_script(&inputs);
        return;
    }
    let exits = cells_marked(&g, &[54]);
    let &(ec, er) = exits.first().expect("an exit");
    let done = search(
        &g,
        hold,
        width,
        steps * 4,
        &[((ec * 16 + 8) as i32, (er * 16 + 8) as i32)],
        &|n: &Game| n.state().l(LEVEL_STATE) == 1,
    );
    match done {
        Some((_, p)) => {
            inputs.extend(p);
            eprintln!("exit {ec},{er}: frame {}", inputs.len());
        }
        None => eprintln!("exit {ec},{er}: not reached"),
    }
    print_script(&inputs);
}

fn print_script(inputs: &[FrameInput]) {
    let mut last = None;
    for (fr, x) in inputs.iter().enumerate() {
        if Some(*x) != last {
            println!("{fr}: {}", InputScript::format_input(*x));
            last = Some(*x);
        }
    }
    // Where the plan ends, so a next leg's prefix runs to here.
    println!("{}: -", inputs.len());
}
