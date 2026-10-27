//! A route's verdict on the page: failed (it reaches no exit), faults (it does, but is a route the
//! plan's own measures call weak) or clean, with the faults and facts named.
//!
//! The faults are the plan's measures (plans/smart-planner DESIGN §4, RESULTS.md): M6 counts
//! routes that lose no heart; C4 wants no route over +25% of the committed reference; and a plan
//! that used half its 30-minute budget or more is flagged as slow (this tool's own line, not a plan
//! measure).

use sf_bench::results::LevelResult;

use crate::dump::{Dump, Event};

pub const LONGER_THAN_REF: f64 = 0.25;
pub const SLOW_PLAN_S: f64 = 15.0 * 60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Clean,
    Faults,
    Failed,
}

impl Status {
    pub fn word(self) -> &'static str {
        match self {
            Status::Clean => "clean",
            Status::Faults => "faults",
            Status::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    pub status: Status,
    pub faults: Vec<String>,
    pub info: Vec<String>,
}

/// The planner's cell for a frog position (sf-plan `facts::frog_cell`, unclamped).
fn cell((x, y): (i16, i16)) -> (i32, i32) {
    ((x as i32 + 0x17).div_euclid(16), (y as i32 - 0x10).div_euclid(16))
}

/// Where a route that reaches no exit stops making progress: the frame it last stood on a cell it
/// had not been on, and (once the exit is open) its closest approach to an exit cell, in cells
/// (Chebyshev). Both become `stall` events; the facts are returned for the verdict. Frames past the
/// route's last line are left out: there the replay only idles.
pub fn progress(d: &mut Dump) -> Vec<String> {
    if d.end.is_some() || d.track.len() < 2 {
        return Vec::new();
    }
    let last = (d.inputs_end as usize).min(d.track.len() - 1);
    let exits: Vec<(i32, i32)> =
        d.objects.iter().filter(|o| o.name == "level exit").map(|o| (o.c as i32, o.r as i32)).collect();
    let mut seen = std::collections::HashSet::new();
    let mut last_new = 0;
    let mut best: Option<(i32, usize)> = None;
    for (k, &p) in d.track[..=last].iter().enumerate() {
        let c = cell(p);
        if seen.insert(c) {
            last_new = k;
        }
        // Before the coins are met the exit is locked, so nearness to it means nothing.
        if d.coins_met_at.is_some_and(|m| k as u32 >= m) {
            if let Some(dist) = exits.iter().map(|&(x, y)| (c.0 - x).abs().max((c.1 - y).abs())).min() {
                if best.is_none_or(|(b, _)| dist < b) {
                    best = Some((dist, k));
                }
            }
        }
    }
    let mut out = Vec::new();
    let mut stall = |k: usize, text: String| {
        let (x, y) = d.track[k];
        d.events.push(Event { k: k as u32, x: x as i32, y: y as i32, kind: "stall", text });
    };
    stall(last_new, format!("last new ground: no cell after this one is new (inputs end {} frames later)", last - last_new));
    out.push(format!("last new ground at frame {last_new}; the route's inputs end at frame {last}"));
    if let Some((dist, k)) = best {
        stall(k, format!("closest to the open exit: {dist} cells; never closer after this"));
        out.push(format!("closest to the open exit {dist} cells, at frame {k}"));
    }
    d.events.sort_by_key(|e| e.k);
    out
}

/// The verdict for `d` (already through [`progress`] if it reaches no exit), with the planner's
/// bench result for its level when the route is the planner's.
pub fn verdict(d: &Dump, has_route: bool, bench: Option<&LevelResult>) -> Verdict {
    let mut faults = Vec::new();
    let mut info = Vec::new();
    let mut status = Status::Clean;
    if let Some(b) = bench.filter(|b| !b.completed) {
        status = Status::Failed;
        faults.push(format!("planner failed: {}", b.reason));
    } else if !has_route {
        status = Status::Failed;
        faults.push("no route".into());
    } else if d.end.is_none() {
        status = Status::Failed;
        faults.push("the replay never reaches the exit".into());
    }
    if has_route && d.end.is_none() && d.coins_met_at.is_none() {
        faults.push(format!("exit still locked at the end: {} coins short", d.coins_left_at_end));
    }
    let (lives, hearts) = (d.lives_lost(), d.hearts_lost());
    if lives > 0 {
        faults.push(format!("lost {lives} {}", if lives == 1 { "life" } else { "lives" }));
    }
    if hearts > 0 {
        faults.push(format!("lost {hearts} heart{}", if hearts == 1 { "" } else { "s" }));
    }
    if let Some(b) = bench {
        info.push(format!("planned in {:.1} min", b.wall_s / 60.0));
        if b.completed && b.wall_s >= SLOW_PLAN_S {
            faults.push(format!("slow plan: {:.1} of 30 min", b.wall_s / 60.0));
        }
        if let (Some(rf), Some(rr)) = (b.route_frames, b.ref_frames) {
            let dl = rf as f64 / rr as f64 - 1.0;
            info.push(format!("{rf} frames vs reference {rr} ({:+.0}%)", dl * 100.0));
            if dl > LONGER_THAN_REF {
                faults.push(format!("{:+.0}% longer than the reference route", dl * 100.0));
            }
        }
    }
    if d.timing != "app" {
        faults.push("timed for the original: the replay may drift".into());
    }
    if status == Status::Clean && !faults.is_empty() {
        status = Status::Faults;
    }
    Verdict { status, faults, info }
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
