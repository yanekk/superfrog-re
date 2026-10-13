fn main() {
    let d = sf_conform::display_data();
    let mut g = sf_core::Game::with_display(d, sf_core::Start::Level("W1-1".parse().unwrap()));
    let t = std::time::Instant::now();
    for _ in 0..500 { g.step(sf_core::input::FrameInput::joy(sf_core::input::Joy::RIGHT)); }
    println!("500 frames {:?}", t.elapsed());
    let t = std::time::Instant::now();
    for _ in 0..100 { let _ = std::hint::black_box(g.clone()); }
    println!("100 clones {:?}", t.elapsed());
}
