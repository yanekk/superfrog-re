// SCRATCH: kept in plans/forest-dumb-level-1/prototype/t84-one-route/, never built by a crate.
// Plans one route for both sides (T84): a step script the engine alone finishes with no life lost
// and the original finishes with no life lost. From T48's tool (../t48-route-on-original): it drives
// the oracle a step at a time with the case's engine beside it and, on the same steps, the engine
// alone. Every step is fed the plan's own input for that step, so the engine alone plays exactly
// the script; the original's passes read it a step late now and then.
//
// The rest of the plan is forecast on both sides and held on both: on the engine alone from its own
// state, and for the original on the engine from the original's state (`Game::own_cuts`), each pass
// reading its own step's input or the next one's by the rule `late` below.
//
// REF=$PWD/crates/fd-plan/tests/routes/W1-1-planned.txt OUT=/tmp/t84/route [RESUME=1] [MAXSTEPS=n]
//   cargo test --release -p fd-plan --test zz_both -- --nocapture
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use amiga_conform::script;
use amiga_input::FrameInput;
use amiga_oracle::Snapshot;
use amiga_plan::run::{RunOptions, plan_engine};
use amiga_plan::{Compass, Goal as PlanGoal, GuidePoint, Options as PlanOptions, PlanGame};
use fd_conform::{
    Case, HUNK_BASES, Live, NEXT_WORLD, OracleMem, PASS_END, PassInputs, Sampler, edges_cut,
    game_data, reach_case_counting, sync,
};
use fd_core::level::EdgesCut;
use fd_core::{Event, Game, GameData, Mem, Start, Transplant, Var, layout_var};
use fd_plan::facts::facts;
use fd_plan::game::PlanState;

const LIVES: i16 = 32;
type Key = [u32; 20];
type Mark = ((u16, u16), u16);

fn key(g: &Game) -> Key {
    let s = g.state();
    let c = &s.edges.creatures;
    let mut k = [0u32; 20];
    k[0] = s.char_x;
    k[1] = s.char_y;
    k[2] = s.lives as u32;
    k[3] = s.hits_taken as u32;
    k[4] = s.items_left as u32;
    for i in 0..5 {
        k[5 + 3 * i] = c.active[i] as u32;
        k[6 + 3 * i] = c.slots[i].x as u32;
        k[7 + 3 * i] = c.slots[i].y as u32;
    }
    k
}
fn mark(g: &Game) -> Mark {
    let p = PlanState(g.clone());
    (p.cell(), p.collectibles_left())
}
fn say(g: &Game) -> String {
    let s = g.state();
    let (cell, items) = mark(g);
    format!(
        "at {:x},{:x} cell {:?} items {} lives {:x} hits {} irq {}",
        s.char_x, s.char_y, cell, items, s.lives, s.hits_taken, s.irq_count
    )
}

/// Whether the original's next pass reads the joystick in the step after its own (measured on
/// route-W1-1, 22 527 passes: its `game_frame` restores and cuts the drawn creatures and draws one
/// list of animated cells before `read_joystick`, and with two creatures drawn the frame ends
/// first). Wrong on about 1 pass in 80; the original's run corrects it.
fn late(g: &Game) -> bool {
    let st = g.state();
    let drawn = st.creature_draw.drawn.iter().filter(|&&f| f != 0).count();
    let list = (st.animation.phase as usize + 1) & 3;
    let cells = st.edges.cells.used[list].iter().filter(|&&f| f != 0).count();
    drawn >= 2 || (drawn == 1 && cells >= 5)
}

/// How far around its step a mark is looked for.
const NEAR: usize = 30;

/// Steps played idle past the plan's end before a side that has not ended the level fails.
const IDLE: usize = 64;

/// One side's forecast: an engine, the step its next pass is the own pass of (`j`, into the plan's
/// rest; -1 when that pass started a step ago and has not read), and whether its passes read late.
#[derive(Clone)]
struct Side {
    g: PlanState,
    j: isize,
    model: bool,
    done: bool,
}
impl Side {
    fn pass(&mut self, s: &[FrameInput]) -> Vec<Event> {
        let lag = (self.model && late(&self.g.0)) as isize;
        let k = (self.j + lag).max(0) as usize;
        self.g.0.step(s.get(k).copied().unwrap_or(FrameInput::NONE));
        self.j += 1;
        let ev = self.g.0.take_events();
        if ev.contains(&Event::LevelEnd) {
            self.done = true;
        }
        ev
    }
    /// At the pass whose own step is `to`, counted from there.
    fn at(&self, s: &[FrameInput], to: usize) -> Side {
        let mut x = self.clone();
        while x.j < to as isize && !x.done {
            x.pass(s);
        }
        x.j -= to as isize;
        x
    }
    /// This side over `span` steps: where it is after each pass (the cell, the items left), from
    /// the pass whose own step is `self.j`, and the first step it fails in: a life lost, an event
    /// that is not the level's end, or the plan run out.
    fn trace(&self, s: &[FrameInput], span: usize) -> (Vec<Mark>, Option<usize>) {
        let mut x = self.clone();
        let mut t = Vec::new();
        while x.j < span as isize && !x.done {
            let k = x.j.max(0) as usize;
            if k >= s.len() + IDLE {
                return (t, Some(k));
            }
            let ev = x.pass(s);
            if x.done {
                break;
            }
            if x.g.lives() != LIVES || x.g.dying() || !ev.is_empty() {
                return (t, Some(k));
            }
            t.push((x.g.cell(), x.g.collectibles_left()));
        }
        (t, None)
    }
    fn why(&self, s: &[FrameInput], f: usize) -> String {
        let x = self.at(s, f + 1);
        if x.g.lives() != LIVES || x.g.dying() {
            return format!("a life lost; {}", say(&x.g.0));
        }
        format!("off the plan, apart from the other side, or the plan ran out; {}", say(&x.g.0))
    }
}
/// Both sides: the first step either fails in, and which (0 the engine alone, 1 the original).
/// Beyond each side's own failures ([`Side::trace`]): the engine alone is held to the plan's marks,
/// every 100 steps of the whole plan (`base` is `s[0]`'s place in it) within two cells of a mark
/// [`NEAR`] steps around, with no more items left than the mark NEAR steps back; and the original
/// is held to the engine alone, every 20 steps: within two cells of where the engine alone is
/// within NEAR steps, and neither an item behind it NEAR steps back nor ahead of it NEAR steps on.
/// An item one side takes and the other misses is never made up.
fn first_fail(sides: &[Side; 2], s: &[FrameInput], marks: &[Mark], base: usize, span: usize) -> Option<(usize, usize)> {
    let (ta, fa) = sides[0].trace(s, span);
    let (to, fo) = sides[1].trace(s, span);
    let mut fail: Option<(usize, usize)> = None;
    let mut note = |f: usize, side: usize| {
        if fail.is_none_or(|(g, _)| f < g) {
            fail = Some((f, side));
        }
    };
    if let Some(f) = fa {
        note(f, 0);
    }
    if let Some(f) = fo {
        note(f, 1);
    }
    let near = |a: (u16, u16), b: (u16, u16)| a.0.abs_diff(b.0) <= 2 && a.1.abs_diff(b.1) <= 2;
    // ta[i] is after the pass whose own step is i + sides[0].j; to[i] likewise.
    let (ja, jo) = (sides[0].j, sides[1].j);
    for (i, &(c, items)) in ta.iter().enumerate() {
        let k = i as isize + ja;
        if k < NEAR as isize || (base as isize + k) % 100 != 0 {
            continue;
        }
        let k = k as usize;
        let around = &marks[(k - NEAR).min(marks.len())..(k + NEAR).min(marks.len())];
        if let Some(&(_, m)) = around.first()
            && (!around.iter().any(|&(cell, _)| near(c, cell)) || items > m)
        {
            note(k, 0);
            break;
        }
    }
    if !ta.is_empty() {
        for (i, &(c, items)) in to.iter().enumerate() {
            let k = i as isize + jo;
            if k < NEAR as isize || k % 20 != 0 {
                continue;
            }
            let lo = (k - NEAR as isize - ja).max(0) as usize;
            let hi = ((k + NEAR as isize - ja).max(0) as usize).min(ta.len() - 1);
            if lo >= ta.len() {
                break;
            }
            if !ta[lo..=hi].iter().any(|&(cell, _)| near(c, cell)) || items > ta[lo].1 || items < ta[hi].1 {
                note(k as usize, 1);
                break;
            }
        }
    }
    fail
}

/// How far ahead a plan is held to: a check fails on anything within this many steps.
const HORIZON: usize = 1200;

/// A failure nearer than this that no change mends sends the run back to a checkpoint.
const CLOSE: usize = 250;

#[derive(Clone, Copy, Debug)]
enum Mod {
    Ins(usize, usize),
    Del(usize, usize),
    /// `n` steps from `q` hold joystick `joy`.
    Set(usize, usize, u8),
}
fn apply(v: &mut Vec<FrameInput>, m: Mod) -> bool {
    match m {
        Mod::Ins(q, n) => {
            if q >= v.len() {
                return false;
            }
            let i = v[q];
            for _ in 0..n {
                v.insert(q, i);
            }
        }
        Mod::Del(q, n) => {
            if q + n >= v.len() {
                return false;
            }
            v.drain(q..q + n);
        }
        Mod::Set(q, n, joy) => {
            if q + n >= v.len() || (q..q + n).all(|k| v[k].joy.0 == joy) {
                return false;
            }
            for k in q..q + n {
                v[k].joy.0 = joy;
            }
        }
    }
    true
}
fn apply_marks(v: &mut Vec<Mark>, m: Mod) {
    match m {
        Mod::Ins(q, n) if q < v.len() => {
            let i = v[q];
            for _ in 0..n {
                v.insert(q, i);
            }
        }
        Mod::Del(q, n) if q + n < v.len() => {
            v.drain(q..q + n);
        }
        _ => {}
    }
}
/// By how many steps `after` runs behind `before` at their ends (negative: ahead): the shift at
/// which the last 100 steps of the shorter agree best, when at least 70 of them do.
fn drift(before: &[Mark], after: &[Mark]) -> Option<isize> {
    let n = before.len().min(after.len());
    if n < 200 {
        return None;
    }
    let mut best: Option<(usize, isize)> = None;
    for d in -60isize..=60 {
        let hits = (n - 100..n).filter(|&k| before.get((k as isize - d) as usize) == Some(&after[k])).count();
        if best.is_none_or(|(h, b)| hits > h || (hits == h && d.abs() < b.abs())) {
            best = Some((hits, d));
        }
    }
    best.filter(|(h, _)| *h >= 70).map(|(_, d)| d)
}
fn shift(m: Mod, by: usize) -> Mod {
    match m {
        Mod::Ins(q, n) => Mod::Ins(q + by, n),
        Mod::Del(q, n) => Mod::Del(q + by, n),
        Mod::Set(q, n, j) => Mod::Set(q + by, n, j),
    }
}

fn leg(data: &Arc<GameData>, start: &Game, to: Option<Mark>, seed: u64) -> Option<Vec<FrameInput>> {
    let s = PlanState(start.clone());
    let goal = match to {
        Some(to) => PlanGoal::Reach {
            point: GuidePoint { cell: to.0, coins_left: Some(to.1), switches: None },
            within_frames: 4000,
            hearts: s.health(),
        },
        None => PlanGoal::Exit,
    };
    let opt = PlanOptions { seed, compass: Compass::Walk, max_frames: 4000, goal, ..PlanOptions::default() };
    let run = RunOptions {
        threads: std::thread::available_parallelism().map_or(4, |n| n.get()),
        budget: Duration::from_secs(120),
        max_rounds: Some(2000),
        ..RunOptions::default()
    };
    plan_engine(s, facts(&data.level), opt, &run).ok().map(|r| r.inputs)
}

/// `r` (with its marks, `base` its place in the whole plan) changed until neither side fails within
/// [`HORIZON`]: small changes before each failure, else an explorer leg over it that holds on both.
fn repair(
    data: &Arc<GameData>,
    sides: &[Side; 2],
    mut r: Vec<FrameInput>,
    mut marks: Vec<Mark>,
    base: usize,
    mut skip: usize,
    mut poke: Option<usize>,
) -> (Vec<FrameInput>, Vec<Mark>, Vec<String>, Option<usize>) {
    let mut notes = Vec::new();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let joys: Vec<u8> = {
        let mut j: Vec<u8> = r.iter().map(|i| i.joy.0).collect();
        j.sort();
        j.dedup();
        j
    };
    for _ in 0..80 {
        // After a rewind from a difference between the engine and the original the plan is
        // changed first just before that step (`poke`), the `skip`th best change: the difference
        // comes again unless the route passes its place another way.
        let poked = poke.take().filter(|_| skip > 0);
        let f = match (poked, first_fail(sides, &r, &marks, base, HORIZON)) {
            (Some(p), _) => p,
            (None, None) if skip == 0 => return (r, marks, notes, None),
            // After any other rewind nothing may fail in the forecast: changed anyway.
            (None, None) => 150,
            (None, Some((f, side))) => {
                notes.push(format!("+{f} fails on {}: {}", ["the engine alone", "the original's forecast"][side], sides[side].why(&r, f)));
                f
            }
        };
        let poked = poked.is_some();
        let f = f.min(r.len().saturating_sub(1));
        // Small changes in the 150 steps before the failure.
        let w0 = f.saturating_sub(if poked { 80 } else { 150 });
        let s0 = [sides[0].at(&r, w0), sides[1].at(&r, w0)];
        let (tail, tmarks) = (r[w0..].to_vec(), marks[w0.min(marks.len())..].to_vec());
        let mut mods = Vec::new();
        // Before a difference only another joystick held 8 or 16 steps: a step doubled or
        // dropped while the character stands changes nothing.
        for q in 0..(if poked { 0 } else { f - w0 + 1 }) {
            for n in 1..=3 {
                mods.push(Mod::Ins(q, n));
                mods.push(Mod::Del(q, n));
            }
        }
        // Then another joystick held a while: a wait, a jump early, a step back.
        for q in (0..=(f - w0)).step_by(2) {
            for n in [2usize, 4, 8, 16] {
                if poked && n < 8 {
                    continue;
                }
                for &j in &joys {
                    mods.push(Mod::Set(q, n, j));
                }
            }
        }
        let span = (f - w0) + 600;
        let next = AtomicUsize::new(0);
        let scores = std::sync::Mutex::new(Vec::<(usize, usize)>::new());
        std::thread::scope(|s| {
            for _ in 0..threads {
                s.spawn(|| loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    if k >= mods.len() {
                        return;
                    }
                    let (mut v, mut m) = (tail.clone(), tmarks.clone());
                    if !apply(&mut v, mods[k]) {
                        continue;
                    }
                    apply_marks(&mut m, mods[k]);
                    let sc = first_fail(&s0, &v, &m, base + w0, span).map_or(usize::MAX, |f| f.0);
                    scores.lock().unwrap().push((sc, k));
                });
            }
        });
        let mut sc = scores.into_inner().unwrap();
        sc.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let pick = sc.get(skip).filter(|s| s.0 > (f - w0) + 100);
        if let Some(&(_, k)) = pick {
            skip = 0;
            let m = shift(mods[k], w0);
            // The marks move with the plan by as many steps as the change makes the engine alone
            // early or late, measured: a joystick held 16 steps can cost none or all of them, and
            // marks left behind would fail every later check.
            let before = sides[0].trace(&r, f + 300).0;
            apply(&mut r, m);
            let after = sides[0].trace(&r, f + 300).0;
            let (q, nominal) = match m {
                Mod::Ins(q, n) => (q, n as isize),
                Mod::Del(q, n) => (q, -(n as isize)),
                Mod::Set(q, _, _) => (q, 0),
            };
            let d = drift(&before, &after).unwrap_or(nominal);
            if q < marks.len() {
                if d > 0 {
                    let i = marks[q];
                    marks.splice(q..q, std::iter::repeat_n(i, d as usize));
                } else {
                    let e = (q + (-d) as usize).min(marks.len() - 1);
                    marks.drain(q..e);
                }
            }
            notes.push(format!("+{f}: {m:?}"));
            continue;
        }
        skip = 0;
        // An explorer leg on the engine alone from before the failure to a mark after it, or to
        // the exit, kept when the original's forecast holds on it too.
        let mut done = false;
        'legs: for (back, on) in [(100usize, 60usize), (100, 150), (200, 300), (300, 600)] {
            for seed in 1..=3u64 {
                let b = f.saturating_sub(back);
                let e = f + on;
                let sb = sides[0].at(&r, b);
                let to = (e + 1 < r.len() && e < marks.len()).then(|| marks[e]);
                let Some(l) = leg(data, &sb.g.0, to, seed) else {
                    notes.push(format!("+{f}: no leg to {to:?}"));
                    continue;
                };
                let lm: Vec<Mark> = {
                    let mut g = sb.g.0.clone();
                    l.iter()
                        .map(|&i| {
                            g.step(i);
                            g.take_events();
                            mark(&g)
                        })
                        .collect()
                };
                let (mut r2, mut m2) = (r[..b].to_vec(), marks[..b.min(marks.len())].to_vec());
                r2.extend_from_slice(&l);
                m2.extend(lm);
                if to.is_some() {
                    r2.extend_from_slice(&r[e + 1..]);
                    m2.extend_from_slice(&marks[e + 1..]);
                }
                // The leg holds as far as it goes; the join is the next round's.
                if first_fail(sides, &r2, &m2, base, b + l.len()).is_none() {
                    notes.push(format!("+{f}: a leg of {} steps from +{b} to {to:?}", l.len()));
                    r = r2;
                    marks = m2;
                    done = true;
                    break 'legs;
                }
            }
        }
        if !done {
            println!("    no way past +{f}: {notes:?}");
            return (r, marks, notes, Some(f));
        }
    }
    println!("    80 changes and still failing: {notes:?}");
    let f = first_fail(sides, &r, &marks, base, HORIZON).map(|f| f.0);
    (r, marks, notes, f)
}

struct Check {
    snap: Snapshot,
    read: PassInputs,
    sampler: Sampler,
    profiling: bool,
    edges: Vec<(u32, Option<EdgesCut>)>,
    engine: Game,
    alone: PlanState,
    alone_end: Option<usize>,
    at: Option<(u32, u32)>,
    steps: usize,
    actual: HashMap<u32, FrameInput>,
    plan: Vec<FrameInput>,
    marks: Vec<Mark>,
    edge_cuts: u32,
}

fn write(out: &str, plan: &[FrameInput], fed: usize, tag: &str) {
    let body = fd_plan::script(plan).to_text();
    std::fs::write(format!("{out}{tag}.txt"), format!("# frames {}\n# fed {fed}\n{body}", plan.len())).unwrap();
}
fn read_script(path: &str) -> (Vec<FrameInput>, usize) {
    let text = std::fs::read_to_string(path).unwrap();
    let sc = script::parse(&text).unwrap();
    let num = |tag: &str| text.lines().find_map(|l| l.strip_prefix(tag)).map(|n| n.parse::<usize>().unwrap());
    let n = num("# frames ").unwrap_or_else(|| sc.lines.last().unwrap().0 as usize);
    ((0..n as u32).map(|k| sc.input_at(k)).collect(), num("# fed ").unwrap_or(0))
}

#[test]
fn on_both() {
    let out = std::env::var("OUT").unwrap();
    let max_steps: usize = std::env::var("MAXSTEPS").map(|s| s.parse().unwrap()).unwrap_or(30000);
    let data = game_data().unwrap();
    const MORE: &str = r#", "clxdat_copy", "creature_active+$0", "creature_active+$1", "creature_active+$2", "creature_active+$3", "creature_active+$4", "creature_slots+$0", "creature_slots+$4", "creature_slots+$8", "creature_slots+$c", "creature_slots+$10", "creature_slots+$14", "creature_slots+$18", "creature_slots+$1c", "creature_slots+$20", "creature_slots+$24""#;
    let vars: Vec<&'static Var> = format!("{}{MORE}", fd_plan::confirm::VARS)
        .split(", ")
        .map(|v| layout_var(v.trim_matches('"')).unwrap())
        .collect();
    let case = Case::parse(
        "route-plan",
        &format!(
            "sync = {{ script = \"{}\", frame = {} }}\nframes = 32000\ncompare = [\"vars\"]\nvars = [\"char_x\"]\ngoal = {{ level_end = true, by_frame = 38301 }}\n",
            sync::LEVEL_1_SCRIPT,
            sync::LEVEL_1_SYNC
        ),
    )
    .unwrap();
    let irq = layout_var("irq_count").unwrap();
    let next_world = HUNK_BASES[NEXT_WORLD.0 as usize] + NEXT_WORLD.1;
    let pcs = vec![next_world, Sampler::CAMERA_UPDATE];
    let (mut am, mut read) = reach_case_counting(&case, 0, &pcs).unwrap();
    let mut all_pcs = pcs.clone();
    all_pcs.extend(read.pcs());
    am.watch_write(Some(PASS_END));

    // The plan, by step: plan[t] is step t's input, the engine alone's pass 2 + t.
    let (mut plan, mut prefix): (Vec<FrameInput>, usize) = read_script(&std::env::var("REF").unwrap());
    if std::env::var("RESUME").is_ok() {
        (plan, prefix) = read_script(&format!("{out}.wip.txt"));
        println!("resuming: {prefix} steps fed of a plan of {}", plan.len());
    } else {
        prefix = 0;
    }
    let mut marks: Vec<Mark> = {
        let mut g = Game::new(data.clone(), Start::Level1);
        plan.iter()
            .map(|&i| {
                g.step(i);
                g.take_events();
                mark(&g)
            })
            .collect()
    };
    // The original's forecast: the key after each pass, by pass.
    let mut keys: HashMap<u32, Key> = HashMap::new();

    let mut sampler = Sampler::default();
    let mut profiling = false;
    let mut edges: Vec<(u32, Option<EdgesCut>)> = Vec::new();
    let mut engine = Game::new(data.clone(), Start::Level1);
    let mut alone = PlanState(Game::new(data.clone(), Start::Level1));
    let mut alone_end: Option<usize> = None;
    let mut orig_end: Option<usize> = None;
    let mut at: Option<(u32, u32)> = None;
    let mut steps = 0usize;
    let mut actual: HashMap<u32, FrameInput> = HashMap::new();
    let mut checks: Vec<Check> = Vec::new();
    let mut edge_cuts = 0u32;
    let (mut devs, mut held, mut fixed, mut rewinds) = (0, 0, 0, 0);
    let mut skip = 0usize;
    let mut force = true;
    let mut differs = 0;
    // The step a difference between the engine and the original sent the run back from.
    let mut poke: Option<usize> = None;
    let t0 = Instant::now();
    let mut log: Vec<String> = Vec::new();

    while steps < max_steps {
        let t = steps;
        if t % 200 == 0 && t >= prefix {
            checks.push(Check {
                snap: am.snapshot(),
                read: read.clone(),
                sampler: sampler.clone(),
                profiling,
                edges: edges.clone(),
                engine: engine.clone(),
                alone: alone.clone(),
                alone_end,
                at,
                steps: t,
                actual: actual.clone(),
                plan: plan.clone(),
                marks: marks.clone(),
                edge_cuts,
            });
            // Snapshots are big: keep the last 40 and every fifth before.
            if checks.len() > 60 {
                let n = checks.len() - 40;
                let mut k = 0;
                checks.retain(|c| {
                    k += 1;
                    k > n || c.steps % 1000 == 0
                });
            }
            write(&out, &plan, t, ".wip");
            if t % 1000 == 0 {
                println!(
                    "step {t}: the original {}; alone {}; {devs} left the forecast ({held} held, {fixed} planned again), {rewinds} rewinds, {edge_cuts} edge cuts, {:.0}s",
                    say(&engine),
                    say(&alone.0),
                    t0.elapsed().as_secs_f32()
                );
            }
        }
        if t >= plan.len() + IDLE {
            println!("the plan ran out {IDLE} steps ago and the original has not ended the level; {}", say(&engine));
            write(&out, &plan, t, ".stuck");
            return;
        }
        let input = plan.get(t).copied().unwrap_or(FrameInput::NONE);
        let nr = read.next_read();
        let started = Live(&am).value(irq);
        let sample = sampler.samples(&am, &case);
        if sample != profiling {
            am.set_profile(sample);
            profiling = sample;
        }
        am.step(input).unwrap();
        steps += 1;
        let now = Live(&am).value(irq);
        if sample && now == started.wrapping_add(1) {
            edges.push((now, edges_cut(&am)));
        }
        sampler.stepped(&am);
        read.step(&am, input).unwrap();
        for p in nr..read.next_read() {
            actual.insert(p, input);
        }
        let mut bad: Option<String> = None;
        // The engine alone on the same step.
        if alone_end.is_none() {
            alone.0.step(input);
            let ev = alone.0.take_events();
            if ev.contains(&Event::LevelEnd) {
                alone_end = Some(t);
                println!("the engine alone found the level's end in step {t}");
            } else if alone.lives() != LIVES || alone.dying() || !ev.is_empty() {
                bad = Some(format!("the engine alone: a life lost or {ev:?}; {}", say(&alone.0)));
            }
        }
        // After a resume the first check waits for the first checkpoint, which a rewind needs.
        let mut check = (force || (t + 1) % 200 == 0) && t >= prefix;
        if let Some(mem) = am.watched_memory() {
            let theirs = OracleMem(&mem);
            let end = theirs.value(irq);
            match at {
                None => {
                    engine.transplant(&theirs);
                    at = Some((end, end));
                    read.drop_through(end);
                }
                Some((f, last)) if end > last => {
                    for n in last + 1..=end {
                        let pi = read.take(n).expect("a pass ended that never read its input");
                        let cut = edges.iter().find(|(p, _)| *p == n).and_then(|c| c.1);
                        edges.retain(|(p, _)| *p > n);
                        edge_cuts += cut.is_some() as u32;
                        engine.cut_foreground(cut);
                        engine.step(pi);
                        let ev = engine.take_events();
                        if !ev.is_empty() {
                            bad = Some(format!("the engine beside the original raised {ev:?} at pass {n}"));
                        }
                        actual.remove(&n);
                    }
                    at = Some((f, end));
                    let diff: Vec<String> = vars
                        .iter()
                        .filter(|v| engine.probe(v) != theirs.value(v))
                        .map(|v| format!("{} engine {:#x} original {:#x}", v.name, engine.probe(v), theirs.value(v)))
                        .collect();
                    if !diff.is_empty() {
                        println!("THE TWO DIFFER at pass {end} (step {t}): {}", diff.join("; "));
                        // A difference between the engine and the original is not this tool's to
                        // mend: the script is kept and the route goes another way.
                        differs += 1;
                        write(&out, &plan[..=t.min(plan.len() - 1)], t + 1, &format!(".differs-{end}"));
                        bad = Some(format!("the engine and the original differ at pass {end}"));
                        poke = Some(t.saturating_sub(4));
                    }
                }
                Some(_) => {}
            }
            if let Some((_, last)) = at {
                if engine.lives() != LIVES || PlanState(engine.clone()).dying() {
                    bad = Some(format!("the original: a life lost at pass {last}; {}", say(&engine)));
                }
                if let Some(k) = keys.get(&last)
                    && *k != key(&engine)
                {
                    check = true;
                    devs += 1;
                    let now = key(&engine);
                    let d: Vec<String> = (0..20).filter(|&j| k[j] != now[j]).map(|j| format!("[{j}] {:x} is {:x}", k[j], now[j])).collect();
                    log.push(format!("step {t} pass {last} leaves the forecast: {}", d.join(", ")));
                    if log.len() > 30 {
                        log.remove(0);
                    }
                    keys.clear();
                }
            }
        }
        if am.pc_hits().iter().any(|&(p, n)| p == next_world && n > 0) {
            engine.step(input);
            let ev = engine.take_events();
            println!("the original ran next_world in step {t}; the engine beside it: {ev:?}, finished {}", engine.finished());
            orig_end = Some(t);
            break;
        }
        if bad.is_none() && check {
            force = false;
            let (_, last) = at.unwrap();
            // The passes that have read their input and not ended yet.
            let mut g = engine.clone();
            g.own_cuts();
            let nr = read.next_read();
            for p in last + 1..nr {
                g.step(actual[&p]);
                g.take_events();
            }
            let a = (t + 1).min(plan.len());
            let sides = [
                Side { g: alone.clone(), j: 0, model: false, done: alone_end.is_some() },
                Side { g: PlanState(g), j: nr as isize - 2 - (t as isize + 1), model: true, done: false },
            ];
            let r = plan[a..].to_vec();
            let old = marks[a.min(marks.len())..].to_vec();
            // A failure left far ahead is the forecast's, which is not exact: play on and plan
            // again nearer to it.
            match repair(&data, &sides, r, old, a, skip, poke.take().and_then(|p| p.checked_sub(a)).filter(|p| *p < HORIZON)) {
                (_, _, _, Some(f)) if f < CLOSE => {
                    bad = Some(format!("no plan from pass {last}, failing {f} steps on; the original {}; alone {}", say(&engine), say(&alone.0)))
                }
                (v, m, notes, _) => {
                    if notes.is_empty() {
                        held += 1;
                    } else {
                        fixed += 1;
                        println!("  step {t} pass {last}: the original {}; alone {}; {notes:?}", say(&engine), say(&alone.0));
                    }
                    skip = 0;
                    plan.truncate(a);
                    marks.truncate(a);
                    plan.extend_from_slice(&v);
                    marks.extend_from_slice(&m);
                    while marks.len() < plan.len() {
                        marks.push(*marks.last().unwrap());
                    }
                    // The original's forecast, by pass, for the passes ahead.
                    keys.clear();
                    let mut x = sides[1].clone();
                    for p in nr..nr + HORIZON as u32 {
                        if x.done {
                            break;
                        }
                        x.pass(&v);
                        keys.insert(p, key(&x.g.0));
                    }
                }
            }
        }
        if let Some(why) = bad {
            rewinds += 1;
            // Back to a checkpoint well before, and a different plan from there.
            while checks.len() > 1 && checks.last().unwrap().steps + 300 > t {
                checks.pop();
            }
            let c = checks.last().unwrap();
            println!("  step {t}: {why}; back to step {} (rewind {rewinds})", c.steps);
            println!("  {log:#?}");
            if rewinds > 60 {
                write(&out, &plan, t, ".stuck");
                return;
            }
            am.restore(&c.snap);
            am.watch_pcs(&all_pcs);
            am.watch_write(Some(PASS_END));
            am.set_profile(c.profiling);
            read = c.read.clone();
            sampler = c.sampler.clone();
            profiling = c.profiling;
            edges = c.edges.clone();
            engine = c.engine.clone();
            alone = c.alone.clone();
            alone_end = c.alone_end;
            at = c.at;
            steps = c.steps;
            actual = c.actual.clone();
            plan = c.plan.clone();
            edge_cuts = c.edge_cuts;
            marks = c.marks.clone();
            keys.clear();
            skip = rewinds;
            force = true;
        }
    }
    // The engine alone to its own end, on the plan's rest and then idle.
    let mut t = steps;
    while alone_end.is_none() && t < plan.len() + IDLE {
        alone.0.step(plan.get(t).copied().unwrap_or(FrameInput::NONE));
        let ev = alone.0.take_events();
        if ev.contains(&Event::LevelEnd) {
            alone_end = Some(t);
        } else if alone.lives() != LIVES || alone.dying() || !ev.is_empty() {
            println!("the engine alone after the original's end, step {t}: a life lost or {ev:?}");
            break;
        }
        t += 1;
    }
    println!(
        "{steps} steps; {differs} differences between the engine and the original gone round; the original ended in step {orig_end:?}, the engine alone in {alone_end:?}; {devs} left the forecast ({held} held, {fixed} planned again), {rewinds} rewinds, {edge_cuts} edge cuts, {:.0}s; {}",
        t0.elapsed().as_secs_f32(),
        say(&engine)
    );
    if let (Some(o), Some(a)) = (orig_end, alone_end) {
        let n = o.max(a) + 1;
        plan.resize(n, FrameInput::NONE);
        write(&out, &plan, n, "");
    } else {
        write(&out, &plan, steps, ".stopped");
    }
}
