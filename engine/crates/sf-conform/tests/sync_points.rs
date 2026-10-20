//! The level sync points on the oracle (T13, T31): the one-drive ones are where they were, W6-2..4
//! take their first map set with df1 connected and their `alt` set on one drive, and the final
//! level follows W6-4's forced exit. Snapshots are cached (`sync`), so after the first run each is
//! a restore.

use sf_assets::LevelId;
use sf_conform::{InputScript, SyncPoint, game_data, sync};
use sf_core::layout::{AmigaMemory, Ram, h0};

const WORLD: u32 = h0(0x40da);
const LEVEL: u32 = h0(0x40de);
const LEVEL_STATE: u32 = h0(0xa06);
const FINAL_LEVEL_FLAG: u32 = h0(0x1e);
/// `probe_drives`' long for df1: 0 connected, $1e not (docs/12 "Reaching every level").
const PROBE_DF1: u32 = h0(0xb6cc);
/// `map_rows` (h0+$17334): the map in RAM, three padding rows first, 304 cells a row, each cell
/// `tile << 6 | flags` (T31 review, measured).
const MAP_ROWS: u32 = h0(0x17334);

/// How many of `level`'s cells hold the tile its own BODY gives there.
fn tiles_matching(m: &AmigaMemory, level: &str) -> usize {
    let body = &game_data().levels[&level.parse::<LevelId>().unwrap()]
        .t7mp
        .body;
    let base = long(m, MAP_ROWS);
    (0..body.len())
        .filter(|&i| {
            let a = base + (((i / 300) as u32 + 3) * 304 + (i % 300) as u32) * 2;
            (m.byte(a) as u16) << 2 | (m.byte(a + 1) as u16) >> 6 == body[i] & 0x3ff
        })
        .count()
}

fn long(m: &dyn Ram, a: u32) -> u32 {
    (0..4).fold(0, |v, k| v << 8 | m.byte(a + k) as u32)
}

fn at(level: &str) -> AmigaMemory {
    let id: LevelId = level.parse().unwrap();
    sync::reach(&SyncPoint::AtLevel(id), &InputScript::default(), 1)
        .unwrap_or_else(|e| panic!("{e}"))
        .memory
}

fn playing(m: &AmigaMemory) -> (u32, u32, bool) {
    (
        long(m, WORLD),
        long(m, LEVEL),
        (long(m, LEVEL_STATE) as i32) < 0,
    )
}

#[test]
fn one_drive_levels_are_reached_as_before() {
    for (id, world, level) in [("W1-1", 1, 1), ("W3-2", 3, 2)] {
        let m = at(id);
        assert_eq!(playing(&m), (world, level, true), "{id}");
        assert_eq!(long(&m, PROBE_DF1), 0x1e, "{id}: one drive");
    }
}

#[test]
fn w6_maps_come_from_the_drive_set_up() {
    let first = at("W6-3");
    let alt = at("W6-3alt");
    assert_eq!(playing(&first), (6, 3, true));
    assert_eq!(playing(&alt), (6, 3, true));
    assert_eq!(long(&first, PROBE_DF1), 0, "df1 connected: first map set");
    assert_eq!(long(&alt, PROBE_DF1), 0x1e, "one drive: second map set");
}

#[test]
fn the_final_level_follows_w6_4s_exit() {
    let m = at("WW-1");
    assert_eq!(playing(&m), (6, 1, true));
    assert_eq!(m.byte(FINAL_LEVEL_FLAG + 1), 1);
}

/// The map in RAM is the one each id names: W6-2..4's sets differ in 13 to 513 cells, the final
/// level is the LW map (T31 review).
#[test]
fn each_sync_point_has_its_own_map_in_ram() {
    for n in 2..=4 {
        let (first, alt) = (format!("W6-{n}"), format!("W6-{n}alt"));
        for (reached, other) in [(&first, &alt), (&alt, &first)] {
            let m = at(reached);
            let (own, theirs) = (tiles_matching(&m, reached), tiles_matching(&m, other));
            assert!(
                own > theirs,
                "{reached}: {own} tiles its own, {theirs} {other}'s"
            );
        }
    }
    let m = at("WW-1");
    assert_eq!(tiles_matching(&m, "WW-1"), 300 * 84);
}

/// The end sequence's sync point: after the final level's forced exit, the original has just
/// entered `end_seq_load` (T35).
#[test]
fn the_end_sequence_starts_in_end_seq_load() {
    use sf_conform::sync::{self, SyncPoint};
    let s = sync::reach(
        &SyncPoint::EndSequence,
        &sf_conform::InputScript::idle(),
        10,
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let chain = sync::main_chain(&s.amiga.cpu(), &s.memory);
    assert!(chain.contains(&sf_core::layout::h0(0x7b6)), "{chain:x?}");
    assert_eq!(s.memory.byte(FINAL_LEVEL_FLAG + 1), 1);
}

/// `slot_state` (h0+$49c6): its first word is the coins the tally put in.
const SLOT_STATE: u32 = h0(0x49c6);
const RNG_STATE: u32 = h0(0x160d4);

fn slot(script: &str, start: Option<&str>) -> sync::Synced {
    sync::reach(
        &SyncPoint::Slot {
            script: script.into(),
            start: start.map(str::to_string),
        },
        &InputScript::idle(),
        1_000,
    )
    .unwrap_or_else(|e| panic!("{e}"))
}

/// The slot machine's sync point (T71): W1-1's exit forced at once, the slot picked in the tally;
/// the original is in `slot_machine` 73 frames on with W1-1's two clock coins, and its load (FRUT, FRUC, FRUM)
/// ends 759 frames on (in `overlays_end`, return address h0+$b7e2), as T19 measured.
#[test]
fn slot_none_reaches_the_slot_machine_with_two_coins() {
    let mut s = slot("slot-none", None);
    let chain = sync::main_chain(&s.amiga.cpu(), &s.memory);
    assert!(chain.contains(&sync::IN_SLOT_MACHINE), "{chain:x?}");
    let coins = (s.memory.byte(SLOT_STATE) as u16) << 8 | s.memory.byte(SLOT_STATE + 1) as u16;
    assert_eq!(coins, 2);
    // Entered 73 frames after the forced exit (T19), which `slot-none` makes W1-1's first frame.
    let w1_1 = sync::w1_1_by("start-w1-1").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(s.amiga.frame() - w1_1.amiga.frame(), 73);
    let mut disks = sync::Disks::at(&s.memory);
    let loaded = (1..=1_000).find(|_| {
        s.amiga.step(sf_core::input::FrameInput::NONE).unwrap();
        disks.serve(&mut s.amiga);
        let m = sync::core_memory(s.amiga.memory());
        sync::main_chain(&s.amiga.cpu(), &m).contains(&h0(0xb7e2))
    });
    assert_eq!(loaded, Some(759));
}

fn rng_at_w1_1(start: &str) -> sf_core::Rng {
    let m = sync::w1_1_by(start)
        .unwrap_or_else(|e| panic!("{e}"))
        .memory;
    sf_core::Rng {
        lo: long(&m, RNG_STATE),
        hi: long(&m, RNG_STATE + 4),
    }
}

/// The title's eye blinks draw from the RNG, so a later START GAME starts W1-1 further on: 50
/// frames later one step, 120 frames later three, neither in the attract demo (T19's finding).
#[test]
fn a_later_start_moves_the_rng_one_and_three_steps() {
    let base = rng_at_w1_1("start-w1-1");
    let mut want = base;
    want.next();
    assert_eq!(rng_at_w1_1("start-w1-1-blink1"), want, "blink1: one step");
    want.next();
    want.next();
    assert_eq!(rng_at_w1_1("start-w1-1-late"), want, "late: three steps");
}

/// Moira's clock after a call, for calibrating the engine's CPU time (T72): `$6238` (the slot's
/// lamps off, 26 lamps) costs 1 650 cycles, as T19 measured.
#[test]
fn moira_counts_the_cycles_of_a_call() {
    let s = slot("slot-none", None);
    let mut cpu = sf_oracle::Cpu68k::new(sf_oracle::AmigaMemory {
        chip: s.memory.chip.clone(),
        slow: s.memory.slow.clone(),
    });
    let regs = sf_oracle::Regs {
        d: [0; 8],
        a: {
            let mut a = [0; 8];
            a[7] = sf_conform::routine::STACK;
            a
        },
        sr: 0x2700,
    };
    let r = cpu.call(h0(0x6238), regs, 10_000).unwrap();
    assert_eq!(r.cycles, 1_650);
}
