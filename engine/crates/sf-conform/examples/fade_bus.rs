//! T130's probe: the level handler's fades run access by access (`sf_core::front::fade::
//! level_bus_times`) on the original's own bus, each DMA cycle taken by whoever the reference
//! emulator's DMA debugger says had it, from a range of dispatch cycles: which start gives the
//! original's acknowledge. `fade_bus <case.toml> <frame> <from> <to>`
use sf_conform::{Case, sync};
use sf_core::front::fade;
use sf_core::front::story::cpu::Bus;
use sf_core::state::State;

struct Owners(Vec<[u8; 228]>);

/// A free bus that counts the accesses asked of it.
struct Count(std::cell::Cell<u32>);

impl Bus for Count {
    fn taken(&self, _: u32) -> bool {
        self.0.set(self.0.get() + 1);
        false
    }
}

impl Bus for Owners {
    fn taken(&self, t: u32) -> bool {
        let (line, h) = ((t / 227) as usize, (t % 227) as usize);
        self.0.get(line).is_some_and(|l| l[h] > 1)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&args[0])).expect("case");
    let frame: u32 = args[1].parse().unwrap();
    let (from, to): (u32, u32) = (args[2].parse().unwrap(), args[3].parse().unwrap());
    let frozen = case.freeze();
    let synced = sync::reach_frozen(&case.sync, &case.script, frame + 10, frozen).expect("sync");
    let mut a = synced.amiga;
    let display = sf_conform::display_data();
    for f in 0..frame {
        frozen.apply(&mut a, &display);
        a.step(case.script.input_at(f)).expect("step");
    }
    let state = State::transplant(&sync::core_memory(a.memory()));
    let walks = fade::level_walks(&state);
    a.set_dma_profile();
    frozen.apply(&mut a, &display);
    a.step(case.script.input_at(frame)).expect("step");
    let ack = a
        .cpu_writes()
        .into_iter()
        .find(|w| w.reg == 0x9c && w.value == 0x20)
        .map(|w| w.vpos as u32 * 227 + w.hpos as u32);
    let mut owners = vec![[0u8; 228]; 313];
    for s in a.line_samples() {
        owners[s.vpos as usize] = s.owners;
    }
    let count = Count(std::cell::Cell::new(0));
    let (_, free_end) = fade::level_bus_times(&walks, 0, &count);
    let cpu = |a: u32, b: u32| {
        (a..b)
            .filter(|&t| {
                owners
                    .get((t / 227) as usize)
                    .is_some_and(|l| l[(t % 227) as usize] == 1)
            })
            .count()
    };
    println!(
        "model: {} accesses, {free_end} cycles on a free bus; the original's CPU accesses from line 17 to its acknowledge {}",
        count.0.get(),
        ack.map_or(0, |a| cpu(17 * 227, a))
    );
    for t in (17 * 227..19 * 227).step_by(20) {
        println!(
            "  original's accesses from {t} ({}:{}) to the acknowledge: {}",
            t / 227,
            t % 227,
            ack.map_or(0, |a| cpu(t, a))
        );
    }
    let bus = Owners(owners);
    println!("frame {frame}: the original's acknowledge {ack:?}");
    for start in from..=to {
        let (writes, end) = fade::level_bus_times(&walks, start, &bus);
        println!(
            "from {start} ({}:{}): ack {end} first writes {:?}",
            start / 227,
            start % 227,
            &writes[..writes.len().min(3)]
        );
    }
}
