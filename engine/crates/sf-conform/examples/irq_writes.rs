//! Scratch (T208): both sides' DMACON, INTENA and INTREQ writes (line:cycle, register=value) over
//! the engine's frames `from..to` of a case, lag frames among them, and the engine's scene and
//! COP1LC each frame. `irq_writes <case.toml> <from> <to>`.
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = sf_conform::Case::load(std::path::Path::new(&a[0])).unwrap();
    let from: u32 = a[1].parse().unwrap();
    let to: u32 = a[2].parse().unwrap();
    let case = sf_conform::Case {
        frames: to.min(case.frames),
        ..case
    };
    let show = |ws: &[sf_paula::PaulaWrite]| {
        ws.iter()
            .filter(|w| matches!(w.reg, 0x96 | 0x9a | 0x9c))
            .map(|w| format!("{}:{} {:x}={:04x}", w.vpos, w.hpos, w.reg, w.value))
            .collect::<Vec<_>>()
            .join("  ")
    };
    let r = sf_conform::run_observing_lag(&case, |f, ours, theirs| {
        if (from..to).contains(&f) {
            println!("{f} E: {}", show(ours));
            println!("{f} O: {}", show(theirs));
        }
    });
    println!("{:?}", r.err().map(|d| d.to_string()));
}
