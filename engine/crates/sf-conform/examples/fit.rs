//! Scratch (T36): fit the level's beam costs (`sf_core::level::beam::COSTS`) to the original's
//! write positions and routine landmarks. `fit <case.toml>...`
#[path = "common/mod.rs"]
mod common;
use common::*;
use sf_conform::{Case, sync};
use sf_core::level::beam::{COSTS, Clock, Cost, Tick};
use sf_core::{Game, Start};

/// A frame: its clock steps, and the original's times (nominal cycles) for its writes (by index)
/// and its landmarks (by the cost they mark).
struct Frame { trace: Vec<Tick>, writes: Vec<(u32, f64)>, labels: Vec<String>, marks: Vec<(Cost, f64)> }

/// Residuals and Jacobian rows for `th`.
fn eval(frames: &[Frame], th: &[f64], with_marks: bool) -> (Vec<f64>, Vec<Vec<f64>>, Vec<bool>) {
    let np = th.len();
    let (mut r, mut jac, mut is_mark) = (Vec::new(), Vec::new(), Vec::new());
    for f in frames {
        let mut c = Clock::START;
        let mut d = vec![0.0; np];
        let mut writes: Vec<(u32, f64, Vec<f64>)> = Vec::new();
        let mut marks: Vec<(Cost, f64, Vec<f64>)> = Vec::new();
        let now = |c: &Clock| c.line as f64 * 454.0 + c.into as f64;
        for t in &f.trace {
            match *t {
                Tick::Cpu(k, n) => {
                    if !marks.iter().any(|m| m.0 == k) { marks.push((k, now(&c), d.clone())); }
                    let fetching = (0x27..0x12c).contains(&(c.line % 313));
                    let rate = if fetching { 454.0 / 414.0 } else { 1.0 };
                    d[k as usize] += n as f64 * rate;
                    c.cpu((n as f64 * th[k as usize]).max(0.0).round() as u32);
                }
                Tick::Blit(s) => c.blit(s),
                Tick::Wait(n) => { c.line = c.line.saturating_add(n); c.into = 0; d.iter_mut().for_each(|x| *x = 0.0); }
                Tick::Write(i) => writes.push((i, now(&c), d.clone())),
            }
        }
        for &(i, t) in &f.writes {
            if let Some((_, e, dd)) = writes.iter().find(|x| x.0 == i) { r.push(t - e); jac.push(dd.clone()); is_mark.push(false); }
        }
        if with_marks {
            for &(k, t) in &f.marks {
                if let Some((_, e, dd)) = marks.iter().find(|x| x.0 == k) { r.push(t - e); jac.push(dd.clone()); is_mark.push(true); }
            }
        }
    }
    (r, jac, is_mark)
}

fn stats(r: &[f64]) -> String {
    let mut a: Vec<f64> = r.iter().map(|x| x / 454.0).collect();
    if a.is_empty() { return "none".into(); }
    a.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let n = a.len();
    let w = |k: f64| a.iter().filter(|e| e.abs() <= k).count() * 100 / n;
    format!("{n}: median {:.1} p5 {:.1} p95 {:.1} lines; |e|<=1 {}% <=2 {}% <=5 {}%", a[n / 2], a[n / 20], a[n * 19 / 20], w(1.0), w(2.0), w(5.0))
}

fn report(frames: &[Frame], th: &[f64], what: &str) {
    let (r, _, m) = eval(frames, th, true);
    let wr: Vec<f64> = r.iter().zip(&m).filter(|x| !x.1).map(|x| *x.0).collect();
    let mr: Vec<f64> = r.iter().zip(&m).filter(|x| *x.1).map(|x| *x.0).collect();
    println!("{what}: writes {} | marks {}", stats(&wr), stats(&mr));
}

fn main() {
    let syms = syms();
    let mut frames = Vec::new();
    for path in std::env::args().skip(1) {
        let case = Case::load(std::path::Path::new(&path)).unwrap();
        let frozen = case.freeze();
        let synced = sync::reach_frozen(&case.sync, &case.script, case.frames + 10, frozen).unwrap();
        let mut a = synced.amiga;
        a.set_profile(true);
        let display = sf_conform::display_data();
        let rom = display.rom().h0.clone();
        let new = |m| { let mut g = Game::with_display(display.clone(), Start::Transplant(m)); if frozen == sf_conform::freeze::Freeze::Objects { g.frog_only(); } g.keep_trace(); g };
        let mut game = new(synced.memory);
        let mut after_mid = false;
        for f in 0..case.frames {
            frozen.apply(&mut a, &display);
            let input = case.script.input_at(f);
            let theirs = a.step(input).unwrap();
            let mem = sync::core_memory(a.memory());
            let pacing = sync::pacing(&a.cpu(), &mem, &theirs.paula);
            let ours = game.step_paced(input, pacing);
            let mid = sync::mid_pass(&a.cpu(), &mem);
            let same = theirs.paula.len() == ours.paula.len() && theirs.paula.iter().zip(&ours.paula).all(|(x, y)| x.reg_value() == y.reg_value());
            if same && !mid && !after_mid {
                let om = original_marks(&syms, &rom, &a.line_samples());
                frames.push(Frame {
                    trace: game.trace().to_vec(),
                    writes: theirs.paula.iter().enumerate().filter(|(_, w)| w.reg < 0xa0)
                        .map(|(i, w)| (i as u32, w.vpos as f64 * 454.0 + w.hpos as f64 * 2.0)).collect(),
                    labels: theirs.paula.iter().filter(|w| w.reg < 0xa0).map(|w| format!("{:02x}={:04x}", w.reg, w.value)).collect(),
                    marks: MARKS.iter().zip(&om).filter_map(|(m, o)| o.map(|l| (m.2, l as f64 * 454.0 + 227.0))).collect(),
                });
            }
            after_mid = mid;
            if !mid && sf_conform::compare::vars(&game.probe(), &mem, &[]).is_some() {
                game = new(mem.clone());
            }
        }
        eprintln!("{}: {} frames so far", case.name, frames.len());
    }
    let mut th: Vec<f64> = COSTS.iter().map(|&c| c as f64).collect();
    report(&frames, &th, "before");
    let np = th.len();
    let weights = |r: &[f64]| -> Vec<f64> { r.iter().map(|r| (2.0 * 454.0 / r.abs().max(1.0)).min(1.0)).collect() };
    let cost = |r: &[f64], w: &[f64]| -> f64 { r.iter().zip(w).map(|(r, w)| r * r * w).sum() };
    let mut lambda = 1.0;
    for it in 0..60 {
        let (r0, jac, _) = eval(&frames, &th, true);
        let w = weights(&r0);
        let c0 = cost(&r0, &w);
        let mut m = vec![vec![0.0; np]; np];
        let mut v = vec![0.0; np];
        for ((row, r), w) in jac.iter().zip(&r0).zip(&w) {
            for i in 0..np {
                if row[i] == 0.0 { continue; }
                v[i] += row[i] * r * w;
                for j in 0..np { m[i][j] += row[i] * row[j] * w; }
            }
        }
        let mut accepted = false;
        for _ in 0..10 {
            let mut a = m.clone();
            let mut b = v.clone();
            for i in 0..np { a[i][i] = a[i][i] * (1.0 + lambda) + 1e-3; }
            for c in 0..np {
                let piv = (c..np).max_by(|&x, &y| a[x][c].abs().partial_cmp(&a[y][c].abs()).unwrap()).unwrap();
                a.swap(c, piv); b.swap(c, piv);
                let d = a[c][c];
                if d.abs() < 1e-12 { continue; }
                for r in 0..np {
                    if r != c {
                        let f = a[r][c] / d;
                        if f != 0.0 { for k in c..np { a[r][k] -= f * a[c][k]; } b[r] -= f * b[c]; }
                    }
                }
            }
            let cand: Vec<f64> = (0..np).map(|k| if a[k][k].abs() > 1e-12 { (th[k] + b[k] / a[k][k]).max(0.0) } else { th[k] }).collect();
            let (r1, _, _) = eval(&frames, &cand, true);
            if cost(&r1, &w) < c0 {
                th = cand;
                lambda = (lambda / 3.0).max(1e-4);
                accepted = true;
                break;
            }
            lambda *= 4.0;
        }
        if it % 5 == 4 || !accepted { report(&frames, &th, &format!("iteration {it} (lambda {lambda:.4})")); }
        if !accepted { break; }
    }
    // Segment errors: consecutive measured points (writes and marks, in the original's time
    // order), the change in residual from one to the next, by (from, to).
    let mut seg: std::collections::BTreeMap<String, Vec<f64>> = Default::default();
    for f in &frames {
        let one = std::slice::from_ref(f);
        let (r, _, is_mark) = eval(one, &th, true);
        // the labels and the original times, in eval's order
        let mut pts: Vec<(f64, String, f64)> = Vec::new();
        let mut k = 0;
        let mut regs = Vec::new();
        for &(i, t) in &f.writes {
            let _ = i;
            regs.push(t);
        }
        let (wr, _, _) = eval(one, &th, false);
        for (n, &(i, t)) in f.writes.iter().enumerate() {
            let _ = i;
            if n < wr.len() { pts.push((t, f.labels[n].clone(), wr[n])); }
        }
        for (n, &(c, t)) in f.marks.iter().enumerate() {
            let idx = wr.len() + n;
            if idx < r.len() && is_mark[idx] { pts.push((t, format!("{c:?}"), r[idx])); }
        }
        let _ = (&mut k, regs);
        pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        for w in pts.windows(2) {
            seg.entry(format!("{} -> {}", w[0].1, w[1].1)).or_default().push((w[1].2 - w[0].2) / 454.0);
        }
    }
    let mut rows: Vec<(f64, String)> = seg.iter().map(|(k, v)| {
        let n = v.len() as f64;
        let mean = v.iter().sum::<f64>() / n;
        let sd = (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n).sqrt();
        (sd * n.sqrt(), format!("{k}: n {} mean {:+.2} sd {:.2}", v.len(), mean, sd))
    }).collect();
    rows.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    for r in rows.iter().take(25) { println!("  {}", r.1); }
    println!("pub const COSTS: [u32; Cost::COUNT] = [");
    for (k, c) in Cost::ALL.iter().enumerate() {
        println!("    {}, // {:?}", th[k].round() as u32, c);
    }
    println!("];");
}

/// A write's label: its register (from the trace we only know the index; good enough).
fn i_label(_trace: &[Tick], i: u32) -> String {
    format!("#{i}")
}
