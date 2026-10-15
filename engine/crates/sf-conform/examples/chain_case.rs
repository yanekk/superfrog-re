//! Scratch (T183): the engine's screens' flags each frame they change over a case.
//! `chain_case <case.toml>`
use sf_core::layout::h0;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = sf_conform::Case::load(std::path::Path::new(&a[0])).unwrap();
    let mut last = String::new();
    let r = sf_conform::run_with(&case, |f, g| {
        let s = g.state();
        let now = format!(
            "cop {:x?} end {} intro {} map {} exit {} timer {:x} f0 {:x} f78 {:x} ovl {:x} un {:?}",
            g.cop1lc(),
            s.l(h0(0x445a)),
            s.l(h0(0x42a6)),
            s.l(h0(0x4112)),
            s.w(h0(0x9f8)),
            s.w(h0(0x123d4)),
            s.l(h0(0x157d8)),
            s.l(h0(0x15850)),
            s.l(h0(0xb806)),
            g.unported()
        );
        if now != last {
            println!("{f}: {now}");
            last = now;
        }
    });
    println!("{:?}", r.err().map(|d| d.to_string()));
}
