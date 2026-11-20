//! Scratch (T129): the original's lamp blits around a frame's end in a slot case: each frame's
//! blit starts and Paula/CPU writes, and the DMA slot owners on lines 296.. and the next frame's
//! first lines. `slot_lamps <case.toml> <original frame>...`
use sf_conform::{Case, sync};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&a[0])).unwrap();
    let frames: Vec<u32> = a[1..].iter().map(|x| x.parse().unwrap()).collect();
    let to = *frames.iter().max().unwrap() + 1;
    let s = sync::reach(&case.sync, &case.script, to + 100).unwrap();
    let mut am = s.amiga;
    let ch = |o: u8| match o {
        0 => '.',
        1 => 'c',
        2 => 'r',
        3 => 'd',
        4..=7 => (b'0' + o - 4) as char,
        8..=15 => 'b',
        16..=23 => 's',
        24 => 'P',
        25 => 'B',
        26 => 'X',
        _ => '?',
    };
    for f in 0..=to {
        let shown = frames.contains(&f);
        let profile = std::env::var("OWNERS").is_ok() && shown;
        if profile {
            am.set_dma_profile();
        } else {
            am.set_profile(false);
        }
        sync::step_inserting(&mut am, case.script.input_at(f)).unwrap();
        if !shown {
            continue;
        }
        println!("== original frame {f}");
        let b: Vec<String> = am
            .blit_starts()
            .iter()
            .map(|b| format!("{:04x}@{}.{}", b.0, b.1, b.2))
            .collect();
        println!("  blits {}", b.join(" "));
        let w: Vec<String> = am
            .cpu_writes()
            .iter()
            .filter(|w| matches!(w.reg, 0x96 | 0x9a | 0x9c) || (0xa0..0xe0).contains(&w.reg))
            .map(|w| format!("{:02x}={:04x}@{}.{}", w.reg, w.value, w.vpos, w.hpos))
            .collect();
        println!("  cpu {}", w.join(" "));
        for s in am.line_samples() {
            if s.vpos >= 296 || s.vpos <= 3 {
                let row: String = s.owners.iter().map(|&o| ch(o)).collect();
                println!("{:3} pc {:6x} {}", s.vpos, s.pc.wrapping_sub(0xc00fa0), row);
            }
        }
    }
}
