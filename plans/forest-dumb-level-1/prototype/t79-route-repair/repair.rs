// SCRATCH: never committed. Repairs a script made with creatures frozen so that, live, the
// character still follows the frozen path: jumps and waits added by random search on the engine.
use std::path::Path;

use amiga_conform::script;
use amiga_oracle::FrameInput;
use fd_conform::{Case, conform_dir, game_data};
use fd_core::{Game, Start};

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

const T: usize = 90;
const SNAP: usize = 32;

/// Runs `inp` live from `snaps` (a snapshot before `from`), returns the first bad step (len if none)
/// and the keys.
fn eval(
    start: &Game,
    snaps: &[Game],
    from: usize,
    inp: &[FrameInput],
    map: &[usize],
    refk: &[Key],
    out_snaps: Option<&mut Vec<Game>>,
) -> usize {
    let s0 = (from / SNAP).min(snaps.len().saturating_sub(1));
    let (mut g, mut i) = if snaps.is_empty() {
        (start.clone(), 0)
    } else {
        (snaps[s0].clone(), s0 * SNAP)
    };
    let mut new_snaps: Vec<Game> = snaps[..s0.min(snaps.len())].to_vec();
    let keep = out_snaps.is_some();
    let mut last_match = i as isize - 1;
    // Assume everything before the snapshot matched (it is the accepted prefix).
    let mut bad = inp.len();
    while i < inp.len() {
        if keep && i % SNAP == 0 {
            new_snaps.push(g.clone());
        }
        g.step(inp[i]);
        let _ = g.take_events();
        if key(&g) == refk[map[i]] {
            last_match = i as isize;
        } else if (i as isize - last_match) as usize > T {
            bad = (last_match + 1) as usize;
            if !keep {
                break;
            }
        }
        i += 1;
    }
    if bad == inp.len() && last_match != inp.len() as isize - 1 {
        bad = (last_match + 1) as usize;
    }
    if let Some(o) = out_snaps {
        *o = new_snaps;
    }
    bad
}

#[test]
fn repair() {
    let name = std::env::var("CASE").unwrap();
    let iters: usize = std::env::var("ITERS").map(|s| s.parse().unwrap()).unwrap_or(20000);
    let seed: u64 = std::env::var("SEED").map(|s| s.parse().unwrap()).unwrap_or(1);
    let data = game_data().unwrap();
    let case = Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap();
    let sname = std::env::var("SCRIPT").unwrap_or(case.input.clone().unwrap());
    let path = conform_dir().join("scripts").join(format!("{sname}.txt"));
    let sc = script::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let lead = (case.sync_frame - 6301) as usize;
    let n = case.frames as usize;
    let mut inp: Vec<FrameInput> = vec![FrameInput::NONE; lead];
    inp.extend((0..n as u32).map(|f| sc.input_at(f)));
    // The frozen path.
    let mut fz = Game::new(data.clone(), Start::Level1);
    fz.freeze_as_a_case();
    let mut refk = Vec::new();
    for &i in &inp {
        fz.step(i);
        let _ = fz.take_events();
        refk.push(key(&fz));
    }
    println!("frozen end {:x?}", refk.last().unwrap());
    let start = Game::new(data.clone(), Start::Level1);
    let refn = inp.len();
    let mut map: Vec<usize> = (0..inp.len()).collect();
    let mut snaps = Vec::new();
    let score = |bad: usize, m: &[usize]| -> (usize, isize) {
        (if bad >= m.len() { refn } else { m[bad] }, -(m.len() as isize))
    };
    let mut bad = eval(&start, &[], 0, &inp, &map, &refk, Some(&mut snaps));
    let mut best = score(bad, &map);
    println!("start: first bad {bad} of {}", inp.len());
    let mut rng = Rng(0x9E3779B97F4A7C15 ^ seed.wrapping_mul(0x100000001B3));
    let mut it = 0;
    while bad < inp.len() && it < iters {
        it += 1;
        let mut c = inp.clone();
        let mut m = map.clone();
        let mut lo = usize::MAX;
        let muts = 1 + (rng.below(10) < 3) as usize;
        for _ in 0..muts {
            let span = if rng.below(5) == 0 { 30 + rng.below(700) } else { 30 + rng.below(170) };
            let t = (bad + 12).saturating_sub(rng.below(span)).max(lead).min(c.len() - 1);
            lo = lo.min(t);
            match rng.below(10) {
                0..=4 => {
                    let d = 1 + rng.below(24);
                    for x in c.iter_mut().skip(t).take(d) {
                        x.joy.0 |= 1;
                    }
                }
                5..=6 => {
                    let d = 1 + rng.below(24);
                    for x in c.iter_mut().skip(t).take(d) {
                        x.joy.0 &= !1;
                    }
                }
                _ => {
                    let k = if rng.below(5) == 0 { 2 + rng.below(260) } else { 2 + rng.below(70) };
                    let r = m[t];
                    let held = if rng.below(2) == 0 { FrameInput::NONE } else { c[t] };
                    for _ in 0..k {
                        c.insert(t, held);
                        m.insert(t, r);
                    }
                }
            }
        }
        let b = eval(&start, &snaps, lo, &c, &m, &refk, None);
        if score(b, &m) > best {
            inp = c;
            map = m;
            let old = std::mem::take(&mut snaps);
            bad = eval(&start, &old, lo, &inp, &map, &refk, Some(&mut snaps));
            best = score(bad, &map);
            println!("iter {it}: first bad {bad} of {} (ref {} of {refn})", inp.len(), best.0);
        }
    }
    println!("done after {it}: first bad {bad} of {}", inp.len());
    if std::env::var("DUMP").is_ok() {
        let mut g = start.clone();
        for i in 0..inp.len().min(bad + 100) {
            g.step(inp[i]);
            let _ = g.take_events();
            if i + 70 >= bad {
                let s = g.state();
                let c = &s.edges.creatures;
                let cr: Vec<String> = (0..5).filter(|&k| c.active[k] != 0).map(|k| format!("k{}@{:x},{:x}", c.slots[k].kind, c.slots[k].x, c.slots[k].y)).collect();
                println!("{i} ref {} joy {:02x} live {:x?} ref {:x?} hits {} dying {} {:?}", map[i], inp[i].joy.0, key(&g), refk[map[i]], s.hits_taken, s.motion.dying, cr);
            }
        }
    }
    let body = script::InputScript::from_steps(&inp[lead..]).to_text();
    let out = std::env::var("OUT").unwrap();
    std::fs::write(&out, format!("# frames {}\n{body}", inp.len() - lead)).unwrap();
}
