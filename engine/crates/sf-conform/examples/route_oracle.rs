//! `route` closed on the original (T105): a case's script is played on the oracle, then each
//! target is planned on the engine transplanted from the original's memory and played on the
//! original a chunk at a time, re-planning from where the original really is. The engine plans
//! unpaced (`examples/plan`, `route`), so a plan played as is drifts from the case run (a pad's
//! bounce two frames late, T85 review); re-planning from the original's state keeps it on the
//! route. Prints the whole script (the case's and the planned part).
//!
//! `cargo run --release --example route_oracle -- <case> <col,row | take,col,row | wait,N>... [--hold <n>] [--chunk <n>]`
//!
//! `col,row`: the frog's feet within a column across and two rows up or down of the cell.
//! `take,col,row`: the cell's marker changes (an item taken there).
use sf_conform::{Case, InputScript, sync};
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::{h0, h1};
use sf_core::{Game, Start};
use sf_oracle::Amiga;

/// The map's row 0 (`map_rows[1]`, h1), and a row's length in bytes.
const ROW0: u32 = 0x218e0 + 0x4c0;
const ROW_BYTES: u32 = 0x260;

#[derive(Clone, Copy)]
enum Target {
    Feet(u32, u32),
    Take(u32, u32),
    Wait(u32),
}

/// The frog's x, y, lives and dying flag as a reader of memory sees them.
struct Frog {
    x: i32,
    y: i32,
    lives: u16,
    dying: u16,
}

fn frog_of(w: impl Fn(u32) -> u16) -> Frog {
    Frog {
        x: w(FROG + f::X) as i16 as i32,
        y: w(FROG + f::Y) as i16 as i32,
        lives: w(FROG + f::LIVES),
        dying: w(FROG + f::DYING),
    }
}

/// The original spinning in `frog_tiles`'s wall test (h0+$e3fe..$e412, DESIGN §2.11).
fn hung(a: &Amiga) -> bool {
    let at = |pc: u32| (h0(0xe3fe)..=h0(0xe412)).contains(&pc);
    let cpu = a.cpu();
    let m = sync::core_memory(a.memory());
    sync::main_chain(&cpu, &m).first().is_some_and(|&pc| at(pc))
}

fn oracle_w(a: &Amiga, addr: u32) -> u16 {
    let b = a.peek(addr, 2);
    u16::from_be_bytes([b[0], b[1]])
}

/// The cell the frog's feet are in (`+$02` is its feet's y, `+$00 + 8` its middle).
fn feet(fr: &Frog) -> (i32, i32) {
    ((fr.x + 8).div_euclid(16), (fr.y - 8).div_euclid(16))
}

fn marker_addr(col: u32, row: u32) -> u32 {
    h1(ROW0) + row * ROW_BYTES + 2 * col
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
        if c > 0 {
            go(c - 1, r);
        }
        if c + 1 < w {
            go(c + 1, r);
        }
        if r > 0 {
            go(c, r - 1);
        }
        if r + 1 < h {
            go(c, r + 1);
        }
    }
    d
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(&sf_conform::cases_dir().join(format!("{}.toml", args[0]))).unwrap();
    let mut targets = Vec::new();
    let mut hold = 4u32;
    let mut chunk = 24usize;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--hold" => {
                hold = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--chunk" => {
                chunk = args[i + 1].parse().unwrap();
                i += 2;
            }
            t => {
                let p: Vec<&str> = t.split(',').collect();
                targets.push(match p[0] {
                    "wait" => Target::Wait(p[1].parse().unwrap()),
                    "take" => Target::Take(p[1].parse().unwrap(), p[2].parse().unwrap()),
                    c => Target::Feet(c.parse().unwrap(), p[1].parse().unwrap()),
                });
                i += 1;
            }
        }
    }
    let frozen = case.freeze();
    let synced =
        sync::reach_frozen(&case.sync, &case.script, case.frames + 20_000, frozen).unwrap();
    let display = sf_conform::display_data();
    let mut amiga = synced.amiga;
    let mut inputs: Vec<FrameInput> = Vec::new();
    let play = |amiga: &mut Amiga, inputs: &mut Vec<FrameInput>, x: FrameInput| {
        frozen.apply(amiga, &display);
        sync::step_inserting(amiga, x).unwrap();
        inputs.push(x);
    };
    for fr in 0..case.frames {
        play(&mut amiga, &mut inputs, case.script.input_at(fr));
    }
    let lives = frog_of(|a| oracle_w(&amiga, a)).lives;
    let id = match &case.sync {
        sf_conform::SyncPoint::AtLevel(id) => *id,
        s => panic!("route_oracle: a level case only ({s:?})"),
    };
    let t7mp = &sf_conform::game_data().levels[&id].t7mp;
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
    let steps = std::env::var("ROUTE_STEPS").map_or(250, |v| v.parse().unwrap());
    let width_beam = std::env::var("ROUTE_BEAM").map_or(24, |v| v.parse().unwrap());
    'targets: for &t in &targets {
        let (tc, tr, take) = match t {
            Target::Wait(n) => {
                for _ in 0..n {
                    play(&mut amiga, &mut inputs, FrameInput::joy(Joy::NONE));
                }
                continue;
            }
            Target::Feet(c, r) => (c, r, None),
            Target::Take(c, r) => (c, r, Some(oracle_w(&amiga, marker_addr(c, r)))),
        };
        let reached = |w: &dyn Fn(u32) -> u16| match take {
            Some(m) => w(marker_addr(tc, tr)) != m,
            None => {
                let (c, r) = feet(&frog_of(w));
                (c - tc as i32).abs() <= 1 && (r - tr as i32).abs() <= 2
            }
        };
        let dist = distances(&open, width, height, tc as usize, tr as usize);
        let score = |g: &Game| {
            let fr = frog_of(|a| g.state().w(a));
            let (c, r) = feet(&fr);
            let d = if (0..width as i32).contains(&c) && (0..height as i32).contains(&r) {
                dist[r as usize * width + c as usize]
            } else {
                u32::MAX / 64
            };
            (d.min(u32::MAX / 64) as i32) * 16 + (fr.x + 8 - (tc * 16 + 8) as i32).abs()
        };
        let mut replans = 0;
        loop {
            // A pass the original has not finished cannot be transplanted: let it finish.
            let x = *inputs.last().unwrap();
            while sync::mid_pass(&amiga.cpu(), &sync::core_memory(amiga.memory())) {
                play(&mut amiga, &mut inputs, x);
            }
            if reached(&|a| oracle_w(&amiga, a)) {
                break;
            }
            replans += 1;
            if replans > 60 {
                eprintln!("target {tc},{tr}: no progress on the original");
                break 'targets;
            }
            let mem = sync::core_memory(amiga.memory());
            let g = Game::with_display(display.clone(), Start::Transplant(mem));
            let mut beam: Vec<(Game, Vec<FrameInput>)> = vec![(g, Vec::new())];
            let mut done = None;
            'search: for _ in 0..steps {
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
                            let fr = frog_of(|a| n.state().w(a));
                            if fr.lives != lives || fr.dying != 0 {
                                dead = true;
                                break;
                            }
                            if reached(&|a| n.state().w(a)) {
                                done = Some(p);
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
                    .take(width_beam)
                    .map(|(_, n, p)| (n, p))
                    .collect();
                if beam.is_empty() {
                    break;
                }
            }
            let Some(p) = done else {
                eprintln!(
                    "target {tc},{tr}: not reached on the engine from frame {}",
                    inputs.len()
                );
                break 'targets;
            };
            let fr = frog_of(|a| oracle_w(&amiga, a));
            eprintln!(
                "target {tc},{tr}: frame {} frog {:?}, plan of {} frames",
                inputs.len(),
                feet(&fr),
                p.len()
            );
            for &x in p.iter().take(chunk) {
                play(&mut amiga, &mut inputs, x);
                let fr = frog_of(|a| oracle_w(&amiga, a));
                if fr.lives != lives || fr.dying != 0 {
                    eprintln!(
                        "target {tc},{tr}: the frog died on the original at {}",
                        inputs.len()
                    );
                    break 'targets;
                }
                if hung(&amiga) {
                    eprintln!(
                        "target {tc},{tr}: the original hangs in frog_tiles at {}",
                        inputs.len()
                    );
                    break 'targets;
                }
                if reached(&|a| oracle_w(&amiga, a)) {
                    break;
                }
            }
        }
        eprintln!(
            "target {tc},{tr}: reached on the original at frame {}",
            inputs.len()
        );
    }
    let mut last = None;
    for (fr, x) in inputs.iter().enumerate() {
        if Some(*x) != last {
            println!("{fr}: {}", InputScript::format_input(*x));
            last = Some(*x);
        }
    }
}
