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

/// The ladder's lamp records (h0+$57b6), read from the original's hunk.
fn ladder() -> Vec<u32> {
    let mem = level();
    let l = |a: u32| (0..4).fold(0u32, |v, k| v << 8 | mem.byte(a + k) as u32);
    (0..)
        .map(|k| l(h0(0x57b6) + 4 * k))
        .take_while(|&a| a != 0)
        .collect()
}

/// The award's stubs: the menu (h0+$5eb4), the ladder's top flash and the bust (h0+$5772,
/// h0+$5712, both on to frames the rig does not run) return at once; `sfx_play` records its effect
/// past the lists, inside `slot_menu`.
fn award_stubs(s: &mut Setup) {
    for at in [0x5eb4, 0x5772, 0x5712] {
        s.pokes.push((h0(at), vec![0x4e, 0x75]));
    }
    s.pokes.push((h0(0x12a72), vec![0x1a, 0xc0, 0x4e, 0x75]));
    s.a[5] = h0(0x6220);
}

/// A recorder writing where [`award_stubs`]' `sfx_play` does.
fn award_recorder() -> impl FnMut(&mut sf_core::State, u8, u8) {
    let mut at = h0(0x6220);
    move |s, id, _| {
        s.set_b(at, id);
        at += 1;
    }
}

/// The award's memory before it: the win, the score shown, lives, every lamp's two words and the
/// holds.
fn award_memory(s: &mut Setup, g: &mut Gen) {
    s.poke_w(h0(0x615e), 0);
    s.poke_l(h0(0x57b2), g.next() as u32 % 300_000);
    s.poke_l(h0(0x49d6), g.next() as u32 % 2_000_000);
    s.poke_w(h0(0x82e), g.below(10) as u16);
    for k in 0..26 {
        s.poke_w(h0(0x6842) + 8 * k, edgy(g));
        s.poke_w(h0(0x6842) + 8 * k + 6, edgy(g));
    }
    for k in 0..3 {
        s.poke_w(HOLDS + 2 * k, edgy(g));
    }
}

/// `slot_award` (h0+$51c4) for every prize but the hi-lo game ($a), the gamble not taken (T74):
/// the jackpot's rung (0) goes to the ladder's top flash, a line's (1..=8) and any past the
/// ladder (9, 11: no rung lit) to the menu and `slot_collect`.
#[test]
fn every_prize_is_awarded_as_the_original_awards_it() {
    let mut rig = rig();
    let mut g = Gen::new(740);
    let ladder = ladder();
    let mut case = 0;
    for d7 in (0..=11u16).filter(|&d| d != 0xa) {
        for _ in 0..16 {
            let mut s = Setup::default();
            award_stubs(&mut s);
            award_memory(&mut s, &mut g);
            s.d[7] = d7 as u32;
            let mut next = None;
            assert!(rig.check("h0+$51c4", case, 0x51c4, &s, |c| {
                let mut rec = award_recorder();
                let n = rounds::award_rung(c.bus.st, &ladder, d7, &mut rec);
                if n == rounds::Ladder::Offer {
                    rounds::collect(c.bus.st, d7, &mut rec);
                }
                next = Some(n);
            }));
            let want = if d7 == 0 {
                rounds::Ladder::Jackpot
            } else {
                rounds::Ladder::Offer
            };
            assert_eq!(next, Some(want), "d7 {d7}");
            case += 1;
        }
    }
}

/// The ladder's scan from h0+$520a, the ladder's lamps left as they were (T74): with a rung lit
/// already, the scan finds the best one; the bust's (the last) goes to the bust, the jackpot's (the
/// first) to the ladder's top, any other to the menu offering the gamble, then `slot_collect`.
#[test]
fn the_ladder_is_scanned_as_the_original_scans_it() {
    let mut rig = rig();
    let mut g = Gen::new(741);
    let ladder = ladder();
    let mut seen = std::collections::BTreeSet::new();
    for case in 0..400 {
        let mut s = Setup::default();
        award_stubs(&mut s);
        award_memory(&mut s, &mut g);
        // Most rungs out, so each one is the best lit in some case.
        for &lamp in &ladder {
            let lit = g.below(6) == 0;
            s.poke_w(lamp + 6, if lit { edgy(&mut g).max(1) } else { 0 });
        }
        let d7 = g.below(12) as u16;
        s.d[7] = d7 as u32;
        assert!(rig.check("h0+$520a", case, 0x520a, &s, |c| {
            let mut rec = award_recorder();
            let n = rounds::ladder_rung(c.bus.st, &ladder, d7, &mut rec);
            if n == rounds::Ladder::Offer {
                rounds::collect(c.bus.st, d7, &mut rec);
            }
            seen.insert(format!("{n:?}"));
        }));
    }
    assert_eq!(seen.len(), 3, "{seen:?}");
}

/// `slot_collect` (h0+$5528) for every rung, the jackpot's (0, `slot_jackpot` stubbed `rts`) and
/// a wrong hi-lo guess's -1 (T74).
#[test]
fn every_rung_is_collected_as_the_original_collects_it() {
    let mut rig = rig();
    let mut g = Gen::new(742);
    let mut case = 0;
    for d7 in (0..=11u16).chain([0xffff]) {
        for _ in 0..8 {
            let mut s = Setup::default();
            award_stubs(&mut s);
            s.pokes.push((h0(0x5628), vec![0x4e, 0x75]));
            award_memory(&mut s, &mut g);
            s.d[7] = d7 as u32 | if d7 == 0xffff { 0xffff_0000 } else { 0 };
            let mut jackpot = None;
            assert!(rig.check("h0+$5528", case, 0x5528, &s, |c| {
                jackpot = Some(rounds::collect(c.bus.st, d7, &mut award_recorder()));
            }));
            assert_eq!(jackpot, Some(d7 == 0), "d7 {d7}");
            case += 1;
        }
    }
}
