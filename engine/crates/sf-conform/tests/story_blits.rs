//! The intro's blits on the original's bus (T89, T90): `story::sim::Sim` on the fixed DMA of
//! `copper::run_bus` over the original's own copper list, poked with the original's own blitter
//! writes, against the original's begin, free and end.

use sf_conform::{Case, sync};
use sf_core::front::story::sim::{self, BlitEventKind, Fixed, Sim};
use sf_core::layout::{AmigaMemory, Ram};
use sf_core::render::copper::{self, Fetch};

/// Scene 1's five mask blits, by the oracle's frame count after the step that runs them
/// (measured, T52).
const MASKS: std::ops::Range<u32> = 3_016..3_021;
/// The first draw's frame: the restores, then the objects (measured, T52).
const DRAW: u32 = 3_027;
/// Frames kept from here on: COP1LC is picked up from the CPU's writes (it sets it in 3 005).
const FROM: u32 = 3_000;

const BLTCON0: u16 = 0x040;
const BLTSIZE: u16 = 0x058;
const DMACON: u16 = 0x096;
const COP1LCH: u16 = 0x080;
const COP1LCL: u16 = 0x082;

/// One oracle frame: the memory it starts with, the CPU's writes and the blitter's events in it.
struct Frame {
    mem: AmigaMemory,
    cop1lc: u32,
    writes: Vec<sf_oracle::CpuWrite>,
    events: Vec<sf_oracle::BlitEvent>,
}

/// One frame of the copper: its bus cycles and writes over the list at `cop1lc` in `mem`, and the
/// bitplane set-up it started with.
struct CopperFrame {
    bus: Vec<copper::Beam>,
    writes: Vec<copper::Write>,
    fetch: Fetch,
}

fn copper_frame(mem: &AmigaMemory, cop1lc: u32, fetch: Fetch) -> CopperFrame {
    let mut bus = Vec::new();
    let word = |a: u32| u16::from_be_bytes([mem.byte(a), mem.byte(a + 1)]);
    let writes = copper::run_bus(&|a, _, _| word(a), cop1lc, fetch, &mut bus);
    CopperFrame { bus, writes, fetch }
}

#[test]
fn scene_1s_masks_and_the_first_draw_begin_free_and_end_on_the_originals_cycles() {
    let case = Case::load(&sf_conform::cases_dir().join("story-intro.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    let synced = sync::reach(&case.sync, &case.script, DRAW + 20).unwrap();
    let mut amiga = synced.amiga;
    let mut frames = Vec::new();
    let mut cop1lc = 0u32;
    let mut of = 0;
    while amiga.frame() <= DRAW {
        let frame = amiga.frame() + 1;
        let mem = (frame >= FROM).then(|| sync::core_memory(amiga.memory()));
        sync::step_inserting(&mut amiga, case.script.input_at(of)).unwrap();
        of += 1;
        let writes = amiga.cpu_writes();
        let start = cop1lc;
        for w in &writes {
            match w.reg {
                COP1LCH => cop1lc = cop1lc & 0xffff | (w.value as u32) << 16,
                COP1LCL => cop1lc = cop1lc & 0xffff_0000 | w.value as u32,
                _ => {}
            }
        }
        if let Some(mem) = mem {
            frames.push(Frame {
                mem,
                cop1lc: start,
                writes,
                events: amiga.blit_events(),
            });
        }
    }

    // The bitplane set-up carried from frame to frame, from the first frame run on.
    let mut fetch = Fetch::default();
    let mut coppers = Vec::new();
    for f in &frames {
        let c = copper_frame(&f.mem, f.cop1lc, fetch);
        for w in &c.writes {
            fetch.write(w.reg, w.value);
        }
        coppers.push(c);
    }

    let line = sf_core::front::story::cpu::LINE;
    use sim::dmacon::*;
    // BLTCON0 and DMACON as the frames before left them (blitter DMA on before the first).
    let (mut con0, mut dma) = (0u16, DMAEN | BLTEN);
    let mut checked = 0;
    for (k, f) in frames.iter().enumerate() {
        let frame = FROM + k as u32;
        let (con0_in, dma_in) = (con0, dma);
        for w in &f.writes {
            match w.reg {
                BLTCON0 => con0 = w.value,
                DMACON if w.value & 0x8000 != 0 => dma |= w.value & 0x7ff,
                DMACON => dma &= !w.value,
                _ => {}
            }
        }
        if !MASKS.contains(&frame) && frame != DRAW {
            continue;
        }
        let sizes: Vec<u16> = f
            .writes
            .iter()
            .filter(|w| w.reg == BLTSIZE)
            .map(|w| w.value)
            .collect();
        if frame != DRAW {
            assert_eq!(sizes, [0x0014], "oracle frame {frame}: one mask blit");
        } else {
            // The restores, then the objects: 40 blits. BLTPRI from before the first to past the
            // last, so the CPU never holds a blit up (T53 needs no BLS here).
            assert_eq!(sizes.len(), 40, "oracle frame {frame}: the first draw");
            let mut d = dma_in;
            for w in &f.writes {
                match w.reg {
                    DMACON if w.value & 0x8000 != 0 => d |= w.value & 0x7ff,
                    DMACON => d &= !w.value,
                    BLTSIZE => assert!(d & BLTPRI != 0, "oracle frame {frame}: BLTPRI"),
                    _ => {}
                }
            }
        }
        // This frame's copper and the next's, one after the other.
        let bus = coppers[k..=k + 1]
            .iter()
            .fold(Fixed::new(), |fx, c| fx.frame(&c.bus, &c.writes, c.fetch));
        let mut s = Sim::new(0, &bus, dma_in);
        s.poke(0, BLTCON0, con0_in);
        for w in &f.writes {
            if matches!(w.reg, BLTCON0 | BLTSIZE | DMACON) {
                s.poke(w.vpos as u32 * line + w.hpos as u32, w.reg, w.value);
            }
        }
        s.finish(2 * sim::FRAME);
        let ours: Vec<(BlitEventKind, u32)> = s.events.iter().map(|e| (e.kind, e.t)).collect();
        let theirs: Vec<(BlitEventKind, u32)> = f
            .events
            .iter()
            .map(|e| {
                let kind = match e.kind {
                    sf_oracle::BlitEventKind::Begin => BlitEventKind::Begin,
                    sf_oracle::BlitEventKind::Free => BlitEventKind::Free,
                    sf_oracle::BlitEventKind::End => BlitEventKind::End,
                };
                let t = e.vpos as u32 * line + e.hpos as u32;
                (kind, if e.next { t + sim::FRAME } else { t })
            })
            .collect();
        assert_eq!(
            theirs.len(),
            3 * sizes.len(),
            "oracle frame {frame}: {theirs:?}"
        );
        assert_eq!(ours, theirs, "oracle frame {frame}");
        checked += 1;
    }
    assert_eq!(checked, MASKS.len() + 1);
}
