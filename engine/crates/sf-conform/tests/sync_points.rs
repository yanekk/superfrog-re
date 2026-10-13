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
