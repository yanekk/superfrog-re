//! The collision model (`fd_core::render::contact`, plans/forest-dumb-level-1 T70) against the
//! original's CLXDAT, as `char_hit_check` copies it to `clxdat_copy` every pass: over fd-plan's route
//! across W1-1 (`contact-route`, nothing frozen), and its parts on the engine alone.
//!
//! Measured (T70): a pass's read sees the frame the display showed since the last pass, the one
//! `Game::picture` draws after it; with that, every pass of `contact-route`, `death-pit` and
//! `death-spikes` agrees. The water (T72): `death-water` draws it on both sides (T47 removed the
//! `h0_6882` freeze, and with it the run that left it undrawn). Needs the disks and Kickstart, as
//! every case does.

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
    lives: Vec<u16>,
}

fn copies() -> Copies {
    Copies {
        game: game(),
        seen: Vec::new(),
        lives: Vec::new(),
    }
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
        let s = self.game.state();
        self.seen.push(s.clxdat_copy);
        self.lives.push(s.lives);
    }
    fn picture(&self) -> Option<Picture> {
        Some(self.game.picture())
    }
    fn events(&mut self) -> Vec<Event> {
        self.game.take_events()
    }
    fn foreground_cut(&mut self, at: Option<fd_core::level::EdgesCut>) {
        self.game.cut_foreground(at);
    }
    // `death-water` takes items since T79, so its status bar is redrawn and a pass cuts it (T71).
    fn cut_redraw(&self, cut: u8) -> Option<Picture> {
        Engine::cut_redraw(&self.game, cut)
    }
    // And a long pass cuts a creature behind the beam (T80): unforwarded, `death-water`'s picture
    // differed at pass 1004 (T83).
    fn creatures_uncut(&self, rows: &[u16]) -> Option<Picture> {
        self.game.creatures_uncut(rows)
    }
}

/// Every pass of the route to the exit's cells: CLXDAT and what contact counts, equal; and the
/// comparison is not of a constant: contact bits on some passes, several values in all.
#[test]
fn the_collision_model_matches_the_originals_clxdat_over_the_route() {
    let case = Case::load(Path::new("cases/contact-route.toml")).unwrap_or_else(|e| panic!("{e}"));
    let mut e = copies();
    let v = run_with(&case, &mut e, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    println!("{v}");
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    // Since T47 the route drops into the pool at W1-1:195..197,94 and loses a life there (its goal);
    // the original's longer passes leave 62 pass ends unseen.
    assert!(v.goal_met.is_some() && v.passes > 1230, "{v}");
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

/// The pool at `W1-1:148,66` (`death-water`), the water drawn on both sides. Measured (T72) with
/// the water undrawn as well, which the `h0_6882` freeze allowed until T47: the pool's tiles carry
/// colour 29 themselves, and the water's frames change CLXDAT on a few passes (docs/10). Here every
/// pass's CLXDAT agrees with the original's through the route there, creatures live (T79), the drop
/// and the two lives it costs.
#[test]
fn the_waters_frames_change_contact_in_the_pool_as_the_originals() {
    let case = Case::load(Path::new("cases/death-water.toml")).unwrap_or_else(|e| panic!("{e}"));
    let mut wet = copies();
    let v = run_with(&case, &mut wet, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    println!("{v}");
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert!(case.vars.iter().any(|n| n == "clxdat_copy"));
    let touch = |c: u16| c & CONTACT != 0;
    // Creatures are brushed on the way (first at pass 137, no life lost, measured 2026-11-10); in
    // the pool contact comes and goes with the water's frames.
    let lost: Vec<usize> = (1..wet.lives.len())
        .filter(|&k| wet.lives[k] < wet.lives[k - 1])
        .collect();
    assert_eq!(lost.len(), 2, "{lost:?}");
    assert!(lost[0] > 2180, "{lost:?}: a life lost before the pool");
    let pool = &wet.seen[lost[0]..];
    assert!(pool.iter().any(|&c| touch(c)) && pool.iter().any(|&c| !touch(c)));
    assert_eq!(wet.lives.last(), Some(&0x30));
}
