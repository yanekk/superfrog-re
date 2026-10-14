//! Scratch (T73): the original from a slot case's sync point, frame by frame: the input, the main
//! program's chain and the slot's variables, printed when they change.
use sf_conform::{Case, sync};
use sf_core::layout::Ram;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let case = Case::load(std::path::Path::new(&args[1])).unwrap();
    let to: u32 = args[2].parse().unwrap();
    let synced = sync::reach(&case.sync, &case.script, to + 100).unwrap();
    let mut a = synced.amiga;
    let h0 = sf_core::layout::h0(0);
    let w = |m: &sf_core::layout::AmigaMemory, off: u32| (m.byte(h0 + off) as u32) << 8 | m.byte(h0 + off + 1) as u32;
    let l = |m: &sf_core::layout::AmigaMemory, off: u32| w(m, off) << 16 | w(m, off + 2);
    let mut prev = String::new();
    for f in 0..=to {
        let input = case.script.input_at(f);
        let cap = sync::step_inserting(&mut a, input).unwrap();
        let m = sync::core_memory(a.memory());
        let chain = sync::main_chain(&a.cpu(), &m);
        let ch: Vec<String> = chain.iter().take(4).map(|&x| format!("{:x}", x.wrapping_sub(h0))).collect();
        let vars = format!(
            "coins={} reels={:x},{:x},{:x} hilo={} win={:x} shown={:x} score={:x} lives={} 49d0={} 49d2={:x} 49da={} holds={:x},{:x},{:x} 615a={} 615c={} 615e={} ptr={:x} cnt={:x},{:x},{:x} 5dc8={} rng={:x}",
            w(&m, 0x49c6), w(&m, 0x49c8), w(&m, 0x49ca), w(&m, 0x49cc), w(&m, 0x49ce), l(&m, 0x57b2), l(&m, 0x49d6),
            l(&m, 0x834), w(&m, 0x7d6 + 0x58), w(&m, 0x49d0), l(&m, 0x49d2), w(&m, 0x49da),
            w(&m, 0x6154), w(&m, 0x6156), w(&m, 0x6158), w(&m, 0x615a), w(&m, 0x615c), w(&m, 0x615e),
            w(&m, 0x11b66), l(&m, 0x5dd6), l(&m, 0x5dda), l(&m, 0x5dde), w(&m, 0x5dc8), l(&m, 0x160d4)
        );
        let sfx: Vec<String> = cap.paula.iter().filter(|p| p.reg == 0x96).map(|p| format!("{:04x}@{}", p.value, p.vpos)).collect();
        let line = format!("{} | {}", ch.join("<"), vars);
        if line != prev || std::env::var("EVERY").is_ok() || input != sf_core::input::FrameInput::NONE {
            println!("{f} {:?} {line} dmacon[{}]", input.joy, sfx.join(" "));
        }
        prev = line;
    }
}
