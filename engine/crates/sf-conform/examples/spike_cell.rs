//! Scratch (bug-recorder T00, deleted before the task ends): which formula names the BODY cell
//! the frog is in. W1-1 played by `frog-run-right` on the engine; each frame the frog stands on
//! the ground, a formula's cell should be open with solid ground under it (docs/07: solid iff
//! `flag & 0b11000 == 0b11000`). `spike_cell <out dir> <frame>...` also writes those frames'
//! pictures as PPM with the formula's cells printed.
use sf_conform::{InputScript, display_data, game_data};
use sf_core::frog::{FROG, f};
use sf_core::input::FrameInput;
use sf_core::{Game, Start, View};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let id: sf_assets::LevelId = "W1-1".parse().unwrap();
    let map = &game_data().levels[&id].t7mp;
    let solid = |c: i32, r: i32| {
        if c < 0 || r < 0 || c >= map.width as i32 || r >= map.height as i32 {
            return false;
        }
        let flag = map.cell(c as u16, r as u16) >> 10;
        flag & 0b11000 == 0b11000
    };
    let script = InputScript::load(&sf_conform::case::script_path("frog-run-right")).unwrap();
    let mut g = Game::with_display(display_data(), Start::Level(id));
    let shots: Vec<u32> = a.iter().skip(1).map(|v| v.parse().unwrap()).collect();
    let formulas: [(&str, fn(i32, i32) -> (i32, i32)); 3] = [
        ("A (x+0x17)/16,(y-0x10)/16", |x, y| ((x + 0x17).div_euclid(16), (y - 0x10).div_euclid(16))),
        ("B (x+8)/16,(y-8)/16", |x, y| ((x + 8).div_euclid(16), (y - 8).div_euclid(16))),
        ("C (x+0x17)/16,(y-0x20)/16", |x, y| ((x + 0x17).div_euclid(16), (y - 0x20).div_euclid(16))),
    ];
    let mut ok = [0u32; 3];
    let mut grounded = 0u32;
    for n in 0..1500u32 {
        let s = g.state();
        let w = |o: u32| s.w(FROG + o) as i16 as i32;
        let (x, y) = (w(f::X), w(f::Y));
        if w(f::ON_GROUND) != 0 && w(f::DYING) == 0 && g.scene() == sf_core::Scene::Level {
            grounded += 1;
            for (k, (_, fm)) in formulas.iter().enumerate() {
                let (c, r) = fm(x, y);
                if !solid(c, r) && solid(c, r + 1) {
                    ok[k] += 1;
                }
            }
        }
        if shots.contains(&n) {
            let cells: Vec<String> = formulas.iter().map(|(name, fm)| format!("{name} -> {:?}", fm(x, y))).collect();
            println!("frame {n}: frog ({x},{y}) on_ground {} camera ({},{}): {}", w(f::ON_GROUND), s.l(sf_core::layout::h0(0x16664)), s.l(sf_core::layout::h0(0x16668)), cells.join("; "));
            let pic = g.render(View::default());
            let rgb = sf_core::to_rgb12(&pic);
            let mut out = format!("P6\n{} {}\n255\n", pic.width, pic.height).into_bytes();
            for v in rgb {
                for sh in [8, 4, 0] {
                    out.push((((v >> sh) & 0xf) * 17) as u8);
                }
            }
            std::fs::write(format!("{}/frame{n}.ppm", a[0]), out).unwrap();
        }
        g.step(script.input_at(n));
        let _ = FrameInput::NONE;
    }
    for (k, (name, _)) in formulas.iter().enumerate() {
        println!("{name}: open cell with solid ground under it on {}/{grounded} grounded frames", ok[k]);
    }
}
