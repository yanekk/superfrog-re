//! What each object value of W1-1 does, measured on the original (plans/forest-dumb-level-1 T21,
//! `docs/10-object-system.md`). Every claim is one scripted run from the level sync point: hold
//! right, and the character slides down the first slope from `W1-1:7,7` to `W1-1:42,35` in about
//! 135 passes, its edge redraws bringing new map cells into view as it goes. A value either lies on
//! that path in the level as shipped (values 1 and 6), or is placed in a map cell on the path at the
//! sync point, before the cell has come into view, so the original's own code meets it there. T31's
//! counter shows each claimed entry point ran; the state after the run shows what it did.
//!
//! Needs the disks and the Kickstart: absent, it fails naming where it looked.

use std::collections::{BTreeMap, BTreeSet};

use amiga_oracle::{Amiga, FrameInput, Joy};
use fd_conform::freeze::{self, FROZEN};
use fd_conform::{HUNK_BASES, sync};

/// Passes run in each claim: the slide ends against a wall at pass ~135.
const PASSES: u32 = 160;

const CHAR_X: u32 = 0x1e5a;
const CHAR_Y: u32 = 0x1e5e;
const SCORE: u32 = 0x2a66;
const COUNTDOWN: u32 = 0x6a80;
const ITEMS_LEFT: u32 = 0x6a88;
const LEVEL_EXIT_TOUCHED: u32 = 0x2696;
/// `creature_active`: five flag bytes; `creature_slots`: five of x.w, y.w, kind.w, dir.b, phase.b.
const CREATURE_ACTIVE: u32 = 0x67da;
const CREATURE_SLOTS: u32 = 0x67ec;
/// `clxdat_copy`: `char_hit_check` copies CLXDAT here each pass.
const CLXDAT_COPY: u32 = 0x1fe6;
/// The object map in hunk 0, 96 cells a column.
const OBJECT_MAP: u32 = 0xcb38;

/// Entry points the claims name (hunk 0 offsets): routine starts, or the first instruction of the
/// branch a value takes inside one.
mod at {
    /// `h0_681c`: a creature value scrolled into view takes a free slot.
    pub const SPAWN: u32 = 0x681c;
    /// `h0_6882`: a value `$20`-`$3f` scrolled into view joins the animated-cell list.
    pub const ANIM_ADD: u32 = 0x6882;
    /// `h0_a06`, `h0_ce4`, `h0_104c`, `h0_1390`: the bottom, top, right and left edge redraws blit a
    /// value `$40`+ into the screen.
    pub const BONUS_DRAW: [u32; 4] = [0xa06, 0xce4, 0x104c, 0x1390];
    /// `h0_26ec` branches: a counted item (`$20`-`$27`), a bonus (`$40`-`$5f`), the exit (`$ff`).
    pub const ITEM_TAKEN: u32 = 0x2722;
    pub const BONUS_TAKEN: u32 = 0x2788;
    pub const EXIT_TOUCHED: u32 = 0x2708;
    /// `h0_24ce` branches, one per spring value `$10`..`$13`.
    pub const SPRING: [u32; 4] = [0x2534, 0x2578, 0x25bc, 0x2602];
    /// `h0_21c0`: redraws the spring's tiles six passes after it fired.
    pub const SPRING_REDRAW: u32 = 0x21d8;
    /// `h0_2fea`'s branch for each creature kind 0 to 5.
    pub const KIND: [u32; 6] = [0x3070, 0x31be, 0x31e4, 0x32da, 0x33ac, 0x3436];
    /// `h0_62b6`'s drawing branches: items `$20`-`$27`, water `$28`, vanishing block `$30`-`$33`.
    pub const DRAW_ITEM: u32 = 0x6398;
    pub const DRAW_WATER: u32 = 0x63b0;
    pub const DRAW_VANISH: u32 = 0x65e2;
    /// `h0_62b6` writing `collision_map` solid under a vanishing block, and clearing it.
    pub const VANISH_SOLID: u32 = 0x6672;
    pub const VANISH_CLEAR: u32 = 0x66c2;
}

fn h0(offset: u32) -> u32 {
    HUNK_BASES[0] + offset
}

fn word(am: &Amiga, offset: u32) -> u16 {
    let b = am.peek(h0(offset), 2);
    u16::from_be_bytes([b[0], b[1]])
}

fn long(am: &Amiga, offset: u32) -> u32 {
    let b = am.peek(h0(offset), 4);
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

fn cell(col: u32, row: u32) -> u32 {
    h0(OBJECT_MAP) + col * 96 + row
}

/// The values the original keeps about the character and the score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct State {
    x: u32,
    y: u32,
    score: u32,
    items_left: u16,
    countdown: u16,
}

fn state(am: &Amiga) -> State {
    State {
        x: long(am, CHAR_X),
        y: long(am, CHAR_Y),
        score: long(am, SCORE),
        items_left: word(am, ITEMS_LEFT),
        countdown: word(am, COUNTDOWN),
    }
}

/// What one run showed.
#[derive(Debug)]
struct Seen {
    before: State,
    after: State,
    /// Executions of each watched entry point, by hunk 0 offset.
    hits: BTreeMap<u32, u32>,
    /// Every creature kind seen in an active slot.
    kinds: BTreeSet<u16>,
    /// The placed cells' values after the run.
    cells: Vec<u8>,
    /// Passes on which `level_exit_touched` was set.
    exit_passes: u32,
    /// Every CLXDAT value `char_hit_check` copied.
    clxdat: BTreeSet<u16>,
}

impl Seen {
    fn hits(&self, offset: u32) -> u32 {
        self.hits.get(&offset).copied().unwrap_or(0)
    }
}

fn watched() -> Vec<u32> {
    let mut v = vec![
        at::SPAWN,
        at::ANIM_ADD,
        at::ITEM_TAKEN,
        at::BONUS_TAKEN,
        at::EXIT_TOUCHED,
        at::SPRING_REDRAW,
        at::DRAW_ITEM,
        at::DRAW_WATER,
        at::DRAW_VANISH,
        at::VANISH_SOLID,
        at::VANISH_CLEAR,
    ];
    v.extend(at::BONUS_DRAW);
    v.extend(at::SPRING);
    v.extend(at::KIND);
    v.extend(FROZEN.iter().map(|f| f.offset));
    v
}

/// From the sync point, place `placed` (col, row, value) in the object map, optionally freeze what
/// `freeze.rs` lists, then hold right for `passes` passes.
fn slide(placed: &[(u32, u32, u8)], frozen: bool, passes: u32) -> Seen {
    let mut am = sync::reach_level_1(passes + 4).unwrap_or_else(|e| panic!("{e}"));
    for &(c, r, v) in placed {
        am.poke(cell(c, r), &[v]);
    }
    if frozen {
        freeze::apply(&mut am, FROZEN).unwrap_or_else(|e| panic!("{e}"));
    }
    let offsets = watched();
    let pcs: Vec<u32> = offsets.iter().map(|&o| h0(o)).collect();
    am.watch_pcs(&pcs);
    let before = state(&am);
    let mut hits = BTreeMap::new();
    let mut kinds = BTreeSet::new();
    let mut exit_passes = 0;
    let mut clxdat = BTreeSet::new();
    for _ in 0..passes {
        am.step(FrameInput::joy(Joy::RIGHT))
            .unwrap_or_else(|e| panic!("{e}"));
        for (pc, n) in am.pc_hits() {
            let i = pcs.iter().position(|p| p == pc).unwrap();
            *hits.entry(offsets[i]).or_insert(0) += n;
        }
        for s in 0..5 {
            if am.peek(h0(CREATURE_ACTIVE + s), 1)[0] != 0 {
                kinds.insert(word(&am, CREATURE_SLOTS + s * 8 + 4));
            }
        }
        exit_passes += u32::from(word(&am, LEVEL_EXIT_TOUCHED) != 0);
        clxdat.insert(word(&am, CLXDAT_COPY));
    }
    Seen {
        before,
        after: state(&am),
        hits,
        kinds,
        cells: placed
            .iter()
            .map(|&(c, r, _)| am.peek(cell(c, r), 1)[0])
            .collect(),
        exit_passes,
        clxdat,
    }
}

/// Body cell on the path at pass ~71, when `h0+$4ed0` is clear: `char_object_cells` reads it.
const BODY: (u32, u32) = (24, 20);
/// A cell the character passes at pass ~83: a creature placed here spawns as the bottom edge
/// scrolls over it.
const AHEAD: (u32, u32) = (28, 23);
/// Under the character's feet at pass ~107, while `h0+$4ed0` is set: `h0_24ce` reads it.
const FEET: (u32, u32) = (36, 30);

/// Every claim is checked; failures are listed together, each naming its value.
fn check(
    claims: Vec<(
        u8,
        Box<dyn Fn(&Seen) -> Result<(), String> + Send>,
        (u32, u32),
    )>,
) {
    let results: Vec<(u8, Result<(), String>)> = std::thread::scope(|s| {
        let handles: Vec<_> = claims
            .into_iter()
            .map(|(v, f, (c, r))| {
                s.spawn(move || {
                    let seen = slide(&[(c, r, v)], false, PASSES);
                    (v, f(&seen).map_err(|e| format!("{e}\n    seen: {seen:x?}")))
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let failed: Vec<String> = results
        .into_iter()
        .filter_map(|(v, r)| r.err().map(|e| format!("value ${v:02x}: {e}")))
        .collect();
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

fn ensure(ok: bool, what: impl Into<String>) -> Result<(), String> {
    if ok { Ok(()) } else { Err(what.into()) }
}

type Claim = (
    u8,
    Box<dyn Fn(&Seen) -> Result<(), String> + Send>,
    (u32, u32),
);

/// Values 1 to 6: a creature of kind `value - 1` takes a slot when its cell scrolls into view, and
/// `h0_2fea` runs that kind's branch every pass. The cell is cleared at the spawn and written again
/// (`h0_2e0e`) only when the creature leaves the view.
#[test]
fn values_1_to_6_spawn_a_creature_of_kind_value_minus_one() {
    let claims: Vec<Claim> = (1u8..=6)
        .map(|v| -> Claim {
            (
                v,
                Box::new(move |s: &Seen| {
                    let kind = v as usize - 1;
                    ensure(s.hits(at::SPAWN) >= 1, "no spawn (h0_681c)")?;
                    ensure(
                        s.kinds.contains(&(kind as u16)),
                        format!("no slot of kind {kind}"),
                    )?;
                    ensure(
                        s.hits(at::KIND[kind]) > 20,
                        format!("kind {kind}'s branch of h0_2fea ran too little"),
                    )?;
                    ensure(s.after.score == s.before.score, "the score moved")
                }),
                AHEAD,
            )
        })
        .collect();
    check(claims);
}

/// The level's own creatures on the path: value 1 at `W1-1:26,19` and value 6 at `W1-1:30,11`
/// spawn as kinds 0 and 5 with nothing placed.
#[test]
fn the_creatures_on_the_first_slope_spawn_unplaced() {
    let s = slide(&[], false, PASSES);
    assert!(s.hits(at::SPAWN) >= 2, "{s:x?}");
    assert!(s.kinds.contains(&0) && s.kinds.contains(&5), "{s:x?}");
    assert!(
        s.hits(at::KIND[0]) > 100 && s.hits(at::KIND[5]) > 5,
        "{s:x?}"
    );
    // Nothing else on the slope: no item, bonus, spring or exit is taken.
    for e in [at::ITEM_TAKEN, at::BONUS_TAKEN, at::EXIT_TOUCHED] {
        assert_eq!(s.hits(e), 0, "h0+${e:x} {s:x?}");
    }
    assert_eq!((s.after.x, s.after.y), (669, 553), "where the slide ends");
}

/// `$20`-`$23`: a counted item. Drawn animated by `h0_62b6`, taken by `h0_26ec`: +100 (BCD) to the
/// score, `items_left` -1, the cell cleared.
#[test]
fn values_20_to_23_are_counted_items() {
    let claims: Vec<Claim> = (0x20u8..=0x23)
        .map(|v| -> Claim {
            (
                v,
                Box::new(|s: &Seen| {
                    ensure(s.hits(at::ANIM_ADD) >= 1, "not added to the animated cells")?;
                    ensure(
                        s.hits(at::DRAW_ITEM) >= 1,
                        "not drawn as an item by h0_62b6",
                    )?;
                    ensure(s.hits(at::ITEM_TAKEN) == 1, "not taken once")?;
                    ensure(s.after.score == s.before.score + 0x100, "score not +100")?;
                    ensure(
                        s.after.items_left == s.before.items_left - 1,
                        "items_left not -1",
                    )?;
                    ensure(s.cells == [0], "cell not cleared")
                }),
                BODY,
            )
        })
        .collect();
    check(claims);
}

/// `$40`-`$44`, `$48`: a bonus. Blitted into the screen by the edge redraw that brings it into view,
/// taken by `h0_26ec` for its score (BCD); `items_left` does not move.
#[test]
fn values_40_to_48_are_bonuses_for_their_score() {
    let claims: Vec<Claim> = [
        (0x40u8, 0x20u32),
        (0x41, 0x30),
        (0x42, 0x40),
        (0x43, 0x50),
        (0x44, 0x60),
        (0x48, 0x100),
    ]
    .into_iter()
    .map(|(v, points)| -> Claim {
        (
            v,
            Box::new(move |s: &Seen| {
                let drawn: u32 = at::BONUS_DRAW.iter().map(|&o| s.hits(o)).sum();
                ensure(drawn >= 1, "not drawn by an edge redraw")?;
                ensure(s.hits(at::BONUS_TAKEN) == 1, "not taken once")?;
                ensure(
                    s.after.score == s.before.score + points,
                    format!("score not +{points:x}"),
                )?;
                ensure(
                    s.after.items_left == s.before.items_left,
                    "items_left moved",
                )?;
                ensure(s.cells == [0], "cell not cleared")
            }),
            BODY,
        )
    })
    .collect();
    check(claims);
}

/// `$28`: water, animated by `h0_62b6`; `h0_26ec` ignores it, so touching it changes no state.
/// `$30`-`$33`: a vanishing block, animated by `h0_62b6`, which writes `collision_map` solid under
/// it and clears it at the empty frame.
#[test]
fn value_28_is_water_and_30_to_33_are_vanishing_blocks() {
    let mut claims: Vec<Claim> = vec![(
        0x28,
        Box::new(|s: &Seen| {
            ensure(s.hits(at::DRAW_WATER) >= 1, "not drawn as water")?;
            ensure(s.after.score == s.before.score, "score moved")?;
            ensure(s.cells == [0x28], "cell changed")
        }),
        BODY,
    )];
    for v in 0x30u8..=0x33 {
        claims.push((
            v,
            Box::new(move |s: &Seen| {
                ensure(
                    s.hits(at::DRAW_VANISH) >= 1,
                    "not drawn as a vanishing block",
                )?;
                ensure(
                    s.hits(at::VANISH_SOLID) + s.hits(at::VANISH_CLEAR) >= 1,
                    "collision_map not written",
                )?;
                ensure(s.cells == [v], "cell changed")
            }),
            BODY,
        ));
    }
    check(claims);
}

/// `$10`-`$13`: a spring. Under the feet while `h0+$4ed0` is set, `h0_24ce` fires once and the
/// character is thrown up instead of ending the slide; `h0_21c0` redraws the spring six passes
/// later. The cell stays.
#[test]
fn values_10_to_13_are_springs() {
    let claims: Vec<Claim> = (0x10u8..=0x13)
        .map(|v| -> Claim {
            (
                v,
                Box::new(move |s: &Seen| {
                    ensure(
                        s.hits(at::SPRING[(v - 0x10) as usize]) == 1,
                        "not fired once",
                    )?;
                    ensure(s.hits(at::SPRING_REDRAW) == 1, "spring not redrawn")?;
                    ensure(s.after.y < 400, format!("not thrown up: y {}", s.after.y))?;
                    ensure(s.cells == [v], "cell changed")
                }),
                FEET,
            )
        })
        .collect();
    check(claims);
}

/// A spring under the feet while `h0+$4ed0` is clear does nothing (`h0_24ce` returns at once).
#[test]
fn a_spring_needs_the_flag_at_4ed0() {
    let s = slide(&[(24, 21, 0x10)], false, PASSES);
    assert_eq!(s.hits(at::SPRING[0]), 0, "{s:x?}");
    assert_eq!((s.after.x, s.after.y), (669, 553), "{s:x?}");
}

/// `$ff`: the exit. `char_object_cells` sets `level_exit_touched` on every pass the character is on
/// it; nothing else happens while items are left (docs/09-play-measured.md measured the level end).
#[test]
fn value_ff_is_the_exit() {
    let s = slide(&[(BODY.0, BODY.1, 0xff)], false, PASSES);
    assert!(
        s.hits(at::EXIT_TOUCHED) >= 1 && s.exit_passes >= 1,
        "{s:x?}"
    );
    assert_eq!(s.after.score, s.before.score, "{s:x?}");
    assert_eq!(s.cells, [0xff], "{s:x?}");
}

/// With `freeze.rs`'s list patched in, every value placed above does nothing: no creature, no
/// animated cell, no bonus drawn, nothing taken, no spring. The character slides as with an empty
/// map, which is what a frozen case compares with an engine that models no objects.
#[test]
fn freezing_stops_every_object() {
    let placed = [
        (AHEAD.0, AHEAD.1, 0x02),
        (BODY.0, BODY.1, 0x20),
        (BODY.0 + 1, BODY.1, 0x40),
        (BODY.0 + 2, BODY.1 + 1, 0x30),
        (FEET.0, FEET.1, 0x10),
    ];
    let s = slide(&placed, true, PASSES);
    assert!(s.kinds.is_empty(), "a creature took a slot: {s:x?}");
    for e in [
        at::ANIM_ADD + 4, // past the frozen `movem`: never reached
        at::ITEM_TAKEN,
        at::BONUS_TAKEN,
        at::DRAW_ITEM,
        at::DRAW_VANISH,
    ]
    .into_iter()
    .chain(at::SPRING)
    .chain(at::KIND)
    {
        assert_eq!(s.hits(e), 0, "h0+${e:x} ran frozen: {s:x?}");
    }
    assert_eq!(
        (s.after.score, s.after.items_left),
        (s.before.score, s.before.items_left),
        "{s:x?}"
    );
    assert_eq!((s.after.x, s.after.y), (669, 553), "{s:x?}");
    // Each frozen entry was still reached (its `rts` ran), so the freeze is not vacuous; of the four
    // edge redraws' bonus blits, the one whose edge brought the bonus in.
    for f in FROZEN
        .iter()
        .filter(|f| !at::BONUS_DRAW.contains(&f.offset))
    {
        assert!(
            s.hits(f.offset) > 0,
            "frozen {} never reached: {s:x?}",
            f.what
        );
    }
    let drawn: u32 = at::BONUS_DRAW.iter().map(|&o| s.hits(o)).sum();
    assert!(drawn > 0, "no frozen bonus blit reached: {s:x?}");
}

/// The oracle reports no sprite-to-playfield collision: vAmiga leaves `DENISE_CLX_SPR_PLF` off and
/// `amiga-oracle` never sets it, so CLXDAT reads `$8000` and `char_hit_check` never sees contact.
/// Creature contact is not measurable until T59 turns it on; T59 changes this test.
#[test]
fn the_oracle_reports_no_sprite_collision_until_t59() {
    let s = slide(&[(AHEAD.0, AHEAD.1, 0x02)], false, PASSES);
    assert_eq!(s.clxdat, BTreeSet::from([0x8000]), "{s:x?}");
}

/// Every non-zero value in `_MapaBobow1`'s used columns, as shipped, has a row in
/// `docs/10-object-system.md`'s table, and that table names no value the map lacks.
#[test]
fn every_value_in_the_w1_1_object_map_has_a_row_in_the_note() {
    let dir = fd_tools::disk_dir().unwrap_or_else(|e| panic!("{e}"));
    let map = fd_tools::game_file(&dir, "disk2/_MapaBobow1").unwrap_or_else(|e| panic!("{e}"));
    let mut in_map: BTreeMap<u8, u32> = BTreeMap::new();
    for &v in map[..240 * 96].iter().filter(|&&v| v != 0) {
        *in_map.entry(v).or_insert(0) += 1;
    }
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/10-object-system.md");
    let note = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let rows: BTreeMap<u8, u32> = note
        .lines()
        .filter_map(|l| {
            let mut cols = l.strip_prefix("| `$")?.split('|');
            let v = u8::from_str_radix(cols.next()?.trim().trim_end_matches('`'), 16).ok()?;
            let count = cols.next()?.trim().parse().ok()?;
            Some((v, count))
        })
        .collect();
    assert_eq!(
        rows, in_map,
        "docs/10's table (value, count) against the map"
    );
}
