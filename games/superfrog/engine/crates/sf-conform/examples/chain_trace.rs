//! Scratch (T183): the engine alone from power-on, fire on the title and on START GAME, each
//! change of copper list, scene and the screens' flags, until `frames`. `chain_trace [frames]`
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::h0;
use sf_core::{Game, Rng, Start};

fn main() {
    let n: u32 = std::env::args().nth(1).map_or(3000, |a| a.parse().unwrap());
    let mut g = Game::with_display(sf_conform::display_data(), Start::PowerOn { seed: Rng::seeded(1, 2) });
    let mut last = String::new();
    for f in 0..n {
        let fire = [300u32, 600, 800].iter().any(|&t| (t..t + 5).contains(&f));
        let input = FrameInput { joy: if fire { Joy::FIRE } else { FrameInput::NONE.joy }, ..FrameInput::NONE };
        let out = g.step(input);
        let s = g.state();
        let now = format!(
            "{:?} mus {} cop {:x?} map {} intro {} exit {} walk {:x} timer {} world {} level {} un {:?}",
            g.scene(),
            s.audio.music_on(),
            g.cop1lc(),
            s.l(h0(0x4112)),
            s.l(h0(0x42a6)),
            s.w(h0(0x9f8)),
            s.l(h0(0x11e92)).wrapping_sub(h0(0)),
            if s.w(h0(0x123d4)) as i16 > 0 { "+" } else { "0" },
            s.l(h0(0x40da)),
            s.l(h0(0x40de)),
            g.unported(),
        );
        if now != last || out.load.is_some() {
            println!("{f}: {now}{}", if out.load.is_some() { " LOAD" } else { "" });
            last = now;
        }
    }
}
