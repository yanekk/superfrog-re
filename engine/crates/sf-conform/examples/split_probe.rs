//! T97's probe: the disk-3 intro's vertical-blank handlers taken late in a frame, from power-on to
//! the insert-disk-1 page. `split_probe <oracle frames>`. Per handler taken on line 280 or later:
//! its entry, its last Paula write before the acknowledge and the acknowledge (`INTREQ = $20`),
//! `+` when they fall in the next frame. A handler whose 8-line wait (h0+$38a02, on VHPOSR's low
//! byte) is read on line 305 or later waits on to line `read + 8 - 256` of the next frame: its
//! acknowledge comes there, and that frame's blank gets no handler of its own (`split`).
use sf_conform::{Case, sync};
use sf_core::layout::{Ram, h0};

fn main() {
    let frames: u32 = std::env::args().nth(1).unwrap().parse().unwrap();
    let case = Case::load(&sf_conform::cases_dir().join("story-scene4-sound.toml")).unwrap();
    let synced = sync::reach(&case.sync, &case.script, frames + 20).unwrap();
    let mut amiga = synced.amiga;
    // A handler taken late in the last step, still to acknowledge: its entry and its writes.
    let mut open: Option<(u32, (u16, u16), Vec<String>)> = None;
    let mut irqs: Vec<sf_oracle::Irq> = Vec::new();
    for of in 0..frames {
        let cap = sync::step_inserting(&mut amiga, case.script.input_at(of)).unwrap();
        let now = amiga.irqs();
        let taken: Vec<(u16, u16)> = irqs
            .iter()
            .filter(|i| i.level == 3 && i.next)
            .chain(now.iter().filter(|i| i.level == 3 && !i.next))
            .map(|i| (i.vpos, i.hpos))
            .collect();
        irqs = now;
        let m = sync::core_memory(amiga.memory());
        let vector = (0x6c..0x70).fold(0u32, |v, a| v << 8 | m.byte(a) as u32);
        if vector != sync::INTRO_VBL {
            continue;
        }
        let mut ws = cap.paula.iter().peekable();
        if let Some((f, entry, mut before)) = open.take() {
            // The handler of the step before goes on here.
            let mut acked = false;
            for w in ws.by_ref() {
                if w.reg == 0x9c && w.value == 0x20 {
                    println!(
                        "O {f} entry {entry:?} writes [{}] ack +{},{} split (O {of}: {} handler(s) taken)",
                        before.join(" "),
                        w.vpos,
                        w.hpos,
                        taken.len()
                    );
                    acked = true;
                    break;
                }
                before.push(format!("+{:x}@{}", w.reg, w.vpos));
            }
            assert!(acked, "O {f}: no acknowledge in the next step either");
        }
        for &entry in &taken {
            let mut before = Vec::new();
            let mut ack = None;
            while let Some(w) = ws.next_if(|_| ack.is_none()) {
                if (w.vpos, w.hpos) < entry {
                    continue;
                }
                if w.reg == 0x9c && w.value == 0x20 {
                    ack = Some((w.vpos, w.hpos));
                } else {
                    before.push(format!("{:x}@{}", w.reg, w.vpos));
                }
            }
            match ack {
                Some(a) if entry.0 >= 280 => println!(
                    "O {of} entry {entry:?} writes [{}] ack {},{}",
                    before.join(" "),
                    a.0,
                    a.1
                ),
                Some(_) => {}
                None => open = Some((of, entry, before)),
            }
        }
        if sync::main_chain(&amiga.cpu(), &m).contains(&h0(0x310)) {
            println!("O {of}: the insert-disk-1 page");
            break;
        }
    }
}
