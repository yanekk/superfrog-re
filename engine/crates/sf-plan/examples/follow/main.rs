//! Route-follower experiment (experiments/route-follower/BRIEF.md): fixed scripts against
//! position-triggered scripts (A) and reactive followers (B1 rules, B2 look-ahead bursts, B3
//! look-ahead over the route's own inputs re-timed), under the brief's timing perturbations.
//! Prototype quality: an example, not a library; nothing here changes the game or the check.
//!
//! ```text
//! follow ref W1-1 [--route complete-w1-1]           # the route on the original, recorded (oracle)
//! follow run W1-1 --exec fixed|A|Apure|B1|B2|B3|B23 --cond P0|P1|P1t|P2|P3:k|P3o:k|P4o|P4e:N:L|
//!            P5:m|P5o:m|P6|P6e [--route R] [--seed 1] [--threads 2] [--k 8] [--h 64]
//!            [--bursts 12] [--eager] [--verify] [--out rec.txt] [--csv results/runs.csv] [--max N]
//! ```
//!
//! `cargo run --release -p sf-plan --example follow -- …`

mod exec;
mod machine;
mod play;
mod reference;

use std::io::Write as _;
use std::path::PathBuf;

use sf_bench::sets::{Set, Start as BStart, StartKind, default_legs_dir, leg_path};
use sf_bench::verify::{committed_script, verify};
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::input::FrameInput;
use sf_core::{Game, LevelId, Start};
use sf_plan::core::facts;
use sf_plan::trace::script_inputs;

use exec::{Fixed, Kind, Mpc, Rules, Triggers, cut_steps};
use machine::{Engine, Lag, Machine, OPT_GAME_MODE, Oracle};
use play::{Exec, Outcome, followed_share, play, script_text};

struct Args {
    rest: Vec<String>,
}

impl Args {
    fn opt(&mut self, name: &str) -> Option<String> {
        let k = self.rest.iter().position(|a| a == name)?;
        let v = self.rest[k + 1].clone();
        self.rest.drain(k..k + 2);
        Some(v)
    }
    fn flag(&mut self, name: &str) -> bool {
        match self.rest.iter().position(|a| a == name) {
            Some(k) => {
                self.rest.remove(k);
                true
            }
            None => false,
        }
    }
}

fn main() {
    let mut a = Args { rest: std::env::args().skip(1).collect() };
    let cmd = a.rest.remove(0);
    let id: LevelId = a.rest.remove(0).parse().expect("a level id");
    let route = a.opt("--route").unwrap_or_else(|| committed_script(Set::A, &id));
    match cmd.as_str() {
        "ref" => make_ref(id, &route),
        "run" => run(id, &route, &mut a),
        "info" => info(&route),
        c => panic!("{c}? (ref, run, info)"),
    }
}

fn load_route(route: &str) -> Vec<FrameInput> {
    let s = InputScript::load(&sf_conform::scripts_dir().join(format!("{route}.txt"))).expect("route");
    script_inputs(&s)
}

/// The original at the level's sync point with room for `cap` frames.
fn oracle_at(id: LevelId, cap: u32) -> Oracle {
    let s = sync::reach(&SyncPoint::AtLevel(id), &InputScript::idle(), cap).expect("oracle at level");
    Oracle::new(s.amiga, sf_conform::display_data())
}

fn oracle_at_leg(id: LevelId, cap: u32) -> (Oracle, PathBuf, u32) {
    let p = leg_path(&default_legs_dir(), &id);
    let frame = sync::load_snapshot_file(&p).expect("leg snapshot").frame;
    let a = sync::at_snapshot_file(&p, cap).expect("oracle at leg");
    (Oracle::new(a, sf_conform::display_data()), p, frame)
}

fn make_ref(id: LevelId, route: &str) {
    let inputs = load_route(route);
    let mut m = oracle_at(id, inputs.len() as u32 + 100);
    let g = m.game();
    let fa = facts(&g, &sf_conform::game_data().levels[&id], sf_conform::display_data().rom());
    let t0 = std::time::Instant::now();
    let r = reference::record(&mut m, &fa, &inputs).expect("reference run");
    reference::save(route, &r).expect("save");
    let lag = r.iter().filter(|f| f.mid).count();
    let steps = cut_steps(&r).len();
    println!(
        "{id} {route}: {} frames recorded, {lag} mid-pass, {steps} input steps, {:.0}s",
        r.len(),
        t0.elapsed().as_secs_f64()
    );
}

/// Sizes of each route form for one route (the report's artefact table).
fn info(route: &str) {
    let r = reference::load(route).expect("reference");
    let script = std::fs::read_to_string(sf_conform::scripts_dir().join(format!("{route}.txt"))).unwrap();
    let lines = script.lines().filter(|l| !l.starts_with('#')).count();
    let steps = cut_steps(&r);
    let waits = steps.iter().filter(|s| s.wait).count();
    let a_bytes: usize = steps
        .iter()
        .map(|s| format!("{} {} {} {} {} {}\n", s.at.x, s.at.y, s.at.coins, s.at.switches, s.len, InputScript::format_input(s.input)).len())
        .sum();
    let g = reference::guide(&r);
    let b_bytes: usize = g
        .iter()
        .map(|p| format!("{},{},{}\n", p.cell.0, p.cell.1, p.coins_left.unwrap()).len())
        .sum();
    // Consecutive equal cells collapse: what a person would edit.
    let mut cells = g.iter().map(|p| (p.cell, p.coins_left)).collect::<Vec<_>>();
    cells.dedup();
    println!(
        "{route}: fixed {} bytes {lines} lines | A {a_bytes} bytes {} steps ({waits} waits) | B {b_bytes} bytes {} points, {} distinct",
        script.len(),
        steps.len(),
        g.len(),
        cells.len()
    );
}

fn run(id: LevelId, route: &str, a: &mut Args) {
    let exec_name = a.opt("--exec").expect("--exec");
    let cond = a.opt("--cond").expect("--cond");
    let seed: u64 = a.opt("--seed").map_or(1, |v| v.parse().unwrap());
    let threads: usize = a.opt("--threads").map_or(2, |v| v.parse().unwrap());
    let k: u32 = a.opt("--k").map_or(8, |v| v.parse().unwrap());
    let h: u32 = a.opt("--h").map_or(64, |v| v.parse().unwrap());
    let bursts: u32 = a.opt("--bursts").map_or(12, |v| v.parse().unwrap());
    let eager = a.flag("--eager");
    let want_verify = a.flag("--verify");
    let out = a.opt("--out");
    let csv = a.opt("--csv");
    let label = a.opt("--label").unwrap_or_default();
    let display = sf_conform::display_data();
    let data = sf_conform::game_data();

    let inputs = load_route(route);
    let r = reference::load(route).ok();
    let max: u32 = a.opt("--max").map_or(inputs.len() as u32 * 2 + 1500, |v| v.parse().unwrap());

    // The machine and start.
    let (kind, arg) = cond.split_once(':').map_or((cond.as_str(), ""), |(k, v)| (k, v));
    let mut delay = 0u32;
    let mut bstart: Option<BStart> = None;
    let mut m: Box<dyn Machine> = match kind {
        "P0" | "P3o" | "P5o" => {
            let mut o = oracle_at(id, max + 50);
            if kind == "P3o" {
                delay = arg.parse().unwrap();
            }
            if kind == "P5o" {
                o.a.poke(OPT_GAME_MODE, &arg.parse::<u16>().unwrap().to_be_bytes());
                o.refresh();
            } else {
                bstart = Some(BStart { level: id, kind: StartKind::AtLevel });
            }
            Box::new(o)
        }
        "P6" => {
            let (o, p, frame) = oracle_at_leg(id, max + 50);
            bstart = Some(BStart { level: id, kind: StartKind::Snap(p, frame) });
            Box::new(o)
        }
        "P6e" => {
            let (o, _, _) = oracle_at_leg(id, 10);
            Box::new(Engine::new(o.game(), Lag::None))
        }
        "P1t" => Box::new(Engine::new(oracle_at(id, 10).game(), Lag::None)),
        "P2" => Box::new(Engine::new(sf_plan::app::app_start(id, seed as u32), Lag::None)),
        _ => {
            let g = Game::with_display(display.clone(), Start::Level(id));
            let lag = match kind {
                "P1" => Lag::None,
                "P3" => {
                    delay = arg.parse().unwrap();
                    Lag::None
                }
                "P4o" => Lag::At(reference::lag_frames(r.as_ref().expect("P4o needs the reference"))),
                "P4e" => {
                    let (p, l) = arg.split_once(':').unwrap();
                    Lag::Every { period: p.parse().unwrap(), len: l.parse().unwrap(), phase: 200 }
                }
                "P5" => Lag::None,
                _ => panic!("condition {cond}?"),
            };
            let mut e = Engine::new(g, lag);
            if kind == "P5" {
                e.g.poke(OPT_GAME_MODE, &arg.parse::<u16>().unwrap().to_be_bytes());
            }
            Box::new(e)
        }
    };
    let fa = facts(&m.game(), &data.levels[&id], display.rom());

    let need_ref = || r.clone().expect("this executor needs `follow ref` first");
    let mut ex: Box<dyn Exec> = match exec_name.as_str() {
        "fixed" => Box::new(Fixed { inputs: inputs.clone() }),
        "A" => Box::new(Triggers::new(cut_steps(&need_ref()), false)),
        "Apure" => Box::new(Triggers::new(cut_steps(&need_ref()), true)),
        "B1" => Box::new(Rules::new(reference::guide_px(&need_ref()))),
        "B2" | "B3" | "B23" => {
            let kind = match exec_name.as_str() {
                "B2" => Kind::Bursts,
                "B3" => Kind::Shifts,
                _ => Kind::Both,
            };
            let mut mpc = Mpc::new(kind, fa.clone(), &need_ref(), seed);
            mpc.k = k;
            mpc.h = h;
            mpc.bursts = bursts;
            mpc.threads = threads;
            mpc.lazy = !eager;
            Box::new(mpc)
        }
        e => panic!("executor {e}?"),
    };

    let o: Outcome = play(m.as_mut(), ex.as_mut(), &fa, max, delay);
    let followed = r.as_ref().map(|r| followed_share(&reference::guide(r), &o.trace));

    let mut verdict = String::from("-");
    if want_verify && let Some(s) = &bstart {
        let set = if matches!(s.kind, StartKind::Snap(..)) { Set::B } else { Set::A };
        let script = InputScript::parse(&script_text("", &o.rec)).expect("own script");
        let v = verify(set, s, &script, seed as u32);
        verdict = if v.completed {
            format!("pass({} frames, {} hearts)", v.route_frames.unwrap_or(0), v.hearts_lost)
        } else {
            format!("FAIL({})", v.reason.replace(',', ";").chars().take(120).collect::<String>())
        };
    }
    if let Some(p) = &out {
        let header = format!(
            "timing: original\n{id} {route} by follow --exec {exec_name} --cond {cond} --seed {seed}\nframe 0 is the start's first frame"
        );
        std::fs::write(p, script_text(&header, &o.rec)).unwrap();
    }
    let peak_mb = sf_plan::run::peak_rss_bytes() >> 20;
    let row = format!(
        "{id},{route},{exec_name},{cond},{seed},{label},{},{},{},{},{},{},{},{},{}:{},{},{:.3},{:.1},{},{:.1},{},{},{}",
        o.exit.is_some(),
        o.exit.map_or(String::from("-"), |e| e.to_string()),
        o.exit.is_some() && o.lives_lost == 0,
        o.lives_lost,
        o.hearts_lost,
        o.first_life_lost.map_or(String::from("-"), |f| f.to_string()),
        o.frames,
        o.last.coins,
        o.last.cell.0,
        o.last.cell.1,
        o.game_over,
        followed.unwrap_or(0.0),
        o.secs,
        o.engine_frames,
        o.engine_frames as f64 / o.frames.max(1) as f64,
        o.lag_frames,
        verdict,
        peak_mb,
    );
    println!("{row}{}", o.err.map_or(String::new(), |e| format!("  ERR {e}")));
    if let Some(p) = csv {
        let new = !std::path::Path::new(&p).exists();
        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&p).unwrap();
        if new {
            writeln!(
                f,
                "level,route,exec,cond,seed,label,exit,exit_frame,clean,lives_lost,hearts_lost,first_life_lost,frames,coins_left,end_cell,game_over,followed,secs,engine_frames,eng_per_frame,lag_frames,verify,peak_mb"
            )
            .unwrap();
        }
        writeln!(f, "{row}").unwrap();
    }
}
