//! The collision model (`fd_core::render::contact`, plans/forest-dumb-level-1 T70) against the
//! original's CLXDAT, as `char_hit_check` copies it to `clxdat_copy` every pass: over fd-plan's route
//! across W1-1 (`contact-route`, every object frozen), and its parts on the engine alone.
//!
//! Measured (T70): a pass's read sees the frame the display showed since the last pass, the one
//! `Game::picture` draws after it; with that, every pass of `contact-route`, `death-pit` and
//! `death-spikes` agrees. Needs the disks and Kickstart, as every case does.

use std::collections::BTreeSet;
use std::path::Path;

use amiga_chips::Picture;
use amiga_oracle::FrameInput;
use fd_conform::{Case, Engine, Options, Outcome, run_with};
use fd_core::render::contact::{CONTACT, clxdat};
use fd_core::{Event, Game, Mem, Start, Transplant, Var};

fn game() -> Game {
    Game::new(
        fd_conform::game_data().unwrap_or_else(|e| panic!("{e}")),
        Start::Level1,
    )
}

/// fd-core's game, keeping `clxdat_copy` after every pass.
struct Copies {
    game: Game,
    seen: Vec<u16>,
}

impl Transplant for Copies {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.game.transplant(mem)
    }
    fn probe(&self, v: &Var) -> u32 {
        self.game.probe(v)
    }
}

impl Engine for Copies {
    fn pass(&mut self, input: FrameInput) {
        self.game.step(input);
        self.seen.push(self.game.state().clxdat_copy);
    }
    fn picture(&self) -> Option<Picture> {
        Some(self.game.picture())
    }
    fn events(&mut self) -> Vec<Event> {
        self.game.take_events()
    }
    fn freeze(&mut self) {
        Engine::freeze(&mut self.game);
    }
}

/// Every pass of the route to the exit's cells: CLXDAT and what contact counts, equal; and the
/// comparison is not of a constant: contact bits on some passes, several values in all.
#[test]
fn the_collision_model_matches_the_originals_clxdat_over_the_route() {
    let case = Case::load(Path::new("cases/contact-route.toml")).unwrap_or_else(|e| panic!("{e}"));
    let mut e = Copies {
        game: game(),
        seen: Vec::new(),
    };
    let v = run_with(&case, &mut e, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    println!("{v}");
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert!(v.goal_met.is_some() && v.passes > 1270, "{v}");
    assert!(case.vars.iter().any(|n| n == "clxdat_copy"));
    let values: BTreeSet<u16> = e.seen.iter().copied().collect();
    let contact = e.seen.iter().filter(|&&c| c & CONTACT != 0).count();
    assert!(contact > 0 && values.len() > 2, "{values:x?}");
}

/// Playfield against playfield (bit 0) is set by the status bar alone: with the play screen blank
/// and the character's sprites cleared, CLXDAT still reads `$8001`, and without the sprites no
/// sprite bit is set however the screen is drawn.
#[test]
fn the_status_bar_alone_sets_the_playfield_bit() {
    let g = game();
    let (data, program) = (&g.data().display, &g.data().program);
    let c = &g.state().camera;
    let mut d = g.display().clone();
    d.sprite.frame = None;
    assert_eq!(clxdat(data, &d, (c.x, c.y), program) & !0x1, 0x8000);
    d.screen.bytes.fill(0);
    assert_eq!(clxdat(data, &d, (c.x, c.y), program), 0x8001);
}

/// A case that freezes the original leaves out the bonus bobs the frozen edge redraws never blit
/// (`camera-bottom-right` passes several), so contact is judged on the same screen; the game as the
/// app and the planner make it draws them, and touches them: its CLXDAT differs where they lie.
#[test]
fn only_a_frozen_case_leaves_the_bonus_items_out() {
    let case =
        Case::load(Path::new("cases/camera-bottom-right.toml")).unwrap_or_else(|e| panic!("{e}"));
    assert!(case.freeze);
    let inputs = fd_conform::Inputs::load(&case).unwrap_or_else(|e| panic!("{e}"));
    let (mut normal, mut frozen) = (game(), game());
    frozen.freeze_as_a_case();
    let bobs = |g: &Game| {
        g.blits()
            .iter()
            .filter(|b| matches!(b, fd_core::level::redraw::Blit::Bob { .. }))
            .count()
    };
    let (mut drawn, mut left_out, mut touched) = (0, 0, 0);
    for k in 0..case.frames {
        let i = inputs.at(case.sync_frame + k);
        normal.step(i);
        frozen.step(i);
        drawn += bobs(&normal);
        left_out += bobs(&frozen);
        touched += usize::from(normal.state().clxdat_copy != frozen.state().clxdat_copy);
    }
    assert!(drawn > 0, "the normal game draws bonus items");
    assert_eq!(left_out, 0);
    assert!(
        touched > 0,
        "the bonus items' colour 29 under the character"
    );
}

/// Nothing outside the conformance runner sets the engine's freeze: not the app, the planner, the map
/// page or the end-to-end harness.
#[test]
fn the_freeze_is_set_by_the_case_runner_alone() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found = Vec::new();
    for c in ["fd-app", "fd-plan", "fd-routeview", "fd-e2e", "fd-tools"] {
        let mut dirs = vec![crates.join(c)];
        while let Some(d) = dirs.pop() {
            for e in std::fs::read_dir(&d).unwrap_or_else(|e| panic!("{}: {e}", d.display())) {
                let p = e.unwrap().path();
                if p.is_dir() {
                    dirs.push(p);
                } else if p.extension().is_some_and(|x| x == "rs")
                    && std::fs::read_to_string(&p)
                        .unwrap()
                        .contains("freeze_as_a_case")
                {
                    found.push(p);
                }
            }
        }
    }
    assert!(found.is_empty(), "{found:?}");
    let lib = std::fs::read_to_string(crates.join("fd-conform/src/lib.rs")).unwrap();
    assert_eq!(lib.matches("freeze_as_a_case").count(), 1);
}
