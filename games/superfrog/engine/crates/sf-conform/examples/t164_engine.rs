//! Scratch (T164): the engine's scene, `demo_index`, `level_started` and `attract_mode` each time
//! one changes over a case. `t164_engine <case.toml>`
use sf_conform::Case;
use sf_core::layout::h0;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&a[0])).unwrap();
    let mut last = String::new();
    let r = sf_conform::run_with(&case, |f, g| {
        let s = g.state();
        let now = format!(
            "{:?} demo_index {} started {} attract {:x}",
            g.scene(),
            s.w(h0(0x9f6)),
            s.w(h0(0x9f0)),
            s.l(h0(0x18))
        );
        if now != last {
            println!("{f}: {now}");
            last = now;
        }
    });
    println!("{:?}", r.err().map(|d| d.to_string()));
}
