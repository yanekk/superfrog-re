//! Scratch (T56): each scene set-up of the disk-3 intro in the original, on one clock (DMA cycles
//! from oracle frame `from`'s start): the buffers' clearing blits, the masks' clearing loop
//! (h0+$8408), DMACON $8400 and the mask blits, the handlers taken.
//! `init_probe <case.toml> <from> <to>`.
use sf_conform::{Case, sync};

const H0: u32 = 0xc0_0fa0;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&a[0])).unwrap();
    let from: u32 = a[1].parse().unwrap();
    let to: u32 = a[2].parse().unwrap();
    let s = sync::reach(&case.sync, &case.script, to + 20000).unwrap();
    let mut am = s.amiga;
    for f in 0..from {
        sync::step_inserting(&mut am, case.script.input_at(f)).unwrap();
    }
    am.set_profile(true);
    let base = am.frame() as i64;
    let mut frame: i64;
    let mut last_v: i64;
    let mut in_clear = false;
    let mut prev_end = am.frame() as i64;
    for f in from..to {
        sync::step_inserting(&mut am, case.script.input_at(f)).unwrap();
        let samples = am.line_samples();
        // The step starts in the frame the last one ended in; wraps count the rest.
        frame = prev_end - base;
        last_v = -1;
        // Lines of the samples on one clock.
        let mut abs = Vec::new();
        for s in &samples {
            if (s.vpos as i64) < last_v {
                frame += 1;
            }
            last_v = s.vpos as i64;
            abs.push((frame, s.vpos, s.pc));
        }
        let first_frame = abs.first().map_or(frame, |x| x.0);
        let t = |fr: i64, v: u16, h: u16| (fr * 313 + v as i64) * 227 + h as i64;
        for &(fr, v, pc) in &abs {
            let c = (H0 + 0x8408..H0 + 0x8410).contains(&pc);
            if c != in_clear {
                println!(
                    "O {f} t {} clear {}",
                    t(fr, v, 0),
                    if c { "on" } else { "off" }
                );
                in_clear = c;
            }
        }
        // Writes: their line, wrapped as the samples are.
        let mut wf = first_frame;
        let mut lv = -1i64;
        for w in am.cpu_writes() {
            if (w.vpos as i64) < lv {
                wf += 1;
            }
            lv = w.vpos as i64;
            let tag = match (w.reg, w.value) {
                (0x58, v) => format!("size {v:x}"),
                (0x96, 0x8400) => "dmacon 8400".into(),
                (0x96, 0x0400) => "dmacon 0400".into(),
                (0x9a, 0x4000) => "intena off".into(),
                (0x9a, 0xc000) => "intena on".into(),
                (0x9c, 0x20) => "ack".into(),
                _ => continue,
            };
            println!("O {f} t {} {} {}:{} {tag}", t(wf, w.vpos, w.hpos), wf, w.vpos, w.hpos);
        }
        for i in am.irqs() {
            if i.level == 3 {
                println!(
                    "O {f} irq {}{}:{}",
                    if i.next { "+" } else { "" },
                    i.vpos,
                    i.hpos
                );
            }
        }
        prev_end = am.frame() as i64;
        if am.frames_run() == 2 {
            println!("O {f} two frames");
        }
    }
}
