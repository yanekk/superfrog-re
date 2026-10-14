//! What the routine-conformance tests share: the original's memory at W1-1 (and WB-1, W3-1), the
//! sound and display data, and generators for the inputs.
#![allow(dead_code)]

use std::sync::OnceLock;

use sf_conform::routine::{Gen, Rig};
use sf_conform::{SyncPoint, sync};
use sf_core::audio::SoundData;
use sf_core::input::{FrameInput, Joy};
use sf_core::layout::{AmigaMemory, MAP_ROW_CELLS, h1};
use sf_core::level::Hw;

pub const CASES: usize = 10_000;

pub fn level() -> &'static AmigaMemory {
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

/// The original's memory at the bonus level's first frame (WB-1, reached by its level code).
pub fn bonus_level() -> &'static AmigaMemory {
    static M: OnceLock<AmigaMemory> = OnceLock::new();
    M.get_or_init(|| {
        let s = sync::reach(
            &SyncPoint::AtLevel("WB-1".parse().unwrap()),
            &sf_conform::InputScript::idle(),
            1,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        s.memory
    })
}

/// The original's memory at W3-1's first frame (world 3's enemies, T44).
pub fn w3_level() -> &'static AmigaMemory {
    static M: OnceLock<AmigaMemory> = OnceLock::new();
    M.get_or_init(|| {
        let s = sync::reach(
            &SyncPoint::AtLevel("W3-1".parse().unwrap()),
            &sf_conform::InputScript::idle(),
            1,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        s.memory
    })
}

pub fn sound() -> &'static SoundData {
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

pub fn rig() -> Rig<'static> {
    static D: OnceLock<std::sync::Arc<sf_core::DisplayData>> = OnceLock::new();
    let d = D.get_or_init(sf_conform::display_data);
    Rig::new(level(), sound(), d)
}

/// A rig on the bonus level's first frame ([`bonus_level`]).
pub fn bonus_rig() -> Rig<'static> {
    static D: OnceLock<std::sync::Arc<sf_core::DisplayData>> = OnceLock::new();
    let d = D.get_or_init(sf_conform::display_data);
    Rig::new(bonus_level(), sound(), d)
}

/// A rig on W3-1's first frame ([`w3_level`]).
pub fn w3_rig() -> Rig<'static> {
    static D: OnceLock<std::sync::Arc<sf_core::DisplayData>> = OnceLock::new();
    let d = D.get_or_init(sf_conform::display_data);
    Rig::new(w3_level(), sound(), d)
}

/// Any joystick position, fire or not.
pub fn any_input(g: &mut Gen) -> Hw {
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

/// 0, 1, -1 or anything.
pub fn edgy(g: &mut Gen) -> u16 {
    let any = g.next() as u16;
    g.pick(&[0, 1, 0xffff, any])
}

/// A word that is usually small and sometimes anything.
pub fn small_word(g: &mut Gen) -> u16 {
    match g.below(4) {
        0 => g.next() as u16,
        1 => 0,
        _ => (g.below(41) as i16 - 20) as u16,
    }
}

/// The address of map cell (`col`, `row`) in `level_map`.
pub fn cell(col: u32, row: u32) -> u32 {
    h1(0x218e0) + 2 * (row * MAP_ROW_CELLS + col)
}
