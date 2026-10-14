//! Scratch (T73): the engine's slot variables before each frame of a case, and its Paula writes'
//! lines next to the original's (DMACON $8400 / INTENA $10 / $8010 / sfx).
use sf_conform::Case;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let case = Case::load(std::path::Path::new(&args[1])).unwrap();
    let from: u32 = args.get(2).map_or(0, |a| a.parse().unwrap());
    let h0 = sf_core::layout::h0(0);
    let show = |w: &[sf_paula::PaulaWrite]| {
        w.iter()
            .filter(|w| matches!(w.reg, 0x96 | 0x9a))
            .map(|w| format!("{:02x}={:04x}@{}", w.reg, w.value, w.vpos))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let lines = std::cell::RefCell::new(Vec::<String>::new());
    let r = sf_conform::run_hooked(
        &case,
        |f, g| {
            if f >= from {
                let s = g.state();
                let w = |o: u32| s.w(h0 + o);
                lines.borrow_mut().push(format!(
                    "E{f} coins={} reels={:x},{:x},{:x} hilo={} tog={:x} holds={},{},{} cnt={:x},{:x},{:x}",
                    w(0x49c6), w(0x49c8), w(0x49ca), w(0x49cc), w(0x49ce), w(0x5dc6), w(0x6154), w(0x6156), w(0x6158),
                    s.l(h0 + 0x5dd6), s.l(h0 + 0x5dda), s.l(h0 + 0x5dde)
                ));
            }
        },
        |f, ours, theirs| {
            if f >= from {
                println!("{}", lines.borrow_mut().pop().unwrap_or_default());
                println!("  ours   {}\n  theirs {}", show(ours), show(theirs));
            }
        },
    );
    if let Err(d) = r {
        println!("{d}");
    }
}
