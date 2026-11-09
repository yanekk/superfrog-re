//! Game-free: `Amiga::set_profile` / `line_samples` leave the run unchanged, on Kickstart 1.3 with
//! no disk as `tests/exec_counts.rs` boots it, under a workload of the test's own whose blits hold
//! the CPU off the bus across the frame's end. Needs the Kickstart (`AMIGA_KICKSTART`, else
//! `~/amiga-roms/kick1.3.rom`) and `toolkit/tools/setup/oracle.sh`'s build.

use amiga_oracle::*;

/// The insert-disk screen is drawn from frame 207 (`tests/boot.rs`); from here on the CPU idles
/// between vertical blanks.
const IDLE: u32 = 240;

/// Frames run with the workload: the sweep below twice over.
const FRAMES: u32 = 520;

/// A window inside Kickstart's display, for re-cutting the last frame.
const KS_WINDOW: (u16, u16) = (0x2C81, 0x2CC1);

const L3: u32 = 0x4_0000;
const COUNTER: u32 = 0x4_0400;

/// A level-3 handler: from line 302 it starts a blit A to D of 2 words by `540 + (n & 255)` rows,
/// `n` counting its runs, with the blitter given priority (BLTPRI) and sprite DMA on, and fetches
/// its next instruction from chip RAM, so the CPU waits for the blit's end. Each frame the blit is a
/// row longer, so that end, and the CPU's next instruction end, sweeps across the frame's end: past
/// line 312's end and before line 0's HSYNC (h $12), where the vertical sync's flag is raised, on
/// some frames (T78: a line sample pausing there ended the step early).
fn l3_handler() -> Vec<u16> {
    vec![
        0x48E7, 0xC080, //                         movem.l d0-d1/a0,-(sp)
        0x33FC, 0x0020, 0x00DF, 0xF09C, //         move.w #$0020,$dff09c   clear VERTB
        0x33FC, 0x0020, 0x00DF, 0xF09C, //         (twice, as the hardware wants)
        0x0839, 0x0000, 0x00DF, 0xF005, // wait:   btst #0,$dff005         V8: line 256 on
        0x67F6, //                                 beq.s wait
        0x0C39, 0x002E, 0x00DF, 0xF006, //         cmpi.b #$2e,$dff006     line 302 on
        0x65EC, //                                 bcs.s wait
        0x33FC, 0x8660, 0x00DF,
        0xF096, //         move.w #$8660,$dff096   BLTPRI DMAEN BLTEN SPREN
        0x33FC, 0x09F0, 0x00DF, 0xF040, //         move.w #$09f0,$dff040   BLTCON0: A, D = A
        0x33FC, 0x0000, 0x00DF, 0xF042, //         move.w #0,$dff042       BLTCON1
        0x33FC, 0xFFFF, 0x00DF, 0xF044, //         move.w #$ffff,$dff044   BLTAFWM
        0x33FC, 0xFFFF, 0x00DF, 0xF046, //         move.w #$ffff,$dff046   BLTALWM
        0x23FC, 0x0005, 0x0000, 0x00DF, 0xF050, // move.l #$50000,$dff050  BLTAPT
        0x23FC, 0x0006, 0x0000, 0x00DF, 0xF054, // move.l #$60000,$dff054  BLTDPT
        0x33FC, 0x0000, 0x00DF, 0xF064, //         move.w #0,$dff064       BLTAMOD
        0x33FC, 0x0000, 0x00DF, 0xF066, //         move.w #0,$dff066       BLTDMOD
        0x3039, 0x0004, 0x0400, //                 move.w $40400,d0
        0x5240, //                                 addq.w #1,d0
        0x33C0, 0x0004, 0x0400, //                 move.w d0,$40400
        0x0240, 0x00FF, //                         andi.w #$ff,d0
        0x0640, 0x021C, //                         addi.w #540,d0
        0xED48, //                                 lsl.w #6,d0
        0x5440, //                                 addq.w #2,d0           2 words wide
        0x33C0, 0x00DF, 0xF058, //                 move.w d0,$dff058       BLTSIZE
        0x33FC, 0x0400, 0x00DF, 0xF096, //         move.w #$0400,$dff096   BLTPRI off (waits)
        0x4CDF, 0x0103, //                         movem.l (sp)+,d0-d1/a0
        0x4E73, //                                 rte
    ]
}

fn bytes(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_be_bytes()).collect()
}

fn with_workload() -> Amiga {
    let rom = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    let mut a = Amiga::power_on(&OracleConfig::new(rom, Vec::new(), IDLE + FRAMES + 2)).unwrap();
    for _ in 0..IDLE {
        a.step(FrameInput::NONE).unwrap();
    }
    a.write_memory(L3, &bytes(&l3_handler()));
    a.write_memory(COUNTER, &[0, 0]);
    a.write_memory(0x6C, &L3.to_be_bytes());
    a
}

/// Every frame's chip and slow RAM, picture and CPU registers.
fn state(a: &Amiga, c: FrameCapture) -> (AmigaMemory, Rgb12Frame, CpuRegs) {
    (a.memory(), c.picture, a.cpu())
}

/// A run sampled line by line on some steps and not on others, switched between steps, equals the
/// run never sampled, every frame, in chip and slow RAM, picture and CPU registers, with blits holding
/// the CPU across the frame's end and sprites on; so are its interrupts, blits and custom-register
/// writes, with their beam positions. Before T78 it differed (the sampled step ended an
/// instruction early where a line's pause came between the frame's end and line 0's HSYNC).
#[test]
fn sampling_does_not_change_the_run() {
    let mut plain = with_workload();
    let mut seen = with_workload();
    let (mut sampled, mut profiling, mut wrapped) = (0, false, 0);
    for f in 0..FRAMES {
        // On for two runs of seven steps in three, so it is switched on and off mid-run; the
        // steps whose blit ends at the frame's end (30 to 32 and 286 to 288) are sampled.
        let sample = (f / 7) % 3 != 0;
        if sample != profiling {
            seen.set_profile(sample);
            profiling = sample;
            // Switching leaves the frame just drawn as it was (before T78 it was cleared).
            assert!(
                seen.picture_in(KS_WINDOW.0, KS_WINDOW.1)
                    == plain.picture_in(KS_WINDOW.0, KS_WINDOW.1),
                "frame {f}: switching sampling changed the last picture"
            );
        }
        let p = plain.step(FrameInput::NONE).unwrap();
        let s = seen.step(FrameInput::NONE).unwrap();
        // A sampled step whose blit ends from line 312's last few slots to line 0's HSYNC: the
        // CPU's stalled instruction then ends past the frame's end and short of the vertical
        // sync's flag (measured: the instruction ends about 6 slots after the blit).
        wrapped += (sample
            && plain.blit_events().iter().any(|e| {
                e.kind == BlitEventKind::End
                    && ((!e.next && e.vpos == 312 && e.hpos >= 222)
                        || (e.next && e.vpos == 0 && e.hpos < 0x12))
            })) as u32;
        if sample {
            let n = seen.line_samples().len();
            assert!(n > 200, "frame {f}: {n} line samples");
            sampled += 1;
        } else {
            assert!(
                seen.line_samples().is_empty(),
                "frame {f}: samples while off"
            );
        }
        let p = state(&plain, p);
        assert!(
            p == state(&seen, s),
            "frame {f}: sampling lines changed the run"
        );
        assert_eq!(plain.irqs(), seen.irqs(), "frame {f}: interrupts");
        assert_eq!(plain.blit_events(), seen.blit_events(), "frame {f}: blits");
        assert_eq!(
            plain.cpu_writes(),
            seen.cpu_writes(),
            "frame {f}: custom-register writes"
        );
        assert_eq!(
            plain.frames_run(),
            seen.frames_run(),
            "frame {f}: frames run"
        );
    }
    assert!(sampled > FRAMES / 2, "{sampled} sampled");
    assert!(
        wrapped > 0,
        "no blit ended between the frame's end and the vertical sync"
    );
}
