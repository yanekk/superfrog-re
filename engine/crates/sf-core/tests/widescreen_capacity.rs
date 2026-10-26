//! real-widescreen T03 (DESIGN §2.4, D4, D11): widescreen has `WIDE_SLOTS` more object slots, bob
//! records, free-list entries, draw-list places and hitbox entries, at an address the original
//! never maps; the 4:3 game never touches them. T00's heaviest scene, replayed from a route file,
//! is `sf-conform/tests/widescreen_capacity.rs` (no files in the pure crates).

use sf_core::input::{FrameInput, Joy};
use sf_core::layout::LAYOUT;
use sf_core::{Game, GameData, LevelId, Start};

fn data() -> &'static GameData {
    sf_conform::game_data()
}

/// Frame `f`'s input: a walk right with jumps and fire, so the camera scrolls and objects spawn.
fn input(f: u32) -> FrameInput {
    let mut joy = Joy::NONE;
    if f % 90 < 60 {
        joy = joy | Joy::RIGHT;
    } else if f % 90 < 75 {
        joy = joy | Joy::LEFT;
    }
    if f % 37 < 4 {
        joy = joy | Joy::UP;
    }
    if f % 53 < 2 {
        joy = joy | Joy::FIRE;
    }
    FrameInput::joy(joy)
}

/// FNV-1a, so a stored hash means the same on every build.
fn fnv(h: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x0100_0000_01b3);
    }
}

/// Every variable of the state but the widescreen ones (named `wide_…`), each frame's picture and
/// Paula writes, over `frames` 4:3 frames of `level`.
fn run_hash(level: &str, frames: u32) -> u64 {
    let mut g = Game::new(data(), Start::Level(level.parse::<LevelId>().unwrap()));
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for f in 0..frames {
        let out = g.step(input(f));
        fnv(&mut h, &out.picture.pixels);
        for p in &out.picture.palettes {
            for c in p {
                fnv(&mut h, &c.to_be_bytes());
            }
        }
        fnv(&mut h, format!("{:?}", out.paula).as_bytes());
    }
    for d in LAYOUT.iter().filter(|d| !d.name.starts_with("wide_")) {
        fnv(&mut h, &g.state().bytes_of(d.id).1);
    }
    h
}

/// The 4:3 game is byte for byte what it was before the extra slots existed: hashes taken on
/// `pir/real-widescreen` at a541fd8, before T03.
#[test]
fn the_4_3_game_is_unchanged() {
    for (level, want) in [
        ("W1-1", 0x6ec8_0e17_29d4_66fe),
        ("W3-2", 0xd828_925a_82f9_071c),
        ("WB-1", 0x27b5_9227_da42_dda9),
    ] {
        assert_eq!(run_hash(level, 2000), want, "{level}: the 4:3 game changed");
    }
}

// --- Widescreen's extra slots, one routine at a time (as `widescreen_spawn.rs` runs them). ---

use std::sync::Arc;

use sf_core::level::beam::Clock;
use sf_core::level::camera::{CAMERA_X, CAMERA_Y, SCROLL_DIRS};
use sf_core::level::objects::{
    self, SLOTS, WIDE_BOB_DRAW_LIST, WIDE_BOB_FREE_LIST, WIDE_HITBOXES, WIDE_LIVE,
    WIDE_SLOTS, draw_list, hitbox_list, is_wide, slot_addrs, slot_at,
};
use sf_core::level::screen::Screen;
use sf_core::level::{LevelCtx, standard_dispatch};
use sf_core::mem::Bus;
use sf_core::render::DisplayData;
use sf_core::state::State;

const CX: u16 = 0x400;
const CY: u16 = 0x200;
/// A hitbox list in the executable: the W2 swooper's, one box.
const ONE_BOX: u32 = sf_core::layout::h0(0x2356e);

fn display() -> Arc<DisplayData> {
    sf_conform::display_data()
}

/// `level`'s state a few frames in, every slot freed, the camera at (`CX`, `CY`) and still (no
/// side to despawn on).
fn empty(level: &str) -> State {
    let mut g = Game::with_display(display(), Start::Level(level.parse::<LevelId>().unwrap()));
    for _ in 0..3 {
        g.step(Default::default());
    }
    let mut st = g.state().clone();
    run(&mut st, Screen::NARROW, objects::obj_free_all);
    let display = display();
    let mut b = Bus::new(&mut st, display.rom());
    b.set_l(CAMERA_X, CX as u32);
    b.set_l(CAMERA_Y, CY as u32);
    b.set_b(SCROLL_DIRS, 0);
    st
}

/// Run `f` on `st` against `screen` with the standard dispatch; the clock where it ended.
fn run<R>(st: &mut State, screen: Screen, f: impl FnOnce(&mut LevelCtx) -> R) -> (R, Clock) {
    let display = display();
    let mut paula = Vec::new();
    let mut c = LevelCtx::new(Bus::new(st, display.rom()), standard_dispatch(), Default::default(), display.sound(), &mut paula);
    c.screen = screen;
    let r = f(&mut c);
    (r, c.clock)
}

fn peek<R>(st: &mut State, f: impl FnOnce(&Bus) -> R) -> R {
    let display = display();
    f(&Bus::new(st, display.rom()))
}

/// `n` GAME OVER signs (template 1: they follow their bob and despawn off the screen), on the
/// screen, `$10` apart; the slots they took.
fn signs(c: &mut LevelCtx, n: u32) -> Vec<u32> {
    (0..n)
        .map(|k| {
            let x = CX + 0x20 + 0x10 * (k % 16) as u16;
            let y = CY + 0x20 + 0x10 * (k / 16) as u16;
            objects::obj_spawn_at_xy_nodedup(c, sf_core::items::template(1), x, y).a0
        })
        .collect()
}

fn live(b: &Bus, screen: Screen) -> Vec<u32> {
    slot_addrs(b, screen).filter(|&s| b.l(s + 0x40) != 0).collect()
}

#[test]
fn the_21st_object_takes_an_extra_slot_and_is_updated_hit_tested_and_drawn() {
    let mut st = empty("W1-1");
    let (slots, _) = run(&mut st, Screen::WIDE, |c| signs(c, 21));
    assert_eq!(&slots[..20], &(0..20).map(slot_at).collect::<Vec<_>>()[..], "the original's 20 first");
    let extra = slots[20];
    assert_eq!(extra, slot_at(SLOTS), "the 21st: the first extra slot");
    let bob = peek(&mut st, |b| {
        assert_eq!(b.w(WIDE_LIVE), 1);
        assert!(is_wide(b.l(extra + 0x50)) && is_wide(b.l(extra + 0x4c)), "its bob and free-list entry are the range's");
        assert_eq!(b.l(extra + 0x4c), WIDE_BOB_FREE_LIST, "the extras' first free entry");
        b.l(extra + 0x50)
    });
    // Updated: the sign's update puts its bob where the slot is.
    let display = display();
    Bus::new(&mut st, display.rom()).set_w(extra + 8, CX + 0x99);
    run(&mut st, Screen::WIDE, objects::obj_run_all);
    assert_eq!(peek(&mut st, |b| b.w(bob - 4)), CX + 0x99, "the extra slot's update ran");
    // Hit-tested: its box is in the wide hitbox list.
    Bus::new(&mut st, display.rom()).set_l(extra + 0x14, ONE_BOX);
    run(&mut st, Screen::WIDE, objects::obj_build_hitboxes);
    peek(&mut st, |b| {
        assert_eq!(hitbox_list(b), WIDE_HITBOXES);
        let x = b.w(extra + 8);
        let hits = objects::overlapping(b, x, x + 4, b.w(extra + 0xa), b.w(extra + 0xa) + 4);
        assert!(hits.contains(&extra), "its hitbox is listed: {hits:x?}");
    });
    // Drawn: its bob in the wide draw list, and the drawer gives it somewhere it was drawn.
    run(&mut st, Screen::WIDE, |c| {
        objects::build_bob_draw_list(c);
        objects::bob_draw(c);
    });
    peek(&mut st, |b| {
        assert_eq!(draw_list(b), WIDE_BOB_DRAW_LIST);
        let listed: Vec<u32> = (0..).map(|k| b.l(WIDE_BOB_DRAW_LIST + 4 * k)).take_while(|&x| x != 0).collect();
        assert_eq!(listed.len(), 22, "the spud and 21 signs");
        assert!(listed.contains(&bob), "the extra's bob is listed");
        assert!([0x10, 0x56, 0x9c, 0xe2].iter().any(|&o| b.l(bob + o) != 0), "the extra's bob was drawn");
    });
    // Freed: its bob goes back to the extras' free list.
    run(&mut st, Screen::WIDE, |c| objects::obj_free(c, extra, 0));
    peek(&mut st, |b| {
        assert_eq!(b.l(WIDE_BOB_FREE_LIST), bob, "back in its entry");
        assert_eq!((b.l(extra + 0x40), b.w(WIDE_LIVE)), (0, 0));
    });
}

#[test]
fn in_4_3_the_21st_object_finds_no_slot() {
    let mut st = empty("W1-1");
    let (slots, _) = run(&mut st, Screen::NARROW, |c| signs(c, 21));
    assert_eq!(slots[20], 0, "no slot");
    assert!(slots[..20].iter().all(|&s| s != 0 && !is_wide(s)));
    peek(&mut st, |b| {
        assert_eq!(b.w(WIDE_LIVE), 0);
        assert_eq!(draw_list(b), objects::BOB_DRAW_LIST);
        assert_eq!(hitbox_list(b), objects::HITBOXES);
    });
}

/// W3's swing with the original's slots full: the ball and its links spawn into extra slots and
/// each link is told its ball's address (`+$64`), which its update follows.
#[test]
fn a_chain_links_ball_can_be_an_extra_slot() {
    let mut st = empty("W3-1");
    run(&mut st, Screen::WIDE, |c| signs(c, 20));
    let cell = peek(&mut st, |b| b.l(sf_core::level::collision::MAP_ROWS + 4 * ((CY as u32 + 0x40) >> 4)) + 2 * ((CX as u32 + 0x80) >> 4));
    run(&mut st, Screen::WIDE, |c| sf_core::enemies::w3::swing::spawn(c, cell));
    let (ball, links) = peek(&mut st, |b| {
        let ball = slot_at(SLOTS);
        assert_eq!(b.l(ball + 0x40), cell, "the ball took the first extra slot");
        let links: Vec<u32> = (1..6).map(|k| slot_at(SLOTS + k)).collect();
        for &l in &links {
            assert_eq!(b.l(l + 0x64), ball, "link {l:x} is told its ball");
        }
        (ball, links)
    });
    // The ball's update counts its five links among the extras' bobs and keeps it; each link
    // finds its ball live.
    run(&mut st, Screen::WIDE, objects::obj_run_all);
    run(&mut st, Screen::WIDE, objects::obj_run_all);
    peek(&mut st, |b| {
        assert!(b.l(ball + 0x40) != 0, "the ball stays: its links are counted");
        for &l in &links {
            assert!(b.l(l + 0x40) != 0, "link {l:x} stays");
        }
    });
}

/// The bonus level's shots spawned with the original's slots full go into extra slots, and
/// `SHOTS` resolves to them (`shots_faster` speeds each one up).
#[test]
fn a_bonus_shots_entry_can_be_an_extra_slot() {
    let mut st = empty("WB-1");
    run(&mut st, Screen::WIDE, |c| signs(c, 20));
    run(&mut st, Screen::WIDE, sf_core::bonus::shots::spawn_shots);
    let shots: Vec<u32> = peek(&mut st, |b| (0..5).map(|k| b.l(sf_core::bonus::shots::SHOTS + 4 * k)).collect());
    assert_eq!(shots, (0..5).map(|k| slot_at(SLOTS + k)).collect::<Vec<_>>(), "the shots are extras");
    let before: Vec<u16> = peek(&mut st, |b| shots.iter().map(|&s| b.w(s + 0xc)).collect());
    run(&mut st, Screen::WIDE, sf_core::bonus::shots::shots_faster);
    peek(&mut st, |b| {
        for (&s, v) in shots.iter().zip(before) {
            assert!(b.l(s + 0x40) != 0);
            assert_eq!(b.w(s + 0xc), if v == 0x1e { v } else { v + 1 }, "shot {s:x} resolved");
        }
    });
}

/// With an evictable object off the screen in an original slot, widescreen still finds an extra
/// slot free and evicts nothing; only with every extra full too is the object evicted.
#[test]
fn find_free_evicts_only_when_every_slot_is_full() {
    let mut st = empty("W1-1");
    let (slots, _) = run(&mut st, Screen::WIDE, |c| signs(c, 20));
    let display = display();
    let evictable = slots[3];
    {
        let mut b = Bus::new(&mut st, display.rom());
        b.set_w(evictable + 0x2a, 1);
        b.set_w(evictable + 8, CX + 0x170 + 65 + 0x40);
        b.set_b(SCROLL_DIRS, 4);
    }
    let one = |st: &mut State| run(st, Screen::WIDE, |c| signs(c, 1)[0]).0;
    assert_eq!(one(&mut st), slot_at(SLOTS), "an extra slot, not an eviction");
    assert!(peek(&mut st, |b| b.l(evictable + 0x40) != 0), "the evictable object is kept");
    let (more, _) = run(&mut st, Screen::WIDE, |c| signs(c, WIDE_SLOTS - 1));
    assert!(more.iter().all(|&s| is_wide(s)), "every extra filled");
    assert_eq!(peek(&mut st, |b| b.w(WIDE_LIVE)), WIDE_SLOTS as u16);
    assert_eq!(one(&mut st), evictable, "now every slot is full: the evictable one is reclaimed");
    assert_eq!(peek(&mut st, |b| live(b, Screen::WIDE).len()), 100);
}

/// D11: a widescreen pass with extra slots live (updated, hit-tested, listed, drawn, collided)
/// spends the same beam time as the same pass with only the original 20 live, and as the 4:3
/// pass of those 20.
#[test]
fn the_extras_cost_no_beam_time() {
    let mut with = empty("W1-1");
    let (slots, _) = run(&mut with, Screen::WIDE, |c| signs(c, 30));
    let display = display();
    {
        let mut b = Bus::new(&mut with, display.rom());
        for &s in &slots {
            b.set_l(s + 0x14, ONE_BOX);
        }
    }
    let mut without = with.clone();
    run(&mut without, Screen::WIDE, |c| {
        for &s in &slots[20..] {
            objects::obj_free(c, s, 0);
        }
    });
    assert_eq!(peek(&mut with, |b| live(b, Screen::WIDE).len()), 30);
    assert_eq!(peek(&mut without, |b| live(b, Screen::WIDE).len()), 20);
    let pass = |c: &mut LevelCtx| {
        objects::obj_run_all(c);
        objects::obj_build_hitboxes(c);
        objects::collide_all(c);
        objects::build_bob_draw_list(c);
        objects::bob_draw(c);
    };
    let (_, wide_extras) = run(&mut with.clone(), Screen::WIDE, pass);
    let (_, wide_20) = run(&mut without.clone(), Screen::WIDE, pass);
    let (_, narrow_20) = run(&mut without.clone(), Screen::NARROW, pass);
    assert!(narrow_20 != Clock::START, "the pass costs time");
    assert_eq!(wide_extras, wide_20, "the extras cost nothing");
    assert_eq!(wide_20, narrow_20, "the 20 cost what they cost in 4:3");
    // Each piece on its own too, so one that charges is named.
    for (name, f) in [
        ("obj_run_all", objects::obj_run_all as fn(&mut LevelCtx)),
        ("obj_build_hitboxes", objects::obj_build_hitboxes),
        ("build_bob_draw_list", objects::build_bob_draw_list),
    ] {
        let (_, a) = run(&mut with.clone(), Screen::WIDE, f);
        let (_, b) = run(&mut without.clone(), Screen::WIDE, f);
        assert_eq!(a, b, "{name}");
    }
}
