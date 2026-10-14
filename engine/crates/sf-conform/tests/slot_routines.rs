//! The slot machine's rounds (T73, `sf_core::slot::rounds`) against the original's routines on the
//! routine oracle: the payout check (`slot_payout`, h0+$4eae) for every reel state, and a reel's
//! stop (`slot_reel_stop`, h0+$5d48) for every reel and position. The payout's lamp flash
//! (h0+$504e) waits on frames, which the rig does not run: it is stubbed `rts`, and proven by
//! `slot-fire`. `sfx_play` (h0+$12a72) is stubbed `move.b d0,(a5)+; rts`, so each sound asked
//! for is a byte in `slot_menu`'s lists (a5 at h0+$6160), which the port writes the same way.

mod common;

use common::*;
use sf_conform::routine::{Gen, Setup};
use sf_core::layout::h0;
use sf_core::slot::rounds;

/// The reels' positions, the holds, `slot_state`'s flags (h0+$49d0, $49d2, $49da).
const REELS: u32 = h0(0x49c8);
const HOLDS: u32 = h0(0x6154);
const FREE_SPIN: u32 = h0(0x49d0);
const LINE_LAMP: u32 = h0(0x49d2);
const PAID: u32 = h0(0x49da);
/// Where the stubbed `sfx_play` writes the effects asked for.
const SOUNDS: u32 = h0(0x6160);

/// The stubs: the payout's flash returns at once, `sfx_play` records its effect.
fn stubbed(s: &mut Setup) {
    s.pokes.push((h0(0x504e), vec![0x4e, 0x75]));
    s.pokes.push((h0(0x12a72), vec![0x1a, 0xc0, 0x4e, 0x75]));
    s.a[5] = SOUNDS;
}

/// The port's sounds, written where the stub writes them.
fn recorder() -> impl FnMut(&mut sf_core::State, u8, u8) {
    let mut at = SOUNDS;
    move |s, id, _| {
        s.set_b(at, id);
        at += 1;
    }
}

#[test]
fn every_reel_state_pays_as_the_original_pays() {
    let mut rig = rig();
    let mut g = Gen::new(730);
    for case in 0..32 * 32 * 32 {
        let mut s = Setup::default();
        stubbed(&mut s);
        let (a, b, c) = (case as u16 >> 10, case as u16 >> 5 & 31, case as u16 & 31);
        s.poke_w(REELS, a);
        s.poke_w(REELS + 2, b);
        s.poke_w(REELS + 4, c);
        s.poke_w(FREE_SPIN, edgy(&mut g));
        let any = g.next() as u32;
        s.poke_l(LINE_LAMP, g.pick(&[0, h0(0x689a), any]));
        s.poke_w(PAID, edgy(&mut g));
        assert!(rig.check("h0+$4eae", case, 0x4eae, &s, |c| {
            rounds::payout(c.bus.st, &mut recorder());
        }));
    }
}

#[test]
fn every_reel_stops_as_the_original_stops_it() {
    let mut rig = rig();
    let mut g = Gen::new(731);
    let mut case = 0;
    for reel in 0..3u32 {
        for pos in 0..32u16 {
            for _ in 0..4 {
                let mut s = Setup::default();
                stubbed(&mut s);
                for k in 0..3 {
                    s.poke_w(REELS + 2 * k, g.below(32) as u16);
                    s.poke_w(HOLDS + 2 * k, edgy(&mut g));
                }
                s.poke_w(REELS + 2 * reel, pos);
                s.d[0] = pos as u32;
                s.a[0] = HOLDS + 2 * reel;
                s.a[1] = REELS + 2 * reel;
                assert!(rig.check("h0+$5d48", case, 0x5d48, &s, |c| {
                    rounds::reel_stop(c.bus.st, reel, pos, &mut recorder());
                }));
                case += 1;
            }
        }
    }
}
