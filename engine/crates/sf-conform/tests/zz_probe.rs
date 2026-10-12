use sf_core::layout::h0;
#[test]
#[ignore]
fn probe() {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("frog-run-w1-1.toml")).unwrap();
    let mut case = case;
    case.frames = 560;
    let _ = sf_conform::run_with(&case, |f, g| {
        if (556..=560).contains(&f) {
            let s = g.state();
            let rd = |a: u32, n: u32| (0..n).fold(0u32, |v, k| v << 8 | s.read(a + k).unwrap_or(0xee) as u32);
            println!("before {f}: inpass={:x} cell={:x} tog={:x} vblc={:x} x={:x} y={:x} camx={:x} camy={:x}",
                rd(h0(0x860), 2), rd(h0(0x776c), 4), rd(h0(0x7770), 2), rd(h0(0x9d8), 2), rd(h0(0x7d6), 2), rd(h0(0x7d8), 2), rd(h0(0x16666), 2), rd(h0(0x16662), 2));
        }
    });
}
