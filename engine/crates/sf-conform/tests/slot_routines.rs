//! The slot machine's rounds (T73, `sf_core::slot::rounds`) against the original's routines on the
//! routine oracle: the payout check (`slot_payout`, h0+$4eae) for every reel state, and a reel's
//! stop (`slot_reel_stop`, h0+$5d48) for every reel and position. The payout's lamp flash
//! (h0+$504e) waits on frames, which the rig does not run: it is stubbed `rts`, and proven by
//! `slot-fire`. `sfx_play` (h0+$12a72) is stubbed `move.b d0,(a5)+; rts`, so each sound asked
//! for is a byte in `slot_menu`'s lists (a5 at h0+$6160), which the port writes the same way.

mod common;

use common::*;
use sf_conform::routine::{Gen, Setup};
use sf_core::layout::{Ram, h0};
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
        let mut prize = None;
        assert!(rig.check("h0+$4eae", case, 0x4eae, &s, |c| {
            prize = Some(rounds::payout(c.bus.st, &mut recorder()));
        }));
        // The prize, in `d7`'s low word.
        let theirs = rig.last_regs().expect("the original returned").d[7] as u16;
        assert_eq!(prize, Some(theirs), "h0+$4eae case {case}: the prize");
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

/// `slot_award` (h0+$51c4) for each rung a line pays (1..=8), its menu (h0+$5eb4) stubbed `rts`
/// and the gamble not taken: the ladder lit and offered, then `slot_collect` (h0+$5528), as
/// [`rounds::award_rung`] then [`rounds::collect`] (T73 review).
#[test]
fn every_rung_a_line_pays_is_collected_as_the_original_collects_it() {
    let mut rig = rig();
    let mut g = Gen::new(732);
    let mem = level();
    let l = |a: u32| (0..4).fold(0u32, |v, k| v << 8 | mem.byte(a + k) as u32);
    let ladder: Vec<u32> = (0..)
        .map(|k| l(h0(0x57b6) + 4 * k))
        .take_while(|&a| a != 0)
        .collect();
    assert_eq!(ladder.len(), 10, "{ladder:x?}");
    // `sfx_play`'s bytes past the lists, inside `slot_menu`.
    const LOG: u32 = h0(0x6220);
    let mut case = 0;
    for d7 in 1..=8u16 {
        for _ in 0..16 {
            let mut s = Setup::default();
            s.pokes.push((h0(0x5eb4), vec![0x4e, 0x75]));
            s.pokes.push((h0(0x12a72), vec![0x1a, 0xc0, 0x4e, 0x75]));
            s.a[5] = LOG;
            s.d[7] = d7 as u32;
            s.poke_w(h0(0x615e), 0);
            s.poke_l(h0(0x57b2), g.next() as u32 % 300_000);
            s.poke_l(h0(0x49d6), g.next() as u32 % 2_000_000);
            s.poke_w(h0(0x82e), g.below(10) as u16);
            for k in 0..26 {
                s.poke_w(h0(0x6842) + 8 * k, edgy(&mut g));
                s.poke_w(h0(0x6842) + 8 * k + 6, edgy(&mut g));
            }
            for k in 0..3 {
                s.poke_w(HOLDS + 2 * k, edgy(&mut g));
            }
            assert!(rig.check("h0+$51c4", case, 0x51c4, &s, |c| {
                let mut at = LOG;
                let mut rec = move |s: &mut sf_core::State, id: u8, _| {
                    s.set_b(at, id);
                    at += 1;
                };
                assert_eq!(
                    rounds::award_rung(c.bus.st, &ladder, d7, &mut rec),
                    rounds::Ladder::Offer
                );
                rounds::collect(c.bus.st, d7, &mut rec);
            }));
            case += 1;
        }
    }
}
