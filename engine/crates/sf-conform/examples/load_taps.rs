//! T30's probe: replay the whole-game script on the original (as `chip_probe`) and, from the first
//! frame at or after `from` whose Paula writes turn DMA off and on again (`DMACON $0000`, then
//! `$8000`: a load's end), print `before` frames earlier through `after` frames later each
//! channel's DAC range (vAmiga's taps) and the frame's channel starts.
//! `load_taps <script.txt> <from> <before> <after>`
use sf_conform::InputScript;
use sf_conform::sync::{Disks, core_memory, step_inserting};
use sf_disk::DiskSlot;
use sf_oracle::{Amiga, OracleConfig};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let s = InputScript::load(std::path::Path::new(&args[0])).expect("script");
    let n = |i: usize| -> u32 { args[i].parse().unwrap() };
    let (from, before, after) = (n(1), n(2), n(3));
    let cfg = OracleConfig::locate(400_000).unwrap().with_boot(DiskSlot::D1).with_drives(2);
    let mut a = Amiga::power_on(&cfg).unwrap();
    let mut disks: Option<Disks> = None;
    let mut lines: std::collections::VecDeque<String> = Default::default();
    let mut left: Option<u32> = None;
    for f in 0..400_000 {
        let c = step_inserting(&mut a, s.input_at(f)).unwrap();
        let m = core_memory(a.memory());
        disks.get_or_insert_with(|| Disks::at(&m)).serve(&mut a);
        let mut range = [(i16::MAX, i16::MIN); 4];
        for t in c.taps() {
            let r = &mut range[t.channel as usize & 3];
            *r = (r.0.min(t.value), r.1.max(t.value));
        }
        let ch: Vec<String> = range
            .iter()
            .map(|r| if r.0 > r.1 { "-".into() } else { format!("{}..{}", r.0, r.1) })
            .collect();
        let w: Vec<String> = c
            .paula
            .iter()
            .filter(|w| w.reg == 0x96 || matches!(w.reg, 0xa2 | 0xb2 | 0xc2 | 0xd2 | 0xa4 | 0xb4 | 0xc4 | 0xd4))
            .map(|w| format!("{:02x}={:04x}", w.reg, w.value))
            .collect();
        lines.push_back(format!("original {f}: ch [{}] {}", ch.join(" "), w.join(" ")));
        if lines.len() > before as usize + 1 {
            lines.pop_front();
        }
        if let Some(k) = left.as_mut() {
            println!("{}", lines.back().unwrap());
            *k -= 1;
            if *k == 0 {
                return;
            }
            continue;
        }
        let off = c.paula.iter().position(|w| w.reg == 0x96 && w.value == 0);
        if f >= from
            && let Some(i) = off
            && c.paula[i..].iter().any(|w| w.reg == 0x96 && w.value == 0x8000)
        {
            for l in &lines {
                println!("{l}");
            }
            left = Some(after);
        }
    }
}
