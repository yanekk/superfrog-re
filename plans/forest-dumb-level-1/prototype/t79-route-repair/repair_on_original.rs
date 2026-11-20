// SCRATCH: never committed. zz_repair with the original in the loop: each round plays the
// candidate on the original, learns which step's input each pass read and where passes cut the
// redraws, and searches on with those.
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

struct Rec {
    game: Game,
    inputs: Vec<FrameInput>,
    cuts: Vec<Option<EdgesCut>>,
    pending: Option<EdgesCut>,
    first_irq: Option<u16>,
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
        self.inputs.push(input);
        self.cuts.push(self.pending.take());
        Engine::pass(&mut self.game, input);
        if self.first_irq.is_none() {
            self.first_irq = Some(self.game.state().irq_count);
        }
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

const T: usize = 90;
const SNAP: usize = 32;

#[derive(Clone)]
struct Cand {
    inp: Vec<FrameInput>,       // by script step
    map: Vec<usize>,            // by script step: the frozen path's step
    lag: Vec<i8>,               // by pass
    cut: Vec<Option<EdgesCut>>, // by pass
}

struct Ctx {
    start: Game,
    off: usize,
    refk: Vec<Key>,
}

/// First bad pass (passes if none); fills `out` with snapshots when given.
fn eval(x: &Ctx, snaps: &[Game], from: usize, c: &Cand, out: Option<&mut Vec<Game>>) -> usize {
    let passes = c.inp.len() - x.off;
    let s0 = (from / SNAP).min(snaps.len().saturating_sub(1));
    let (mut g, mut k) = if snaps.is_empty() {
        (x.start.clone(), 0)
    } else {
        (snaps[s0].clone(), s0 * SNAP)
    };
    let mut new_snaps: Vec<Game> = snaps[..s0.min(snaps.len())].to_vec();
    let keep = out.is_some();
    let mut last_match = k as isize - 1;
    let mut bad = passes;
    while k < passes {
        if keep && k % SNAP == 0 {
            new_snaps.push(g.clone());
        }
        let i = k + x.off;
        let j = (i as isize + c.lag[k] as isize).clamp(0, c.inp.len() as isize - 1) as usize;
        Engine::foreground_cut(&mut g, c.cut[k]);
        Engine::pass(&mut g, c.inp[j]);
        let _ = g.take_events();
        if key(&g) == x.refk[c.map[i]] {
            last_match = k as isize;
        } else if (k as isize - last_match) as usize > T {
            bad = (last_match + 1) as usize;
            if !keep {
                break;
            }
        }
        k += 1;
    }
    if bad == passes && last_match != passes as isize - 1 {
        bad = (last_match + 1) as usize;
    }
    if let Some(o) = out {
        *o = new_snaps;
    }
    bad
}

#[test]
fn repair2() {
    let name = std::env::var("CASE").unwrap();
    let iters: usize = std::env::var("ITERS").map(|s| s.parse().unwrap()).unwrap_or(30000);
    let rounds: usize = std::env::var("ROUNDS").map(|s| s.parse().unwrap()).unwrap_or(30);
    let seed: u64 = std::env::var("SEED").map(|s| s.parse().unwrap()).unwrap_or(1);
    let out_path = std::env::var("OUT").unwrap();
    let data = game_data().unwrap();
    let mut case = Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap();
    case.compare.picture = false;
    let read = |n: &str| {
        let path = conform_dir().join("scripts").join(format!("{n}.txt"));
        script::parse(&std::fs::read_to_string(&path).unwrap()).unwrap()
    };
    // The frozen path: REF names the script made with creatures frozen and its frame count.
    let refspec = std::env::var("REF").unwrap();
    let (refname, refn) = refspec.split_once(':').unwrap();
    let refn: usize = refn.parse().unwrap();
    let rsc = read(refname);
    let lead = (case.sync_frame - 6301) as usize;
    let mut fz = Game::new(data.clone(), Start::Level1);
    fz.freeze_as_a_case();
    for _ in 0..lead {
        fz.step(FrameInput::NONE);
    }
    let mut refk = Vec::new();
    let mut refin = Vec::new();
    for f in 0..refn as u32 {
        refin.push(rsc.input_at(f));
        fz.step(rsc.input_at(f));
        let _ = fz.take_events();
        refk.push(key(&fz));
    }
    println!("frozen end {:x?}", refk.last().unwrap());
    // Start from START (a script file path, with a `# map` line if this tool wrote it) or the frozen script.
    let mut c = Cand { inp: refin.clone(), map: (0..refn).collect(), lag: vec![0; refn], cut: vec![None; refn] };
    if let Ok(p) = std::env::var("START") {
        let text = std::fs::read_to_string(&p).unwrap();
        let sc = script::parse(&text).unwrap();
        let frames: usize = text.lines().find_map(|l| l.strip_prefix("# frames ")).unwrap().parse().unwrap();
        c.inp = (0..frames as u32).map(|f| sc.input_at(f)).collect();
        c.map = text.lines().find_map(|l| l.strip_prefix("# map ")).unwrap().split(' ').map(|v| v.parse().unwrap()).collect();
        assert_eq!(c.map.len(), frames);
        c.lag = vec![0; frames];
        c.cut = vec![None; frames];
    }
    let mut start = Game::new(data.clone(), Start::Level1);
    let mut x = Ctx { start: start.clone(), off: 0, refk };
    let mut rng = Rng(0x9E3779B97F4A7C15 ^ seed.wrapping_mul(0x100000001B3));
    let write = |c: &Cand, tag: &str| {
        let body = script::InputScript::from_steps(&c.inp).to_text();
        let map: Vec<String> = c.map.iter().map(|m| m.to_string()).collect();
        std::fs::write(format!("{out_path}{tag}"), format!("# frames {}\n# map {}\n{body}", c.inp.len(), map.join(" "))).unwrap();
    };
    for round in 0..rounds {
        // On the original.
        case.frames = c.inp.len() as u32;
        if let Some(Goal::All { by_frame, .. }) = &mut case.goal {
            *by_frame = case.sync_frame + case.frames;
        }
        let mut rec = Rec { game: Game::new(data.clone(), Start::Level1), inputs: Vec::new(), cuts: Vec::new(), pending: None, first_irq: None };
        let inputs = Inputs::with_after(&case, script::InputScript::from_steps(&c.inp)).unwrap();
        let v = run_inputs(&case, &inputs, &mut rec, &Options::default()).unwrap();
        let head: String = format!("{v}").lines().take(1).collect();
        println!("round {round}: {} frames; original: {head}; goal {:?}", c.inp.len(), v.goal_met);
        if round == 0 {
            let lead_passes = rec.first_irq.unwrap() as usize - 2;
            for _ in 0..lead_passes {
                start.step(FrameInput::NONE);
            }
            let _ = start.take_events();
            x.start = start.clone();
        }
        // Which script step each pass read.
        let n = rec.inputs.len();
        if round == 0 {
            x.off = (0..3).min_by_key(|&o| (0..n).filter(|&k| k + o < c.inp.len() && c.inp[k + o] != rec.inputs[k]).count()).unwrap();
        }
        let passes = c.inp.len() - x.off;
        c.lag = vec![0; passes];
        c.cut = vec![None; passes];
        let mut unexplained = 0;
        for k in 0..n.min(passes) {
            let i = (k + x.off) as isize;
            let at = |d: isize| c.inp.get((i + d) as usize).copied();
            c.lag[k] = [0isize, 1, -1, 2, -2].into_iter().find(|&d| i + d >= 0 && at(d) == Some(rec.inputs[k])).unwrap_or_else(|| { unexplained += 1; 0 }) as i8;
            c.cut[k] = rec.cuts[k];
        }
        let mut snaps = Vec::new();
        let mut bad = eval(&x, &[], 0, &c, Some(&mut snaps));
        println!("  off {}, {} passes on the original, {} lagged, {unexplained} unexplained; first bad pass {bad} of {passes}", x.off, n, c.lag.iter().filter(|&&l| l != 0).count());
        if bad >= passes && v.passed() {
            write(&c, "");
            println!("DONE: {} frames", c.inp.len());
            return;
        }
        if bad >= passes {
            println!("  model says done but the original does not: {v}");
            write(&c, ".model");
            return;
        }
        let score = |bad: usize, c: &Cand| -> (usize, isize) {
            (if bad + x.off >= c.map.len() { refn } else { c.map[bad + x.off] }, -(c.map.len() as isize))
        };
        let mut best = score(bad, &c);
        let mut it = 0;
        while bad < c.inp.len() - x.off && it < iters {
            it += 1;
            let mut d = c.clone();
            let mut lo = usize::MAX;
            let muts = 1 + (rng.below(10) < 3) as usize;
            for _ in 0..muts {
                let span = if rng.below(5) == 0 { 30 + rng.below(700) } else { 30 + rng.below(170) };
                let t = (bad + x.off + 12).saturating_sub(rng.below(span)).max(x.off + 1).min(d.inp.len() - 1);
                lo = lo.min(t - x.off);
                match rng.below(13) {
                    // An input change a step or a few earlier or later.
                    10 | 11 => {
                        let n = 1 + rng.below(3);
                        if let Some(e) = (t..d.inp.len().min(t + 40)).find(|&e| e > 0 && d.inp[e] != d.inp[e - 1]) {
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
                    12 => {
                        // A step taken out where the input holds.
                        if t + 1 < d.inp.len() && d.inp.len() > x.off + 8 {
                            d.inp.remove(t);
                            d.map.remove(t);
                            d.lag.remove(t - x.off);
                            d.cut.remove(t - x.off);
                        }
                    }
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
                    _ => {
                        let w = if rng.below(5) == 0 { 2 + rng.below(260) } else { 2 + rng.below(70) };
                        let r = d.map[t];
                        for _ in 0..w {
                            d.inp.insert(t, FrameInput::NONE);
                            d.map.insert(t, r);
                            d.lag.insert(t - x.off, 0);
                            d.cut.insert(t - x.off, None);
                        }
                    }
                }
            }
            let lo = lo.saturating_sub(3);
            let b = eval(&x, &snaps, lo, &d, None);
            if score(b, &d) > best {
                c = d;
                let old = std::mem::take(&mut snaps);
                bad = eval(&x, &old, lo, &c, Some(&mut snaps));
                best = score(bad, &c);
            }
        }
        println!("  search: {it} tries; first bad pass {bad} of {} (frozen step {} of {refn})", c.inp.len() - x.off, best.0);
        write(&c, ".wip");
    }
    println!("rounds ran out");
}
