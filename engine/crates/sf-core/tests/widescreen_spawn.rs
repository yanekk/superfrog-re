//! real-widescreen T02 (DESIGN §2.2–§2.4, §2.6): in widescreen objects wake as their cell reaches
//! the wide view's edge and are freed after leaving it, by the original's margins measured from
//! the wide edges; the 4:3 game is the original's.
//!
//! Each test takes a level's state as `Start::Level` leaves it, places the camera, a map marker or
//! an object by hand, and runs one routine with the original's screen and with widescreen's.

use std::sync::Arc;

use sf_core::level::beam::{Clock, Cost};
use sf_core::level::camera::{CAMERA_X, CAMERA_Y, SCROLL_DIRS, SHOOTER_MODE};
use sf_core::level::collision::{LEVEL_STATE, MAP_ROWS, OBJ_WORLD_OFFSET};
use sf_core::level::objects::{self, BOB_FREE_LIST, OBJ_SLOTS, SPUD_BOB};
use sf_core::level::screen::Screen;
use sf_core::level::{Dispatch, Hw, LevelCtx, spawn, standard_dispatch};
use sf_core::mem::Bus;
use sf_core::render::DisplayData;
use sf_core::state::State;
use sf_core::{Game, LevelId, Start};

const SLOT_SIZE: u32 = 0x7c;
/// Scroll directions (`scroll_dirs`): the camera last moved left (the left column is scanned, the
/// right despawn tested), right (the left despawn), and up (the top row).
const LEFT: u8 = 4;
const RIGHT: u8 = 2;
const UP: u8 = 0x10;
/// The marker the test spawners answer to.
const MARK: u16 = 33;

fn display() -> Arc<DisplayData> {
    sf_conform::display_data()
}

/// `level`'s state a few frames in, as `Start::Level` enters it.
fn level(id: &str) -> State {
    let mut g = Game::with_display(display(), Start::Level(id.parse::<LevelId>().unwrap()));
    for _ in 0..3 {
        g.step(Default::default());
    }
    assert!(g.scene().is_level());
    g.state().clone()
}

/// Run `f` on `st` against `screen`, with `dispatch`; the clock where it ended.
fn run<R>(
    st: &mut State,
    screen: Screen,
    dispatch: &Dispatch,
    f: impl FnOnce(&mut LevelCtx) -> R,
) -> (R, Clock) {
    let display = display();
    let mut paula = Vec::new();
    let mut c = LevelCtx::new(Bus::new(st, display.rom()), dispatch, Hw::default(), display.sound(), &mut paula);
    c.screen = screen;
    let r = f(&mut c);
    (r, c.clock)
}

/// The test spawner: clears its cell's marker, so a test sees which cells were spawned, and
/// spends a spawn's time, so a test sees what a spawn cost.
fn take(c: &mut LevelCtx, cell: u32) {
    c.spend(Cost::Spawn);
    c.bus.set_w(cell, c.bus.w(cell) & !0x3f);
}

/// A dispatch whose only spawner is [`take`], for marker [`MARK`] in `world`.
fn taker(world: u8) -> Dispatch {
    let mut d = Dispatch::default();
    d.register_marker(world, MARK as u8, take);
    d
}

fn world(st: &mut State) -> u8 {
    let display = display();
    (Bus::new(st, display.rom()).l(OBJ_WORLD_OFFSET) >> 8) as u8
}

/// The map cell under level pixel (`x`, `y`), as the original addresses it.
fn cell(b: &Bus, x: u16, y: u16) -> u32 {
    b.l(MAP_ROWS + 4 * (y as u32 >> 4)) + 2 * (x as u32 >> 4)
}

/// Set up `st`: the camera at (`cx`, `cy`), `scroll_dirs` `dirs`, every marker in the cells
/// around the view cleared, then `marks` (level pixels) marked [`MARK`].
fn place(st: &mut State, cx: u16, cy: u16, dirs: u8, marks: &[(u16, u16)]) {
    let display = display();
    let mut b = Bus::new(st, display.rom());
    b.set_l(CAMERA_X, cx as u32);
    b.set_l(CAMERA_Y, cy as u32);
    b.set_b(SCROLL_DIRS, dirs);
    for row in (cy.saturating_sub(0x40) >> 4)..((cy + 0x180) >> 4) {
        for col in (cx.saturating_sub(0x80) >> 4)..((cx + 0x200) >> 4) {
            let at = cell(&b, col * 16, row * 16);
            b.set_w(at, b.w(at) & !0x3f);
        }
    }
    for &(x, y) in marks {
        let at = cell(&b, x, y);
        b.set_w(at, b.w(at) & !0x3f | MARK);
    }
}

/// Which of `marks` were spawned (their marker taken).
fn taken(st: &mut State, marks: &[(u16, u16)]) -> Vec<bool> {
    let display = display();
    let b = Bus::new(st, display.rom());
    marks.iter().map(|&(x, y)| b.w(cell(&b, x, y)) & 0x3f != MARK).collect()
}

/// `scan_edges` with the camera at (`cx`, `cy`) moving `dirs`: which of `marks` spawned, per screen.
fn edges(st: &State, cx: u16, cy: u16, dirs: u8, marks: &[(u16, u16)]) -> [Vec<bool>; 2] {
    [Screen::NARROW, Screen::WIDE].map(|screen| {
        let mut st = st.clone();
        let d = taker(world(&mut st));
        place(&mut st, cx, cy, dirs, marks);
        run(&mut st, screen, &d, spawn::scan_edges);
        taken(&mut st, marks)
    })
}

const CX: u16 = 0x400;
const CY: u16 = 0x200;

#[test]
fn the_right_column_moves_out_by_five_cells() {
    let st = level("W1-1");
    // The original's column is the cell 336 pixels right of the camera (`$20` back, 23 cells on);
    // widescreen's 5 cells further.
    let marks = [(CX + 336, CY), (CX + 416, CY)];
    let [narrow, wide] = edges(&st, CX, CY, RIGHT, &marks);
    assert_eq!(narrow, [true, false]);
    assert_eq!(wide, [false, true]);
}

#[test]
fn the_left_column_moves_out_by_five_cells() {
    let st = level("W1-1");
    let marks = [(CX - 0x20, CY), (CX - 0x20 - 80, CY)];
    let [narrow, wide] = edges(&st, CX, CY, LEFT, &marks);
    assert_eq!(narrow, [true, false]);
    assert_eq!(wide, [false, true]);
}

#[test]
fn a_row_spans_the_wider_width() {
    let st = level("W1-1");
    // Below the screen (19 rows on from `$20` above it): 25 cells from `$20` left of the camera,
    // or 35 from 5 cells further left. Above it, the origin's row.
    for (dirs, y) in [(RIGHT, CY - 0x20 + 19 * 16), (RIGHT | UP, CY - 0x20)] {
        let marks = [
            (CX - 0x20 - 80, y),
            (CX - 0x20 - 16, y),
            (CX - 0x20, y),
            (CX - 0x20 + 24 * 16, y),
            (CX - 0x20 + 25 * 16, y),
            (CX - 0x20 + 29 * 16, y),
            (CX - 0x20 + 30 * 16, y),
        ];
        let [narrow, wide] = edges(&st, CX, CY, dirs, &marks);
        assert_eq!(narrow, [false, false, true, true, false, false, false], "dirs {dirs:#x}");
        assert_eq!(wide, [true, true, true, true, true, true, false], "dirs {dirs:#x}");
    }
}

#[test]
fn at_the_levels_left_edge_the_wide_origin_clamps_at_0() {
    let st = level("W1-1");
    // `$30` from the left: the original's origin is cell 1, widescreen's (80 pixels further left)
    // clamps to cell 0, and no scan reaches the row above's end.
    let cx = 0x30;
    let marks = [(0, CY), (16, CY)];
    let [narrow, wide] = edges(&st, cx, CY, LEFT, &marks);
    assert_eq!(narrow, [false, true]);
    assert_eq!(wide, [true, false]);
    let mut st = st.clone();
    let d = taker(world(&mut st));
    place(&mut st, cx, CY, LEFT | UP, &[]);
    let display = display();
    let row_end = {
        let b = Bus::new(&mut st, display.rom());
        cell(&b, 0, CY - 0x20) - 2
    };
    {
        let mut b = Bus::new(&mut st, display.rom());
        b.set_w(row_end, b.w(row_end) & !0x3f | MARK);
    }
    run(&mut st, Screen::WIDE, &d, spawn::scan_edges);
    assert_eq!(Bus::new(&mut st, display.rom()).w(row_end) & 0x3f, MARK, "the scan wrapped");
}

#[test]
fn the_start_scan_spans_the_wider_width() {
    let st = level("W1-1");
    // 24 cells from one cell past the origin; widescreen 34 from 5 cells further left.
    let y = CY + 0x40;
    let marks = [
        (CX - 0x10 - 80, y),
        (CX - 0x10 - 16, y),
        (CX - 0x10, y),
        (CX - 0x10 + 23 * 16, y),
        (CX - 0x10 + 24 * 16, y),
        (CX - 0x10 + 28 * 16, y),
        (CX - 0x10 + 29 * 16, y),
    ];
    let [narrow, wide] = [Screen::NARROW, Screen::WIDE].map(|screen| {
        let mut st = st.clone();
        let d = taker(world(&mut st));
        place(&mut st, CX, CY, RIGHT, &marks);
        run(&mut st, screen, &d, spawn::scan_screen);
        taken(&mut st, &marks)
    });
    assert_eq!(narrow, [false, false, true, true, false, false, false]);
    assert_eq!(wide, [true, true, true, true, true, true, false]);
}

/// DESIGN §2.4, D11: the cells widescreen adds, and what their spawners do, cost no beam time.
#[test]
fn the_scan_cells_widescreen_adds_cost_nothing() {
    let st = level("W1-1");
    let y_row = CY - 0x20 + 19 * 16;
    // Markers in the original's cells of the row and the start scan's first row.
    let originals = [(CX - 0x20, y_row), (CX + 0x100, y_row), (CX - 0x10, CY - 0x20), (CX + 0x80, CY - 0x20)];
    // Markers only widescreen scans: the row's ends, the start scan's ends.
    let extras = [
        (CX - 0x20 - 80, y_row),
        (CX - 0x20 + 30 * 16 - 16, y_row),
        (CX - 0x10 - 80, CY - 0x20),
        (CX - 0x10 + 28 * 16, CY - 0x20),
    ];
    let clock = |screen: Screen, marks: &[(u16, u16)], f: fn(&mut LevelCtx)| {
        let mut st = st.clone();
        let d = taker(world(&mut st));
        place(&mut st, CX, CY, RIGHT, marks);
        let ((), clock) = run(&mut st, screen, &d, f);
        (clock, taken(&mut st, marks))
    };
    let both: Vec<_> = originals.iter().chain(&extras).copied().collect();
    for f in [spawn::scan_edges as fn(&mut LevelCtx), spawn::scan_screen] {
        let (narrow, _) = clock(Screen::NARROW, &originals, f);
        assert!(narrow != Clock::START, "the scan costs time");
        let (wide, _) = clock(Screen::WIDE, &originals, f);
        assert_eq!(wide, narrow, "the original's cells cost what they cost");
        let (wide, spawned) = clock(Screen::WIDE, &both, f);
        assert_eq!(wide, narrow, "widescreen's cells and spawns cost nothing");
        let n = if std::ptr::fn_addr_eq(f, spawn::scan_edges as fn(&mut LevelCtx)) { 2 } else { 4 };
        assert!(spawned.iter().filter(|&&s| s).count() >= n, "the extras were scanned: {spawned:?}");
    }
}

/// Slot 18 holds the test object: its bob is the spud's record, whose frame (width `w`) lives in
/// slot 19's bytes; slot 19 stays free.
fn object(st: &mut State, x: u16, y: u16, w: u16) -> u32 {
    let display = display();
    let mut b = Bus::new(st, display.rom());
    let slot = OBJ_SLOTS + 18 * SLOT_SIZE;
    let spare = OBJ_SLOTS + 19 * SLOT_SIZE;
    b.set_l(spare + 0x40, 0);
    b.set_l(slot + 0x40, 1);
    b.set_l(slot + 0x50, SPUD_BOB);
    b.set_l(slot + 0x4c, spare + 0x30);
    b.set_w(slot + 8, x);
    b.set_w(slot + 0xa, y);
    b.set_l(SPUD_BOB + 0x16, spare + 0x10);
    b.set_l(spare + 0x10, spare + 0x20);
    b.set_w(spare + 0x24, w);
    b.set_w(spare + 0x26, 0x10);
    slot
}

/// Whether the object at (`x`, `y`), `w` wide, survives `f` with the camera at (`CX`, `CY`)
/// moving `dirs`, per screen.
fn survives(st: &State, dirs: u8, x: u16, w: u16, f: fn(&mut LevelCtx, u32, u16)) -> [bool; 2] {
    [Screen::NARROW, Screen::WIDE].map(|screen| {
        let mut st = st.clone();
        place(&mut st, CX, CY, dirs, &[]);
        let slot = object(&mut st, x, CY + 0x40, w);
        run(&mut st, screen, standard_dispatch(), |c| f(c, slot, 0));
        let display = display();
        Bus::new(&mut st, display.rom()).l(slot + 0x40) != 0
    })
}

#[test]
fn the_right_despawn_is_65_further_out() {
    let st = level("W1-1");
    let f = objects::obj_despawn_if_offscreen;
    assert_eq!(survives(&st, LEFT, CX + 0x170 + 64, 0x20, f), [false, true]);
    assert_eq!(survives(&st, LEFT, CX + 0x170 + 65, 0x20, f), [false, false]);
    assert_eq!(survives(&st, LEFT, CX + 0x170 - 1, 0x20, f), [true, true]);
}

#[test]
fn the_left_despawn_is_65_further_out_by_the_frame_width() {
    let st = level("W1-1");
    let f = objects::obj_despawn_if_offscreen;
    for w in [0x10, 0x30] {
        assert_eq!(survives(&st, RIGHT, CX - 0x30 - 65 - w, w, f), [false, true], "width {w}");
        assert_eq!(survives(&st, RIGHT, CX - 0x30 - 65 - w - 1, w, f), [false, false], "width {w}");
        assert_eq!(survives(&st, RIGHT, CX - 0x30 - w, w, f), [true, true], "width {w}");
    }
}

#[test]
fn left_behind_is_measured_from_the_wide_left_edge() {
    let st = level("W1-1");
    let f = objects::obj_free_if_left_behind;
    let w = 0x20;
    assert_eq!(survives(&st, RIGHT, CX - w - 1, w, f), [false, true]);
    assert_eq!(survives(&st, RIGHT, CX - 65 - w, w, f), [false, false]);
    assert_eq!(survives(&st, RIGHT, CX - 65 - w + 1, w, f), [false, true]);
    assert_eq!(survives(&st, RIGHT, CX - w + 1, w, f), [true, true]);
}

/// `find_free`, every slot full (widescreen's extras too, T03): an evictable object in the right
/// strip is on screen in widescreen (kept, no slot found) and off it in 4:3 (evicted).
#[test]
fn find_free_keeps_an_object_in_the_strip() {
    let st = level("W1-1");
    let [narrow, wide] = [Screen::NARROW, Screen::WIDE].map(|screen| {
        let mut st = st.clone();
        place(&mut st, CX, CY, LEFT, &[]);
        let slot = object(&mut st, CX + 0x170 + 30, CY + 0x40, 0x20);
        let display = display();
        {
            let mut b = Bus::new(&mut st, display.rom());
            for k in 0..20 {
                let s = OBJ_SLOTS + k * SLOT_SIZE;
                b.set_l(s + 0x40, 1);
                b.set_w(s + 0x2a, (s == slot) as u16);
                b.set_l(BOB_FREE_LIST + 4 * k, 0);
            }
            for k in 0..objects::WIDE_SLOTS {
                b.set_l(objects::slot_at(objects::SLOTS + k) + 0x40, 1);
            }
            b.set_w(objects::WIDE_LIVE, objects::WIDE_SLOTS as u16);
        }
        let template = objects::OBJ_TEMPLATES;
        let (spawned, _) = run(&mut st, screen, standard_dispatch(), |c| {
            objects::obj_spawn_at_xy_nodedup(c, template, CX, CY)
        });
        assert_eq!(spawned.a0, 0, "no bob record was free");
        Bus::new(&mut st, display.rom()).l(slot + 0x40) != 0
    });
    assert!(!narrow, "4:3 evicts it");
    assert!(wide, "widescreen keeps it");
}

/// The bonus level a few frames in, the camera moved to (`CX`, `CY`) so no scan clamps.
fn bonus() -> (State, u16, u16) {
    let mut st = level("WB-1");
    let display = display();
    let mut b = Bus::new(&mut st, display.rom());
    assert_ne!(b.w(SHOOTER_MODE), 0);
    assert_eq!(b.l(OBJ_WORLD_OFFSET) >> 8, 6);
    b.set_l(CAMERA_X, CX as u32);
    b.set_l(CAMERA_Y, CY as u32);
    (st, CX, CY)
}

/// The bonus level's right column (3 cells down, 16 cells): the cell `cells` on from the
/// original's at `row` cells into it.
fn bonus_cell(st: &mut State, cx: u16, cy: u16, cells: u16, row: u16) -> u32 {
    let display = display();
    let b = Bus::new(st, display.rom());
    cell(&b, cx - 0x20 + (23 + cells) * 16, cy - 0x20 + (3 + row) * 16)
}

fn set_marker(st: &mut State, at: u32, m: u16) {
    let display = display();
    let mut b = Bus::new(st, display.rom());
    b.set_w(at, b.w(at) & !0x3f | m);
}

/// DESIGN §2.6, D12: markers 11 (the chains fly off) and 55 (the level's end) act when the
/// widened right column reaches them, 5 cells before the original's does.
#[test]
fn bonus_markers_11_and_55_act_five_cells_sooner() {
    let (st, cx, cy) = bonus();
    // `bonus_fly_off` is a word, `level_state` a long.
    for (m, flag, set) in [(11u16, objects_fly_off(), 0u32), (55, LEVEL_STATE, 2)] {
        let long = m == 55;
        let [narrow, wide] = [Screen::NARROW, Screen::WIDE].map(|screen| {
            let mut st = st.clone();
            for cells in [0, 5] {
                for row in 0..16 {
                    let at = bonus_cell(&mut st, cx, cy, cells, row);
                    set_marker(&mut st, at, 0);
                }
            }
            let at = bonus_cell(&mut st, cx, cy, 5, 4);
            set_marker(&mut st, at, m);
            let display = display();
            let mut b = Bus::new(&mut st, display.rom());
            if long { b.set_l(flag, 0) } else { b.set_w(flag, 0) }
            run(&mut st, screen, standard_dispatch(), spawn::scan_right_column);
            let b = Bus::new(&mut st, display.rom());
            if long { b.l(flag) } else { b.w(flag) as u32 }
        });
        assert_eq!(narrow, 0, "marker {m}: the original's column has not reached it");
        if set != 0 {
            assert_eq!(wide, set, "marker {m}");
        } else {
            assert_ne!(wide, 0, "marker {m}");
        }
    }
}

fn objects_fly_off() -> u32 {
    sf_core::bonus::enemies::FLY_OFF
}

/// DESIGN §2.3's warning: the bonus level's right column and its despawn move together, so an
/// enemy spawned by widescreen's column is not freed in the pass it spawned.
#[test]
fn a_bonus_enemy_spawned_in_the_strip_is_not_freed_at_once() {
    let (mut st, cx, cy) = bonus();
    for row in 0..16 {
        let at = bonus_cell(&mut st, cx, cy, 5, row);
        set_marker(&mut st, at, 0);
    }
    let at = bonus_cell(&mut st, cx, cy, 5, 4);
    set_marker(&mut st, at, 1);
    run(&mut st, Screen::WIDE, standard_dispatch(), spawn::scan_right_column);
    let display = display();
    let slot = (0..20)
        .map(|k| OBJ_SLOTS + k * SLOT_SIZE)
        .find(|&s| Bus::new(&mut st, display.rom()).l(s + 0x40) == at)
        .expect("marker 1 spawned in the strip");
    let live = |st: &mut State| Bus::new(st, display.rom()).l(slot + 0x40) != 0;
    for f in [objects::obj_despawn_if_offscreen, objects::obj_free_if_left_behind] {
        let mut wide = st.clone();
        run(&mut wide, Screen::WIDE, standard_dispatch(), |c| f(c, slot, 0));
        assert!(live(&mut wide), "widescreen keeps what its column spawned");
    }
    // The hazard the rule avoids: the original's despawn would free it in the same pass.
    let mut narrow = st.clone();
    run(&mut narrow, Screen::NARROW, standard_dispatch(), |c| objects::obj_despawn_if_offscreen(c, slot, 0));
    assert!(!live(&mut narrow));
}

/// The rule's sites hold no literal horizontal edge (DESIGN §3.2, D7): each asks `Screen`.
#[test]
fn no_literal_horizontal_edge_is_left_in_the_sites() {
    let spawn = include_str!("../src/level/spawn.rs");
    for lit in ["0x2e", "0x19", "0x18", "sub(0x20 as"] {
        assert!(!spawn.contains(lit), "spawn.rs still has {lit}");
    }
    let objects = include_str!("../src/level/objects.rs");
    let body = |name: &str| {
        let at = objects.find(&format!("pub fn {name}(")).unwrap();
        let end = objects[at..].find("\n}\n").unwrap();
        &objects[at..at + end]
    };
    let despawn = body("obj_despawn_if_offscreen");
    assert!(!despawn.contains("0x170"));
    // The one `$30` left is the vertical test's.
    assert_eq!(despawn.matches("0x30").count(), 1);
    assert!(despawn.contains("frame + 6)).wrapping_add(0x30)"));
    assert!(body("obj_free_if_left_behind").contains("c.screen.left_behind()"));
}

/// The live objects' x, from the camera.
fn live_x(g: &Game) -> Vec<i32> {
    let w = |a: u32| u16::from_be_bytes(g.peek(a, 2).try_into().unwrap());
    let l = |a: u32| u32::from_be_bytes(g.peek(a, 4).try_into().unwrap());
    let cx = w(CAMERA_X + 2) as i32;
    (0..20)
        .map(|k| OBJ_SLOTS + k * SLOT_SIZE)
        .filter(|&s| l(s + 0x40) != 0)
        .map(|s| w(s + 8) as i32 - cx)
        .collect()
}

/// A restart's `level_init` runs on the front end, which gets the mode through `Front.screen`
/// (T01 review): its start scan is widescreen's, so the restarted level wakes what is in the
/// left strip, which no 4:3 scan reaches.
#[test]
fn a_restart_in_widescreen_wakes_the_strips() {
    use sf_core::frog::FROG;
    use sf_core::input::{FrameInput, Joy};
    let mut off = Game::with_display(display(), Start::Level("W1-1".parse::<LevelId>().unwrap()));
    // No heart left: the first touch costs a life.
    off.poke(FROG + 0x5a, &[0, 0]);
    let walk = |f: u32| {
        let run = if f % 90 < 60 { Joy::RIGHT } else { Joy::NONE };
        FrameInput::joy(run | if f % 37 < 4 { Joy::UP } else { Joy::NONE })
    };
    let mut f = 0;
    while off.scene().is_level() {
        off.step(walk(f));
        f += 1;
        assert!(f < 1500, "the frog never died");
    }
    // Switched on while the front end restarts the level: nothing before it differs.
    let mut on = off.clone();
    on.set_widescreen(true);
    while !off.scene().is_level() {
        off.step(FrameInput::default());
        on.step(FrameInput::default());
        f += 1;
        assert!(f < 2000, "the level never restarted");
    }
    let strip = |g: &Game| live_x(g).into_iter().filter(|&x| x < -0x40).count();
    assert_eq!(strip(&off), 0, "4:3: {:?}", live_x(&off));
    assert!(strip(&on) > 0, "widescreen: {:?}", live_x(&on));
}
