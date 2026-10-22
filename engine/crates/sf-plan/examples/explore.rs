//! Plans each level named with the explorer, engine only, unguided, and prints time, rounds, cells,
//! engine frames and peak resident memory (T08's FINDINGS rows). The search starts once the frog
//! has landed past the level-start freeze (`tests/explore.rs`'s `at_start`); `--out` writes the
//! route from there in script format.
//!
//! `cargo run --release -p sf-plan --example explore -- [--threads N] [--minutes M] [--seed S]
//!  [--mem-gb G] [--shorten] [--out route.txt] W3-2 ...`
//!
//! `--shorten` turns route shortening on with `sf-plan`'s defaults (DESIGN §2.12, T22).

use std::time::{Duration, Instant};

use sf_conform::InputScript;
use sf_core::input::FrameInput;
use sf_core::{Game, LevelId, Start};
use sf_plan::core::{Options, facts, landed};
use sf_plan::run::{RunOptions, peak_rss_bytes, plan_engine};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut opt = |name: &str| -> Option<String> {
        let k = args.iter().position(|a| a == name)?;
        let v = args[k + 1].clone();
        args.drain(k..k + 2);
        Some(v)
    };
    let threads: usize = opt("--threads").map_or(10, |v| v.parse().unwrap());
    let minutes: f64 = opt("--minutes").map_or(10.0, |v| v.parse().unwrap());
    let seed: u64 = opt("--seed").map_or(1, |v| v.parse().unwrap());
    let mem_gb: f64 = opt("--mem-gb").map_or(4.0, |v| v.parse().unwrap());
    let out = opt("--out");
    let shorten = match args.iter().position(|a| a == "--shorten") {
        Some(k) => {
            args.remove(k);
            true
        }
        None => false,
    };
    let data = sf_conform::game_data();
    let display = sf_conform::display_data();
    for a in &args {
        let id: LevelId = a.parse().unwrap();
        let mut g = Game::with_display(display.clone(), Start::Level(id));
        while !(landed(g.state()) && g.frame() > 150) && g.frame() < 400 {
            g.step_unseen(FrameInput::default());
        }
        let fa = facts(&g, &data.levels[&id], display.rom());
        let o = Options {
            seed,
            max_mem_bytes: (mem_gb * (1u64 << 30) as f64) as u64,
            ..Options::default()
        };
        let run = RunOptions {
            threads,
            budget: Duration::from_secs_f64(minutes * 60.0),
            ..if shorten {
                RunOptions::shortening()
            } else {
                RunOptions::default()
            }
        };
        if std::env::var_os("SF_STATE_SIZE").is_some() {
            eprintln!("state size {}", sf_plan::core::archive::state_size(&g));
        }
        let t0 = Instant::now();
        let r = plan_engine(g, fa, o, &run);
        let secs = t0.elapsed().as_secs_f64();
        let mb = peak_rss_bytes() >> 20;
        match r {
            Ok(r) => {
                println!(
                    "{id}: EXIT {secs:.0}s route {} frames, rounds {}, cells {}, engine frames {}, hearts lost {}, peak {mb} MB",
                    r.inputs.len(),
                    r.rounds,
                    r.cells,
                    r.engine_frames,
                    r.hearts_lost
                );
                if let Some(p) = &out {
                    let mut s = format!(
                        "# timing: engine. {id} planned by `cargo run --release -p sf-plan --example explore -- \
                         --threads {threads} --seed {seed} --out {p} {id}`;\n# frame 0 is the first frame after \
                         Start::Level({id}) idled until the frog lands past frame 150.\n"
                    );
                    let mut last = None;
                    for (k, x) in r.inputs.iter().enumerate() {
                        if Some(*x) != last {
                            s += &format!("{k}: {}\n", InputScript::format_input(*x));
                            last = Some(*x);
                        }
                    }
                    s += &format!("{}: -\n", r.inputs.len());
                    std::fs::write(p, s).unwrap();
                }
            }
            Err((r, why)) => println!(
                "{id}: TIMEOUT {secs:.0}s partial {} frames, coins left {}, exit distance with coins met {:?}, cells {}, \
                 rounds {}, engine frames {}, peak {mb} MB",
                r.inputs.len(),
                why.coins_left,
                why.exit_distance_with_coins_met,
                why.cells,
                r.rounds,
                r.engine_frames
            ),
        }
    }
}
