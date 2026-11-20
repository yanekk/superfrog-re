//! A world start's recording replayed (title plan T08, DESIGN §2.5): made headless the way the app
//! makes it, from power-on through the title on `to_level_1_input` with `world_start_bytes` written
//! as `load_world` is entered, then `fd_conform::bug::replay_translated` of it read back from its
//! text. Full: the owner's disks and the generated tree (`tools/translate.py --build`).

use amiga_oracle::{FrameInput, Joy};
use fd_conform::FrameEngine;
use fd_conform::bug::replay_translated;
use fd_conform::translated_engine::TranslatedEngine;
use fd_core::layout_var;
use fd_core::recording::{
    EngineKind, LOAD_WORLD, LevelId, Recording, StartPoint, TO_LEVEL_1_FRAMES, location_in_ram,
    recorder_from, start_reached, to_level_1_input, world_start_bytes,
};
use fd_translated::TranslatedGame;

/// Chip RAM's size: where slow RAM starts in `TranslatedEngine::mem`.
const CHIP: usize = 512 << 10;

/// Steps of play recorded: right held, with a jump now and then.
const STEPS: u64 = 200;

fn input(k: u64) -> FrameInput {
    match k % 50 {
        0..5 => FrameInput::joy(Joy::RIGHT | Joy::UP),
        _ => FrameInput::joy(Joy::RIGHT),
    }
}

fn var(ram: &[u8], name: &str) -> u32 {
    let v = layout_var(name).unwrap();
    let a = fd_conform::HUNK_BASES[v.hunk as usize] + v.offset;
    let i = match a.checked_sub(amiga_oracle::SLOW_BASE) {
        Some(s) => s as usize + CHIP,
        None => a as usize,
    };
    ram[i..i + v.width.bytes()]
        .iter()
        .fold(0, |acc, &b| acc << 8 | b as u32)
}

#[test]
#[ignore = "full: the disks and the generated tree; power-on to world 3 twice"]
fn a_world_3_cheat_recording_replays_to_its_recorded_state() {
    assert!(
        TranslatedGame::generated(),
        "no translated tree is built in: run `python3 -I games/forest-dumb/tools/translate.py \
         --build`, then this test again"
    );
    let disks = fd_conform::disks::load().unwrap_or_else(|e| panic!("{e}"));
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    let (level, cheat) = (LevelId::W3_1, true);
    let start = StartPoint::World { level, cheat };

    // Recorded as the app records it.
    let mut e = TranslatedEngine::power_on(&disks).unwrap();
    e.mount_level_1(data.clone());
    for (addr, bytes) in world_start_bytes(level, cheat) {
        e.write_at(LOAD_WORLD, addr, &bytes).unwrap();
    }
    let split = |m: &[u8]| (m[..CHIP].to_vec(), m[CHIP..].to_vec());
    let reached = (0..TO_LEVEL_1_FRAMES).find(|&n| {
        e.frame(to_level_1_input(n));
        assert_eq!(e.stopped(), None, "frame {n}");
        let (chip, slow) = split(e.mem());
        start_reached(start, &chip, &slow)
    });
    let reached = reached.expect("world 3 reached");
    println!("world 3 reached at frame {}", reached + 1);
    assert_eq!(e.writes_at_pending(), 0);
    let mut r = recorder_from(start, EngineKind::Translated, level, fd_tools::ENGINE_BUILD);
    for k in 0..STEPS {
        r.step(input(k));
        e.frame(input(k));
        assert_eq!(e.stopped(), None, "step {k}");
    }
    let recorded = e.mem().to_vec();
    let (chip, slow) = split(&recorded);
    let at = location_in_ram(&chip, &slow).expect("in play");
    assert_eq!(at.level, LevelId::W3_1);
    let rec = r.finish(Some(at), "world 3, cheat on".into()).unwrap();
    let rec = Recording::parse(&rec.to_text()).unwrap();
    assert_eq!(rec.start, start);

    let r = replay_translated(&disks, data, &rec).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(r.frames, STEPS as usize);
    assert_eq!(r.stop, None);
    assert_eq!(r.location, Some(at));
    assert_eq!(var(&r.ram, "world_index"), 2);
    assert_eq!(var(&r.ram, "cheat_flag"), 1);
    assert!(var(&r.ram, "irq_count") > STEPS as u32 / 2, "play ran");
    let apart = (0..recorded.len()).filter(|&i| recorded[i] != r.ram[i]).count();
    assert_eq!(apart, 0, "RAM bytes apart from the recording's final state");
}
