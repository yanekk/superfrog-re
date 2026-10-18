//! Scratch (T212): the original's CPU at the end of each frame `from..to` of a case
//! (`cpu_probe <case.toml> <from> <to>`): its main PC, a0 and the pass step it counts as.
use sf_conform::{Case, sync};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&a[0])).unwrap();
    let from: u32 = a[1].parse().unwrap();
    let to: u32 = a[2].parse().unwrap();
    let synced = sync::reach(&case.sync, &case.script, to + 20).unwrap();
    let mut amiga = synced.amiga;
    for of in 0..to {
        sync::step_inserting(&mut amiga, case.script.input_at(of)).unwrap();
        if of + 1 < from {
            continue;
        }
        let cpu = amiga.cpu();
        let m = sync::core_memory(amiga.memory());
        println!(
            "{of} pc {:x} a0 {:x} a2 {:x} d0 {:x} step {} point {} chain {:x?}",
            cpu.pc,
            cpu.a[0],
            cpu.a[2],
            cpu.d[0],
            sync::pass_step(&cpu, &m),
            sync::pass_point(&cpu, &m),
            sync::main_chain(&cpu, &m)
        );
    }
}
