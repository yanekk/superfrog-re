//! What times the original's blits (T51): every custom-register write the CPU makes, the blitter's
//! begin, free and end events (core patch 0005) and the frames a step ran. Needs what
//! `determinism.rs` needs.

use sf_oracle::*;

/// A disk-3 boot (the story intro, as conformance's `power-on-disk3`) has loaded and is drawing
/// the first scene's objects by this frame: 20 blits in frame 3 022 (measured, T51).
const SCENE: std::ops::Range<u32> = 2_900..3_100;

const BLTSIZE: u16 = 0x58;

#[test]
fn a_frame_of_the_intro_has_its_bltsize_writes_and_each_blits_begin_free_end() {
    let cfg = OracleConfig::locate(DEFAULT_MAX_FRAMES)
        .unwrap_or_else(|e| panic!("{e}"))
        .with_boot(DiskSlot::D3);
    let mut a = Amiga::power_on(&cfg).unwrap();
    for _ in 0..SCENE.start {
        a.step(FrameInput::NONE).unwrap();
    }
    // The first frame drawing ten or more objects, with nothing of it run into the next frame.
    let (sizes, events) = SCENE
        .clone()
        .find_map(|_| {
            a.step(FrameInput::NONE).unwrap();
            let sizes: Vec<CpuWrite> = a
                .cpu_writes()
                .into_iter()
                .filter(|w| w.reg == BLTSIZE)
                .collect();
            let events = a.blit_events();
            (sizes.len() >= 10 && events.iter().all(|e| !e.next)).then_some((sizes, events))
        })
        .expect("a frame of the intro's first scene with ten blits");
    assert_eq!(a.frames_run(), 1);

    // The CPU starts every blit: BLTSIZE as the shim's blit starts see it, write for write.
    let starts: Vec<(u16, u16, u16)> = sizes.iter().map(|w| (w.value, w.vpos, w.hpos)).collect();
    assert_eq!(starts, a.blit_starts());

    // Each blit begins, frees its busy flag and ends, in that order, after its BLTSIZE write.
    assert_eq!(events.len(), 3 * sizes.len());
    for (blit, (w, ev)) in sizes.iter().zip(events.chunks(3)).enumerate() {
        let kinds: Vec<BlitEventKind> = ev.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            [
                BlitEventKind::Begin,
                BlitEventKind::Free,
                BlitEventKind::End
            ],
            "blit {blit}"
        );
        let beams = [
            (w.vpos, w.hpos),
            (ev[0].vpos, ev[0].hpos),
            (ev[1].vpos, ev[1].hpos),
            (ev[2].vpos, ev[2].hpos),
        ];
        assert!(
            beams[0] < beams[1] && beams.is_sorted(),
            "blit {blit}: {w:?} {ev:?}"
        );
    }
    // Other registers are kept too: the blits are set up through BLTCON0 before each starts.
    assert!(a.cpu_writes().iter().any(|w| w.reg == 0x40));
}
