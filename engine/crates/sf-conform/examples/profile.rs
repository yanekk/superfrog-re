//! Scratch (T36): where the original's CPU is at each line's end, per frame of a case.
//! `profile <case> <from> <to> [deep|top]`
use sf_conform::{Case, sync};
use sf_core::layout::h0;

fn syms() -> Vec<(u32, String)> {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../re/symbols.toml")).unwrap();
    let t: toml::Table = text.parse().unwrap();
    let mut v: Vec<(u32, String)> = t["sym"].as_array().unwrap().iter().filter_map(|s| {
        let s = s.as_table()?;
        if s.get("exe")?.as_str()? != "d1_c03" || s.get("hunk")?.as_integer()? != 0 { return None; }
        if s.get("kind")?.as_str()? != "func" { return None; }
        Some((h0(s["offset"].as_integer()? as u32), s["name"].as_str()?.to_string()))
    }).collect();
    if let Ok(e) = std::fs::read_to_string("/tmp/t36/engine_syms.txt") {
        for l in e.lines() {
            let mut it = l.split(' ');
            let a = h0(u32::from_str_radix(it.next().unwrap(), 16).unwrap());
            let n = it.next().unwrap().to_string();
            if !v.iter().any(|s| s.0 == a) { v.push((a, n)); }
        }
    }
    v.sort();
    v
}

fn func(syms: &[(u32, String)], pc: u32) -> Option<&(u32, String)> {
    let i = syms.partition_point(|s| s.0 <= pc);
    if i == 0 || pc >= h0(0x4834c) { return None; }
    Some(&syms[i - 1])
}

fn name(syms: &[(u32, String)], pc: u32) -> String {
    match func(syms, pc) { Some((a, n)) => format!("{n}+{:x}", pc - a), None => format!("${pc:x}") }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&args[0])).unwrap();
    let from: u32 = args[1].parse().unwrap();
    let to: u32 = args[2].parse().unwrap();
    let mode = args.get(3).map(String::as_str).unwrap_or("");
    let syms = syms();
    let frozen = case.freeze();
    let synced = sync::reach_frozen(&case.sync, &case.script, to + 10, frozen).unwrap();
    let mut a = synced.amiga;
    let display = sf_conform::display_data();
    let code = h0(0)..h0(0x4834c);
    let rom = display.rom().h0.clone();
    let vbl = h0(0x11bf8);
    let lfl = h0(0xaa6);
    let mut sum: std::collections::BTreeMap<String, (u32, u32, u32)> = Default::default();
    for f in 0..to {
        if f == from { a.set_profile(true); }
        frozen.apply(&mut a, &display);
        let theirs = a.step(case.script.input_at(f)).unwrap();
        if f < from { continue; }
        let mem = sync::core_memory(a.memory());
        let p = sync::pacing(&a.cpu(), &mem, &theirs.paula);
        let mid = sync::mid_pass(&a.cpu(), &mem);
        println!("== {f} {:?}{}", p, if mid { " MID" } else { "" });
        let mut runs: Vec<(u16, u16, String)> = Vec::new();
        for s in a.line_samples() {
            let int = s.sr & 0x700 != 0;
            let w = |a: u32| -> u16 { let o = (a - h0(0)) as usize; u16::from_be_bytes([rom[o], rom[o + 1]]) };
            let is_ret = |r: u32| code.contains(&r) && r & 1 == 0 && r >= h0(8) && (
                w(r - 6) == 0x4eb9 || w(r - 4) == 0x6100 || (w(r - 2) & 0xff00 == 0x6100 && w(r - 2) & 0xff != 0)
                || (w(r - 2) & 0xfff8) == 0x4e90 || (w(r - 4) & 0xfff8) == 0x4ea8 || w(r - 4) == 0x4eba);
            let rets: Vec<u32> = s.stack.iter().copied().filter(|&r| is_ret(r)).collect();
            let label = if mode == "top" || mode == "sum" {
                // the direct callee of vbl_handler / level_frame_loop
                let root = if int { vbl } else { lfl };
                let roots = if int { vec![vbl] } else { vec![lfl, h0(0xab8)] };
                let _ = root;
                let in_root = |r: u32| func(&syms, r).is_some_and(|f| roots.contains(&f.0));
                let mut child = s.pc;
                let mut found = in_root(s.pc);
                if !found {
                    for &r in &rets {
                        if in_root(r) { found = true; break; }
                        child = r;
                    }
                }
                let n = if found { func(&syms, child).map_or(format!("${child:x}"), |f| f.1.clone()) } else { "?".into() };
                format!("{} {}", if int { "I" } else { "M" }, n)
            } else {
                let mut label = format!("{} {}", if int { "I" } else { "M" }, name(&syms, s.pc));
                if mode != "deep" { label = label.split('+').next().unwrap().to_string(); }
                let callers: Vec<String> = rets.iter().take(if mode == "deep" { 4 } else { 2 })
                    .map(|&r| name(&syms, r).split('+').next().unwrap().to_string()).collect();
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
                if seen.insert(r.2.clone()) { e.1 += 1; e.2 += r.0 as u32; }
            }
            continue;
        }
        for r in runs { println!("  {:3}-{:3} {}", r.0, r.1, r.2); }
    }
    for (k, (lines, n, start)) in &sum {
        println!("{k:40} frames {n:4} lines/frame {:6.1} first line {:6.1}", *lines as f64 / *n as f64, *start as f64 / *n as f64);
    }
}
