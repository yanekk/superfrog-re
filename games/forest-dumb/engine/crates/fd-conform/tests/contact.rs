//! The collision model (`fd_core::render::contact`, plans/forest-dumb-level-1 T70) against the
//! original's CLXDAT, as `char_hit_check` copies it to `clxdat_copy` every pass: over fd-plan's route
//! across W1-1 (`contact-route`, frozen as `freeze.rs` says), and its parts on the engine alone.
//!
//! Measured (T70): a pass's read sees the frame the display showed since the last pass, the one
//! `Game::picture` draws after it; with that, every pass of `contact-route`, `death-pit` and
//! `death-spikes` agrees. The water (T72): `death-water` thaws `h0_6882`, so both sides draw it, and
//! the same run with it frozen shows that its pixels alone give the contact. Needs the disks and
//! Kickstart, as every case does.

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
    fn freeze(&mut self) {
        Engine::freeze(&mut self.game);
    }
    fn thaw(&mut self, offset: u32) {
        Engine::thaw(&mut self.game, offset);
    }
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

/// The pool at `W1-1:148,66` (`death-water`), drawn and not: the same run with `h0_6882` frozen
/// draws no water on either side. Measured (T72): the pool's tiles under the water carry colour 29
/// themselves, so both runs lose two lives there; the water's frames change CLXDAT on
/// a few passes, setting a contact bit the tiles alone do not (pass 773) and covering the tiles'
/// pixels (passes 786 to 793). Both runs agree with the original on every compared pass and frame,
/// so the engine draws the water's pixels where the original does and reads contact from them.
#[test]
fn the_waters_frames_change_contact_in_the_pool_as_the_originals() {
    let case = Case::load(Path::new("cases/death-water.toml")).unwrap_or_else(|e| panic!("{e}"));
    let mut dry_case = case.clone();
    dry_case.thaw.clear();
    // Frozen, no animated cell's blit runs a pass late: the edge latches are compared, not excluded.
    dry_case.exclude.clear();
    dry_case
        .vars
        .extend(["edge_latch_down", "edge_latch_up"].map(String::from));
    let run = |case: &Case| {
        let mut e = copies();
        let v = run_with(case, &mut e, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
        println!("{v}");
        (e, v)
    };
    let ((wet, v), (dry, d)) = std::thread::scope(|s| {
        let wet = s.spawn(|| run(&case));
        let dry = s.spawn(|| run(&dry_case));
        (wet.join().unwrap(), dry.join().unwrap())
    });
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert_eq!(d.outcome, Outcome::Same, "{d}");
    assert_eq!(dry.seen.len(), wet.seen.len());
    let touch = |c: u16| c & CONTACT != 0;
    let first = wet.seen.iter().position(|&c| touch(c)).unwrap();
    // Before the drop neither side touches anything, and the tiles alone cost two lives as well
    // (the first a pass earlier: the drawn water covers the tiles' colour 29 on pass 758).
    assert!(!dry.seen[..first - 1].iter().any(|&c| touch(c)));
    let lost = |e: &Copies| {
        (1..e.lives.len())
            .filter(|&k| e.lives[k] < e.lives[k - 1])
            .collect::<Vec<_>>()
    };
    let (wet_lost, dry_lost) = (lost(&wet), lost(&dry));
    assert_eq!(wet_lost.len(), 2, "{wet_lost:?}");
    assert_eq!(dry_lost.len(), 2, "{dry_lost:?}");
    assert_eq!(wet.lives.last(), Some(&0x30));
    let only = |a: &Copies, b: &Copies| {
        (first..a.seen.len())
            .filter(|&k| touch(a.seen[k]) && !touch(b.seen[k]))
            .count()
    };
    let (water, covered) = (only(&wet, &dry), only(&dry, &wet));
    assert!(water >= 1 && covered >= 8, "{water} {covered}");
}
