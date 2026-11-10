//! Creatures in play against the original (plans/forest-dumb-level-1 T46, DESIGN §2.1, §2.2, §4.3):
//! T24 ported their movement, sort and stomp; these cases run fd-core's `Game` from the level sync
//! point with nothing frozen and compare all five creature slots and flags (`creature_active+$N`,
//! `creature_slots+$8N`, `+$8N+4` in `LAYOUT`) on every pass, with the character, camera, contact
//! and status. Needs the disks and the Kickstart: absent, it fails naming where it looked.
//!
//! Each case runs its source script's whole length. A pass made longer by drawn creatures can cut
//! an edge redraw before its scan reaches a creature cell (measured: the interrupted PC in
//! `h0_3ac2`); the original then starts that creature after the pass. The runner hands the engine
//! each measured cut (`fd_conform::edges_cut`, T74; with creatures live since T78) and the engine
//! starts the scan's later creatures after that pass too (T75, `Edges::deferred`), so every case
//! compares the picture's latches and the slots through those passes.
//!
//! T77: `enemy-kind4` meets kind 4, the one creature kind T46's routes never brought into view, and
//! two tests read the engine's slots for its walk and turn and for a kind 2 turning right.

use std::path::Path;
use std::sync::Arc;
use std::thread;

use amiga_chips::Picture;
use amiga_oracle::FrameInput;
use fd_conform::{Case, Engine, Goal, Options, Outcome, Verdict, run_with};
use fd_core::level::EdgesCut;
use fd_core::level::redraw::{Deferred, OBJECT_MAP};
use fd_core::objects::creature::{LEFT, STOMPED, WALK4_RIGHT, WALK5_LEFT};
use fd_core::program::Program;
use fd_core::{Event, Game, GameData, LAYOUT, Mem, PORTED, Start, Transplant, Var};

/// The planner's route (kinds 0, 3, 5, stomps, lives lost to contact), the route to the exit's cells
/// (kinds 0, 1, 2, 5, the picture compared), T43's run right and back (kind 3 hopping left) and the
/// bottom corridor's kind 4 (T77).
const CASES: [&str; 4] = ["enemy-route", "enemy-contact", "enemy-hop", "enemy-kind4"];

/// The routines T24 registered (`fd-core/src/ported.rs`, its section), the stomp's caller, and the
/// write-back T22 ported that waited for creatures (T45's `NOT_REACHED`).
const T24: [&str; 14] = [
    "h0_2fae",
    "h0_2fea",
    "h0_320a",
    "h0_3222",
    "h0_32f8",
    "h0_3322",
    "h0_3530",
    "h0_3566",
    "h0_35a0",
    "h0_35dc",
    "h0_3698",
    "h0_3dd8",
    "char_hit_check",
    "h0_2e0e",
];

/// T24's routines no enemy case reaches, each with why and the task whose cases do. None since the
/// cases run their whole scripts (T75): kind 2 steps right (`h0_3222`) past the first race.
const NOT_REACHED: [(&str, &str); 0] = [];

fn load(name: &str) -> Case {
    Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap_or_else(|e| panic!("{e}"))
}

fn data() -> Arc<GameData> {
    fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"))
}

/// The creature slots' fields `LAYOUT` holds.
fn creature_vars() -> Vec<&'static Var> {
    LAYOUT
        .iter()
        .filter(|v| v.name.starts_with("creature_"))
        .collect()
}

fn hits(verdicts: &[(Case, Verdict)], name: &str) -> u32 {
    verdicts
        .iter()
        .flat_map(|(_, v)| &v.coverage.hits)
        .filter(|(n, _)| *n == name)
        .map(|(_, h)| h)
        .sum()
}

#[test]
fn creatures_match_the_original_on_every_pass() {
    let runs: Vec<_> = CASES
        .iter()
        .map(|n| {
            let case = load(n);
            thread::spawn(move || {
                let v = fd_conform::run(&case).unwrap_or_else(|e| panic!("{}: {e}", case.name));
                (case, v)
            })
        })
        .collect();
    let verdicts: Vec<(Case, Verdict)> = runs.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(creature_vars().len(), 15);
    for (case, v) in &verdicts {
        let name = &case.name;
        assert_eq!(v.outcome, Outcome::Same, "{name}: {v}");
        assert!(
            matches!(case.goal, Some(Goal::All { .. })),
            "{name}: a goal"
        );
        assert!(v.goal_met.is_some(), "{name}: {v}");
        assert!(v.events.is_empty(), "{name}: {v}");
        assert!(v.passes + v.unseen + 5 >= case.frames, "{name}: {v}");
        // Every slot and flag, the contact and the lives compared on every pass.
        for var in creature_vars().iter().map(|v| v.name).chain([
            "hits_taken",
            "clxdat_copy",
            "lives",
            "score",
            "char_x",
        ]) {
            assert!(case.vars.iter().any(|v| v == var), "{name}: {var}");
        }
        // Nothing is set aside, and the runner handed the engine cuts (T74, T75).
        assert!(v.excluded.is_empty(), "{name}: {v}");
        assert!(v.edge_cuts > 0, "{name}: {v}");
    }
    assert!(
        verdicts.iter().any(|(c, _)| c.compare.picture),
        "creatures drawn"
    );

    // Every T24 routine ran on the original, but the ones named with the task that reaches them.
    let mut none = Vec::new();
    for name in T24 {
        assert!(
            PORTED.iter().any(|p| p.name == name),
            "{name} not in PORTED"
        );
        let h = hits(&verdicts, name);
        let named = NOT_REACHED.iter().any(|(n, _)| *n == name);
        if h == 0 && !named {
            none.push(name);
        }
        assert!(
            !(named && h > 0),
            "{name} ran {h} times: no longer an exclusion"
        );
    }
    assert!(none.is_empty(), "never reached: {none:?}");
    // Contact cost lives, and the creatures were cut onto the screen and restored.
    for name in [
        "lives_minus_one",
        "h0_2d4c",
        "h0_2c7e",
        "h0_5820",
        "h0_5e44",
    ] {
        assert!(hits(&verdicts, name) > 0, "{name}");
    }
}

/// fd-core's game, keeping after every pass the score, the lives, the hits and each slot's kind.
struct Watched {
    game: Game,
    seen: Vec<(u32, u16, u16, [u16; 5], [u8; 5])>,
}

impl Transplant for Watched {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.game.transplant(mem);
    }
    fn probe(&self, v: &Var) -> u32 {
        self.game.probe(v)
    }
}

impl Engine for Watched {
    fn pass(&mut self, input: FrameInput) {
        self.game.step(input);
        let s = self.game.state();
        let c = &s.edges.creatures;
        self.seen.push((
            s.score,
            s.lives,
            s.hits_taken,
            c.slots.map(|s| s.kind),
            c.active,
        ));
    }
    fn picture(&self) -> Option<Picture> {
        Some(self.game.picture())
    }
    fn events(&mut self) -> Vec<Event> {
        self.game.take_events()
    }
    fn foreground_cut(&mut self, at: Option<EdgesCut>) {
        Engine::foreground_cut(&mut self.game, at);
    }
}

/// A BCD score as a number.
fn bcd(v: u32) -> u32 {
    (0..8).rev().fold(0, |n, d| n * 10 + (v >> (4 * d) & 0xf))
}

/// On `enemy-contact`, as the original (every pass compared): a falling character stomps a creature,
/// which turns kind 7 with 250 added in the same pass; and contact passes count 1, 2, 3, the third
/// costing a life.
#[test]
fn a_stomp_scores_and_three_contacts_cost_a_life() {
    let case = load("enemy-contact");
    let mut w = Watched {
        game: Game::new(data(), Start::Level1),
        seen: Vec::new(),
    };
    let v = run_with(&case, &mut w, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    let stomped =
        |p: &(u32, u16, u16, [u16; 5], [u8; 5]), i: usize| p.3[i] == STOMPED && p.4[i] != 0;
    let mut stomps = 0;
    let mut lives_lost = 0;
    for k in 1..w.seen.len() {
        let (before, now) = (&w.seen[k - 1], &w.seen[k]);
        for i in 0..5 {
            if stomped(now, i) && !stomped(before, i) {
                stomps += 1;
                assert_eq!(
                    bcd(now.0),
                    bcd(before.0) + 250,
                    "pass {k}: +250 with the stomp"
                );
            }
        }
        if now.1 < before.1 {
            lives_lost += 1;
            let hits: Vec<u16> = w.seen[k - 2..=k].iter().map(|p| p.2).collect();
            assert_eq!(hits, [1, 2, 3], "pass {k}: the third contact pass");
        }
    }
    assert!(stomps >= 1, "no stomp");
    assert_eq!(lives_lost, 6);
}

/// Kind 5 walking left a pixel further each step in the program the engine reads: `enemy-contact`
/// fails on a creature's place, so the slots are compared, not only what the character feels.
#[test]
fn a_mutated_walk_table_fails_on_a_creature() {
    let data = data();
    let mut bytes = data.program.bytes().to_vec();
    for b in &mut bytes[WALK5_LEFT as usize..WALK5_LEFT as usize + 32] {
        *b = b.wrapping_add(1);
    }
    let mutated = Arc::new(GameData {
        program: Program::hunk0(bytes),
        ..(*data).clone()
    });
    let mut game = Game::new(mutated, Start::Level1);
    let v = run_with(&load("enemy-contact"), &mut game, &Options::default())
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        matches!(&v.outcome, Outcome::Differs { var, .. } if var.starts_with("creature_slots")),
        "{v}"
    );
}

/// An active slot after a pass: its kind, x, y and facing.
type Slot = (u16, u16, u16, u8);

/// fd-core's game, keeping after every pass each active slot and how many cells of the object map
/// hold kind 4's value.
struct Slots {
    game: Game,
    seen: Vec<[Option<Slot>; 5]>,
    fives: Vec<usize>,
}

impl Transplant for Slots {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.game.transplant(mem);
    }
    fn probe(&self, v: &Var) -> u32 {
        self.game.probe(v)
    }
}

impl Engine for Slots {
    fn pass(&mut self, input: FrameInput) {
        self.game.step(input);
        let s = self.game.state();
        let c = &s.edges.creatures;
        self.seen.push(std::array::from_fn(|i| {
            let slot = &c.slots[i];
            (c.active[i] != 0).then_some((slot.kind, slot.x, slot.y, slot.facing))
        }));
        self.fives
            .push(s.object_map.iter().filter(|&&v| v == 5).count());
    }
    fn picture(&self) -> Option<Picture> {
        Some(self.game.picture())
    }
    fn events(&mut self) -> Vec<Event> {
        self.game.take_events()
    }
    fn foreground_cut(&mut self, at: Option<EdgesCut>) {
        Engine::foreground_cut(&mut self.game, at);
    }
}

/// The passes after which a creature of `kind` faces the other way than the pass before, with the
/// x it turned at and whether it now faces left. The sort moves creatures between slots, so one is
/// followed by its kind, its y and an x within a step's reach.
fn turns(seen: &[[Option<Slot>; 5]], kind: u16) -> Vec<(usize, u16, bool)> {
    let of_kind = |p: &[Option<Slot>; 5]| -> Vec<Slot> {
        p.iter()
            .flatten()
            .filter(|s| s.0 == kind)
            .copied()
            .collect()
    };
    let mut out = Vec::new();
    for k in 1..seen.len() {
        for now in of_kind(&seen[k]) {
            let before = of_kind(&seen[k - 1])
                .into_iter()
                .find(|b| b.2 == now.2 && b.1.abs_diff(now.1) <= 4);
            if let Some(b) = before {
                if (b.3 == LEFT) != (now.3 == LEFT) {
                    out.push((k, now.1, now.3 == LEFT));
                }
            }
        }
    }
    out
}

/// `enemy-kind4`, as the original (every slot compared on every pass): W1-1:45,92's kind 4 starts
/// from the left column's redraw facing right, walks right to the mound at column 52, turns there
/// once and walks left past where it started until it leaves the view and is written back into
/// the object map. Kind 4 has no routine of its own (its walk is inline in `h0_2fea`), so no
/// coverage count shows it: the slots do.
#[test]
fn kind_4_walks_right_turns_at_a_wall_and_is_written_back() {
    let case = load("enemy-kind4");
    let mut w = Slots {
        game: Game::new(data(), Start::Level1),
        seen: Vec::new(),
        fives: Vec::new(),
    };
    let v = run_with(&case, &mut w, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert!(v.goal_met.is_some(), "{v}");

    // Its (pass, x, facing) while it is in a slot; W1-1 never shows two at once here.
    let walk: Vec<(usize, u16, u8)> = w
        .seen
        .iter()
        .enumerate()
        .filter_map(|(k, p)| {
            let mut four = p.iter().flatten().filter(|s| s.0 == 4);
            let s = four.next()?;
            assert!(four.next().is_none(), "pass {k}: two of kind 4");
            Some((k, s.1, s.3))
        })
        .collect();
    assert!(walk.len() > 400, "kind 4 in view for {} passes", walk.len());
    let (first, last) = (walk[0], walk[walk.len() - 1]);
    assert_eq!(last.0 - first.0 + 1, walk.len(), "one stay in a slot");
    // Column 45 is x 720; the mound's wall stops it at 784.
    assert!(
        (720..=726).contains(&first.1) && first.2 != LEFT,
        "{first:?}"
    );
    let turned = turns(&w.seen, 4);
    assert_eq!(turned.len(), 1, "{turned:?}");
    let (at, x, left) = turned[0];
    assert_eq!((x, left), (784, true));
    for pair in walk.windows(2) {
        let (k, before, now) = (pair[1].0, pair[0].1, pair[1].1);
        // It steps, then the probe turns it: the turning pass still steps right.
        if k <= at {
            assert!((before..=before + 1).contains(&now), "pass {k}: right");
        } else {
            assert!((now..=now + 1).contains(&before), "pass {k}: left");
        }
    }
    assert!(last.1 < first.1 - 100 && last.2 == LEFT, "{last:?}");
    // The map holds four `$05` cells: three while this one is in its slot, four again after.
    assert_eq!(
        (
            w.fives[first.0 - 1],
            w.fives[first.0],
            w.fives[last.0],
            w.fives[last.0 + 1]
        ),
        (4, 3, 3, 4)
    );
}

/// Kind 4 walking right a pixel further each step in the program the engine reads: `enemy-kind4`
/// fails on a creature slot once W1-1:45,92's creature starts (pass 854 on), so the case holds
/// kind 4's own table and not only the kinds the other cases meet.
#[test]
fn a_mutated_kind_4_table_fails_once_kind_4_walks() {
    let data = data();
    let mut bytes = data.program.bytes().to_vec();
    for b in &mut bytes[WALK4_RIGHT as usize..WALK4_RIGHT as usize + 32] {
        *b = b.wrapping_add(1);
    }
    let mutated = Arc::new(GameData {
        program: Program::hunk0(bytes),
        ..(*data).clone()
    });
    let mut game = Game::new(mutated, Start::Level1);
    let v = run_with(&load("enemy-kind4"), &mut game, &Options::default())
        .unwrap_or_else(|e| panic!("{e}"));
    assert!(
        matches!(&v.outcome, Outcome::Differs { pass, var, .. } if var.starts_with("creature_slots") && (854..870).contains(pass)),
        "{v}"
    );
}

/// `enemy-contact`'s first 800 steps, state only, as the original: a kind 2 walking left turns
/// right at x 1720 (pass 665) and steps on rightwards (`h0_3222`), the half T46's cases missed
/// while they stopped at the first race.
#[test]
fn kind_2_turns_right_and_steps_on() {
    let mut case = load("enemy-contact");
    case.frames = 800;
    case.compare.picture = false;
    let mut w = Slots {
        game: Game::new(data(), Start::Level1),
        seen: Vec::new(),
        fives: Vec::new(),
    };
    let v = run_with(&case, &mut w, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    let turned = turns(&w.seen, 2);
    let &(at, x, left) = turned.first().expect("a turn");
    assert_eq!((x, left), (1720, false), "pass {at}");
    // It is still there 50 passes on, further right.
    let later = w.seen[at + 50]
        .iter()
        .flatten()
        .find(|s| s.0 == 2)
        .expect("kind 2 still in a slot");
    assert!(later.1 > x && later.3 != LEFT, "{later:?}");
    assert!(hits(&[(case, v)], "h0_3222") > 0);
}

/// What a test does to the measured cuts on their way to the engine.
#[derive(Clone, Copy, PartialEq)]
enum Change {
    /// Handed as measured.
    Measured,
    /// The scan's place left out of each cut: the latches' half kept (T74), no creature waits.
    NoScan,
    /// A pass whose foreground starts a creature, measured as not cut, said to have cut the scan.
    LateUnmeasured,
}

/// fd-core's game with the measured cuts changed on their way to it.
struct Cuts {
    game: Game,
    change: Change,
    /// The creatures waiting after each pass, counted.
    waited: u32,
    /// The waiting creatures a slot took after their pass, and those of them whose object map
    /// cell was then cleared.
    taken: u32,
    cleared: u32,
}

impl Transplant for Cuts {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.game.transplant(mem);
    }
    fn probe(&self, v: &Var) -> u32 {
        self.game.probe(v)
    }
}

impl Engine for Cuts {
    fn pass(&mut self, input: FrameInput) {
        // The step's foreground starts the creatures the last pass left waiting, before anything
        // else changes the slots: one finding a slot free has its cell cleared (the scan's
        // `clr.b (a2)`, `h0+$b1a`), or the next redraw over the cell starts it again.
        let s = self.game.state();
        let free = s.edges.creatures.active.iter().filter(|&&f| f == 0).count();
        let cells: Vec<usize> = s
            .edges
            .deferred
            .iter()
            .filter_map(|d| match d {
                Deferred::Creature { cell, .. } => Some((cell - OBJECT_MAP) as usize),
                Deferred::Cell(_) => None,
            })
            .take(free)
            .collect();
        self.game.step(input);
        self.taken += cells.len() as u32;
        let map = &self.game.state().object_map;
        self.cleared += cells.iter().filter(|&&c| map[c] == 0).count() as u32;
        let waiting = &self.game.state().edges.deferred;
        self.waited += waiting
            .iter()
            .filter(|d| matches!(d, Deferred::Creature { .. }))
            .count() as u32;
    }
    fn picture(&self) -> Option<Picture> {
        None
    }
    fn events(&mut self) -> Vec<Event> {
        self.game.take_events()
    }
    fn foreground_cut(&mut self, at: Option<EdgesCut>) {
        let at = match self.change {
            Change::Measured => at,
            Change::NoScan => at.map(|c| EdgesCut {
                defer_from: None,
                ..c
            }),
            Change::LateUnmeasured => {
                // Whether the foreground before this pass starts a creature, on a copy.
                let mut g = self.game.clone();
                let before = g.state().edges.creatures.active;
                g.foreground();
                if at.is_none() && g.state().edges.creatures.active != before {
                    Some(EdgesCut {
                        next: 4,
                        defer_from: Some(0),
                    })
                } else {
                    at
                }
            }
        };
        Engine::foreground_cut(&mut self.game, at);
    }
}

/// `enemy-hop`'s first 200 steps, state only: W1-1:27,2's hopper starts at pass 84 from a redraw no
/// pass cut, and pass 93 cuts the redraw that starts the next creature.
fn with_cuts(change: Change) -> (Verdict, Cuts) {
    let mut case = load("enemy-hop");
    case.frames = 200;
    case.compare.picture = false;
    let mut c = Cuts {
        game: Game::new(data(), Start::Level1),
        change,
        waited: 0,
        taken: 0,
        cleared: 0,
    };
    let v = run_with(&case, &mut c, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    (v, c)
}

/// A creature start waits for the next pass exactly where the original's pass was measured to cut
/// the scan before it (T75). Handed the cuts as measured, a creature waits and the engine agrees
/// through the race, each waiting creature's cell cleared once a slot takes it. Without the scan's
/// place none waits, and the engine has the creature a pass
/// before the original (pass 93). Told of a cut scan where none was measured, the engine starts
/// the hopper a pass after the original (pass 84).
#[test]
fn a_creature_start_waits_only_where_the_measured_cut_says() {
    let (v, c) = with_cuts(Change::Measured);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert!(c.waited > 0);
    assert!(c.taken > 0);
    assert_eq!(c.cleared, c.taken);

    let (v, c) = with_cuts(Change::NoScan);
    assert_eq!(c.waited, 0);
    assert!(
        matches!(&v.outcome, Outcome::Differs { pass: 93, var, .. } if var.starts_with("creature_")),
        "{v}"
    );

    let (v, c) = with_cuts(Change::LateUnmeasured);
    assert!(c.waited > 0);
    assert!(
        matches!(
            &v.outcome,
            Outcome::Differs { pass: 84, var, engine: 0, oracle: 0xff, .. } if var.starts_with("creature_active")
        ),
        "{v}"
    );
}
