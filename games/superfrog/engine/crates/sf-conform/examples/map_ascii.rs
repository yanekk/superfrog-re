//! A level's markers as text, a cell a character, for steering scripts: `.` 0, `#` solid
//! (`marker & $18 == $18`), `E` exit (54), `S` start (`$35`), `x` deadly (6), digits/letters others
//! (marker in base 36).
//!
//! `cargo run --release --example map_ascii -- <level> [c0 r0 c1 r1]`
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let id: sf_core::LevelId = a[0].parse().unwrap();
    let data = sf_conform::game_data();
    let t = &data.levels.iter().find(|l| *l.0 == id).unwrap().1.t7mp;
    let n = |k: usize, d: u32| a.get(k).map_or(d, |s| s.parse().unwrap());
    let (w, h) = (t.width as u32, t.height as u32);
    let (c0, r0, c1, r1) = (n(1, 0), n(2, 0), n(3, w - 1), n(4, h - 1));
    print!("    ");
    for c in c0..=c1 {
        print!("{}", if c % 10 == 0 { ((c / 10) % 10).to_string() } else { " ".into() });
    }
    println!();
    for r in r0..=r1 {
        print!("{r:3} ");
        for c in c0..=c1 {
            let m = t.body[(r * w + c) as usize] >> 10;
            let ch = match m {
                0 => '.',
                54 => 'E',
                0x35 => 'S',
                6 => 'x',
                _ if m & 0x18 == 0x18 => '#',
                _ => std::char::from_digit(m as u32 % 36, 36).unwrap(),
            };
            print!("{ch}");
        }
        println!();
    }
}
