// SCRATCH: kept in plans/forest-dumb-level-1/prototype/t48-route-on-original/, never built by a crate.
// Plans W1-1's route on the original: drives the oracle a step at a time with the case's engine
// beside it (as fd-conform's run_inputs does, variables only), feeds each step the plan's input for
// the pass that reads next, and plans again from the original's state when it leaves the plan.
//
// REF=tests/routes/W1-1.txt OUT=/tmp/t48/route [RESUME=1] [MAXSTEPS=n]
//   cargo test --release -p fd-plan --test zz_onoriginal -- --nocapture
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

/// The engine alone on `inputs` from `start`.
struct Sim {
    /// The step that raised LevelEnd, if any.
    end: Option<usize>,
    /// The first step a life was lost or the character was dying in.
    lost: Option<usize>,
    keys: Vec<Key>,
    marks: Vec<Mark>,
}
fn sim(start: &Game, inputs: &[FrameInput], limit: usize, keep: bool) -> Sim {
    let mut g = PlanState(start.clone());
    let mut out = Sim { end: None, lost: None, keys: Vec::new(), marks: Vec::new() };
    for (k, &i) in inputs.iter().enumerate().take(limit) {
        g.0.step(i);
        let ev = g.0.take_events();
        if keep {
            out.keys.push(key(&g.0));
            out.marks.push((g.cell(), g.collectibles_left()));
        }
        if ev.contains(&Event::LevelEnd) {
            out.end = Some(k);
            return out;
        }
        if g.lives() != LIVES || g.dying() || !ev.is_empty() {
            out.lost = Some(k);
            return out;
        }
    }
    out
}
/// How far ahead a plan is held to: a check fails on anything within this many steps.
const HORIZON: usize = 1200;

/// The first step within `span` on which `r` from `start` fails: a life lost, an event that is not
/// the level's end, the plan running out before it, or, every 100 steps of the whole plan (`base`
/// is `r[0]`'s place in it), the character more than two cells from its mark or with more items left.
fn first_fail(start: &Game, r: &[FrameInput], marks: &[Mark], base: usize, span: usize) -> Option<usize> {
    let mut g = PlanState(start.clone());
    for k in 0..span {
        let Some(&i) = r.get(k) else { return Some(k) };
        g.0.step(i);
        let ev = g.0.take_events();
        if ev.contains(&Event::LevelEnd) {
            return None;
        }
        if g.lives() != LIVES || g.dying() || !ev.is_empty() {
            return Some(k);
        }
        if (base + k) % 100 == 0 {
            if let Some(&(cell, items)) = marks.get(k) {
                let c = g.cell();
                if c.0.abs_diff(cell.0) > 2 || c.1.abs_diff(cell.1) > 2 || g.collectibles_left() > items {
                    return Some(k);
                }
            }
        }
    }
    None
}
/// Why [`first_fail`] fails at `f`.
fn why(start: &Game, r: &[FrameInput], marks: &[Mark], f: usize) -> String {
    let Some(_) = r.get(f) else { return "the plan ran out".into() };
    let g = after(start, &r[..=f]);
    let p = PlanState(g.clone());
    if p.lives() != LIVES || p.dying() {
        return format!("a life lost; {}", say(&g));
    }
    format!("mark {:?} missed; {}", marks.get(f), say(&g))
}
fn after(start: &Game, r: &[FrameInput]) -> Game {
    let mut g = start.clone();
    for &i in r {
        g.step(i);
        g.take_events();
    }
    g
}

#[derive(Clone, Copy, Debug)]
enum Mod {
    Ins(usize, usize),
    Del(usize, usize),
}
fn apply<T: Clone>(v: &mut Vec<T>, m: Mod) -> bool {
    match m {
        Mod::Ins(q, n) => {
            if q >= v.len() {
                return false;
            }
            let i = v[q].clone();
            for _ in 0..n {
                v.insert(q, i.clone());
            }
        }
        Mod::Del(q, n) => {
            if q + n >= v.len() {
                return false;
            }
            v.drain(q..q + n);
        }
    }
    true
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

/// `r` (with its marks, `base` its place in the whole plan) changed until nothing fails within
/// [`HORIZON`] from `start`: small changes before each failure, else an explorer leg over it.
fn repair(
    data: &Arc<GameData>,
    start: &Game,
    mut r: Vec<FrameInput>,
    mut marks: Vec<Mark>,
    base: usize,
    mut skip: usize,
) -> Option<(Vec<FrameInput>, Vec<Mark>, Vec<String>)> {
    let mut notes = Vec::new();
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    for _ in 0..60 {
        let f = match first_fail(start, &r, &marks, base, HORIZON) {
            None if skip == 0 => return Some((r, marks, notes)),
            None => 150,
            Some(f) => {
                notes.push(format!("+{f} fails: {}", why(start, &r, &marks, f)));
                f
            }
        };
        // Small changes in the 150 steps before the failure.
        let w0 = f.saturating_sub(150);
        let s0 = after(start, &r[..w0]);
        let (tail, tmarks) = (r[w0..].to_vec(), marks[w0.min(marks.len())..].to_vec());
        let mut mods = Vec::new();
        for q in 0..=(f - w0) {
            for n in 1..=3 {
                mods.push(Mod::Ins(q, n));
                mods.push(Mod::Del(q, n));
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
                    apply(&mut m, mods[k]);
                    let sc = first_fail(&s0, &v, &m, base + w0, span).unwrap_or(usize::MAX);
                    scores.lock().unwrap().push((sc, k));
                });
            }
        });
        let mut sc = scores.into_inner().unwrap();
        sc.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        if let Some(&(score, k)) = sc.get(skip)
            && score > (f - w0) + 100
        {
            skip = 0;
            let m = match mods[k] {
                Mod::Ins(q, n) => Mod::Ins(q + w0, n),
                Mod::Del(q, n) => Mod::Del(q + w0, n),
            };
            apply(&mut r, m);
            apply(&mut marks, m);
            notes.push(format!("+{f}: {m:?}"));
            continue;
        }
        skip = 0;
        // An explorer leg from before the failure to a mark after it, or to the exit.
        let mut done = false;
        for (n, (back, on)) in [(100usize, 60usize), (100, 150), (200, 300), (300, 600)].into_iter().enumerate() {
            let b = f.saturating_sub(back);
            let e = f + on;
            let sb = after(start, &r[..b]);
            let to = (e + 1 < r.len() && e < marks.len()).then(|| marks[e]);
            let Some(l) = leg(data, &sb, to, 1 + n as u64) else {
                notes.push(format!("+{f}: no leg to {to:?}"));
                continue;
            };
            let lm: Vec<Mark> = {
                let mut g = sb.clone();
                l.iter()
                    .map(|&i| {
                        g.step(i);
                        g.take_events();
                        mark(&g)
                    })
                    .collect()
            };
            let (mut r2, mut m2) = (r[..b].to_vec(), marks[..b].to_vec());
            r2.extend_from_slice(&l);
            m2.extend(lm);
            if to.is_some() {
                r2.extend_from_slice(&r[e + 1..]);
                m2.extend_from_slice(&marks[e + 1..]);
            }
            // The leg holds as far as it goes; the join is the next round's.
            if first_fail(start, &r2, &m2, base, b + l.len()).is_none() {
                notes.push(format!("+{f}: a leg of {} steps from +{b} to {to:?}", l.len()));
                r = r2;
                marks = m2;
                done = true;
                break;
            }
        }
        if !done {
            println!("    no way past +{f}: {notes:?}");
            return None;
        }
    }
    println!("    60 changes and still failing: {notes:?}");
    None
}

struct Check {
    snap: Snapshot,
    read: PassInputs,
    sampler: Sampler,
    profiling: bool,
    edges: Vec<(u32, Option<EdgesCut>)>,
    engine: Game,
    at: Option<(u32, u32)>,
    steps: usize,
    actual: HashMap<u32, FrameInput>,
    plan: Vec<FrameInput>,
    marks: Vec<Mark>,
    edge_cuts: u32,
}

fn write(out: &str, steps: &[FrameInput], plan: &[FrameInput], tag: &str) {
    let body = fd_plan::script(steps).to_text();
    std::fs::write(format!("{out}{tag}.txt"), format!("# frames {}\n{body}", steps.len())).unwrap();
    let pb = script::InputScript::from_steps(plan).to_text();
    std::fs::write(format!("{out}{tag}.plan"), format!("# frames {}\n{pb}", plan.len())).unwrap();
}
fn read_script(path: &str) -> Vec<FrameInput> {
    let text = std::fs::read_to_string(path).unwrap();
    let sc = script::parse(&text).unwrap();
    let n = text
        .lines()
        .find_map(|l| l.strip_prefix("# frames "))
        .map(|n| n.parse().unwrap())
        .unwrap_or_else(|| sc.lines.last().unwrap().0);
    (0..n).map(|k| sc.input_at(k)).collect()
}

#[test]
fn on_original() {
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
    let pcs = vec![next_world, Sampler::CAMERA_UPDATE, HUNK_BASES[0] + 0x8e1c, HUNK_BASES[0] + 0x8e22, HUNK_BASES[0] + 0x8ebe];
    let (mut am, mut read) = reach_case_counting(&case, 0, &pcs).unwrap();
    let mut all_pcs = pcs.clone();
    all_pcs.extend(read.pcs());
    am.watch_write(Some(PASS_END));

    // The plan, by pass: plan[i] is pass 2 + i's input (the sync point is pass 1's end).
    let mut plan: Vec<FrameInput> = read_script(&std::env::var("REF").unwrap());
    let mut prefix: Vec<FrameInput> = Vec::new();
    if std::env::var("RESUME").is_ok() {
        prefix = read_script(&format!("{out}.wip.txt"));
        plan = read_script(&format!("{out}.wip.plan"));
        println!("resuming: {} steps fed, a plan of {}", prefix.len(), plan.len());
    }
    let alone = Game::new(data.clone(), Start::Level1);
    let first = sim(&alone, &plan, usize::MAX, true);
    println!("the plan on the engine alone: end {:?}, lost {:?}", first.end, first.lost);
    let mut keys: Vec<Option<Key>> = first.keys.iter().map(|k| Some(*k)).collect();
    let mut marks: Vec<Mark> = first.marks.clone();
    if !prefix.is_empty() {
        keys = vec![None; plan.len()];
    }

    let mut sampler = Sampler::default();
    let mut profiling = false;
    let mut edges: Vec<(u32, Option<EdgesCut>)> = Vec::new();
    let mut engine = Game::new(data.clone(), Start::Level1);
    let mut at: Option<(u32, u32)> = None;
    let mut steps: Vec<FrameInput> = Vec::new();
    let mut actual: HashMap<u32, FrameInput> = HashMap::new();
    let mut checks: Vec<Check> = Vec::new();
    let mut edge_cuts = 0u32;
    let (mut devs, mut held, mut fixed, mut rewinds) = (0, 0, 0, 0);
    let mut skip = 0usize;
    let mut force = !prefix.is_empty();
    let mut ended = false;
    let t0 = Instant::now();
    let mut idle_tail = 0;
    let mut hist: Vec<(u32, FrameInput, Vec<u32>)> = Vec::new();
    let mut pi_last = FrameInput::NONE;
    let mut cutlog: Vec<String> = Vec::new();
    let mut steplog: Vec<String> = Vec::new();

    while steps.len() < max_steps {
        let t = steps.len();
        if t % 200 == 0 && t >= prefix.len() {
            checks.push(Check {
                snap: am.snapshot(),
                read: read.clone(),
                sampler: sampler.clone(),
                profiling,
                edges: edges.clone(),
                engine: engine.clone(),
                at,
                steps: t,
                actual: actual.clone(),
                plan: plan.clone(),
                marks: marks.clone(),
                edge_cuts,
            });
            write(&out, &steps, &plan, ".wip");
            if t % 1000 == 0 {
                println!(
                    "step {t}: {}; {devs} left the plan ({held} held, {fixed} planned again), {rewinds} rewinds, {edge_cuts} edge cuts, {:.0}s",
                    say(&engine),
                    t0.elapsed().as_secs_f32()
                );
            }
        }
        let nr = read.next_read();
        let input = if t < prefix.len() {
            prefix[t]
        } else {
            plan.get(nr as usize - 2).copied().unwrap_or(FrameInput::NONE)
        };
        if t >= prefix.len() && nr as usize - 2 >= plan.len() {
            idle_tail += 1;
            if idle_tail > 30 {
                println!("the plan ran out {idle_tail} steps ago and the original has not ended the level; {}", say(&engine));
                write(&out, &steps, &plan, ".stuck");
                return;
            }
        }
        let started = Live(&am).value(irq);
        let sample = sampler.samples(&am, &case);
        if sample != profiling {
            am.set_profile(sample);
            profiling = sample;
        }
        am.step(input).unwrap();
        steps.push(input);
        let now = Live(&am).value(irq);
        if sample && now == started.wrapping_add(1) {
            edges.push((now, edges_cut(&am)));
            cutlog.push(format!("pass {now}: pc {:x?} {:?}", fd_conform::interrupted_pc(&am).map(|p| p.wrapping_sub(HUNK_BASES[0])), edges_cut(&am)));
            if cutlog.len() > 6 {
                cutlog.remove(0);
            }
        }
        sampler.stepped(&am);
        {
            let hits = |pc: u32| am.pc_hits().iter().find(|(p, _)| *p == pc).map_or(0, |(_, n)| *n);
            let pc = read.pcs();
            steplog.push(format!("step {t}: in {:x} irq {started}->{now} reads {} paused {} next_read {} log {} frames_run {}", input.joy.0, hits(pc[0]), hits(pc[1]), read.next_read(), Live(&am).value(layout_var("input_log_index").unwrap()), am.frames_run()));
            if steplog.len() > 60 {
                steplog.remove(0);
            }
            let (a, b, c, d) = (hits(pc[0]), hits(HUNK_BASES[0] + 0x8e1c), hits(HUNK_BASES[0] + 0x8e22), hits(HUNK_BASES[0] + 0x8ebe));
            if a != b || b != c || c != d {
                println!("  SPLIT READ step {t}: in {:x} irq {started}->{now} entry {a} at-read {b} after-read {c} rts {d}", input.joy.0);
            }
        }
        read.step(&am, input).unwrap();
        for p in nr..read.next_read() {
            actual.insert(p, input);
        }
        let mut bad: Option<String> = None;
        let mut check = (force || (t + 1) % 200 == 0) && t + 1 >= prefix.len();
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
                        pi_last = pi;
                        let ev = engine.take_events();
                        if !ev.is_empty() {
                            bad = Some(format!("the engine raised {ev:?} at pass {n}"));
                        }
                        actual.remove(&n);
                    }
                    at = Some((f, end));
                    let row: Vec<u32> = vars.iter().map(|v| theirs.value(v)).collect();
                    hist.push((end, pi_last, row));
                    if hist.len() > 14 {
                        hist.remove(0);
                    }
                    let diff: Vec<String> = vars
                        .iter()
                        .filter(|v| engine.probe(v) != theirs.value(v))
                        .map(|v| format!("{} engine {:#x} original {:#x}", v.name, engine.probe(v), theirs.value(v)))
                        .collect();
                    if !diff.is_empty() {
                        println!("THE TWO DIFFER at pass {end} (step {t}): {}", diff.join("; "));
                        for (j, v) in vars.iter().enumerate() {
                            let vals: Vec<String> = hist.iter().map(|h| format!("{:x}", h.2[j])).collect();
                            println!("  {:18} {}", v.name, vals.join(" "));
                        }
                        let ins: Vec<String> = hist.iter().map(|h| format!("{}:{:x}", h.0, h.1.joy.0)).collect();
                        println!("  the original's values at each pass end; pass:input {}", ins.join(" "));
                        if let Ok(d) = std::env::var("DUMP") {
                            // DUMP=shape-address-hex,count,stride : bytes of hunk 0; and collision rows e8, f1.
                            let p: Vec<u32> = d.split(',').map(|v| u32::from_str_radix(v, 16).unwrap()).collect();
                            let b: Vec<String> = (0..p[1]).map(|k| format!("{:02x}", am.peek(HUNK_BASES[0] + p[0] + k * p[2], 1)[0])).collect();
                            println!("  h0+${:x} every ${:x}: {}", p[0], p[2], b.join(" "));
                            for row in [0xe8u32, 0xf1] {
                                let o: Vec<String> = am.peek(HUNK_BASES[3] + 0xfa50 + row * 0x2e + p[3], 0x2e).iter().map(|v| format!("{v:02x}")).collect();
                                let e: Vec<String> = (0..0x2e).map(|i| format!("{:02x}", engine.plane().byte(row * 0x2e + p[3] + i))).collect();
                                println!("  row {row:x} original {}", o.join(""));
                                println!("  row {row:x} engine   {}", e.join(""));
                            }
                        }
                        println!("  cuts: {cutlog:#?}");
                        println!("  steps: {steplog:#?}");
                        write(&out, &steps, &plan, ".differs");
                        return;
                    }
                }
                Some(_) => {}
            }
            if let Some((_, last)) = at {
                let i = last as usize - 2;
                if engine.lives() != LIVES || PlanState(engine.clone()).dying() {
                    bad = Some(format!("a life lost at pass {last}"));
                }
                if let Some(k) = keys.get(i).copied().flatten()
                    && k != key(&engine)
                {
                    check = true;
                    devs += 1;
                    let now = key(&engine);
                    let d: Vec<String> = (0..20).filter(|&j| k[j] != now[j]).map(|j| format!("[{j}] plan {:x} is {:x}", k[j], now[j])).collect();
                    println!("  step {t} pass {last} leaves the plan: {}; planned input {:?}", d.join(", "), plan.get(i));
                }
            }
        }
        if am.pc_hits().iter().any(|&(p, n)| p == next_world && n > 0) {
            engine.step(input);
            let ev = engine.take_events();
            println!("the original ran next_world in step {t}; the engine: {ev:?}, finished {}", engine.finished());
            ended = engine.finished();
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
                if let Some(k) = keys.get_mut(p as usize - 2) {
                    *k = Some(key(&g));
                }
            }
            let a = (nr as usize - 2).min(plan.len());
            let r = plan[a..].to_vec();
            let old = marks[a.min(marks.len())..].to_vec();
            match repair(&data, &g, r, old, a, skip) {
                Some((v, m, notes)) => {
                    if notes.is_empty() {
                        held += 1;
                    } else {
                        fixed += 1;
                        println!("  step {t} pass {last}: {}; {notes:?}", say(&engine));
                    }
                    skip = 0;
                    plan.truncate(a);
                    marks.truncate(a);
                    keys.truncate(a);
                    let mut s = PlanState(g.clone());
                    for (k, &i) in v.iter().enumerate() {
                        if k < HORIZON {
                            s.0.step(i);
                            let ev = s.0.take_events();
                            keys.push(Some(key(&s.0)));
                            if ev.contains(&Event::LevelEnd) {
                                // The plan ends in the step that finds the level's end.
                                plan.extend_from_slice(&v[..=k]);
                                marks.extend_from_slice(&m[..=k.min(m.len() - 1)]);
                                break;
                            }
                        } else {
                            keys.push(None);
                        }
                        if k + 1 == v.len() {
                            plan.extend_from_slice(&v);
                            marks.extend_from_slice(&m);
                        }
                    }
                    keys.truncate(plan.len());
                    while marks.len() < plan.len() {
                        marks.push(*marks.last().unwrap());
                    }
                }
                None => bad = Some(format!("no plan from pass {last}; {}", say(&engine))),
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
            if rewinds > 60 {
                write(&out, &steps, &plan, ".stuck");
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
            at = c.at;
            steps.truncate(c.steps);
            actual = c.actual.clone();
            plan = c.plan.clone();
            edge_cuts = c.edge_cuts;
            marks = c.marks.clone();
            keys = vec![None; plan.len()];
            skip = rewinds;
            force = true;
        }
    }
    println!(
        "{} steps; ended {ended}; {devs} left the plan ({held} held, {fixed} planned again), {rewinds} rewinds, {edge_cuts} edge cuts, {:.0}s; {}",
        steps.len(),
        t0.elapsed().as_secs_f32(),
        say(&engine)
    );
    write(&out, &steps, &plan, if ended { "" } else { ".stopped" });
}
