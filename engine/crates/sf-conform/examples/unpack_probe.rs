//! Scratch (T56): each unpack of the disk-3 intro in the original (`cf5c`, by its call's return
//! address): the step it starts in, the steps after that it ends in, and the line of the last
//! line sample inside it. `unpack_probe <case.toml> <to>`.
use sf_conform::{Case, sync};

const H0: u32 = 0xc0_0fa0;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&a[0])).unwrap();
    let to: u32 = a[1].parse().unwrap();
    let s = sync::reach(&case.sync, &case.script, to + 20000).unwrap();
    let mut am = s.amiga;
    am.set_profile(true);
    let rets: Vec<u32> = sf_core::front::story::scenes::UNPACK_FRAMES
        .iter()
        .map(|r| r.0)
        .collect();
    let mut inside: Option<(u32, u32)> = None;
    for f in 0..to {
        sync::step_inserting(&mut am, case.script.input_at(f)).unwrap();
        let m = sync::core_memory(am.memory());
        let chain = sync::main_chain(&am.cpu(), &m);
        let now = rets.iter().copied().find(|r| chain.contains(r));
        match (inside, now) {
            (None, Some(r)) => inside = Some((r, f)),
            (Some((r, from)), n) if n != Some(r) => {
                let last = am
                    .line_samples()
                    .iter()
                    .filter(|s| (H0 + 0xcf5c..H0 + 0xd200).contains(&s.pc))
                    .map(|s| s.vpos)
                    .last();
                println!(
                    "ret {:x} from {from} frames {} last line {last:?}",
                    r - H0,
                    f - from
                );
                inside = n.map(|n| (n, f));
            }
            _ => {}
        }
    }
}
