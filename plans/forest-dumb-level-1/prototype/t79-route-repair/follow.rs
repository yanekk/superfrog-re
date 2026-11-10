// SCRATCH: never committed (kept in plans/forest-dumb-level-1/prototype/t79-route-repair/).
// Repairs a script made with creatures frozen so that, live on the original, the character still
// follows the frozen path. Each round plays the candidate on the original, takes the engine's state
// at a pass shortly before the run leaves the path (the case's engine is the original's state there,
// the two agreeing), and searches on the engine alone from that state for jumps and waits that keep
// the path; the next round checks them on the original.
//
// CASE=death-water REF=<frozen script>:<frames> OUT=/tmp/x.txt [START=<file this tool wrote>]
// [SEED=n] [ITERS=n] [ROUNDS=n] [NOLIVES=1] [ADV=n]
use std::collections::VecDeque;
use std::path::Path;

use amiga_chips::Picture;
use amiga_conform::script;
use amiga_oracle::FrameInput;
use fd_conform::{Case, Engine, Goal, Inputs, Options, conform_dir, game_data, run_inputs};
use fd_core::level::EdgesCut;
use fd_core::{Event, Game, Mem, Start, Transplant, Var};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

type Key = (u32, u32, u16);
fn key(g: &Game) -> Key {
    let s = g.state();
    (s.char_x, s.char_y, if std::env::var("NOLIVES").is_ok() { 0 } else { s.lives })
}

/// Passes not judged: a lead-in taken from another case's script (`FREE`).
fn free() -> usize {
    std::env::var("FREE").map(|s| s.parse().unwrap()).unwrap_or(0)
}

/// Passes a run may stay off the path before it counts as having left it.
fn tol() -> usize { std::env::var("TOL").map(|s| s.parse().unwrap()).unwrap_or(90) }
const RING: usize = 520;
const SNAP: usize = 32;

#[derive(Clone)]
struct Cand {
    inp: Vec<FrameInput>, // by script step
    map: Vec<usize>,      // by script step: the frozen path's step
    lag: Vec<i8>,               // by pass: the step it read, less its own
    cut: Vec<Option<EdgesCut>>, // by pass: the cut the original handed
}

impl Cand {
    fn want(&self, refk: &[Key], off: usize, k: usize) -> Key {
        refk[self.map[(k + off).min(self.map.len() - 1)]]
    }
    /// On the path at pass `k`, give or take two steps: a pass that read its input a step late
    /// stays a step behind.
    fn on(&self, refk: &[Key], off: usize, k: usize, key: Key) -> bool {
        k < free() || (k.saturating_sub(2)..=k + 2).any(|j| self.want(refk, off, j) == key)
    }
    fn input(&self, off: usize, k: usize) -> FrameInput {
        let j = (k + off) as isize + self.lag.get(k).copied().unwrap_or(0) as isize;
        self.inp.get(j.max(0) as usize).copied().unwrap_or(FrameInput::NONE)
    }
    fn insert(&mut self, t: usize, i: FrameInput, r: usize) {
        self.inp.insert(t, i);
        self.map.insert(t, r);
        self.lag.insert(t, 0);
        self.cut.insert(t, None);
    }
    fn remove(&mut self, t: usize) {
        self.inp.remove(t);
        self.map.remove(t);
        self.lag.remove(t);
        self.cut.remove(t);
    }
    fn passes(&self, off: usize) -> usize {
        self.inp.len() - off
    }
}

struct Rec<'a> {
    game: Game,
    c: &'a Cand,
    refk: &'a [Key],
    off: usize,
    inputs: Vec<FrameInput>,
    cuts: Vec<Option<EdgesCut>>,
    pending: Option<EdgesCut>,
    last_match: isize,
    bad: Option<usize>,
    ring: VecDeque<(usize, Game)>,
    trace: Vec<String>,
}
impl Transplant for Rec<'_> {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.game.transplant(mem);
    }
    fn probe(&self, v: &Var) -> u32 {
        self.game.probe(v)
    }
}
impl Engine for Rec<'_> {
    fn pass(&mut self, input: FrameInput) {
        let k = self.inputs.len();
        self.inputs.push(input);
        self.cuts.push(self.pending.take());
        Engine::pass(&mut self.game, input);
        if self.bad.is_none() {
            if self.ring.len() == RING {
                self.ring.pop_front();
            }
            self.ring.push_back((k, self.game.clone()));
            if k < self.c.passes(self.off) && self.c.on(self.refk, self.off, k, key(&self.game)) {
                self.last_match = k as isize;
            } else if (k as isize - self.last_match) as usize > tol() {
                self.bad = Some((self.last_match + 1) as usize);
            }
        }
        let s = self.game.state();
        let c = &s.edges.creatures;
        let cr: Vec<String> = (0..5)
            .filter(|&i| c.active[i] != 0)
            .map(|i| format!("k{}@{:x},{:x}", c.slots[i].kind, c.slots[i].x, c.slots[i].y))
            .collect();
        self.trace.push(format!(
            "{k} irq {} joy {:02x} at {:x},{:x} lives {:x} want {:x?} hits {} {:?}",
            s.irq_count,
            input.joy.0,
            s.char_x,
            s.char_y,
            s.lives,
            self.c.want(self.refk, self.off, k),
            s.hits_taken,
            cr
        ));
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
        self.pending = at;
        Engine::foreground_cut(&mut self.game, at)
    }
}

/// The engine from `g0` (the state after pass `pk`): the first pass off the path for good, `passes`
/// if none, never looking past pass `limit`. `mode` 0 replays the input lags and cuts the original
/// last showed, 1 is the engine with neither, 2 reads every input a step late.
fn eval1(refk: &[Key], off: usize, pk: usize, g0: &Game, c: &Cand, limit: usize, mode: u8) -> usize {
    let passes = c.passes(off).min(limit);
    let mut g = g0.clone();
    let mut k = pk;
    let mut last_match = k as isize;
    let mut bad = passes;
    loop {
        k += 1;
        if k >= passes {
            break;
        }
        let (i, cut) = match mode {
            0 => (c.input(off, k), c.cut.get(k).copied().flatten()),
            1 => (c.inp[k + off], None),
            _ => (c.inp[(k + off + 1).min(c.inp.len() - 1)], None),
        };
        g.cut_foreground(cut);
        g.step(i);
        let _ = g.take_events();
        if c.on(refk, off, k, key(&g)) {
            last_match = k as isize;
        } else if (k as isize - last_match) as usize > tol() {
            bad = (last_match + 1) as usize;
            break;
        }
    }
    if bad == passes && last_match + 1 < passes as isize && passes == c.passes(off) {
        bad = (last_match + 1) as usize;
    }
    bad
}

/// The worst of the three modes: a repair has to hold whichever way the original's timing falls.
fn eval(refk: &[Key], off: usize, pk: usize, g0: &Game, c: &Cand, limit: usize) -> usize {
    (0..3).map(|m| eval1(refk, off, pk, g0, c, limit, m)).min().unwrap()
}

#[test]
fn route() {
    let name = std::env::var("CASE").unwrap();
    let iters: usize = std::env::var("ITERS").map(|s| s.parse().unwrap()).unwrap_or(6000);
    let rounds: usize = std::env::var("ROUNDS").map(|s| s.parse().unwrap()).unwrap_or(40);
    let seed: u64 = std::env::var("SEED").map(|s| s.parse().unwrap()).unwrap_or(1);
    let adv: usize = std::env::var("ADV").map(|s| s.parse().unwrap()).unwrap_or(260);
    let out_path = std::env::var("OUT").unwrap();
    let data = game_data().unwrap();
    let mut case = Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap();
    case.compare.picture = false;
    case.input = None;
    assert_eq!(case.sync_frame, 6301);
    let read = |n: &str| {
        let path = conform_dir().join("scripts").join(format!("{n}.txt"));
        script::parse(&std::fs::read_to_string(&path).unwrap()).unwrap()
    };
    // The frozen path.
    let refspec = std::env::var("REF").unwrap();
    let (refname, refn) = refspec.split_once(':').unwrap();
    let refn: usize = refn.parse().unwrap();
    let rsc = read(refname);
    let mut fz = Game::new(data.clone(), Start::Level1);
    fz.freeze_as_a_case();
    let mut refk = Vec::new();
    let mut refin = Vec::new();
    for f in 0..refn as u32 {
        refin.push(rsc.input_at(f));
        fz.step(rsc.input_at(f));
        let _ = fz.take_events();
        refk.push(key(&fz));
        if f < 8 || (74..80).contains(&f) { println!("ref step {f} irq {} joy {:02x} key {:x?}", fz.state().irq_count, rsc.input_at(f).joy.0, key(&fz)); }
    }
    println!("frozen end {:x?}", refk.last().unwrap());
    let mut c = Cand { inp: refin.clone(), map: (0..refn).collect(), lag: vec![0; refn], cut: vec![None; refn] };
    if let Ok(p) = std::env::var("START") {
        let text = std::fs::read_to_string(&p).unwrap();
        let sc = script::parse(&text).unwrap();
        let frames: usize =
            text.lines().find_map(|l| l.strip_prefix("# frames ")).unwrap().parse().unwrap();
        c.inp = (0..frames as u32).map(|f| sc.input_at(f)).collect();
        c.map = text
            .lines()
            .find_map(|l| l.strip_prefix("# map "))
            .unwrap()
            .split(' ')
            .map(|v| v.parse().unwrap())
            .collect();
        assert_eq!(c.map.len(), frames);
        c.lag = vec![0; frames];
        c.cut = vec![None; frames];
    }
    let mut rng = Rng(0x9E3779B97F4A7C15 ^ seed.wrapping_mul(0x100000001B3));
    let write = |c: &Cand, tag: &str| {
        let body = script::InputScript::from_steps(&c.inp).to_text();
        let map: Vec<String> = c.map.iter().map(|m| m.to_string()).collect();
        std::fs::write(
            format!("{out_path}{tag}"),
            format!("# frames {}\n# map {}\n{body}", c.inp.len(), map.join(" ")),
        )
        .unwrap();
    };
    let mut off: Option<usize> = std::env::var("OFF").ok().map(|s| s.parse().unwrap());
    for round in 0..rounds {
        // On the original.
        case.frames = c.inp.len() as u32;
        if let Some(Goal::All { by_frame, .. }) = &mut case.goal {
            *by_frame = case.sync_frame + case.frames;
        }
        let inputs = Inputs::with_after(&case, script::InputScript::from_steps(&c.inp)).unwrap();
        if off.is_none() {
            // Which script step a pass reads: a first run only to see.
            let mut rec = Rec {
                game: Game::new(data.clone(), Start::Level1),
                c: &c,
                refk: &refk,
                off: 0,
                inputs: Vec::new(),
                cuts: Vec::new(),
                pending: None,
                last_match: -1,
                bad: None,
                ring: VecDeque::new(),
                trace: Vec::new(),
            };
            let mut short = case.clone();
            short.frames = short.frames.min(400);
            let _ = run_inputs(&short, &inputs, &mut rec, &Options::default()).unwrap();
            let n = rec.inputs.len();
            let o = (0..3)
                .min_by_key(|&o| (0..n).filter(|&k| c.inp[k + o] != rec.inputs[k]).count())
                .unwrap();
            println!("a pass reads the script's step {o} later");
            off = Some(o);
        }
        let off = off.unwrap();
        let mut rec = Rec {
            game: Game::new(data.clone(), Start::Level1),
            c: &c,
            refk: &refk,
            off,
            inputs: Vec::new(),
            cuts: Vec::new(),
            pending: None,
            last_match: -1,
            bad: None,
            ring: VecDeque::new(),
            trace: Vec::new(),
        };
        let v = run_inputs(&case, &inputs, &mut rec, &Options::default()).unwrap();
        let head: String = format!("{v}").lines().take(1).collect();
        let n = rec.inputs.len();
        if head.contains("differs") {
            write(&c, &format!(".differs{round}"));
        }
        let passes = c.passes(off);
        let ob = rec.bad.unwrap_or_else(|| {
            if rec.last_match + 1 >= n as isize { passes } else { (rec.last_match + 1) as usize }
        });
        println!(
            "round {round}: {} steps, {n} passes on the original: {}; goal {:?}; leaves the path at pass {ob} (frozen step {})",
            c.inp.len(),
            &head[..head.len().min(160)],
            v.goal_met,
            c.map[(ob + off).min(c.map.len() - 1)],
        );
        if ob >= passes.min(n) {
            write(&c, "");
            println!("DONE: {} steps; {v}", c.inp.len());
            return;
        }
        if std::env::var("TRACE").is_ok() {
            for l in rec.trace.iter().skip(std::env::var("TFROM").map(|s| s.parse().unwrap()).unwrap_or(ob.saturating_sub(30))).take(60) {
                println!("    {l}");
            }
        }
        // The latest state the engine alone leaves the path from as the original did.
        let ring = std::mem::take(&mut rec.ring);
        let (rin, rcut) = (std::mem::take(&mut rec.inputs), std::mem::take(&mut rec.cuts));
        drop(rec);
        let total = c.inp.len();
        c.lag = vec![0; total];
        c.cut = vec![None; total];
        let mut lagged = 0;
        for k in 0..n.min(total) {
            let at = |d: isize| c.inp.get((k as isize + d) as usize).copied();
            c.lag[k] = [0isize, 1, -1, 2, -2]
                .into_iter()
                .find(|&d| k as isize + d >= 0 && at(d) == Some(rin[k]))
                .unwrap_or(0) as i8;
            lagged += (c.lag[k] != 0) as usize;
            c.cut[k] = rcut[k];
        }
        println!("  {lagged} passes read another step's input, {} cuts handed", rcut.iter().flatten().count());
        let mut chosen = None;
        for back in [150usize, 250, 400, 80, 40, 15] {
            let pk = ob.saturating_sub(back).max(ring.front().unwrap().0);
            let g0 = ring.iter().find(|(k, _)| *k == pk).unwrap().1.clone();
            let mb = eval1(&refk, off, pk, &g0, &c, usize::MAX, 0);
            println!("  from pass {pk} the engine alone leaves the path at pass {mb}");
            if mb <= ob + 20 {
                chosen = Some((pk, g0, mb));
                break;
            }
        }
        let Some((pk, g0, _)) = chosen else {
            println!("  the engine alone does not leave the path where the original does: stopped");
            write(&c, ".wip");
            return;
        };
        let limit = ob + adv;
        let mut bad = eval(&refk, off, pk, &g0, &c, limit);
        let score = |bad: usize, c: &Cand| -> (usize, isize) {
            (c.map[(bad + off).min(c.map.len() - 1)], -(c.map.len() as isize))
        };
        let mut best = score(bad, &c);
        let goal_ref = |c: &Cand| c.map[(limit + off).min(c.map.len() - 1)];
        let mut it = 0;
        while bad < c.passes(off).min(limit) && it < iters {
            it += 1;
            let mut d = c.clone();
            let mut lo = usize::MAX;
            let muts = 1 + (rng.below(10) < 3) as usize;
            for _ in 0..muts {
                let wide = if rng.below(4) == 0 { 300 } else { 120 };
                let span = 30 + rng.below(wide);
                let t = (bad + off + 12)
                    .saturating_sub(rng.below(span))
                    .max(pk + off + 2)
                    .max(free())
                    .min(d.inp.len() - 1);
                lo = lo.min(t - off);
                match rng.below(12) {
                    0..=4 => {
                        let n = 1 + rng.below(24);
                        for v in d.inp.iter_mut().skip(t).take(n) {
                            v.joy.0 |= 1;
                        }
                    }
                    5..=6 => {
                        let n = 1 + rng.below(24);
                        for v in d.inp.iter_mut().skip(t).take(n) {
                            v.joy.0 &= !1;
                        }
                    }
                    7 => {
                        // An input change a step or two earlier or later.
                        if let Some(e) = (t..d.inp.len().min(t + 40)).find(|&e| d.inp[e] != d.inp[e - 1]) {
                            let n = 1 + rng.below(3);
                            if rng.below(2) == 0 {
                                let v = d.inp[e];
                                for q in e.saturating_sub(n)..e {
                                    d.inp[q] = v;
                                }
                            } else {
                                let v = d.inp[e - 1];
                                for q in e..(e + n).min(d.inp.len()) {
                                    d.inp[q] = v;
                                }
                            }
                        }
                    }
                    _ => {
                        let w = if rng.below(5) == 0 { 2 + rng.below(260) } else { 2 + rng.below(70) };
                        let r = d.map[t];
                        let held = if rng.below(2) == 0 { FrameInput::NONE } else { d.inp[t] };
                        for _ in 0..w {
                            d.insert(t, held, r);
                        }
                    }
                }
            }
            // Inserted steps move the horizon with them.
            let lim = limit + d.inp.len().saturating_sub(c.inp.len());
            let _ = lo;
            let b = eval(&refk, off, pk, &g0, &d, lim);
            let done = b >= d.passes(off).min(lim);
            let sc = if done { (goal_ref(&d).max(score(b, &d).0) + 1, -(d.map.len() as isize)) } else { score(b, &d) };
            if sc > best {
                c = d;
                bad = b;
                best = sc;
                if done {
                    bad = usize::MAX;
                }
            }
        }
        println!(
            "  search: {it} tries; the engine alone now {} (frozen step {} of {refn}); {} steps",
            if bad == usize::MAX { "keeps the path to the horizon".to_string() } else { format!("leaves it at pass {bad}") },
            best.0,
            c.inp.len()
        );
        write(&c, ".wip");
    }
    println!("rounds ran out");
}
