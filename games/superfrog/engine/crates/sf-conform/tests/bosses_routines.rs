//! The witch (`sf_core::bosses::witch`, T18) against the original's routines, on the routine
//! oracle: her marker's spawner, her init, each of her updates and `touch_boss_hit`.
//!
//! Each routine runs on Moira from the original's memory at the final level's first frame and the
//! port from the same memory transplanted, over generated states. Every byte written, the sound
//! driver's state and the Paula writes must match (`sf_conform::routine`). Her slot is made by the
//! engine's own spawner, so a generated slot is as consistent as the game makes one.

mod common;

use std::sync::OnceLock;

use common::*;
use sf_conform::routine::{Gen, Rig, Setup};
use sf_conform::{SyncPoint, sync};
use sf_core::bosses::witch;
use sf_core::items::template;
use sf_core::layout::{AmigaMemory, Ram, h0};
use sf_core::level::objects::{self, OBJ_SLOTS};

const N: usize = 3000;

const CAMERA_X: u32 = h0(0x16664);
const CAMERA_Y: u32 = h0(0x16660);
const FROG: u32 = h0(0x7d6);
const SLOT: u32 = 0x7c;
/// The final level's row of the dispatch (W6's).
const FINAL: u8 = 5;

/// The original's memory at the final level's first frame.
fn final_level() -> &'static AmigaMemory {
    static M: OnceLock<AmigaMemory> = OnceLock::new();
    M.get_or_init(|| {
        sync::reach(
            &SyncPoint::AtLevel("WW-1".parse().unwrap()),
            &sf_conform::InputScript::idle(),
            1,
        )
        .unwrap_or_else(|e| panic!("{e}"))
        .memory
    })
}

fn final_rig() -> Rig<'static> {
    static D: OnceLock<std::sync::Arc<sf_core::DisplayData>> = OnceLock::new();
    let d = D.get_or_init(sf_conform::display_data);
    Rig::new(final_level(), sound(), d)
}

/// The rig's state with `s`'s pokes in.
fn poked(rig: &Rig, s: &Setup) -> sf_core::State {
    let mut st = rig.state().clone();
    for (a, b) in &s.pokes {
        st.poke(*a, b);
    }
    st
}

fn enough(name: &str, compared: usize, n: usize) {
    assert!(
        compared * 100 >= n * 99,
        "{name}: only {compared} of {n} cases compared"
    );
}

fn pick_or_any(g: &mut Gen, choices: &[u16]) -> u16 {
    if g.one_in(choices.len() as u64 + 1) {
        g.next() as u16
    } else {
        g.pick(choices)
    }
}

/// The witch spawned by the engine at a cell of the arena's (the slots she would take freed
/// first), then `pre`'s pokes: her slot and bob.
fn with_witch(rig: &Rig, g: &mut Gen, s: &mut Setup) -> (u32, u32) {
    let (col, row) = (2 + g.below(290) as u32, 10 + g.below(64) as u32);
    let mut st = rig.state().clone();
    for (a, b) in &s.pokes {
        st.poke(*a, b);
    }
    let free: Vec<u32> = (0..20).filter(|_| g.one_in(3)).collect();
    let display = sf_conform::display_data();
    let mut paula = Vec::new();
    let base = st.clone();
    let slot;
    {
        let mut c = sf_core::level::LevelCtx::new(
            sf_core::mem::Bus::new(&mut st, display.rom()),
            sf_core::level::standard_dispatch(),
            sf_core::level::Hw::default(),
            sound(),
            &mut paula,
        );
        for k in free {
            objects::obj_free(&mut c, OBJ_SLOTS + k * SLOT, 0);
        }
        slot = objects::obj_spawn_cell_alt(&mut c, template(134), cell(col, row), 0, 0).a0;
    }
    s.pokes.extend(diff(&base, &st));
    let bob = if slot == 0 { 0 } else { st.l(slot + 0x50) };
    (slot, bob)
}

/// Every `LAYOUT` byte that differs from `a` in `b`, as pokes.
fn diff(a: &sf_core::State, b: &sf_core::State) -> Vec<(u32, Vec<u8>)> {
    use sf_core::layout::{LAYOUT, VarId};
    let mut out = Vec::new();
    for (d, id) in LAYOUT.iter().zip(VarId::ALL.iter()) {
        let (x, y) = (a.var(*id), b.var(*id));
        let mut k = 0;
        while k < x.len() {
            if x[k] == y[k] {
                k += 1;
                continue;
            }
            let start = k;
            while k < x.len() && x[k] != y[k] {
                k += 1;
            }
            out.push((d.addr + start as u32, y[start..k].to_vec()));
        }
    }
    out
}

/// The camera near her and the frog anywhere on the screen.
fn camera_near(g: &mut Gen, s: &mut Setup, x: u16, y: u16) {
    let near = |g: &mut Gen, v: u16, span: u16| {
        v.wrapping_add(g.below(2 * span as u64 + 1) as u16)
            .wrapping_sub(span)
    };
    s.poke_l(CAMERA_X, near(g, x.saturating_sub(0xa0), 0x100) as u32);
    s.poke_l(CAMERA_Y, near(g, y.saturating_sub(0x80), 0x100) as u32);
    s.poke_w(FROG, near(g, x, 0xa0));
    s.poke_w(FROG + 2, near(g, y, 0x80));
}

/// Her paths' lengths in bytes: the bob path to its `$ffff`, the loop path to its `$ffffffff`.
fn path_len(rig: &Rig, at: u32, long_end: bool) -> u32 {
    let st = rig.state();
    (0..)
        .map(|k| at + 4 * k)
        .find(|&a| if long_end { st.l(a) == 0xffff_ffff } else { st.w(a) == 0xffff })
        .unwrap()
        - at
}

/// Her fields an update reads, varied: speed, the counters, where on either path she is.
fn vary(rig: &Rig, g: &mut Gen, s: &mut Setup, slot: u32) {
    s.poke_w(
        slot + 0xe,
        pick_or_any(g, &[0xffeb, 0xfffd, 0xfffe, 0xffff, 0, 1]),
    );
    s.poke_w(slot + 0x68, pick_or_any(g, &[0, 1, 2, 0x64, 0xffff, 0x8000]));
    s.poke_w(slot + 0x6c, pick_or_any(g, &[0, 1, 2, 0x32, 0xffff]));
    let bob_len = path_len(rig, witch::WITCH_BOB_PATH, false);
    let loop_len = path_len(rig, witch::WITCH_LOOP_PATH, true);
    let (start, at) = if g.one_in(2) {
        let k = g.below(bob_len as u64 / 4 + 1) as u32 * 4;
        (witch::WITCH_BOB_PATH, witch::WITCH_BOB_PATH + k)
    } else {
        let k = g.below(loop_len as u64 / 4 + 1) as u32 * 4;
        (witch::WITCH_LOOP_PATH, witch::WITCH_LOOP_PATH + k)
    };
    s.poke_l(slot + 0x58, start);
    s.poke_l(slot + 0x54, at);
}

#[test]
fn the_witch_and_her_marker_are_registered() {
    let d = sf_core::level::standard_dispatch();
    let m = final_level();
    let long = |a: u32| (0..4).fold(0u32, |v, k| v << 8 | m.byte(a + k) as u32);
    let t = template(134);
    assert_eq!(t, h0(0x221f6));
    assert!(d.kind(134).is_some());
    assert_eq!(long(t), witch::WITCH_INIT);
    assert_eq!(long(t + 4), witch::WITCH_UPDATE);
    assert_eq!(long(t + 0x10), witch::TOUCH_BOSS_HIT);
    assert_eq!(long(t + 0x1c) >> 16, 0x80, "128 hit points");
    for a in [
        witch::WITCH_INIT,
        witch::WITCH_UPDATE,
        witch::WITCH_WAIT,
        witch::WITCH_BOB,
        witch::WITCH_LOOP,
        witch::WITCH_BEATEN,
        witch::TOUCH_BOSS_HIT,
    ] {
        assert!(d.routine(a).is_some(), "${a:x}");
    }
    assert!(d.marker(FINAL, 1).is_some());
}

#[test]
fn the_spawner_matches_the_original() {
    let mut rig = final_rig();
    let mut g = Gen::new(1801);
    let spawn = sf_core::level::standard_dispatch()
        .marker(FINAL, 1)
        .expect("registered");
    let mut compared = 0;
    for case in 0..N {
        let mut s = Setup::default();
        let (slot, _) = if g.one_in(3) {
            with_witch(&rig, &mut g, &mut s)
        } else {
            (0, 0)
        };
        let at = if slot != 0 && g.one_in(2) {
            poked(&rig, &s).l(slot + 0x40) + 0x1300
        } else {
            cell(2 + g.below(290) as u32, 10 + g.below(64) as u32)
        };
        s.poke_l(CAMERA_X, g.below(0x1300) as u32);
        s.poke_l(CAMERA_Y, g.below(0x500) as u32);
        s.a[3] = at;
        compared += rig.check("marker 1", case, 0x18aa8, &s, |c| spawn(c, at)) as usize;
    }
    enough("spawner", compared, N);
}

#[test]
fn init_matches_the_original() {
    let mut rig = final_rig();
    let mut g = Gen::new(1802);
    let port = sf_core::level::standard_dispatch()
        .routine(witch::WITCH_INIT)
        .unwrap();
    let (mut compared, mut cases) = (0, 0);
    for case in 0..N {
        let mut s = Setup::default();
        let (slot, _) = with_witch(&rig, &mut g, &mut s);
        if slot == 0 {
            continue;
        }
        cases += 1;
        let st = poked(&rig, &s);
        camera_near(&mut g, &mut s, st.w(slot + 8), st.w(slot + 0xa));
        s.a[0] = slot;
        compared += rig.check("init", case, 0x259ae, &s, |c| port(c, slot, 0)) as usize;
    }
    enough("init", compared, cases);
}

#[test]
fn updates_match_the_original() {
    let mut rig = final_rig();
    let mut g = Gen::new(1803);
    let updates = [0x25a16, 0x259fc, 0x25a7c, 0x25b40, 0x259ec];
    let (mut compared, mut cases) = (0, 0);
    for case in 0..N {
        let mut s = Setup::default();
        let (slot, _) = with_witch(&rig, &mut g, &mut s);
        if slot == 0 {
            continue;
        }
        cases += 1;
        let update = g.pick(&updates);
        vary(&rig, &mut g, &mut s, slot);
        let st = poked(&rig, &s);
        let (x, y) = (st.w(slot + 8), st.w(slot + 0xa));
        let y = if g.one_in(2) { y } else { y.wrapping_add(g.below(0x200) as u16) };
        s.poke_w(slot + 0xa, y);
        camera_near(&mut g, &mut s, x, y);
        s.a[0] = slot;
        let port = sf_core::level::standard_dispatch()
            .routine(h0(update))
            .expect("ported");
        compared += rig.check(&format!("update h0+${update:x}"), case, update, &s, |c| {
            port(c, slot, 0)
        }) as usize;
    }
    enough("updates", compared, cases);
}

/// Both of her patterns end where they turn into the other: the bob path at its `$ffff` with
/// `+$68` either side of 0, the loop path at its end (which runs the bob pattern the same pass).
#[test]
fn the_patterns_turn_at_their_ends() {
    let mut rig = final_rig();
    let mut g = Gen::new(1804);
    let mut case = 0;
    let bob_end = witch::WITCH_BOB_PATH + path_len(&rig, witch::WITCH_BOB_PATH, false);
    let loop_end = witch::WITCH_LOOP_PATH + path_len(&rig, witch::WITCH_LOOP_PATH, true);
    for (update, at, start) in [
        (0x25a7c, bob_end, witch::WITCH_BOB_PATH),
        (0x25a7c, bob_end - 4, witch::WITCH_BOB_PATH),
        (0x25b40, loop_end, witch::WITCH_LOOP_PATH),
        (0x25b40, loop_end - 4, witch::WITCH_LOOP_PATH),
    ] {
        for left in [0u16, 1, 2, 0xffff] {
            case += 1;
            let mut s = Setup::default();
            let (slot, _) = with_witch(&rig, &mut g, &mut s);
            assert!(slot != 0);
            s.poke_l(slot + 0x54, at);
            s.poke_l(slot + 0x58, start);
            s.poke_w(slot + 0x68, left);
            let st = poked(&rig, &s);
            camera_near(&mut g, &mut s, st.w(slot + 8), st.w(slot + 0xa));
            s.a[0] = slot;
            let port = sf_core::level::standard_dispatch().routine(h0(update)).unwrap();
            assert!(
                rig.check("turn", case, update, &s, |c| port(c, slot, 0)),
                "h0+${update:x} at ${at:x}, +$68 {left}: not compared"
            );
        }
    }
}

#[test]
fn the_hit_counter_matches_the_original() {
    let mut rig = final_rig();
    let mut g = Gen::new(1805);
    let port = sf_core::level::standard_dispatch()
        .routine(witch::TOUCH_BOSS_HIT)
        .unwrap();
    let (mut compared, mut cases) = (0, 0);
    for case in 0..N {
        let mut s = Setup::default();
        let (slot, bob) = with_witch(&rig, &mut g, &mut s);
        if slot == 0 {
            continue;
        }
        cases += 1;
        s.poke_w(slot + 0x1c, pick_or_any(&mut g, &[1, 1, 2, 0x80, 0]));
        s.poke_w(bob + 0x1a, g.next() as u16);
        let d7 = g.below(2) as u16;
        s.d[7] = d7 as u32;
        s.a[5] = slot;
        compared += rig.check("touch_boss_hit", case, 0x236b4, &s, |c| port(c, slot, d7)) as usize;
    }
    enough("touch", compared, cases);
}
