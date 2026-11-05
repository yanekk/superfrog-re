//! The measurements of play in W1-1 that later tasks rest on (docs/09-play-measured.md), kept as
//! tests so a change to the oracle or the boot that moves them fails here. Needs the disks and the
//! Kickstart: absent, it fails naming where it looked.

use amiga_conform::script::{self, InputScript};
use amiga_oracle::{Amiga, OracleConfig, kickstart_path};
use fd_conform::{HUNK_BASES, conform_dir, disks};

/// Frames run before the first frame of play: the SCENA 1 fire is held in step 6300, whose frame
/// already runs the first level-3 interrupt.
const SYNC: u32 = 6301;
/// boot-to-title's disk change, which `boot-to-level1` shares.
const SWAP: u32 = 3600;

const IRQ_COUNT: u32 = 0x1e6a;
const GAME_RUNNING: u32 = 0x1ed4;
const WORLD_INDEX: u32 = 0x6dfe;
const GAME_FRAME_TICK: u32 = 0x20ac;
const CHAR_X: u32 = 0x1e5a;
const CHAR_Y: u32 = 0x1e5e;

fn h0(offset: u32) -> u32 {
    HUNK_BASES[0] + offset
}

fn word(am: &Amiga, addr: u32) -> u16 {
    let b = am.peek(addr, 2);
    u16::from_be_bytes([b[0], b[1]])
}

fn long(am: &Amiga, addr: u32) -> u32 {
    let b = am.peek(addr, 4);
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

/// `boot-to-level1` followed by `after` (frames from `SYNC` on, in the script format).
fn script(after: &str) -> InputScript {
    let path = conform_dir().join("scripts/boot-to-level1.txt");
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("script {}: {e}", path.display()));
    script::parse(&format!("{text}\n{after}")).unwrap_or_else(|e| panic!("{e}"))
}

/// Power on and run `frames` frames of `sc`, swapping disk 2 in as the case format does.
fn boot(sc: &InputScript, frames: u32, cap: u32) -> Amiga {
    let kickstart = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    let disks = disks::load().unwrap_or_else(|e| panic!("{e}"));
    let mut am = Amiga::power_on(&OracleConfig::new(kickstart, disks, cap))
        .unwrap_or_else(|e| panic!("{e}"));
    run(&mut am, sc, frames);
    am
}

fn run(am: &mut Amiga, sc: &InputScript, to: u32) {
    for frame in am.frame()..to {
        if frame == SWAP {
            am.insert_disk(0, 1);
        }
        am.step(script::input_at(&sc.lines, frame))
            .unwrap_or_else(|e| panic!("{e}"));
    }
}

/// The sync point is play's first frame, and two boots reach it with the same memory.
#[test]
fn the_script_reaches_level_1_twice_with_identical_ram_at_the_sync_frame() {
    let sc = script("");
    let (a, b) = (boot(&sc, SYNC, SYNC), boot(&sc, SYNC, SYNC));
    for am in [&a, &b] {
        assert_eq!(
            word(am, h0(IRQ_COUNT)),
            1,
            "one interrupt of play at the sync frame"
        );
        assert_eq!(word(am, h0(GAME_RUNNING)), 1);
        assert_eq!(long(am, h0(WORLD_INDEX)), 0, "world 1");
        assert_eq!(
            (long(am, h0(CHAR_X)), long(am, h0(CHAR_Y))),
            (0x70, 0x70),
            "world 1's start, set by load_list_world_1"
        );
    }
    let (ma, mb) = (a.memory(), b.memory());
    assert!(ma.chip == mb.chip, "chip RAM differs between two boots");
    assert!(ma.slow == mb.slow, "slow RAM differs between two boots");
    // The frame before has not started play: SCENA 1 is still waiting.
    let before = boot(&sc, SYNC - 1, SYNC - 1);
    assert_eq!(word(&before, h0(IRQ_COUNT)), 0);
}

/// Right held from frame N moves the character in the frame that holds it: the state after N+1
/// steps, and not after N.
#[test]
fn an_input_on_frame_n_moves_the_character_on_frame_n_plus_1_not_sooner() {
    const N: u32 = 6400;
    let idle = script("");
    let right = script(&format!("{N}: R\n{}: -\n", N + 20));
    // Two boots rather than a snapshot: a restored snapshot is not a boot this file measures.
    let mut a = boot(&idle, N - 10, N + 5);
    let mut b = boot(&right, N - 10, N + 5);
    for frame in N - 10..N + 2 {
        let (xa, xb) = (long(&a, h0(CHAR_X)), long(&b, h0(CHAR_X)));
        if frame <= N {
            assert_eq!(xa, xb, "x moved at frame {frame}, before the input");
            assert_eq!(
                a.memory().slow,
                b.memory().slow,
                "hunk 0 differs at frame {frame}"
            );
        } else {
            assert_eq!(xb, xa + 1, "x at frame {frame}: one pixel right of {xa:#x}");
        }
        a.step(script::input_at(&idle.lines, frame)).unwrap();
        b.step(script::input_at(&right.lines, frame)).unwrap();
    }
}

/// Over 3 000 frames of moving, jumping play: one copper interrupt a frame, `irq_count` +1 a
/// frame, and `game_frame` once per interrupt, though its tail sometimes ends in the next frame.
#[test]
fn irq_count_and_game_frame_run_once_a_frame_over_3000_frames_of_play() {
    const FRAMES: u32 = 3000;
    let mut moves = String::new();
    let mut f = SYNC + 50;
    while f < SYNC + FRAMES {
        for (d, held) in [
            (0, "R"),
            (40, "R F"),
            (52, "R"),
            (90, "L"),
            (130, "L U"),
            (150, "R"),
        ] {
            moves += &format!("{}: {held}\n", f + d);
        }
        f += 170;
    }
    let sc = script(&moves);
    let mut am = boot(&sc, SYNC, SYNC + FRAMES);
    let (mut irq, mut tick) = (
        word(&am, h0(IRQ_COUNT)),
        am.peek(h0(GAME_FRAME_TICK) + 3, 1)[0],
    );
    let (mut passes, mut late) = (0u32, 0u32);
    for frame in SYNC..SYNC + FRAMES {
        am.step(script::input_at(&sc.lines, frame)).unwrap();
        let level3 = am.irqs().iter().filter(|i| i.level == 3).count();
        assert_eq!(level3, 1, "level-3 interrupts in frame {frame}");
        let now = word(&am, h0(IRQ_COUNT));
        assert_eq!(now, irq.wrapping_add(1), "irq_count in frame {frame}");
        irq = now;
        // game_frame's own counter (h0+$20b0) runs 0..=62 and wraps; it advances once a pass.
        let t = am.peek(h0(GAME_FRAME_TICK) + 3, 1)[0];
        let advanced = (t as i32 - tick as i32).rem_euclid(63) as u32;
        assert!(
            advanced <= 2,
            "game_frame ran {advanced} times by frame {frame}"
        );
        late += (advanced != 1) as u32;
        passes += advanced;
        tick = t;
    }
    assert!(
        (FRAMES - 1..=FRAMES).contains(&passes),
        "{passes} game_frame passes in {FRAMES} frames"
    );
    // Moving play loads the frame enough that some passes end after the frame boundary; idle
    // play never does (docs/09-play-measured.md). A pass that straddles shows as 0 then 2.
    assert!(
        late > 0,
        "no pass straddled a frame boundary: the load is not exercised"
    );
}
