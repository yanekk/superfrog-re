//! A route's verdict on the page: failed (it reaches no exit), faults (it does, but is a route the
//! planner's own measures call weak) or clean, with the faults and facts named; and, for a route
//! that reaches no exit, where it stops making progress.
//!
//! The faults are the planner's measures: a route should lose no heart, should be no more than 25%
//! longer than the committed reference route, and a plan that used half its 30-minute budget or
//! more is flagged as slow (this tool's own line, not a planner measure).
//!
//! Everything here is decided from plain facts a game reads off its own replay ([`Outcome`],
//! [`Facts`]) and its planner's result for the level ([`Bench`]), so no game's types enter it.

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

/// Something that happened on a frame of the replay, drawn on the map and listed under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub k: u32,
    /// Level pixels: the character's track position for its own events, a cell's top-left for a
    /// map marker's.
    pub x: i32,
    pub y: i32,
    /// `marker`, `hurt`, `death`, `jump`, `land`, `fire`, `end`, `stall`: the page's event types.
    pub kind: &'static str,
    pub text: String,
}

/// What [`progress`] needs of a replay, read off it by the game.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Facts {
    /// The character's track position after each frame played (an [`Event`]'s x, y).
    pub track: Vec<(i32, i32)>,
    /// The planner's cell for each of those positions, in the game's own cell rule (unclamped).
    pub cells: Vec<(i32, i32)>,
    /// The frame of the route's last input line: past it the replay only idles.
    pub inputs_end: u32,
    /// The cells of the level's exits.
    pub exits: Vec<(i32, i32)>,
    /// The first frame the exit is open (every coin it needs met), if it ever is.
    pub exit_open_at: Option<u32>,
}

/// A place the route stops making progress: the event drawn on the map and the fact the verdict
/// lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stall {
    pub event: Event,
    pub fact: String,
}

/// Where a route that reaches no exit stops making progress: the frame it last stood on a cell it
/// had not been on, and (once the exit is open) its closest approach to an exit cell, in cells
/// (Chebyshev). A replay whose `events` hold an `end` reached its exit and has none, as has one of
/// fewer than two frames. Frames past the route's last input line are left out: there the replay
/// only idles.
pub fn progress(events: &[Event], facts: &Facts) -> Vec<Stall> {
    let n = facts.track.len().min(facts.cells.len());
    if events.iter().any(|e| e.kind == "end") || n < 2 {
        return Vec::new();
    }
    let last = (facts.inputs_end as usize).min(n - 1);
    let mut seen = std::collections::HashSet::new();
    let mut last_new = 0;
    let mut best: Option<(i32, usize)> = None;
    for (k, &c) in facts.cells[..=last].iter().enumerate() {
        if seen.insert(c) {
            last_new = k;
        }
        // Before the coins are met the exit is locked, so nearness to it means nothing.
        if facts.exit_open_at.is_some_and(|m| k as u32 >= m) {
            if let Some(dist) = facts.exits.iter().map(|&(x, y)| (c.0 - x).abs().max((c.1 - y).abs())).min() {
                if best.is_none_or(|(b, _)| dist < b) {
                    best = Some((dist, k));
                }
            }
        }
    }
    let stall = |k: usize, text: String, fact: String| {
        let (x, y) = facts.track[k];
        Stall { event: Event { k: k as u32, x, y, kind: "stall", text }, fact }
    };
    let mut out = vec![stall(
        last_new,
        format!("last new ground: no cell after this one is new (inputs end {} frames later)", last - last_new),
        format!("last new ground at frame {last_new}; the route's inputs end at frame {last}"),
    )];
    if let Some((dist, k)) = best {
        out.push(stall(
            k,
            format!("closest to the open exit: {dist} cells; never closer after this"),
            format!("closest to the open exit {dist} cells, at frame {k}"),
        ));
    }
    out
}

/// What [`verdict`] needs of a replay, read off it by the game.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome<'a> {
    /// Whether a route was played at all (a level drawn with none is not).
    pub has_route: bool,
    /// Whether the replay reached the level's end.
    pub ended: bool,
    /// Whether the exit was open (every coin it needs met) by the end.
    pub exit_open: bool,
    /// Coins the exit still needed on the last frame played.
    pub coins_left_at_end: u32,
    pub lives_lost: usize,
    pub hearts_lost: usize,
    /// The route's `# timing:` (`original` when it has none): only an `app`-timed route replays
    /// from the app's start without drifting.
    pub timing: &'a str,
}

/// The planner's bench result for the route's level, when the route is the planner's.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bench {
    pub completed: bool,
    /// Why the plan failed, when it did.
    pub reason: String,
    /// Planning time, seconds.
    pub wall_s: f64,
    pub route_frames: Option<u32>,
    /// The committed reference route's length.
    pub ref_frames: Option<u32>,
}

/// The verdict for a replay (whose [`progress`] facts, if any, the caller adds to `info`).
pub fn verdict(o: &Outcome, bench: Option<&Bench>) -> Verdict {
    let mut faults = Vec::new();
    let mut info = Vec::new();
    let mut status = Status::Clean;
    if let Some(b) = bench.filter(|b| !b.completed) {
        status = Status::Failed;
        faults.push(format!("planner failed: {}", b.reason));
    } else if !o.has_route {
        status = Status::Failed;
        faults.push("no route".into());
    } else if !o.ended {
        status = Status::Failed;
        faults.push("the replay never reaches the exit".into());
    }
    if o.has_route && !o.ended && !o.exit_open {
        faults.push(format!("exit still locked at the end: {} coins short", o.coins_left_at_end));
    }
    let (lives, hearts) = (o.lives_lost, o.hearts_lost);
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
    if o.timing != "app" {
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

    fn ended() -> Outcome<'static> {
        Outcome {
            has_route: true,
            ended: true,
            exit_open: true,
            coins_left_at_end: 0,
            lives_lost: 0,
            hearts_lost: 0,
            timing: "app",
        }
    }

    fn bench(completed: bool, wall_s: f64, route: Option<u32>, refr: Option<u32>) -> Bench {
        Bench {
            completed,
            reason: if completed { String::new() } else { "over-budget".into() },
            wall_s,
            route_frames: route,
            ref_frames: refr,
        }
    }

    #[test]
    fn a_clean_route_has_no_faults() {
        let v = verdict(&ended(), Some(&bench(true, 300.0, Some(110), Some(100))));
        assert_eq!(v.status, Status::Clean);
        assert!(v.faults.is_empty());
        assert_eq!(v.info, vec!["planned in 5.0 min", "110 frames vs reference 100 (+10%)"]);
    }

    #[test]
    fn hearts_length_and_slow_plans_are_faults() {
        let o = Outcome { hearts_lost: 2, ..ended() };
        let v = verdict(&o, Some(&bench(true, 15.0 * 60.0, Some(126), Some(100))));
        assert_eq!(v.status, Status::Faults);
        assert_eq!(v.faults, vec!["lost 2 hearts", "slow plan: 15.0 of 30 min", "+26% longer than the reference route"]);
        // At exactly +25% and just under half the budget, nothing.
        let v = verdict(&ended(), Some(&bench(true, 899.0, Some(125), Some(100))));
        assert_eq!(v.status, Status::Clean);
    }

    #[test]
    fn a_failed_plan_no_route_or_no_exit_is_failed() {
        let none = Outcome { has_route: false, ended: false, ..ended() };
        let v = verdict(&none, Some(&bench(false, 1800.0, None, Some(100))));
        assert_eq!((v.status, v.faults.clone()), (Status::Failed, vec!["planner failed: over-budget".to_string()]));
        assert_eq!(verdict(&none, None).faults, vec!["no route"]);
        let stuck = Outcome { ended: false, exit_open: false, coins_left_at_end: 3, ..ended() };
        let v = verdict(&stuck, None);
        assert_eq!(v.status, Status::Failed);
        assert_eq!(v.faults, vec!["the replay never reaches the exit", "exit still locked at the end: 3 coins short"]);
        assert_eq!(verdict(&Outcome { timing: "original", ..ended() }, None).status, Status::Faults);
    }

    #[test]
    fn progress_marks_last_new_ground_and_nearest_open_exit() {
        // Cells 0..=10 along row 5, then back to 9, 8, 8; the exit is at (20, 5), open from frame 0.
        let mut cells: Vec<(i32, i32)> = (0..=10).map(|c| (c, 5)).collect();
        cells.extend([(9, 5), (8, 5), (8, 5)]);
        let facts = Facts {
            track: cells.iter().map(|&(c, r)| (c * 10, r * 10)).collect(),
            inputs_end: cells.len() as u32 - 1,
            cells,
            exits: vec![(20, 5)],
            exit_open_at: Some(0),
        };
        let s = progress(&[], &facts);
        assert_eq!(
            s.iter().map(|s| s.fact.as_str()).collect::<Vec<_>>(),
            ["last new ground at frame 10; the route's inputs end at frame 13", "closest to the open exit 10 cells, at frame 10"]
        );
        assert_eq!(s[0].event, Event { k: 10, x: 100, y: 50, kind: "stall", text: "last new ground: no cell after this one is new (inputs end 3 frames later)".into() });
        // A locked exit gives no nearness; an ended replay has no stalls at all.
        assert_eq!(progress(&[], &Facts { exit_open_at: None, ..facts.clone() }).len(), 1);
        let end = Event { k: 13, x: 0, y: 0, kind: "end", text: String::new() };
        assert!(progress(&[end], &facts).is_empty());
    }
}
