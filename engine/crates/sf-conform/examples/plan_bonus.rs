//! Plays the bonus level (WB-1, shooter mode) to its end without losing a life (T207), for the
//! whole game: `chain-bonus-to-w6-1`'s fire held loses 16 lives, and the chain arrives with 10.
//! Prints the inputs as an input script.
//!
//! `cargo run --release --example plan_bonus -- --snap <leg-NN.snap> [--prefix script.txt]
//! [--out script.txt] [--frames N]`
//!
//! The camera creeps right on its own and marker 55 ends the level, so the only aim is to live:
//! fire is held throughout (as `bonus-fire`) and a beam search over the nine directions, each held
//! 4 frames, looks `HORIZON` frames ahead on the engine from the original's memory, scoring each
//! end by lives, then hearts, then shot power and score, less for the frog near the screen's edges.
//! The best path's first `COMMIT` frames are played on the original once the whole path has kept
//! its frog alive there, else the next best; with none, the chunk before takes its next. The
//! original lags in a busy level and the engine free-running does not, so only the original's
//! outcome counts.
use sf_conform::{InputScript, sync};
use sf_core::frog::{FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::h0;
use sf_core::{Game, Start};

const LEVEL_STATE: u32 = h0(0xa06);
const LIVES: u32 = h0(0x82e);
const HEARTS: u32 = h0(0x830);
const SCORE: u32 = h0(0x834);
const CAMERA_X: u32 = h0(0x16664);
const CAMERA_Y: u32 = h0(0x16660);
/// `shot_power` (`bonus/shots.rs`).
const SHOT_POWER: u32 = sf_core::bonus::shots::SHOT_POWER;
const HORIZON: u32 = 30;
const HOLD: u32 = 4;
const COMMIT: usize = 48;
const BEAM: usize = 32;

/// How good a state is: more lives, hearts, shot power and score; the frog away from the edges of
/// the screen (where it has no room to dodge).
fn score(st: &sf_core::State) -> i64 {
    let x = st.w(FROG + f::X) as i16 as i64 - st.l(CAMERA_X) as i32 as i64;
    let y = st.w(FROG + f::Y) as i16 as i64 - st.l(CAMERA_Y) as i32 as i64;
    let edge = (x - 100).abs() + (y - 100).abs();
    -(st.w(LIVES) as i16 as i64) * 1_000_000 - st.w(HEARTS) as i64 * 10_000
        - st.w(SHOT_POWER) as i64 * 100
        - (st.l(SCORE) as i64).min(1 << 40) / 1000
        + edge
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut snap, mut prefix, mut out, mut budget) = (None, Vec::new(), None, 9000usize);
    let mut i = 0;
    while i < args.len() {
        let v = &args[i + 1];
        match args[i].as_str() {
            "--snap" => snap = Some(v.clone()),
            "--out" => out = Some(v.clone()),
            "--frames" => budget = v.parse().unwrap(),
            "--prefix" => {
                let p = InputScript::load(std::path::Path::new(v)).unwrap();
                let n = p.lines.last().map_or(0, |l| l.0);
                prefix = (0..n).map(|k| p.input_at(k)).collect();
            }
            a => panic!("{a}?"),
        }
        i += 2;
    }
    let snap = snap.expect("--snap <leg-NN.snap>");
    let amiga = &mut sync::at_snapshot_file(std::path::Path::new(&snap), budget as u32 + 100)
        .unwrap_or_else(|e| panic!("{e}"));
    let display = sf_conform::display_data();
    let peek_w = |a: &sf_oracle::Amiga, addr: u32| {
        let b = a.peek(addr, 2);
        u16::from_be_bytes([b[0], b[1]])
    };
    let peek_l = |a: &sf_oracle::Amiga, addr: u32| {
        let b = a.peek(addr, 4);
        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
    };
    let acts: Vec<Joy> = [
        Joy::NONE,
        Joy::UP,
        Joy::DOWN,
        Joy::LEFT,
        Joy::RIGHT,
        Joy::UP | Joy::LEFT,
        Joy::UP | Joy::RIGHT,
        Joy::DOWN | Joy::LEFT,
        Joy::DOWN | Joy::RIGHT,
    ]
    .into_iter()
    .map(|j| j | Joy::FIRE)
    .collect();
    let mut inputs: Vec<FrameInput> = Vec::new();
    for &x in &prefix {
        sync::step_inserting(amiga, x).unwrap();
        inputs.push(x);
    }
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    // Each chunk kept: the original before it, the inputs then, and the candidate it took.
    let mut history: Vec<(sf_oracle::Snapshot, usize, usize)> = Vec::new();
    let (mut skip, mut backtracks) = (0usize, 0);
    while inputs.len() < budget {
        let mem = sync::core_memory(amiga.memory());
        let start = Game::with_display(display.clone(), Start::Transplant(mem));
        let lives = start.state().w(LIVES) as i16;
        let mut beam: Vec<(Game, Vec<FrameInput>)> = vec![(start, Vec::new())];
        let mut alive = Vec::new();
        for _ in 0..HORIZON {
            let jobs: Vec<(&Game, &Vec<FrameInput>, Joy)> =
                beam.iter().flat_map(|(b, p)| acts.iter().map(move |&a| (b, p, a))).collect();
            let chunk = jobs.len().div_ceil(threads).max(1);
            let mut next: Vec<(i64, Game, Vec<FrameInput>)> = std::thread::scope(|sc| {
                let hs: Vec<_> = jobs
                    .chunks(chunk)
                    .map(|js| {
                        sc.spawn(move || {
                            js.iter()
                                .filter_map(|&(b, path, a)| {
                                    let (mut n, mut p) = (b.clone(), path.clone());
                                    for _ in 0..HOLD {
                                        let x = FrameInput::joy(a);
                                        p.push(x);
                                        let stepped = std::panic::catch_unwind(
                                            std::panic::AssertUnwindSafe(|| n.step(x)),
                                        );
                                        if stepped.is_err()
                                            || (n.state().w(LIVES) as i16) < lives
                                            || n.state().w(FROG + f::DYING) != 0
                                        {
                                            return None;
                                        }
                                    }
                                    Some((score(n.state()), n, p))
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
                    seen.insert((w(f::X) / 4, w(f::Y) / 4))
                })
                .take(BEAM)
                .map(|(_, n, p)| (n, p))
                .collect();
            if beam.is_empty() {
                beam = alive;
                break;
            }
            alive = beam.clone();
        }
        let snap = amiga.snapshot();
        let mut taken = None;
        for (_, p) in beam.iter().skip(skip).take(8) {
            let mut ok = true;
            for (k, &x) in p.iter().enumerate() {
                sync::step_inserting(amiga, x).unwrap();
                if (peek_w(amiga, LIVES) as i16) < lives || peek_w(amiga, FROG + f::DYING) != 0 {
                    eprintln!("  a path dies {k} frames in on the original");
                    ok = false;
                    break;
                }
                if peek_l(amiga, LEVEL_STATE) != 0xffff_fffe {
                    break;
                }
            }
            amiga.restore(&snap);
            if ok {
                let mut k = 0;
                while k < p.len().min(COMMIT) {
                    sync::step_inserting(amiga, p[k]).unwrap();
                    k += 1;
                    if inputs.len() + k > 10 && peek_l(amiga, LEVEL_STATE) != 0xffff_fffe {
                        break;
                    }
                }
                taken = Some(p[..k].to_vec());
                break;
            }
        }
        let Some(p) = taken else {
            eprintln!("frame {}: every path loses a life on the original", inputs.len());
            backtracks += 1;
            let mut popped = history.pop();
            while popped.as_ref().is_some_and(|h| h.2 + 1 >= 3) {
                popped = history.pop();
            }
            let Some((s, len, sk)) = popped.filter(|_| backtracks <= 400) else {
                break;
            };
            amiga.restore(&s);
            inputs.truncate(len);
            skip = sk + 1;
            eprintln!("  back to frame {len}, candidate {skip}");
            continue;
        };
        history.push((snap, inputs.len(), skip));
        skip = 0;
        inputs.extend(p);
        if let Some(o) = &out {
            std::fs::write(o, script(&inputs)).unwrap();
        }
        eprintln!(
            "frame {}: lives {} hearts {} power {} score {} camera {}",
            inputs.len(),
            peek_w(amiga, LIVES) as i16,
            peek_w(amiga, HEARTS),
            peek_w(amiga, SHOT_POWER),
            peek_l(amiga, SCORE),
            peek_l(amiga, CAMERA_X)
        );
        if inputs.len() > 10 && peek_l(amiga, LEVEL_STATE) != 0xffff_fffe {
            eprintln!("level ends at frame {}", inputs.len());
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
