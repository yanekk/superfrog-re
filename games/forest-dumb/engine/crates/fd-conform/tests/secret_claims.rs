//! The secrets `fd_assets::secrets` lists, held to the files and the original (plan
//! forest-dumb-map-page T07, `docs/17-secrets.md`).
//!
//! The counts test reads the six levels from the disks. Each oracle claim reaches a world's play on
//! the original (W1-1 from the cached level sync point; worlds 2 to 6 by level 1's cheat end with
//! `world_index` set, as `code_seen`'s world starts do), sets `cheat_flag` so no life is lost on the
//! way, and carries the character to a spot beside a hidden passage a few pixels a frame, poking
//! `char_x` and `char_y`: the camera follows as in play and the edge redraws build `collision_map`
//! there from the level's shapes. It then lets go, lets the character land, and holds a direction:
//! the original's own movement takes the character into the cells the rule lists, cells drawn as
//! wall. A wall of solid cells stops it at the passage's mouth instead (the rim claim).
//!
//! Needs the disks and the Kickstart: absent, it fails naming where it looked.

use amiga_oracle::{Amiga, FrameInput, Joy};
use fd_assets::{AssetError, Level, SecretKind, read_level, secrets};
use fd_conform::code_seen::{LEVEL_END_CASE, WORLD_REACH_CAP};
use fd_conform::{Case, HUNK_BASES, Inputs, address, conform_dir, sync};

const CHAR_X: u32 = 0x1e5a;
const CHAR_Y: u32 = 0x1e5e;
/// `cheat_flag`: set, contact costs no life (`char_hit_check+$22e`), so the trip there is safe.
const CHEAT_FLAG: u32 = 0x780c;
/// Pixels the character is carried a frame: under the redraws' cell, so no edge is skipped.
const STEP: i64 = 3;
/// Frames held at the spot after the trip, so the camera's scrolls end, and then free to land.
const SETTLE: u32 = 120;
const LAND: u32 = 80;
/// Frames the direction is held.
const HOLD: u32 = 200;
/// Fire held this many frames in every `FIRE_EVERY` until a world's SCENA screen gives way to play
/// (`code_seen`'s world start does the same).
const FIRE_EVERY: u32 = 60;
const FIRE_HELD: u32 = 11;

fn files(name: &str) -> Result<Vec<u8>, AssetError> {
    let dir = fd_tools::disk_dir().map_err(|e| AssetError::missing(name, e))?;
    fd_tools::game_file(&dir, &format!("disk2/{name}")).map_err(|e| AssetError::missing(name, e))
}

fn level(world: u8) -> Level {
    read_level(world, &files).unwrap_or_else(|e| panic!("{e}"))
}

fn h0(offset: u32) -> u32 {
    HUNK_BASES[0] + offset
}

fn long(am: &Amiga, offset: u32) -> u32 {
    let b = am.peek(h0(offset), 4);
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

/// The character's body cell, by the game's own cell rule (`char_object_cells`: column
/// `char_x + $e`, row `char_y + $19`).
fn body(am: &Amiga) -> (u16, u16) {
    let (x, y) = (long(am, CHAR_X), long(am, CHAR_Y));
    (((x + 0xe) / 16) as u16, ((y + 0x19) / 16) as u16)
}

/// Rule's counts per world, W1-1 to W6-1, as `docs/17-secrets.md` gives them (measured
/// 2026-11-20).
const COUNTS: [usize; 6] = [133, 0, 292, 63, 0, 0];

#[test]
fn the_rule_lists_the_counts_docs_17_gives() {
    let counts: Vec<usize> = (1..=6).map(|w| secrets(&level(w)).len()).collect();
    assert_eq!(counts, COUNTS);
    for w in 1..=6 {
        let l = level(w);
        for s in secrets(&l) {
            assert_eq!(s.kind, SecretKind::HiddenPassage);
            // Every listed cell is passable in the files: its shape empty.
            assert_eq!(l.shapes[l.shape(s.c, s.r) as usize], [0; 16], "W{w}-1:{},{}", s.c, s.r);
        }
    }
}

/// The original in play in `world` (1 to 6), `cheat_flag` set.
fn in_play(world: u32) -> Amiga {
    let room = 3000;
    if world == 1 {
        let mut am = sync::reach_level_1(room).unwrap_or_else(|e| panic!("{e}"));
        am.poke(h0(CHEAT_FLAG), &[0, 1]);
        return am;
    }
    let end = Case::load(&conform_dir().join("cases").join(LEVEL_END_CASE))
        .unwrap_or_else(|e| panic!("{e}"));
    let inputs = Inputs::load(&end).unwrap_or_else(|e| panic!("{e}"));
    let world_index = address("world_index").unwrap_or_else(|e| panic!("{e}"));
    let irq_count = address("irq_count").unwrap_or_else(|e| panic!("{e}"));
    let case_end = end.sync_frame + end.frames;
    let mut am = sync::reach_level_1(case_end - sync::LEVEL_1_SYNC + WORLD_REACH_CAP + room)
        .unwrap_or_else(|e| panic!("{e}"));
    let mut frame = sync::LEVEL_1_SYNC;
    let mut last_irq = am.peek(irq_count, 2);
    let mut left = false;
    loop {
        if frame == end.sync_frame {
            for s in &end.set {
                am.poke(s.address().unwrap(), &s.bytes());
            }
            am.poke(world_index, &(world - 2).to_be_bytes());
        }
        let input = if frame < case_end {
            inputs.at(frame)
        } else if (frame - case_end) % FIRE_EVERY < FIRE_HELD {
            FrameInput::joy(Joy::FIRE)
        } else {
            FrameInput::NONE
        };
        am.step(input).unwrap_or_else(|e| panic!("{e}"));
        frame += 1;
        let irq = am.peek(irq_count, 2);
        let running = irq != last_irq;
        last_irq = irq;
        let here = am.peek(world_index, 4) == (world - 1).to_be_bytes();
        if !left {
            left = here && !running;
        } else if running {
            assert!(here, "world {world}: play started in another world");
            break;
        }
        assert!(
            frame < case_end + WORLD_REACH_CAP,
            "world {world}: play not started {WORLD_REACH_CAP} frames after the level's end"
        );
    }
    // A few frames of play, so fire's last press is gone.
    for _ in 0..20 {
        am.step(FrameInput::NONE).unwrap_or_else(|e| panic!("{e}"));
    }
    am.poke(h0(CHEAT_FLAG), &[0, 1]);
    am
}

/// What a claim saw: the body cell after landing, and every body cell while the direction was held.
#[derive(Debug)]
struct Walk {
    landed: (u16, u16),
    cells: Vec<(u16, u16)>,
}

/// In `world`'s play, carry the character to the position whose body cell is `at` with its feet
/// just above the floor of row `at.1 + 1`, let it land, and hold `joy`.
fn walk(world: u32, at: (u16, u16), joy: Joy, setup: impl FnOnce(&mut Amiga)) -> Walk {
    let mut am = in_play(world);
    setup(&mut am);
    let target = (
        i64::from(at.0) * 16 + 8 - 0xe,
        // Feet (`char_y + $23`) four pixels above the floor row's top.
        (i64::from(at.1) + 1) * 16 - 0x23 - 4,
    );
    let mut pos = (i64::from(long(&am, CHAR_X)), i64::from(long(&am, CHAR_Y)));
    let toward = |p: i64, t: i64| p + (t - p).clamp(-STEP, STEP);
    let put = |am: &mut Amiga, (x, y): (i64, i64)| {
        am.poke(h0(CHAR_X), &(x as u32).to_be_bytes());
        am.poke(h0(CHAR_Y), &(y as u32).to_be_bytes());
    };
    while pos != target {
        pos = (toward(pos.0, target.0), toward(pos.1, target.1));
        put(&mut am, pos);
        am.step(FrameInput::NONE).unwrap_or_else(|e| panic!("{e}"));
    }
    for _ in 0..SETTLE {
        put(&mut am, target);
        am.step(FrameInput::NONE).unwrap_or_else(|e| panic!("{e}"));
    }
    for _ in 0..LAND {
        am.step(FrameInput::NONE).unwrap_or_else(|e| panic!("{e}"));
    }
    let landed = body(&am);
    let mut cells = Vec::new();
    for _ in 0..HOLD {
        am.step(FrameInput::joy(joy)).unwrap_or_else(|e| panic!("{e}"));
        let c = body(&am);
        if cells.last() != Some(&c) {
            cells.push(c);
        }
    }
    Walk { landed, cells }
}

/// The listed passage cells of `world` the walk's body cell entered.
fn entered(world: u8, w: &Walk) -> Vec<(u16, u16)> {
    let listed: Vec<(u16, u16)> = secrets(&level(world)).iter().map(|s| (s.c, s.r)).collect();
    let mut v: Vec<(u16, u16)> = w.cells.iter().copied().filter(|c| listed.contains(c)).collect();
    v.dedup();
    v
}

/// One claim: from `at`, holding `joy`, the character walks into at least `cells` listed cells.
fn passes(world: u8, at: (u16, u16), joy: Joy, cells: usize) {
    let w = walk(u32::from(world), at, joy, |_| {});
    eprintln!("W{world}-1 {w:?}");
    assert_eq!(w.landed, at, "W{world}-1: the character did not land where placed");
    let inside = entered(world, &w);
    assert!(
        inside.len() >= cells,
        "W{world}-1: from {at:?} the character entered {} listed cells, {inside:?}, of {:?}",
        inside.len(),
        w.cells
    );
}

/// The level's collision map as the game loaded it (`_MapaN-1.Mapa`, hunk 0 `$28f18`), 96 cells a
/// column; the edge redraws copy a cell's shape from it as the cell comes into view.
const COLLISION_MAP: u32 = 0x28f18;

/// The control: W1-1's tunnel with its first three columns made solid (shape 0) in the game's own
/// collision map before they come into view. The same walk stops at the mouth and enters no listed
/// cell, so the claims see a wall where the shapes put one.
#[test]
#[ignore = "full: oracle"]
fn w1_the_tunnel_made_solid_stops_the_walk() {
    let w = walk(1, (115, 93), Joy::RIGHT, |am| {
        for c in 117..120u32 {
            for r in 91..94u32 {
                am.poke(h0(COLLISION_MAP + c * 96 + r), &[0]);
            }
        }
    });
    eprintln!("W1-1 solid {w:?}");
    assert_eq!(w.landed, (115, 93));
    assert_eq!(entered(1, &w), [], "{:?}", w.cells);
    assert!(w.cells.iter().all(|&(c, _)| c <= 117), "{:?}", w.cells);
}

/// W1-1's tunnel under the hill, `W1-1:117,91` to `159,93`, drawn as the hill's stone: in from the
/// sky at its west mouth.
#[test]
#[ignore = "full: oracle"]
fn w1_the_tunnel_under_the_hill_is_passable() {
    passes(1, (115, 93), Joy::RIGHT, 10);
}

/// W3-1's stone room `W3-1:67,25` to `77,28`: in from the lava room on its east.
#[test]
#[ignore = "full: oracle"]
fn w3_the_stone_room_beside_the_lava_is_passable() {
    passes(3, (80, 28), Joy::LEFT, 4);
}

/// W4-1's passage `W4-1:199,90` to `210,92`: in from the brick hall on its west.
#[test]
#[ignore = "full: oracle"]
fn w4_the_passage_by_the_statues_is_passable() {
    passes(4, (197, 93), Joy::RIGHT, 4);
}
