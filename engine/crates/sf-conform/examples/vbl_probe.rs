//! T130's probe: the front end's vertical-blank handler on chosen frames of a case, side by side:
//! its entry, every custom-register write the CPU made up to the acknowledge (cycles from the
//! frame's top, `line × 227 + slot`), and per line the DMA slots each owner took and where the CPU
//! was at the line's end. `vbl_probe <case.toml> <frame>...`
use sf_conform::{Case, sync};

const LINE: u32 = 227;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&args[0])).expect("case");
    let frames: Vec<u32> = args[1..].iter().map(|a| a.parse().unwrap()).collect();
    let to = *frames.iter().max().unwrap() + 1;
    let frozen = case.freeze();
    let synced = sync::reach_frozen(&case.sync, &case.script, to + 10, frozen).expect("sync");
    let mut a = synced.amiga;
    let display = sf_conform::display_data();
    let mut irqs = Vec::new();
    for f in 0..to {
        let probe = frames.contains(&f);
        if probe {
            a.set_dma_profile();
        }
        frozen.apply(&mut a, &display);
        a.step(case.script.input_at(f)).expect("step");
        let entry = sync::handler_entry(&irqs, &a.irqs());
        irqs = a.irqs();
        if !probe {
            continue;
        }
        println!("== frame {f} entry {entry:?}");
        let mut ack_line = 80;
        for w in a.cpu_writes() {
            let t = w.vpos as u32 * LINE + w.hpos as u32;
            println!(
                "  w {:3}:{:3} t{:6} ${:03x}={:04x}",
                w.vpos, w.hpos, t, w.reg, w.value
            );
            if w.reg == 0x9c && w.value == 0x20 {
                ack_line = w.vpos;
                break;
            }
        }
        for s in a.line_samples() {
            if s.vpos > ack_line {
                break;
            }
            let mut n = [0u32; 8];
            for &o in s.owners.iter() {
                let k = match o {
                    0 => 0,
                    1 => 1,
                    2 => 2,
                    3 => 3,
                    4..=7 => 4,
                    8..=15 => 5,
                    16..=23 => 6,
                    24 => 7,
                    _ => 7,
                };
                n[k] += 1;
            }
            let row: String = s
                .owners
                .iter()
                .map(|&o| match o {
                    0 => '.',
                    1 => 'c',
                    2 => 'r',
                    3 => 'd',
                    4..=7 => 'a',
                    8..=15 => (b'1' + o - 8) as char,
                    16..=23 => 's',
                    24 => 'P',
                    25 => 'B',
                    _ => '?',
                })
                .collect();
            println!(
                "  l{:3} pc h0+{:x} free {:3} cpu {:3} cop {:2} spr {:2} bpl {:3} {}",
                s.vpos,
                s.pc.wrapping_sub(sf_core::layout::h0(0)),
                n[0],
                n[1],
                n[7],
                n[6],
                n[5],
                row
            );
        }
        a.set_profile(false);
    }
}
