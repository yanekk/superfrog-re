//! Scratch (T208): the engine's words at the given addresses at the end of each frame `from..to`
//! of a case (`eng_watch <case.toml> <from> <to> <h0 offset hex>...`).
use sf_core::layout::h0;
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = sf_conform::Case::load(std::path::Path::new(&a[0])).unwrap();
    let from: u32 = a[1].parse().unwrap();
    let to: u32 = a[2].parse().unwrap();
    let addrs: Vec<u32> = a[3..]
        .iter()
        .map(|s| h0(u32::from_str_radix(s, 16).unwrap()))
        .collect();
    let case = sf_conform::Case {
        frames: to.min(case.frames),
        ..case
    };
    let r = sf_conform::run_hooked(
        &case,
        |f, g| {
            if f > from && f <= to {
                let s = g.state();
                let v: Vec<String> = addrs.iter().map(|&x| format!("{:x}", s.w(x))).collect();
                println!("{} E {} cop {:x?}", f - 1, v.join(" "), g.cop1lc());
            }
        },
        |_, _, _| {},
    );
    println!("{:?}", r.err().map(|d| d.to_string()));
}
