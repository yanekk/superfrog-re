//! T130's probe: per frame of a case, the original's vertical-blank acknowledge (`INTREQ = $20`,
//! cycles from the frame's top, `line × 227 + slot`) and the audio DMA slots taken between line 17
//! and it. `vbl_audio <case.toml> <from> <to>`
use sf_conform::{Case, sync};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&args[0])).expect("case");
    let (from, to): (u32, u32) = (args[1].parse().unwrap(), args[2].parse().unwrap());
    let frozen = case.freeze();
    let synced = sync::reach_frozen(&case.sync, &case.script, to + 10, frozen).expect("sync");
    let mut a = synced.amiga;
    let display = sf_conform::display_data();
    for f in 0..to {
        if f == from {
            a.set_dma_profile();
        }
        frozen.apply(&mut a, &display);
        a.step(case.script.input_at(f)).expect("step");
        if f < from {
            continue;
        }
        let Some(ack) = a
            .cpu_writes()
            .into_iter()
            .find(|w| w.reg == 0x9c && w.value == 0x20)
            .map(|w| w.vpos as u32 * 227 + w.hpos as u32)
        else {
            continue;
        };
        let mut n = 0;
        for s in a.line_samples() {
            for (h, &o) in s.owners.iter().enumerate() {
                let t = s.vpos as u32 * 227 + h as u32;
                if (4..=7).contains(&o) && t >= 17 * 227 && t < ack {
                    n += 1;
                }
            }
        }
        println!("O {f} ack {ack} ({}:{}) audio {n}", ack / 227, ack % 227);
    }
}
