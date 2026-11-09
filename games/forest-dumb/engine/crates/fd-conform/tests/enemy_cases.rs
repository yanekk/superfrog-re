//! Creatures in play against the original (plans/forest-dumb-level-1 T46, DESIGN §2.1, §2.2, §4.3):
//! T24 ported their movement, sort and stomp; these cases run fd-core's `Game` from the level sync
//! point with nothing frozen and compare all five creature slots and flags (`creature_active+$N`,
//! `creature_slots+$8N`, `+$8N+4` in `LAYOUT`) on every pass, with the character, camera, contact
//! and status. Needs the disks and the Kickstart: absent, it fails naming where it looked.
//!
//! Each case stops before the first pass where an edge redraw the next pass cut short starts a
//! creature a pass later in the original than in the engine (measured: the interrupted PC in
//! `h0_3ac2`); T75 models that race. The runner hands the engine each measured edge cut here too
//! (`fd_conform::records_edge_cuts`, T74; with creatures live since T78), so `enemy-route` compares
//! `edge_latch_right`.

use std::path::Path;
use std::sync::Arc;
use std::thread;

use amiga_chips::Picture;
use amiga_oracle::FrameInput;
use fd_conform::{Case, Engine, Goal, Options, Outcome, Verdict, run_with};
use fd_core::objects::creature::{STOMPED, WALK5_LEFT};
use fd_core::program::Program;
use fd_core::{Event, Game, GameData, LAYOUT, Mem, PORTED, Start, Transplant, Var};

/// The planner's route (kinds 0, 3, 5, stomps, lives lost to contact), the route to the exit's first
/// steps (kinds 0, 1, 2, 5, the picture compared) and kind 3 hopping left.
const CASES: [&str; 3] = ["enemy-route", "enemy-contact", "enemy-hop"];

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

/// T24's routines no enemy case reaches, each with why and the task whose cases do.
const NOT_REACHED: [(&str, &str); 1] = [
    // Kind 2 stepping right: both of its turns on these routes come after the race (T75).
    ("h0_3222", "T77"),
];

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
        assert!(!case.freeze, "{name}: nothing frozen");
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
        // Only the latch T75 hands over is set aside, and it hid something.
        for (e, n) in &v.excluded {
            assert_eq!(e.task, "T75", "{name}: {}", e.what);
            assert!(*n > 0, "{name}: {} hid nothing", e.what);
        }
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
    assert_eq!(lives_lost, 2);
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
