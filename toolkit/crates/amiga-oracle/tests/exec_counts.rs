//! Game-free: `Amiga::watch_pcs` / `pc_hits` (core patch 0006) on Kickstart 1.3 with no disk, as
//! `tests/boot.rs` boots it. Needs the Kickstart (`AMIGA_KICKSTART`, else `~/amiga-roms/kick1.3.rom`)
//! and `toolkit/tools/setup/oracle.sh`'s build.

use amiga_oracle::*;

/// The insert-disk screen is drawn from frame 207 (`tests/boot.rs`); from here on the CPU idles
/// between vertical blanks.
const IDLE: u32 = 240;

fn no_disk(frames: u32) -> OracleConfig {
    let rom = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    OracleConfig::new(rom, Vec::new(), frames)
}

fn idle(frames: u32) -> Amiga {
    let mut a = Amiga::power_on(&no_disk(IDLE + frames + 2)).unwrap();
    for _ in 0..IDLE {
        a.step(FrameInput::NONE).unwrap();
    }
    a
}

/// The level-3 autovector's target: Kickstart's level-3 handler, read from RAM rather than
/// assumed. On the idle screen its only level-3 interrupt is the vertical blank, and a step ends
/// just after that interrupt's exception, on the handler's first instruction (measured 2026-11-05).
fn level3_handler(a: &Amiga) -> u32 {
    a.memory().long(0x6C).unwrap()
}

fn count(a: &Amiga, pc: u32) -> u32 {
    a.pc_hits().iter().find(|h| h.0 == pc).unwrap().1
}

/// A short test, labelled smoke: Kickstart's level-3 handler runs once a frame and an address no
/// instruction starts at counts zero.
#[test]
fn the_vertical_blank_handler_counts_once_a_frame_and_an_unreached_address_zero() {
    let mut a = idle(300);
    let vbl = level3_handler(&a);
    // An address in the zeroed chip RAM no code runs from (the idle screen leaves $D838 on zero).
    let never = 0x0004_0000;
    a.watch_pcs(&[never, vbl, vbl]);
    assert_eq!(a.pc_hits(), &[(never, 0), (vbl, 0)], "sorted, distinct, zero until a step");
    for f in 0..300 {
        a.step(FrameInput::NONE).unwrap();
        assert_eq!(a.pc_hits(), &[(never, 0), (vbl, 1)], "frame {f} after the idle start");
    }
    a.watch_pcs(&[]);
    a.step(FrameInput::NONE).unwrap();
    assert!(a.pc_hits().is_empty(), "an empty list turns it off");
}

// A level-3 handler of the test's own at $40000 that asks for a level-1 (software) interrupt and
// then lowers the mask in front of a run of NOPs, so that interrupt pre-empts one of them; and a
// level-1 handler at $40100 that records the PC it interrupted at $40200.
const L3: u32 = 0x4_0000;
const L1: u32 = 0x4_0100;
const PREEMPTED: u32 = 0x4_0200;
const NOPS: u32 = 16;
const NOP0: u32 = L3 + 0x24;

fn l3_handler() -> Vec<u16> {
    let mut code = vec![
        0x33FC, 0x8004, 0x00DF, 0xF09A, // move.w #$8004,$dff09a   INTENA: SOFTINT on
        0x33FC, 0x0070, 0x00DF, 0xF09C, // move.w #$0070,$dff09c   clear the level-3 requests
        0x33FC, 0x0070, 0x00DF, 0xF09C, // (twice, as the hardware wants)
        0x33FC, 0x8004, 0x00DF, 0xF09C, // move.w #$8004,$dff09c   request SOFTINT, level 1
        0x46FC, 0x2000, //                 move.w #$2000,sr        let it in
    ];
    code.extend(std::iter::repeat_n(0x4E71, NOPS as usize)); // nop
    code.push(0x4E73); // rte
    code
}

fn l1_handler() -> Vec<u16> {
    vec![
        0x23EF, 0x0002, 0x0004, 0x0200, // move.l 2(sp),$40200     the interrupted PC
        0x33FC, 0x0007, 0x00DF, 0xF09C, // move.w #$0007,$dff09c   clear the level-1 requests
        0x33FC, 0x0007, 0x00DF, 0xF09C,
        0x4E73, // rte
    ]
}

fn bytes(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_be_bytes()).collect()
}

#[test]
fn an_instruction_an_interrupt_pre_empts_counts_once() {
    let mut a = idle(60);
    a.write_memory(L3, &bytes(&l3_handler()));
    a.write_memory(L1, &bytes(&l1_handler()));
    a.write_memory(0x64, &L1.to_be_bytes());
    a.write_memory(0x6C, &L3.to_be_bytes());
    // The step ended with this frame's vertical blank already taken through the old vector.
    a.step(FrameInput::NONE).unwrap();
    let l3: Vec<u32> = [L3, L3 + 8, L3 + 16, L3 + 24, L3 + 32]
        .into_iter()
        .chain((0..NOPS).map(|k| NOP0 + 2 * k))
        .chain([NOP0 + 2 * NOPS])
        .collect();
    let watched: Vec<u32> = l3.iter().copied().chain([L1]).collect();
    a.watch_pcs(&watched);
    let mut preempted_nops = std::collections::BTreeSet::new();
    for f in 0..60 {
        a.write_memory(PREEMPTED, &[0; 4]);
        a.step(FrameInput::NONE).unwrap();
        let entries = count(&a, L3);
        assert!(entries >= 1, "frame {f}: the test's level-3 handler ran");
        for &pc in &l3 {
            assert_eq!(count(&a, pc), entries, "frame {f}: {pc:#x} once per entry");
        }
        assert_eq!(count(&a, L1), entries, "frame {f}: one level-1 interrupt per entry");
        assert!(a.irqs().iter().any(|i| i.level == 1), "frame {f}");
        let at = a.memory().long(PREEMPTED).unwrap();
        assert!(
            (NOP0..NOP0 + 2 * NOPS).contains(&at),
            "frame {f}: the level-1 interrupt pre-empted a NOP, not {at:#x}"
        );
        preempted_nops.insert(at);
    }
    // Each of those pre-empted NOPs was counted once per entry above, not once more for the
    // pre-emption.
    assert!(!preempted_nops.is_empty());
}

/// Every frame's chip and slow RAM, picture and CPU registers.
fn state(a: &Amiga, c: FrameCapture) -> (AmigaMemory, Rgb12Frame, CpuRegs) {
    (a.memory(), c.picture, a.cpu())
}

#[test]
fn watching_does_not_change_the_run() {
    const FRAMES: u32 = 3000;
    let cfg = no_disk(FRAMES);
    let mut plain = Amiga::power_on(&cfg).unwrap();
    let mut empty = Amiga::power_on(&cfg).unwrap();
    empty.watch_pcs(&[]);
    let mut full = Amiga::power_on(&cfg).unwrap();
    // 200 addresses through the Kickstart ROM, the vertical-blank handler among them once known.
    let mut pcs: Vec<u32> = (0..199).map(|k| 0xFC_0000 + k * 0x400).collect();
    full.watch_pcs(&pcs);
    let mut hits = 0u64;
    for f in 0..FRAMES {
        let p = plain.step(FrameInput::NONE).unwrap();
        let e = empty.step(FrameInput::NONE).unwrap();
        let w = full.step(FrameInput::NONE).unwrap();
        let p = state(&plain, p);
        assert!(p == state(&empty, e), "frame {f}: an empty watch list changed the run");
        assert!(p == state(&full, w), "frame {f}: a full watch list changed the run");
        hits += full.pc_hits().iter().map(|h| u64::from(h.1)).sum::<u64>();
        if f == IDLE {
            pcs.push(level3_handler(&full));
            full.watch_pcs(&pcs);
        }
    }
    assert!(hits > FRAMES as u64 / 2, "the full list saw the run: {hits} hits");
}

#[test]
fn a_restored_run_counts_as_the_run_that_took_the_snapshot() {
    const MID: u32 = 120;
    const AFTER: u32 = 150;
    let cfg = no_disk(MID + AFTER + 40);
    // Addresses through the ROM, and the vertical-blank handler, so the boot's work is seen.
    let mut pcs: Vec<u32> = (0..199).map(|k| 0xFC_0000 + k * 0x400).collect();
    let mut probe = Amiga::power_on(&cfg).unwrap();
    for _ in 0..MID {
        probe.step(FrameInput::NONE).unwrap();
    }
    pcs.push(level3_handler(&probe));

    let counts = |a: &mut Amiga| -> Vec<Vec<(u32, u32)>> {
        (0..AFTER)
            .map(|_| {
                a.step(FrameInput::NONE).unwrap();
                a.pc_hits().to_vec()
            })
            .collect()
    };
    let mut a = Amiga::power_on(&cfg).unwrap();
    a.watch_pcs(&pcs);
    for _ in 0..MID {
        a.step(FrameInput::NONE).unwrap();
    }
    let snap = a.snapshot();
    let whole = counts(&mut a);
    assert!(whole.iter().flatten().any(|h| h.1 > 0), "the list saw the run");

    a.restore(&snap);
    assert!(a.pc_hits().iter().all(|h| h.1 == 0), "nothing counted since the restore");
    assert_eq!(counts(&mut a), whole, "restored in place");

    let mut b = Amiga::power_on(&cfg).unwrap();
    b.watch_pcs(&pcs);
    b.restore(&snap);
    assert_eq!(counts(&mut b), whole, "restored into a new machine");
}
