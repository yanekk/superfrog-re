//! A route's verdict on the page: failed (it reaches no exit), faults (it does, but is a route the
//! plan's own measures call weak) or clean, with the faults and facts named.
//!
//! The rules are the toolkit's (`amiga_routeview::verdict`, moved there in forest-dumb-level-1
//! T18); this module reads what they need off a Superfrog [`Dump`] and an sf-bench result.

use amiga_routeview::verdict::{self as page, Bench, Facts, Outcome};
use sf_bench::results::LevelResult;

use crate::dump::Dump;
#[cfg(test)]
use crate::dump::Event;

pub use amiga_routeview::verdict::{LONGER_THAN_REF, SLOW_PLAN_S, Status, Verdict};

/// The planner's cell for a frog position (sf-plan `facts::frog_cell`, unclamped).
fn cell((x, y): (i16, i16)) -> (i32, i32) {
    ((x as i32 + 0x17).div_euclid(16), (y as i32 - 0x10).div_euclid(16))
}

/// Where a route that reaches no exit stops making progress (the toolkit's
/// `amiga_routeview::progress`): its `stall` events go into `d.events`, in frame order, and its facts
/// are returned for the verdict.
pub fn progress(d: &mut Dump) -> Vec<String> {
    if d.end.is_some() {
        return Vec::new();
    }
    let facts = Facts {
        track: d.track.iter().map(|&(x, y)| (x as i32, y as i32)).collect(),
        cells: d.track.iter().map(|&p| cell(p)).collect(),
        inputs_end: d.inputs_end,
        exits: d.objects.iter().filter(|o| o.name == "level exit").map(|o| (o.c as i32, o.r as i32)).collect(),
        exit_open_at: d.coins_met_at,
    };
    let stalls = page::progress(&d.events, &facts);
    let mut out = Vec::new();
    for s in stalls {
        d.events.push(s.event);
        out.push(s.fact);
    }
    if !out.is_empty() {
        d.events.sort_by_key(|e| e.k);
    }
    out
}

/// The planner's result as the toolkit's verdict reads it.
pub fn bench(b: &LevelResult) -> Bench {
    Bench {
        completed: b.completed,
        reason: b.reason.clone(),
        wall_s: b.wall_s,
        route_frames: b.route_frames,
        ref_frames: b.ref_frames,
    }
}

/// The verdict for `d` (already through [`progress`] if it reaches no exit), with the planner's
/// bench result for its level when the route is the planner's.
pub fn verdict(d: &Dump, has_route: bool, bench_result: Option<&LevelResult>) -> Verdict {
    let o = Outcome {
        has_route,
        ended: d.end.is_some(),
        exit_open: d.coins_met_at.is_some(),
        coins_left_at_end: d.coins_left_at_end as u32,
        lives_lost: d.lives_lost(),
        hearts_lost: d.hearts_lost(),
        timing: &d.timing,
    };
    page::verdict(&o, bench_result.map(bench).as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dump::Object;

    fn dump(end: Option<u32>, track: Vec<(i16, i16)>, events: Vec<Event>) -> Dump {
        Dump {
            level: "W1-1".parse().unwrap(),
            timing: "app".into(),
            seed: 1,
            coins_needed: 0,
            coins_met_at: Some(0),
            coins_left_at_end: 0,
            notes: vec![],
            palette: vec![],
            body: vec![],
            tiles: vec![],
            objects: vec![Object { c: 20, r: 5, marker: 54, cat: "exit", name: "level exit".into() }],
            inputs_end: track.len() as u32 - 1,
            track,
            inputs: vec![],
            live: vec![],
            events,
            end,
            guide: None,
        }
    }

    fn result(completed: bool, wall_s: f64, route: Option<u32>, refr: Option<u32>) -> LevelResult {
        LevelResult {
            planner: "new".into(),
            set: "C".into(),
            level: "W1-1".into(),
            commit: "x".into(),
            completed,
            wall_s,
            engine_frames: 0,
            oracle_frames: 0,
            confirm_share: 0.0,
            route_frames: route,
            ref_frames: refr,
            hearts_lost: 0,
            reason: if completed { String::new() } else { "over-budget".into() },
            tool: None,
            guided: true,
            threads: None,
        }
    }

    fn hurt(k: u32) -> Event {
        Event { k, x: 0, y: 0, kind: "hurt", text: String::new() }
    }

    #[test]
    fn a_clean_route_has_no_faults() {
        let d = dump(Some(10), vec![(0, 0); 11], vec![]);
        let v = verdict(&d, true, Some(&result(true, 300.0, Some(110), Some(100))));
        assert_eq!(v.status, Status::Clean);
        assert!(v.faults.is_empty());
        assert_eq!(v.info, vec!["planned in 5.0 min", "110 frames vs reference 100 (+10%)"]);
    }

    #[test]
    fn hearts_length_and_slow_plans_are_faults() {
        let d = dump(Some(10), vec![(0, 0); 11], vec![hurt(3), hurt(5)]);
        let v = verdict(&d, true, Some(&result(true, 15.0 * 60.0, Some(126), Some(100))));
        assert_eq!(v.status, Status::Faults);
        assert_eq!(v.faults, vec!["lost 2 hearts", "slow plan: 15.0 of 30 min", "+26% longer than the reference route"]);
        // At exactly +25% and just under half the budget, nothing.
        let d = dump(Some(10), vec![(0, 0); 11], vec![]);
        let v = verdict(&d, true, Some(&result(true, 899.0, Some(125), Some(100))));
        assert_eq!(v.status, Status::Clean);
    }

    #[test]
    fn a_failed_plan_or_no_exit_is_failed() {
        let d = dump(None, vec![(0, 0); 11], vec![]);
        let v = verdict(&d, false, Some(&result(false, 1800.0, None, Some(100))));
        assert_eq!(v.status, Status::Failed);
        assert_eq!(v.faults, vec!["planner failed: over-budget"]);
        let v = verdict(&d, true, None);
        assert_eq!((v.status, v.faults[0].as_str()), (Status::Failed, "the replay never reaches the exit"));
        let mut original = dump(Some(10), vec![(0, 0); 11], vec![]);
        original.timing = "original".into();
        assert_eq!(verdict(&original, true, None).status, Status::Faults);
    }

    #[test]
    fn progress_marks_last_new_ground_and_nearest_open_exit() {
        // The frog walks right one cell a frame to cell 10 (x+$17 = 16c), then back to cell 8 and
        // idles; the exit is at column 20, row 5 (y-$10 = 16r).
        let mut track: Vec<(i16, i16)> = (0..=10).map(|c| (c * 16 - 0x17, 5 * 16 + 0x10)).collect();
        track.extend([(9 * 16 - 0x17, 96), (8 * 16 - 0x17, 96), (8 * 16 - 0x17, 96)]);
        let mut d = dump(None, track, vec![]);
        let facts = progress(&mut d);
        assert_eq!(facts, vec!["last new ground at frame 10; the route's inputs end at frame 13", "closest to the open exit 10 cells, at frame 10"]);
        assert_eq!(d.events.iter().filter(|e| e.kind == "stall").count(), 2);
        // With the coins never met, no nearness to the (locked) exit, and the verdict says so.
        let mut d = dump(None, vec![(0, 96); 4], vec![]);
        d.coins_met_at = None;
        d.coins_left_at_end = 3;
        assert_eq!(progress(&mut d).len(), 1);
        assert!(verdict(&d, true, None).faults.contains(&"exit still locked at the end: 3 coins short".to_string()));
    }
}
