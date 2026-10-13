//! T54's probe: for each frame of the disk-3 intro whose draw follows the vertical blank's handler,
//! where the main program was interrupted (its wait's `tst.w`/`beq.b`, or `draw_next`'s
//! `cmpi.w`/`bmi.b`), the handler's acknowledge (`INTREQ = $20`) and the draw's DMACON `$8400`,
//! as cycles of the frame. `handler_end_probe <oracle frames>`. Run to 6 460 it showed the entry
//! does not tell `tst.w` from `beq.b` (entry cycles 10, 14 and 16 come with both), so conformance
//! feeds the PC ([`sync::handler_end`]).
use sf_conform::{Case, sync};
use sf_core::layout::h0;

const LINE: u32 = 227;

fn main() {
    let frames: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    let case = Case::load(&sf_conform::cases_dir().join("story-intro.toml")).unwrap();
    let synced = sync::reach(&case.sync, &case.script, frames + 20).unwrap();
    let mut amiga = synced.amiga;
    let mut of = 0;
    let mut irqs = Vec::new();
    while amiga.frame() < frames {
        let resumes = sync::intro_resumes(&amiga);
        sync::step_inserting(&mut amiga, case.script.input_at(of)).unwrap();
        of += 1;
        let entry = sync::handler_entry(&irqs, &amiga.irqs());
        irqs = amiga.irqs();
        let Some(pc) = resumes else {
            continue;
        };
        let rel = pc.wrapping_sub(h0(0));
        let mut t = Vec::new();
        let mut last = 0;
        let mut wrap = 0;
        for w in amiga.cpu_writes() {
            let mut c = wrap + w.vpos as u32 * LINE + w.hpos as u32;
            if c < last {
                wrap += 313 * LINE;
                c += 313 * LINE;
            }
            last = c;
            t.push((c, w.reg, w.value));
        }
        let Some(ack) = t.iter().position(|w| w.1 == 0x9c && w.2 == 0x20) else {
            continue;
        };
        // The draw right after it: its DMACON $8400 the CPU's next DMACON write.
        let Some(d) = t[ack + 1..].iter().find(|w| w.1 == 0x96) else {
            continue;
        };
        if d.2 != 0x8400 {
            continue;
        }
        println!(
            "O {} pc {rel:x} entry {entry:?} ack {} ({},{}) dmacon {} delta {}",
            amiga.frame(),
            t[ack].0,
            t[ack].0 / LINE,
            t[ack].0 % LINE,
            d.0,
            d.0 - t[ack].0
        );
    }
}
