//! Game-free: `Amiga::poke_at` (a write made when the CPU reaches an address, core patch 0006) on
//! Kickstart 1.3 with no disk, a level-3 handler of the test's own standing in for a program that
//! writes a variable and reads it within one frame. Needs the Kickstart (`AMIGA_KICKSTART`, else
//! `~/amiga-roms/kick1.3.rom`) and `toolkit/tools/setup/oracle.sh`'s build.

use amiga_oracle::*;

/// The insert-disk screen is drawn from frame 207 (`tests/boot.rs`); from here on the CPU idles
/// between vertical blanks.
const IDLE: u32 = 240;

const L3: u32 = 0x4_0000;
/// The variable the handler sets, and where it copies it in the next instruction.
const VAR: u32 = 0x4_0200;
const COPY: u32 = 0x4_0204;
/// The copy: `move.l VAR,COPY`, right after the handler's `move.l #0,VAR`.
const READER: u32 = L3 + 10;
/// An address in the zeroed chip RAM no code runs from.
const NEVER: u32 = 0x4_8000;

/// Clears VAR, copies it to COPY, clears the level-3 requests and returns: VAR is written and read
/// in the same frame, so a poke between frames never reaches the copy.
fn l3_handler() -> Vec<u8> {
    [
        0x23FC, 0x0000, 0x0000, 0x0004, 0x0200, // move.l #0,$40200
        0x23F9, 0x0004, 0x0200, 0x0004, 0x0204, // move.l $40200,$40204
        0x33FC, 0x0070, 0x00DF, 0xF09C, //         move.w #$0070,$dff09c
        0x33FC, 0x0070, 0x00DF, 0xF09C, //         (twice, as the hardware wants)
        0x4E73, //                                 rte
    ]
    .iter()
    .flat_map(|w: &u16| w.to_be_bytes())
    .collect()
}

fn machine() -> Amiga {
    let rom = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    let mut a = Amiga::power_on(&OracleConfig::new(rom, Vec::new(), IDLE + 40)).unwrap();
    for _ in 0..IDLE {
        a.step(FrameInput::NONE).unwrap();
    }
    a.write_memory(L3, &l3_handler());
    a.write_memory(0x6C, &L3.to_be_bytes());
    // The step ended with this frame's vertical blank already taken through the old vector.
    a.step(FrameInput::NONE).unwrap();
    a
}

fn long(a: &Amiga, at: u32) -> u32 {
    a.memory().long(at).unwrap()
}

/// A short test, labelled smoke: a poke between frames is overwritten before the copy reads it;
/// the same value written at the copy's address reaches it, once, and is then gone.
#[test]
fn a_write_at_an_address_lands_between_two_instructions_of_one_frame_once() {
    let mut a = machine();
    a.poke(VAR, &7u32.to_be_bytes());
    a.step(FrameInput::NONE).unwrap();
    assert_eq!(long(&a, COPY), 0, "a poke between frames is cleared before the copy");

    a.poke_at(READER, VAR, &7u32.to_be_bytes());
    assert_eq!(a.pokes_at_pending(), 1);
    a.step(FrameInput::NONE).unwrap();
    assert_eq!(long(&a, COPY), 7, "the copy read the value written as it was reached");
    assert_eq!(a.pokes_at_pending(), 0, "made, then dropped");

    a.step(FrameInput::NONE).unwrap();
    assert_eq!(long(&a, COPY), 0, "once: the next frame's copy reads the handler's own 0");
}

#[test]
fn a_write_at_an_unreached_address_waits_and_changes_nothing() {
    let mut a = machine();
    let mut b = machine();
    b.poke_at(NEVER, VAR, &7u32.to_be_bytes());
    for f in 0..20 {
        a.step(FrameInput::NONE).unwrap();
        b.step(FrameInput::NONE).unwrap();
        assert!(a.memory() == b.memory(), "frame {f}: an unreached write changed the run");
        assert_eq!(b.pokes_at_pending(), 1, "frame {f}: still waiting");
    }
    // Two writes at one address are both made, in the order given.
    b.poke_at(READER, VAR, &5u32.to_be_bytes());
    b.poke_at(READER, VAR + 2, &[0, 9]);
    b.step(FrameInput::NONE).unwrap();
    assert_eq!(long(&b, COPY), 9, "both made before the copy, the later over the earlier's low word");
    assert_eq!(b.pokes_at_pending(), 1, "only the unreached one is left");
}
