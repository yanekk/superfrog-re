//! What the live map page shows, read from the game's memory at a frame's end (plans/forest-dumb-map-page
//! DESIGN §2.5, §2.7, §3.3): whether a level is in play, which, where the character is, the items
//! left, the creatures in view and the object cells that differ from the level file.
//!
//! Two sources, one value: the translated program's RAM ([`live_from_ram`]), which holds the truth
//! with level 1's hand engine mounted too since the seam writes back what it computes, and the hand
//! engine's [`Game`] under `--engine hand` ([`live_from_game`]). Pure: the previous sample's
//! `irq_count` and the level file's map arrive as arguments.

use fd_assets::{LEVEL_ROWS, OBJECT_COLUMNS};

use crate::Game;
use crate::level::redraw::OBJECT_MAP;
use crate::recording::{ram_byte, ram_var};
use crate::scene::Scene;

/// One sample of play for the live page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Live {
    pub playing: bool,
    /// `world_index + 1`; `None` when not playing.
    pub world: Option<u8>,
    /// The character at the spot the cell rule uses (`char_x + $e`, `char_y + 9`, `recording.rs`),
    /// in level pixels; `None` when not playing.
    pub char_xy: Option<(u32, u32)>,
    /// `items_left` decoded from BCD; `None` when not playing.
    pub items_left: Option<u16>,
    /// The active creature slots: x, y as the slot holds them, and kind (the object value less one,
    /// 7 dying, docs/10). Empty when not playing.
    pub creatures: Vec<(i32, i32, u8)>,
    /// The object cells whose value differs from the level file's: column, row, value now (0: gone).
    /// Empty when not playing or when there is no level file for the world.
    pub changed: Vec<(u16, u16, u8)>,
    /// `irq_count`, the next call's `prev_irq`.
    pub irq: u16,
}

/// Bytes the object map takes in memory: the game's whole buffer, whatever the level's width.
const MAP_BYTES: u32 = OBJECT_COLUMNS as u32 * LEVEL_ROWS as u32;

/// Creature slots (`creature_active`, `creature_slots`).
const SLOTS: usize = 5;

/// The LAYOUT variables [`live_from_ram`] reads.
const VARS: &[&str] = &["irq_count", "world_index", "char_x", "char_y", "items_left"];

/// A creature slot's two longs: x and y; kind, facing and phase.
fn slot_vars(i: usize) -> [String; 3] {
    [
        format!("creature_active+${i:x}"),
        format!("creature_slots+${:x}", 8 * i),
        format!("creature_slots+${:x}", 8 * i + 4),
    ]
}

/// The A500 address of hunk 0's byte `off`.
fn h0(off: u32) -> u32 {
    crate::HUNK_BASES[0] + off
}

/// A BCD word as a number: `$282` is 282. A nibble past 9 counts as its value, as the original's
/// digits never hold one.
fn from_bcd(w: u16) -> u16 {
    (0..4)
        .rev()
        .fold(0, |acc, k| acc * 10 + (w >> (4 * k) & 0xf))
}

/// The spot the cell rule reads (`recording::Location::of_character`).
fn spot(char_x: u32, char_y: u32) -> (u32, u32) {
    (char_x.wrapping_add(0xe), char_y.wrapping_add(9))
}

/// The cells of `now` that differ from `start`, both column-major, 96 rows a column. A cell past
/// `start`'s end is compared with 0.
fn differences(now: impl Iterator<Item = u8>, start: &[u8]) -> Vec<(u16, u16, u8)> {
    now.enumerate()
        .filter(|&(i, b)| start.get(i).copied().unwrap_or(0) != b)
        .map(|(i, b)| {
            let rows = LEVEL_ROWS as usize;
            ((i / rows) as u16, (i % rows) as u16, b)
        })
        .collect()
}

/// A world index as `Live::world`: one more, held at 255 when memory holds nonsense, so the page
/// says it has no such level rather than showing another.
fn world_of(world_index: u32) -> u8 {
    world_index.saturating_add(1).min(255) as u8
}

/// The live sample from chip RAM and slow RAM at a frame's end. Play is `irq_count` having moved
/// since `prev_irq` (DESIGN §2.7: it stands still, not at 0, at the game over screen and the SCENA
/// screens once a level was played, and counts while paused); the first sample (`None`) is not play.
/// `start(world)` is the level file's object map for that world, column-major as RAM holds it.
pub fn live_from_ram(
    chip: &[u8],
    slow: &[u8],
    prev_irq: Option<u16>,
    start: &dyn Fn(u8) -> Option<Vec<u8>>,
) -> Live {
    let var = |name: &str| ram_var(chip, slow, name);
    let irq = var("irq_count") as u16;
    if prev_irq.is_none_or(|p| p == irq) {
        return Live {
            irq,
            ..Live::default()
        };
    }
    let world = world_of(var("world_index"));
    let creatures = (0..SLOTS)
        .filter_map(|i| {
            let [active, xy, kind] = slot_vars(i);
            (var(&active) != 0).then(|| {
                let (xy, kind) = (var(&xy), var(&kind));
                (
                    (xy >> 16) as i16 as i32,
                    xy as i16 as i32,
                    (kind >> 16) as u8,
                )
            })
        })
        .collect();
    let changed = match start(world) {
        Some(file) => differences(
            (0..MAP_BYTES).map(|k| ram_byte(chip, slow, h0(OBJECT_MAP + k))),
            &file,
        ),
        None => Vec::new(),
    };
    Live {
        playing: true,
        world: Some(world),
        char_xy: Some(spot(var("char_x"), var("char_y"))),
        items_left: Some(from_bcd(var("items_left") as u16)),
        creatures,
        changed,
        irq,
    }
}

/// The live sample from the hand engine's game. Play is its scene being the level's: the hand
/// engine has no title or SCENA screen, and its game over and level end are scenes of their own.
/// `start` is W1-1's object map from the level file.
pub fn live_from_game(g: &Game, start: &[u8]) -> Live {
    let s = g.state();
    let irq = s.irq_count;
    if !matches!(g.scene(), Scene::Level) {
        return Live {
            irq,
            ..Live::default()
        };
    }
    let c = &s.edges.creatures;
    let creatures = (0..SLOTS)
        .filter(|&i| c.active[i] != 0)
        .map(|i| {
            let slot = c.slots[i];
            (slot.x as i16 as i32, slot.y as i16 as i32, slot.kind as u8)
        })
        .collect();
    Live {
        playing: true,
        world: Some(world_of(s.world_index)),
        char_xy: Some(spot(s.char_x, s.char_y)),
        items_left: Some(from_bcd(s.items_left)),
        creatures,
        changed: differences(s.object_map.iter().copied(), start),
        irq,
    }
}

/// The A500 address ranges, `[start, end)`, that [`live_from_ram`] reads, so the app's program
/// thread copies only these each sample (DESIGN §3.4); every other byte may be 0.
pub fn live_ranges() -> Vec<(u32, u32)> {
    let mut names: Vec<String> = VARS.iter().map(|s| s.to_string()).collect();
    names.extend((0..SLOTS).flat_map(slot_vars));
    let mut r: Vec<(u32, u32)> = names
        .iter()
        .map(|n| {
            let v = crate::layout_var(n).expect("in LAYOUT");
            let a = crate::HUNK_BASES[v.hunk as usize] + v.offset;
            (a, a + v.width.bytes() as u32)
        })
        .collect();
    r.push((h0(OBJECT_MAP), h0(OBJECT_MAP) + MAP_BYTES));
    r.sort_unstable();
    // Touching or overlapping ranges as one, so the copy is a few slices.
    let mut out: Vec<(u32, u32)> = Vec::new();
    for (a, b) in r {
        match out.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => out.push((a, b)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bcd_words_decode() {
        assert_eq!(from_bcd(0x282), 282);
        assert_eq!(from_bcd(0), 0);
        assert_eq!(from_bcd(0x9999), 9999);
    }
}
