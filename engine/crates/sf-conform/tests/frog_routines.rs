//! The frog's routines (`sf_core::frog`, T14) against the original's, on the routine oracle.
//!
//! Each routine runs on Moira from the original's memory at the first frame of W1-1, and the port
//! from the same memory transplanted, over 10 000 generated states: the frog's fields, the
//! joystick and the fire button varied. Every byte written, the sound driver's state and the Paula
//! writes must match (`sf_conform::routine`).
//!
//! Lives here, not in `sf-core/tests/`, because the purity test keeps the emulator out of the pure
//! crates' tests.

use std::sync::OnceLock;

use sf_conform::routine::{Gen, Rig, Setup};
use sf_conform::{SyncPoint, sync};
use sf_core::audio::SoundData;
use sf_core::frog::{self, FROG, f};
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::AmigaMemory;
use sf_core::level::Hw;

const CASES: usize = 10_000;

fn level() -> &'static AmigaMemory {
    static M: OnceLock<AmigaMemory> = OnceLock::new();
    M.get_or_init(|| {
        let s = sync::reach(
            &SyncPoint::AtLevel("W1-1".parse().unwrap()),
            &sf_conform::InputScript::idle(),
            1,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        s.memory
    })
}

fn sound() -> &'static SoundData {
    static S: OnceLock<SoundData> = OnceLock::new();
    S.get_or_init(|| {
        let files = sf_conform::disks::read_disk_files().unwrap_or_else(|e| panic!("{e}"));
        let offered: Vec<sf_disk::Offered> = files
            .into_iter()
            .map(|(name, bytes)| sf_disk::Offered { name, bytes })
            .collect();
        let disks = sf_disk::identify(&offered).unwrap_or_else(|e| panic!("{e:?}"));
        SoundData::from_disks(&disks).unwrap_or_else(|e| panic!("{e}"))
    })
}

fn rig() -> Rig<'static> {
    static D: OnceLock<std::sync::Arc<sf_core::DisplayData>> = OnceLock::new();
    let d = D.get_or_init(sf_conform::display_data);
    Rig::new(level(), sound(), d)
}

/// Any joystick position, fire or not.
fn any_input(g: &mut Gen) -> Hw {
    let dirs = [
        Joy::NONE,
        Joy::LEFT,
        Joy::RIGHT,
        Joy::UP,
        Joy::DOWN,
        Joy::UP | Joy::LEFT,
        Joy::UP | Joy::RIGHT,
        Joy::DOWN | Joy::LEFT,
        Joy::DOWN | Joy::RIGHT,
    ];
    let mut j = g.pick(&dirs);
    if g.one_in(3) {
        j = j | Joy::FIRE;
    }
    Hw::of(FrameInput::joy(j))
}

/// A word that is usually small and sometimes anything.
fn small_word(g: &mut Gen) -> u16 {
    match g.below(4) {
        0 => g.next() as u16,
        1 => 0,
        _ => (g.below(41) as i16 - 20) as u16,
    }
}

#[test]
fn frog_move_matches_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(1);
    for case in 0..CASES {
        let mut s = Setup {
            hw: any_input(&mut g),
            ..Setup::default()
        };
        for o in [f::X, f::Y] {
            s.poke_w(FROG + o, g.below(0x1400) as u16);
        }
        for o in [f::VX, f::VY] {
            s.poke_w(FROG + o, small_word(&mut g));
        }
        s.poke_w(FROG + f::FROZEN, if g.one_in(8) { 1 } else { 0 });
        rig.check("frog_move", case, 0xdb58, &s, frog::physics::frog_move);
    }
}
