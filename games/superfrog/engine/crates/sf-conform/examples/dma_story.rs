//! Scratch (T50): the original's DMA slot owners per line in a power-on-disk3 run, stepping as
//! story_dbg does. `dma_story <case> <oracle-frame> <line>...`
use sf_conform::{Case, sync};
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&a[0])).unwrap();
    let fr: u32 = a[1].parse().unwrap();
    let lines: Vec<u16> = a[2..].iter().map(|x| x.parse().unwrap()).collect();
    let s = sync::reach(&case.sync, &case.script, fr + 20000).unwrap();
    let mut am = s.amiga;
    for f in 0..=fr {
        if f == fr {
            am.set_dma_profile();
        }
        sync::step_inserting(&mut am, case.script.input_at(f)).unwrap();
    }
    let ch = |o: u8| match o {
        0 => '.',
        1 => 'c',
        2 => 'r',
        3 => 'd',
        4..=7 => 'a',
        8..=15 => (b'1' + o - 8) as char,
        16..=23 => 's',
        24 => 'P',
        25 => 'B',
        26 => 'X',
        _ => '?',
    };
    for s in am.line_samples() {
        if lines.contains(&s.vpos) || lines.is_empty() {
            let row: String = s.owners.iter().map(|&o| ch(o)).collect();
            println!("{:3} pc {:x} {}", s.vpos, s.pc.wrapping_sub(0xc00fa0), row);
        }
    }
}
