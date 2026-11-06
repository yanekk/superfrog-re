//! The collision model (`fd_core::render::contact`, plans/forest-dumb-level-1 T70) against the
//! original's CLXDAT, as `char_hit_check` copies it to `clxdat_copy` every pass: over fd-plan's route
//! across W1-1 (`contact-route`, frozen as `freeze.rs` says), and its parts on the engine alone.
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
