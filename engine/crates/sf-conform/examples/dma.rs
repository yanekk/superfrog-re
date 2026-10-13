//! Scratch (T36): the original's DMA slot owners per line. `dma <case> <frame> <from-line> <to-line>`
use sf_conform::{Case, sync};
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&a[0])).unwrap();
    let fr: u32 = a[1].parse().unwrap();
    let (l0, l1): (u16, u16) = (a[2].parse().unwrap(), a[3].parse().unwrap());
    let frozen = case.freeze();
    let s = sync::reach_frozen(&case.sync, &case.script, fr + 10, frozen).unwrap();
    let mut am = s.amiga;
    let d = sf_conform::display_data();
    for f in 0..=fr {
        if f == fr { am.set_dma_profile(); }
        frozen.apply(&mut am, &d);
        am.step(case.script.input_at(f)).unwrap();
    }
    let ch = |o: u8| match o { 0 => '.', 1 => 'c', 2 => 'r', 3 => 'd', 4..=7 => 'a', 8..=15 => (b'1' + o - 8) as char, 16..=23 => 's', 24 => 'P', 25 => 'B', 26 => 'X', _ => '?' };
    for s in am.line_samples() {
        if (l0..=l1).contains(&s.vpos) {
            let row: String = s.owners.iter().map(|&o| ch(o)).collect();
            let bs: Vec<usize> = s.owners.iter().enumerate().filter(|x| *x.1 == 25).map(|x| x.0).collect();
            println!("{:3} h{:3} B {:?}", s.vpos, s.hpos, bs.iter().take(60).collect::<Vec<_>>());
        }
    }
}
