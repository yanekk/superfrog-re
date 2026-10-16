//! Scratch (T199): where the original's CPU is, line by line, over frames `from..to` of a case
//! (its freeze applied as the runner does): the PC as each line ends, the return addresses in
//! hunk 0 on its stack, and the DMACON, BLTSIZE and INTENA writes. `restart_probe <case.toml>
//! <from> <to>`; a case with no `[[exclude]]` runs live.
use sf_conform::{Case, sync};
use sf_core::layout::h0;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&a[0])).unwrap();
    let from: u32 = a[1].parse().unwrap();
    let to: u32 = a[2].parse().unwrap();
    let frozen = case.freeze();
    let s = sync::reach_frozen(&case.sync, &case.script, to + 20000, frozen).unwrap();
    let mut am = s.amiga;
    let display = sf_conform::display_data();
    for f in 0..from {
        frozen.apply(&mut am, &display);
        sync::step_inserting(&mut am, case.script.input_at(f)).unwrap();
    }
    am.set_profile(true);
    let dma = std::env::var_os("DMA").is_some();
    if dma {
        am.set_dma_profile();
    }
    let rel = |x: u32| {
        if (h0(0)..h0(0x4834c)).contains(&x) {
            format!("{:x}", x - h0(0))
        } else {
            format!("@{x:x}")
        }
    };
    for f in from..to {
        frozen.apply(&mut am, &display);
        sync::step_inserting(&mut am, case.script.input_at(f)).unwrap();
        for s in am.line_samples() {
            let rets: Vec<String> = s
                .stack
                .iter()
                .filter(|&&r| (h0(0x100)..h0(0x30000)).contains(&r) && r & 1 == 0)
                .take(4)
                .map(|&r| rel(r))
                .collect();
            println!("L {f} {} {} {}", s.vpos, rel(s.pc), rets.join(","));
            if dma {
                let mut h = std::collections::BTreeMap::new();
                for &o in s.owners.iter() {
                    *h.entry(o).or_insert(0) += 1;
                }
                println!("D {f} {} {h:?}", s.vpos);
            }
        }
        let list = |at: u32| {
            let m = am.peek(at, 0x400);
            m.chunks(4).take_while(|c| c != &[0, 0, 0, 0]).count()
        };
        let slots = am.peek(h0(0x1ba6c), 20 * 0x7c);
        let live = (0..20).filter(|k| slots[k * 0x7c + 0x40..k * 0x7c + 0x44] != [0, 0, 0, 0]).count();
        let n = am.peek(h0(0x1ba6a), 2);
        println!(
            "N {f} draw {} free {} slots {live} count {}",
            list(h0(0x2970e)),
            list(h0(0x1ba16)),
            u16::from_be_bytes([n[0], n[1]])
        );
        for w in am.cpu_writes() {
            if matches!(w.reg, 0x96 | 0x58 | 0x9a) {
                println!("W {f} {}:{} {:x}={:04x}", w.vpos, w.hpos, w.reg, w.value);
            }
        }
    }
}
