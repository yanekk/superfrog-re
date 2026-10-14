//! What a case's objects do on the original, frame by frame: every slot that spawns (its
//! template, the marker and map cell it came from) or is freed, every slot whose update turns
//! into a killed or stunned enemy's (h0+$23b34, $23bee) or a reward's arc ($23e6c), and every
//! heart the frog loses, with the slots then within 64 pixels of it. For the coverage lists of
//! the completion cases (T103).
//!
//! `cargo run --release --example slots_log -- <case.toml>`
use sf_conform::{Case, sync};
use sf_core::frog::{FROG, f};
use sf_core::layout::{h0, h1};

const SLOTS: u32 = 0x1ba6c;
/// `obj_templates` (h0+$1c6e2): 160 templates of `$ae` bytes; a slot's first long is its
/// template's init routine.
const TEMPLATES: u32 = 0x1c6e2;
const ROW0: u32 = 0x218e0 + 0x4c0;
const HEARTS: u32 = h0(0x830);

fn main() {
    let path = std::env::args().nth(1).expect("a case");
    let case = Case::load(std::path::Path::new(&path)).unwrap_or_else(|e| panic!("{e}"));
    let sf_conform::SyncPoint::AtLevel(id) = case.sync else { panic!("a level case") };
    let data = sf_conform::game_data();
    let t = &data.levels.iter().find(|l| *l.0 == id).expect("level").1.t7mp;
    let mut synced = sync::reach(&case.sync, &sf_conform::InputScript::idle(), case.frames + 100)
        .unwrap_or_else(|e| panic!("{e}"));
    let a = &mut synced.amiga;
    let l = |a: &sf_oracle::Amiga, x: u32| {
        let b = a.peek(x, 4);
        u32::from_be_bytes([b[0], b[1], b[2], b[3]])
    };
    let w = |a: &sf_oracle::Amiga, x: u32| {
        let b = a.peek(x, 2);
        u16::from_be_bytes([b[0], b[1]])
    };
    let template = |a: &sf_oracle::Amiga, init: u32| {
        (0..160).find(|&k| l(a, h0(TEMPLATES + k * 0xae)) == init)
    };
    let cell = |c: u32| {
        let d = c.wrapping_sub(h1(ROW0));
        let (r, col) = (d / 0x260, (d % 0x260) / 2);
        let m = if r < 84 && col < 300 { t.body[(r * 300 + col) as usize] >> 10 } else { 99 };
        format!("marker {m} at {col},{r}")
    };
    let dead = [0x23b34, 0x23bee, 0x23e6c].map(h0);
    let slot = |k: u32| h0(SLOTS + k * 0x7c);
    let mut before = vec![(0u32, 0u32, 0u32); 20];
    let mut hearts = w(a, HEARTS);
    for fr in 0..case.frames {
        sync::step_inserting(a, case.script.input_at(fr)).unwrap();
        for k in 0..20 {
            let s = slot(k);
            let now = (l(a, s + 0x40), l(a, s), l(a, s + 4));
            let was = before[k as usize];
            if now.0 != 0 && (was.0 == 0 || was.1 != now.1 || was.0 != now.0) {
                let tm = template(a, now.1).map_or("?".to_string(), |t| t.to_string());
                println!("{fr}: slot {k} spawns template {tm}, {}", cell(now.0));
            } else if now.0 == 0 && was.0 != 0 {
                println!("{fr}: slot {k} freed");
            } else if now.0 != 0 && dead.contains(&now.2) && !dead.contains(&was.2) {
                let tm = template(a, now.1).map_or("?".to_string(), |t| t.to_string());
                println!("{fr}: slot {k} (template {tm}, {}) killed", cell(now.0));
            }
            before[k as usize] = now;
        }
        let h = w(a, HEARTS);
        if h < hearts {
            let (fx, fy) = (w(a, FROG + f::X) as i16 as i32 + 0x17, w(a, FROG + f::Y) as i16 as i32 + 0x2a);
            let near: Vec<String> = (0..20)
                .filter_map(|k| {
                    let s = slot(k);
                    let (x, y) = (w(a, s + 8) as i16 as i32, w(a, s + 0xa) as i16 as i32);
                    (l(a, s + 0x40) != 0 && (x - fx).abs() < 64 && (y - fy).abs() < 64).then(|| {
                        let tm = template(a, l(a, s)).map_or("?".to_string(), |t| t.to_string());
                        format!("slot {k} template {tm} ({})", cell(l(a, s + 0x40)))
                    })
                })
                .collect();
            println!("{fr}: hurt, hearts {h}; near: {}", near.join("; "));
        }
        hearts = h;
    }
}
