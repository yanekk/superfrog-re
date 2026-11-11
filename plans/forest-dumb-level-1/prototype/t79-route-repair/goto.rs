// SCRATCH: never committed (kept in plans/forest-dumb-level-1/prototype/t79-route-repair/).
// Builds an input script with creatures live, the original in the loop: each op is planned on the
// engine alone from the state the case's engine holds on the original (the two agreeing, it is the
// original's), played on the original, and planned again from the pass before the first one where
// the original's run left the planned one.
//
// CASE=death-water OUT=/tmp/x.txt PROG="G166,71 G152,61 H04x30 H00x200" [PREFIX=file] [SEED=n]
//   Gc,r    the planner's explorer to cell c,r (Goal::Reach)
//   Hjjxn   joystick byte jj (hex) held n steps
use std::path::Path;
use std::time::Duration;

use amiga_chips::Picture;
use amiga_conform::script;
use amiga_input::FrameInput;
use amiga_plan::run::{RunOptions, plan_engine};
use amiga_plan::{Compass, Goal as PlanGoal, GuidePoint, Options as PlanOptions, PlanGame};
use fd_conform::{Case, Engine, Goal, Inputs, Options, game_data, run_inputs};
use fd_core::level::EdgesCut;
use fd_core::{Event, Game, Mem, Start, Transplant, Var};
use fd_plan::facts::facts;
use fd_plan::game::PlanState;

type Key = (u32, u32, u16);
fn key(g: &Game) -> Key {
    let s = g.state();
    (s.char_x, s.char_y, s.lives)
}

struct Rec {
    game: Game,
    games: Vec<Game>,
}
impl Transplant for Rec {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.game.transplant(mem);
    }
    fn probe(&self, v: &Var) -> u32 {
        self.game.probe(v)
    }
}
impl Engine for Rec {
    fn pass(&mut self, input: FrameInput) {
        Engine::pass(&mut self.game, input);
        self.games.push(self.game.clone());
    }
    fn picture(&self) -> Option<Picture> {
        None
    }
    fn events(&mut self) -> Vec<Event> {
        Engine::events(&mut self.game)
    }
    fn off_play(&mut self, input: FrameInput) {
        Engine::off_play(&mut self.game, input)
    }
    fn game_over(&self) -> bool {
        Engine::game_over(&self.game)
    }
    fn level_end(&self) -> bool {
        Engine::level_end(&self.game)
    }
    fn foreground_cut(&mut self, at: Option<EdgesCut>) {
        Engine::foreground_cut(&mut self.game, at)
    }
}

#[derive(Clone, Debug)]
enum Op {
    Go(u16, u16),
    Hold(u8, usize),
}

fn say(g: &Game) -> String {
    let s = g.state();
    let p = PlanState(g.clone());
    let c = &s.edges.creatures;
    let cr: Vec<String> = (0..5)
        .filter(|&i| c.active[i] != 0)
        .map(|i| format!("k{}@{:x},{:x}", c.slots[i].kind, c.slots[i].x, c.slots[i].y))
        .collect();
    format!("at {:x},{:x} cell {:?} lives {:x} hits {} irq {} {:?}", s.char_x, s.char_y, p.cell(), s.lives, s.hits_taken, s.irq_count, cr)
}

#[test]
fn goto() {
    let name = std::env::var("CASE").unwrap();
    let out_path = std::env::var("OUT").unwrap();
    let seed: u64 = std::env::var("SEED").map(|s| s.parse().unwrap()).unwrap_or(1);
    let max_rounds: usize = std::env::var("ROUNDS").map(|s| s.parse().unwrap()).unwrap_or(60);
    let ops: Vec<Op> = std::env::var("PROG")
        .unwrap()
        .split_whitespace()
        .map(|t| {
            if let Some(r) = t.strip_prefix('G') {
                let (c, r) = r.split_once(',').unwrap();
                Op::Go(c.parse().unwrap(), r.parse().unwrap())
            } else {
                let (j, n) = t[1..].split_once('x').unwrap();
                Op::Hold(u8::from_str_radix(j, 16).unwrap(), n.parse().unwrap())
            }
        })
        .collect();
    let data = game_data().unwrap();
    let mut case = Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap();
    case.compare.picture = false;
    case.input = None;
    // Any exclusion would have to hide a difference; none is meant here.
    case.exclude.clear();
    let mut cand: Vec<FrameInput> = Vec::new();
    // Per step: the key the engine alone predicts after it, and the op it belongs to.
    let mut pred: Vec<Option<Key>> = Vec::new();
    let mut owner: Vec<usize> = Vec::new();
    if let Ok(p) = std::env::var("PREFIX") {
        let text = std::fs::read_to_string(&p).unwrap();
        let sc = script::parse(&text).unwrap();
        let frames: usize = text.lines().find_map(|l| l.strip_prefix("# frames ")).unwrap().parse().unwrap();
        cand = (0..frames as u32).map(|f| sc.input_at(f)).collect();
        pred = vec![None; frames];
        owner = vec![usize::MAX; frames];
    }
    let write = |cand: &[FrameInput], tag: &str| {
        let body = script::InputScript::from_steps(cand).to_text();
        std::fs::write(format!("{out_path}{tag}"), format!("# frames {}\n{body}", cand.len())).unwrap();
    };
    let mut next = 0usize; // the op to plan
    let mut g = Game::new(data.clone(), Start::Level1);
    let mut last_d = usize::MAX;
    let mut stuck = 0;
    for round in 0..max_rounds {
        // The original on what there is.
        if !cand.is_empty() {
            case.frames = cand.len() as u32 + 8;
            if let Some(Goal::All { by_frame, .. }) = &mut case.goal {
                *by_frame = case.sync_frame + case.frames;
            }
            let inputs = Inputs::with_after(&case, fd_plan::script(&cand)).unwrap();
            let mut rec = Rec { game: Game::new(data.clone(), Start::Level1), games: Vec::new() };
            let v = run_inputs(&case, &inputs, &mut rec, &Options::default()).unwrap();
            let head: String = format!("{v}").lines().take(1).collect();
            if head.contains("differs") {
                write(&cand, &format!(".differs{round}"));
                println!("round {round}: THE TWO DIFFER: {head}");
            }
            let n = rec.games.len().min(cand.len());
            // A leg that still arrives where it was sent, no life lost, holds: a pass reading its
            // input a step late shifts the path a little and harms nothing.
            let mut d;
            loop {
                d = (0..n).find(|&k| pred[k].is_some_and(|p| p != key(&rec.games[k])));
                let Some(at) = d else { break };
                let op = owner[at];
                let e = (0..cand.len()).rev().find(|&k| owner[k] == op).unwrap();
                let arrived = match ops[op] {
                    Op::Go(c, r) if e < rec.games.len() => {
                        let here = PlanState(rec.games[e].clone()).cell();
                        (here.0 / 2).abs_diff(c / 2) <= 1
                            && (here.1 / 2).abs_diff(r / 2) <= 1
                            && pred[e].is_some_and(|p| p.2 == rec.games[e].state().lives)
                    }
                    _ => false,
                };
                if !arrived {
                    break;
                }
                for k in 0..cand.len() {
                    if owner[k] == op {
                        pred[k] = None;
                    }
                }
            }
            match d {
                Some(d) => {
                    println!(
                        "round {round}: {} steps; the original leaves the plan at pass {d} (op {}): planned {:x?}, {}",
                        cand.len(), owner[d], pred[d].unwrap(), say(&rec.games[d])
                    );
                    stuck = if d <= last_d.saturating_add(2) && last_d != usize::MAX { stuck + 1 } else { 0 };
                    last_d = d;
                    // Plan again from the pass before, held on a little where that did not help.
                    let keep = d;
                    next = owner[d];
                    while keep > 0 && owner[keep - 1] != usize::MAX && owner[keep - 1] > next {
                        next = owner[keep - 1];
                    }
                    cand.truncate(keep);
                    pred.truncate(keep);
                    owner.truncate(keep);
                    g = if keep == 0 { Game::new(data.clone(), Start::Level1) } else { rec.games[keep - 1].clone() };
                    for _ in 0..stuck.min(1) * 2 {
                        // The same input on: a pass reading a step late changes nothing here.
                        let i = cand.last().copied().unwrap_or(FrameInput::NONE);
                        g.step(i);
                        let _ = g.take_events();
                        cand.push(i);
                        pred.push(None);
                        owner.push(next);
                    }
                }
                None => {
                    if rec.games.len() < cand.len() {
                        println!("round {round}: only {} passes for {} steps: {head}", rec.games.len(), cand.len());
                        write(&cand, ".wip");
                        return;
                    }
                    g = rec.games[cand.len() - 1].clone();
                    println!("round {round}: {} steps hold on the original; {}", cand.len(), say(&g));
                    if next >= ops.len() {
                        write(&cand, "");
                        println!("DONE: {} steps; {head}; goal {:?}", cand.len(), v.goal_met);
                        for k in (0..cand.len()).step_by(std::env::var("EVERY").map(|s| s.parse().unwrap()).unwrap_or(50)) {
                            println!("  {k}: {}", say(&rec.games[k]));
                        }
                        return;
                    }
                }
            }
        }
        // Plan the next op on the engine alone from `g`.
        let op = ops[next].clone();
        let seg: Vec<FrameInput> = match op {
            Op::Hold(j, n) => vec![FrameInput::joy(amiga_input::Joy(j)); n],
            Op::Go(c, r) => {
                let start = PlanState(g.clone());
                let opt = PlanOptions {
                    seed: seed + round as u64,
                    compass: Compass::Walk,
                    max_frames: 4000,
                    goal: PlanGoal::Reach {
                        point: GuidePoint { cell: (c, r), coins_left: None, switches: None },
                        within_frames: 4000,
                        hearts: start.health(),
                    },
                    ..PlanOptions::default()
                };
                let run = RunOptions {
                    threads: 8,
                    budget: Duration::from_secs(150),
                    max_rounds: Some(3000),
                    ..RunOptions::default()
                };
                match plan_engine(start, facts(&data.level), opt, &run) {
                    Ok(route) => {
                        println!("  op {next} to {c},{r}: {} steps, {} rounds", route.inputs.len(), route.rounds);
                        route.inputs
                    }
                    Err((route, why)) => {
                        println!("  op {next} to {c},{r}: NOT REACHED after {} rounds ({why:?}); the furthest is {} steps", route.rounds, route.inputs.len());
                        write(&cand, ".wip");
                        return;
                    }
                }
            }
        };
        let judged = matches!(ops[next], Op::Go(..));
        for i in seg {
            g.step(i);
            let _ = g.take_events();
            cand.push(i);
            pred.push(judged.then(|| key(&g)));
            owner.push(next);
        }
        println!("  op {next} {:?} planned: {} steps; {}", ops[next], cand.len(), say(&g));
        next += 1;
        write(&cand, ".wip");
    }
    println!("rounds ran out");
}
