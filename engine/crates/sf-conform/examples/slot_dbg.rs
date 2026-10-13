//! Scratch (T92): the original from the slot-load case's sync point, frame by frame.
use sf_conform::{Case, sync};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let case = Case::load(std::path::Path::new(&args[1])).unwrap();
    let from: u32 = args[2].parse().unwrap();
    let to: u32 = args[3].parse().unwrap();
    let synced = sync::reach(&case.sync, &case.script, to + 100).unwrap();
    let mut a = synced.amiga;
    let h0 = sf_core::layout::h0(0);
    for f in 0..=to {
        let cap = sync::step_inserting(&mut a, case.script.input_at(f)).unwrap();
        if f < from { continue; }
        let m = sync::core_memory(a.memory());
        let chain = sync::main_chain(&a.cpu(), &m);
        let ch: Vec<String> = chain.iter().map(|&x| format!("{:x}", x.wrapping_sub(h0))).collect();
        println!("F{f} chain {}", ch.join("<"));
        println!("  irqs {:?}", a.irqs().iter().map(|i| (i.level, i.next, i.vpos, i.hpos)).collect::<Vec<_>>());
        let w: Vec<String> = a.cpu_writes().iter().filter(|w| matches!(w.reg, 0x96 | 0x9a | 0x80 | 0x82 | 0x9c)).map(|w| format!("{:02x}={:04x}@{}.{}", w.reg, w.value, w.vpos, w.hpos)).collect();
        println!("  cpu {}", w.join(" "));
        let b: Vec<String> = a.blit_starts().iter().map(|b| format!("{:04x}@{}.{}", b.0, b.1, b.2)).collect();
        println!("  blits {} {}", b.len(), b.join(" "));
        if std::env::var("PAULA").is_ok() {
            let p: Vec<String> = cap.paula.iter().map(|w| format!("{:02x}={:04x}@{}", w.reg, w.value, w.vpos)).collect();
            println!("  paula {}", p.join(" "));
        }
    }
}
