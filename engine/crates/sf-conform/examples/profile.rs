//! Where the original's CPU is at each raster line's end, frame by frame through a case, by
//! routine: what the level's beam costs (`sf_core::level::beam`) are fitted against.
//!
//! `cargo run --example profile -- <case.toml> <from> <to> [deep|top|sum]`
//!
//! Each frame prints its pacing and runs of lines in one routine: `I` in an interrupt, `M` in the
//! main loop, with its callers (`deep`: four of them, with offsets), or only the vertical blank's
//! or the pass's direct callee (`top`); `sum` prints, per such callee, lines a frame and the first
//! line it is seen on, averaged over the frames.
#[path = "common/mod.rs"]
mod common;
use common::{func, is_return, name, syms};
use sf_conform::{Case, sync};
use sf_core::layout::h0;

/// The vertical blank's handler, the level's frame loop and its pass.
const VBL: u32 = 0x11bf8;
const FRAME_LOOP: u32 = 0xaa6;
const PASS: u32 = 0xab8;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [case, from, to, ..] = &args[..] else {
        eprintln!("usage: profile <case.toml> <from> <to> [deep|top|sum]");
        std::process::exit(2);
    };
    let case = Case::load(std::path::Path::new(case)).expect("case");
    let from: u32 = from.parse().expect("from");
    let to: u32 = to.parse().expect("to");
    let mode = args.get(3).map(String::as_str).unwrap_or("");
    let syms = syms();
    let frozen = case.freeze();
    let synced = sync::reach_frozen(&case.sync, &case.script, to + 10, frozen).expect("sync");
    let mut a = synced.amiga;
    let display = sf_conform::display_data();
    let rom = display.rom().h0.clone();
    let mut sum: std::collections::BTreeMap<String, (u32, u32, u32)> = Default::default();
    for f in 0..to {
        if f == from {
            a.set_profile(true);
        }
        frozen.apply(&mut a, &display);
        let theirs = a.step(case.script.input_at(f)).expect("step");
        if f < from {
            continue;
        }
        let mem = sync::core_memory(a.memory());
        let p = sync::pacing(&a.cpu(), &mem, &theirs.paula);
        let mid = sync::mid_pass(&a.cpu(), &mem);
        println!("== {f} {p:?}{}", if mid { " MID" } else { "" });
        let mut runs: Vec<(u16, u16, String)> = Vec::new();
        for s in a.line_samples() {
            let int = s.sr & 0x700 != 0;
            let rets: Vec<u32> = s
                .stack
                .iter()
                .copied()
                .filter(|&r| is_return(&rom, r))
                .collect();
            let label = if mode == "top" || mode == "sum" {
                let roots = if int {
                    vec![h0(VBL)]
                } else {
                    vec![h0(FRAME_LOOP), h0(PASS)]
                };
                let in_root = |r: u32| func(&syms, r).is_some_and(|f| roots.contains(&f.0));
                let mut child = s.pc;
                let mut found = in_root(s.pc);
                if !found {
                    for &r in &rets {
                        if in_root(r) {
                            found = true;
                            break;
                        }
                        child = r;
                    }
                }
                let n = if found {
                    func(&syms, child).map_or(format!("${child:x}"), |f| f.1.clone())
                } else {
                    "?".into()
                };
                format!("{} {n}", if int { "I" } else { "M" })
            } else {
                let mut label = format!("{} {}", if int { "I" } else { "M" }, name(&syms, s.pc));
                if mode != "deep" {
                    label = label.split('+').next().unwrap_or_default().to_string();
                }
                let callers: Vec<String> = rets
                    .iter()
                    .take(if mode == "deep" { 4 } else { 2 })
                    .map(|&r| {
                        name(&syms, r)
                            .split('+')
                            .next()
                            .unwrap_or_default()
                            .to_string()
                    })
                    .collect();
                format!("{label} < {}", callers.join(" < "))
            };
            match runs.last_mut() {
                Some(r) if r.2 == label => r.1 = s.vpos,
                _ => runs.push((s.vpos, s.vpos, label)),
            }
        }
        if mode == "sum" {
            let mut seen = std::collections::BTreeSet::new();
            for r in &runs {
                let e = sum.entry(r.2.clone()).or_default();
                e.0 += (r.1 - r.0 + 1) as u32;
                if seen.insert(r.2.clone()) {
                    e.1 += 1;
                    e.2 += r.0 as u32;
                }
            }
            continue;
        }
        for r in runs {
            println!("  {:3}-{:3} {}", r.0, r.1, r.2);
        }
    }
    for (k, (lines, n, start)) in &sum {
        println!(
            "{k:40} frames {n:4} lines/frame {:6.1} first line {:6.1}",
            *lines as f64 / *n as f64,
            *start as f64 / *n as f64
        );
    }
}
