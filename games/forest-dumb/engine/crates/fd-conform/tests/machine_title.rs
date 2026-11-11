//! The machine's picture against the original's (translator T24): the original is run from
//! power-on to the title, and at the end of the title's first whole frame (4922) the machine is
//! given its RAM and the custom registers the CPU had written; the machine's next two frames, the
//! title's copper list run and drawn, equal the original's 4923 and 4924 pixel for pixel. No
//! translated code runs: the title's picture is the copper's and Denise's work alone. Needs the
//! disks and the Kickstart; full.
//!
//! Why 4922 and not 4921: the CPU strobes COPJMP1 with the title's list on line 3 of 4921, and in
//! 4922 still writes 185 bytes of chip RAM, part of the picture (a 32 x 15 box at (280, 195)); a
//! snapshot taken before them cannot show them. From 4923 one byte a frame changes, outside the
//! picture (measured 2026-11-12).

use amiga_chips::to_rgb12;
use amiga_conform::script::{self, InputScript};
use amiga_oracle::{Amiga, OracleConfig, Rgb12Frame, kickstart_path};
use fd_conform::{conform_dir, disks};
use pcode_machine::{Config, Machine, Picture, custom, irq};

/// boot-to-title's disk change, which `boot-to-level1` shares.
const SWAP: u32 = 3600;
/// The title's first whole frame: the snapshot is taken at its end (measured 2026-11-12).
const SNAP: u32 = 4922;

fn same(ours: &Picture, theirs: &Rgb12Frame, frame: u32) {
    assert_eq!(
        (ours.left, ours.top, ours.width, ours.height),
        (theirs.left as i32, theirs.top, theirs.width, theirs.height),
        "frame {frame}: the window"
    );
    let ours = to_rgb12(ours);
    let differ = ours
        .iter()
        .zip(&theirs.pixels)
        .filter(|(a, b)| a != b)
        .count();
    let first = ours
        .iter()
        .zip(&theirs.pixels)
        .position(|(a, b)| a != b)
        .map(|i| (i % theirs.width as usize, i / theirs.width as usize));
    assert_eq!(
        differ, 0,
        "frame {frame}: pixels differ, the first at {first:?}"
    );
}

#[test]
fn the_titles_first_picture_from_a_snapshot_equals_the_originals() {
    let path = conform_dir().join("scripts/boot-to-level1.txt");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("script {}: {e}", path.display()));
    let sc: InputScript = script::parse(&text).unwrap_or_else(|e| panic!("{e}"));
    let kickstart = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    let disks = disks::load().unwrap_or_else(|e| panic!("{e}"));
    let mut am = Amiga::power_on(&OracleConfig::new(kickstart, disks, SNAP + 3))
        .unwrap_or_else(|e| panic!("{e}"));
    // The registers as the CPU leaves them: the last word written to each, and the set/clear
    // registers' state. The copper's writes are not reported; the title's list makes them again.
    let mut regs = [0u16; 256];
    let (mut dmacon, mut intena, mut intreq) = (0u16, 0u16, 0u16);
    for frame in 0..=SNAP {
        if frame == SWAP {
            am.insert_disk(0, 1);
        }
        am.step(script::input_at(&sc.lines, frame))
            .unwrap_or_else(|e| panic!("{e}"));
        for w in am.cpu_writes() {
            let reg = w.reg & 0x1FE;
            regs[reg as usize >> 1] = w.value;
            match reg {
                custom::DMACON => dmacon = irq::set_clear(dmacon, w.value),
                irq::INTENA => intena = irq::set_clear(intena, w.value),
                irq::INTREQ => intreq = irq::set_clear(intreq, w.value),
                _ => {}
            }
        }
    }
    regs[custom::DMACONR as usize >> 1] = dmacon & 0x07FF;
    regs[irq::INTENAR as usize >> 1] = intena;
    regs[irq::INTREQR as usize >> 1] = intreq;
    // A step ends as the next frame's vertical blank begins: the beam at line 0.
    regs[custom::VPOSR as usize >> 1] = 0x8000;
    regs[custom::VHPOSR as usize >> 1] = 0;
    let mem = am.memory();
    let ram: Vec<u8> = mem.chip.iter().chain(&mem.slow).copied().collect();
    let mut m = Machine::from_snapshot(Config::a500(vec![]), &ram, &regs);
    for frame in SNAP + 1..=SNAP + 2 {
        let theirs = am
            .step(script::input_at(&sc.lines, frame))
            .unwrap_or_else(|e| panic!("{e}"))
            .picture;
        let ours = m.end_frame();
        assert_eq!(m.fault(), None, "frame {frame}");
        same(&ours.picture, &theirs, frame);
    }
}
