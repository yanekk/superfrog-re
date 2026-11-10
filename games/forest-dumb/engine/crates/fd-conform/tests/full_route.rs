//! The whole of W1-1 on the original (plans/forest-dumb-level-1 T48, DESIGN §1, §2.11, §4.1): the
//! route `conform/scripts/route-W1-1.txt` played on the original and on the engine from the level
//! sync point to the level's end, nothing frozen, as `cases/route-W1-1.toml` states it. No life is
//! lost on the way, no object is left unported, and every ported routine runs on it or in the case
//! named here. Needs the disks and the Kickstart: absent, it fails naming where it looked.
//!
//! The route is T27's (`fd-plan/tests/routes/W1-1.txt`) planned again from the original's states
//! (`games/forest-dumb/docs/13-full-route.md`, "On the original"). A change to the engine that
//! moves the character or a creature on it differs here at the pass it does.

use std::collections::BTreeSet;
use std::path::Path;

use amiga_chips::Picture;
use amiga_conform::Mode;
use amiga_oracle::FrameInput;
use fd_conform::{Case, Engine, Goal, Options, Outcome, Verdict, game_data, run, run_with};
use fd_core::level::{AddCut, EdgesCut};
use fd_core::{Event, Game, Mem, PORTED, Start, Transplant, Var};

/// Lives at the level's start (docs/09), BCD as `lives` holds them.
const LIVES: u16 = 0x32;

/// The ported routines the route never runs, each with the case that runs it, or why none can.
const NOT_ON_ROUTE: [(&str, Option<&str>, &str); 9] = [
    (
        "view_tiles",
        None,
        "level_init, before the level sync point every case starts from",
    ),
    ("view_tile", None, "level_init, before the level sync point"),
    (
        "view_shapes",
        None,
        "level_init, before the level sync point",
    ),
    (
        "view_shape",
        None,
        "level_init, before the level sync point",
    ),
    ("h0_9a34", None, "level_init, before the level sync point"),
    ("h0_988a", None, "level_init, before the level sync point"),
    (
        "countdown_plus_100",
        None,
        "W1-1 holds no $4c; proven on the rig (T45)",
    ),
    (
        "h0_6c80",
        Some("death-pit"),
        "the lives digit, redrawn when a life is lost",
    ),
    ("lives_minus_one", Some("death-pit"), "a life lost"),
];

fn case(name: &str) -> Case {
    Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap_or_else(|e| panic!("{e}"))
}

/// What became of the shapes a cut redraw had not copied ([`EdgesCut::shape_words`]).
#[derive(Clone, Copy, PartialEq)]
enum Shapes {
    Measured,
    /// As the engine did before T48: every shape copied before the pass.
    CopiedEarly,
    /// As before T82: an animated cell's add cut after its counts is made whole after the pass
    /// ([`AddCut`]).
    AddedWhole,
    /// As before T82: the status bar a redraw never reached is not handed over
    /// ([`Engine::redraw_skipped`]).
    AlwaysRedrawn,
    /// The bar as it stood before each redraw, on every period, reached or not.
    NeverRedrawn,
}

/// The game, watched: the first pass a life was lost or the character was dying in.
struct Watched {
    game: Game,
    lost: Option<u16>,
    shapes: Shapes,
}

impl Watched {
    fn new(shapes: Shapes) -> Watched {
        let data = game_data().unwrap_or_else(|e| panic!("{e}"));
        Watched {
            game: Game::new(data, Start::Level1),
            lost: None,
            shapes,
        }
    }
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
        Engine::pass(&mut self.game, input);
        let s = self.game.state();
        if self.lost.is_none() && (s.lives != LIVES || s.motion.dying != 0) {
            self.lost = Some(s.irq_count);
        }
    }
    fn picture(&self) -> Option<Picture> {
        let stale = match self.shapes {
            Shapes::NeverRedrawn => self.game.redraw_skipped(None),
            _ => None,
        };
        stale.or_else(|| Engine::picture(&self.game))
    }
    fn events(&mut self) -> Vec<Event> {
        Engine::events(&mut self.game)
    }
    fn off_play(&mut self, input: FrameInput) {
        Engine::off_play(&mut self.game, input)
    }
    fn game_over(&self) -> bool {
        Engine::game_over(&self.game)
    }
    fn level_end(&self) -> bool {
        Engine::level_end(&self.game)
    }
    fn cut_redraw(&self, cut: u8) -> Option<Picture> {
        Engine::cut_redraw(&self.game, cut)
    }
    fn foreground_cut(&mut self, at: Option<EdgesCut>) {
        let at = match self.shapes {
            Shapes::CopiedEarly => at.map(|c| EdgesCut {
                shape_words: None,
                ..c
            }),
            Shapes::AddedWhole => at.map(|c| EdgesCut {
                add: AddCut::Whole,
                ..c
            }),
            _ => at,
        };
        Engine::foreground_cut(&mut self.game, at)
    }
    fn creatures_uncut(&self, rows: &[u16]) -> Option<Picture> {
        Engine::creatures_uncut(&self.game, rows)
    }
    fn redraw_skipped(&self, uncut: Option<&[u16]>) -> Option<Picture> {
        match self.shapes {
            Shapes::AlwaysRedrawn => None,
            _ => Engine::redraw_skipped(&self.game, uncut),
        }
    }
}

/// The first lines of a verdict's report.
fn head(v: &Verdict) -> String {
    format!("{v}")
        .lines()
        .take(8)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The route's opening, in smoke: 600 steps down the first slope and on, creatures live, 10 items
/// taken, variables and the whole picture, nothing excluded.
#[test]
fn the_routes_first_600_steps_agree_with_the_original() {
    let mut c = case("route-W1-1");
    c.frames = 600;
    c.exclude.clear();
    c.goal = Some(Goal::All {
        all: vec![
            ("items_left".into(), OPENING_ITEMS),
            ("lives".into(), LIVES as u32),
        ],
        by_frame: c.sync_frame + c.frames,
    });
    let mut g = Watched::new(Shapes::Measured);
    let v = run_with(&c, &mut g, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        v.outcome,
        Outcome::Same,
        "items_left {:x}\n{}",
        g.game.state().items_left,
        head(&v)
    );
    assert!(
        v.goal_met.is_some() && v.passes > 590 && v.pictures > 590,
        "{}",
        head(&v)
    );
    assert_eq!((g.lost, v.events.len()), (None, 0), "{}", head(&v));
}

/// The route's first `frames` steps, nothing excluded, the goal that no life is lost.
fn opening(frames: u32) -> Case {
    let mut c = case("route-W1-1");
    c.frames = frames;
    c.goal = Some(Goal::All {
        all: vec![("lives".into(), LIVES as u32)],
        by_frame: c.sync_frame + frames,
    });
    c
}

fn run_as(c: &Case, shapes: Shapes, mode: Mode) -> Verdict {
    let opts = Options {
        mode,
        ..Options::default()
    };
    run_with(c, &mut Watched::new(shapes), &opts).unwrap_or_else(|e| panic!("{e}"))
}

/// T82, a status bar redraw never reached: `main_loop` makes no call of `h0_6a96` between the
/// starts of passes 576 and 577, so the original shows the old countdown through passes 576 to
/// 607 (frames 6877 to 6908). With the bar the engine draws for that, the route's first 640 steps
/// are the original's to the pixel, strict; an engine that always redraws differs at frame 6877.
#[test]
fn the_opening_is_the_originals_to_the_pixel_through_a_redraw_never_reached() {
    let c = opening(640);
    let (with, without) = std::thread::scope(|s| {
        let with = s.spawn(|| run_as(&c, Shapes::Measured, Mode::Strict));
        let without = s.spawn(|| run_as(&c, Shapes::AlwaysRedrawn, Mode::Strict));
        (with.join().unwrap(), without.join().unwrap())
    });
    assert_eq!(with.outcome, Outcome::Same, "{}", head(&with));
    assert_eq!(with.skipped_redraws, 32, "{}", head(&with));
    assert!(
        matches!(without.outcome, Outcome::PictureDiffers { frame: 6877, .. }),
        "{}",
        head(&without)
    );
    assert_eq!(without.skipped_redraws, 0, "{}", head(&without));
}

/// The rule takes the old bar only for a period whose redraw was counted as never called
/// (coordinator, 2026-11-10): an engine showing the bar as it stood before every redraw fails in
/// a period the original redrew, before the first one it did not (pass 576).
#[test]
fn a_bar_left_unredrawn_where_the_original_redrew_fails() {
    let v = run_as(&opening(570), Shapes::NeverRedrawn, Mode::Faithful);
    assert!(
        matches!(v.outcome, Outcome::PictureDiffers { .. }),
        "{}",
        head(&v)
    );
    assert_eq!(v.skipped_redraws, 0, "{}", head(&v));
}

/// T82 on the frames T48 named: without the bar a skipped redraw leaves, the route fails at frame
/// 18 367, the third of the period from pass 12 064; with an add cut after its counts made whole
/// after the pass, two coins run a phase apart and it fails at frame 18 896, the third in a row.
#[test]
#[ignore = "full: 12 600 frames on the original with the picture, twice at once, about 60 s"]
fn without_the_skipped_redraw_or_the_cut_add_the_route_fails_on_the_frames_named() {
    let c = opening(12_600);
    let (bar, add) = std::thread::scope(|s| {
        let bar = s.spawn(|| run_as(&c, Shapes::AlwaysRedrawn, Mode::Faithful));
        let add = s.spawn(|| run_as(&c, Shapes::AddedWhole, Mode::Faithful));
        (bar.join().unwrap(), add.join().unwrap())
    });
    assert!(
        matches!(bar.outcome, Outcome::PictureDiffers { frame: 18367, .. }),
        "{}",
        head(&bar)
    );
    assert!(
        matches!(add.outcome, Outcome::PictureDiffers { frame: 18896, .. }),
        "{}",
        head(&add)
    );
}

/// `items_left` (BCD) after the route's first 600 steps, measured 2026-11-10.
const OPENING_ITEMS: u32 = 0x272;

/// The whole route: the case passes with the level's end as its goal, no life is lost on any pass,
/// nothing but the level's end is raised, only T82's boxes are left out of the picture, and the ported
/// routines the route never runs are exactly the named ones (DESIGN §4.1).
#[test]
#[ignore = "full: 22 536 frames on the original with the picture, about 80 s"]
fn the_route_reaches_the_levels_end_on_the_original_with_no_life_lost() {
    let c = case("route-W1-1");
    assert!(c.compare.vars && c.compare.picture);
    assert!(matches!(c.goal, Some(Goal::LevelEnd { .. })));
    // Sound is not compared (DESIGN §2.1). The whole picture is, on every frame (T82).
    assert!(c.exclude.is_empty(), "{:?}", c.exclude);
    for slot in [
        "creature_active+$0",
        "creature_slots+$24",
        "clxdat_copy",
        "lives",
        "hits_taken",
        "items_left",
        "score",
    ] {
        assert!(c.vars.iter().any(|v| v == slot), "{slot} is not compared");
    }
    let mut g = Watched::new(Shapes::Measured);
    let v = run_with(&c, &mut g, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v.outcome, Outcome::Same, "{}", head(&v));
    assert!(
        v.level_end.is_some() && v.goal_met == v.level_end,
        "{}",
        head(&v)
    );
    assert_eq!(g.lost, None, "a life lost at pass {:?}", g.lost);
    // No `Event::Unported`, no game over, no restart: the level's end and nothing else.
    let events: Vec<Event> = v.events.iter().map(|(_, e)| *e).collect();
    assert_eq!(events, [Event::LevelEnd]);
    assert_eq!(g.game.state().items_left, 0);
    assert!(v.passes > 21_000 && v.pictures > 21_000, "{}", head(&v));
    // The picture, measured 2026-11-10 (docs/13, "The picture on the route"): seven status
    // periods whose redraw the original never reached, and nine frames past a few pixels, each a
    // taken item erased behind the beam, never three in a row.
    assert_eq!(v.skipped_redraws, 220, "{}", head(&v));
    assert_eq!(v.tolerated[2], 9, "{}", head(&v));
    // Coverage: what the route never ran is the named list, no more and no fewer.
    let unreached: BTreeSet<&str> = v.coverage.unreached().into_iter().collect();
    let named: BTreeSet<&str> = NOT_ON_ROUTE.iter().map(|(n, _, _)| *n).collect();
    assert_eq!(unreached, named, "{}", v.coverage);
    // Every routine an object value of W1-1 runs through (re/objects-W1-1.toml) that is ported ran
    // here, the time bonus's apart.
    let listed = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../re/objects-W1-1.toml"),
    )
    .unwrap();
    let ported: BTreeSet<&str> = PORTED.iter().map(|p| p.name).collect();
    let mut by_objects = 0;
    for line in listed.lines().filter(|l| l.starts_with("routines = ")) {
        for r in line.split('"').skip(1).step_by(2) {
            if ported.contains(r) {
                by_objects += 1;
                assert!(
                    !unreached.contains(r) || r == "countdown_plus_100",
                    "{r} never ran"
                );
            }
        }
    }
    assert!(by_objects > 100, "{by_objects} routine names read");
}

/// The named list holds: each routine with a case runs in it, and the rest are the seven no case
/// from the level sync point can run.
#[test]
#[ignore = "full: death-pit on the original, about 5 s"]
fn the_routines_the_route_never_runs_run_in_the_case_named() {
    let cases: BTreeSet<&str> = NOT_ON_ROUTE.iter().filter_map(|(_, c, _)| *c).collect();
    for name in cases {
        let v = run(&case(name)).unwrap_or_else(|e| panic!("{e}"));
        assert!(v.passed(), "{name}: {}", head(&v));
        let unreached = v.coverage.unreached();
        for (routine, _, why) in NOT_ON_ROUTE.iter().filter(|(_, c, _)| *c == Some(name)) {
            assert!(
                !unreached.contains(routine),
                "{routine} ({why}) did not run in {name}"
            );
        }
    }
    assert_eq!(
        NOT_ON_ROUTE.iter().filter(|(_, c, _)| c.is_none()).count(),
        7
    );
    for (routine, _, _) in NOT_ON_ROUTE {
        assert!(
            PORTED.iter().any(|p| p.name == routine),
            "{routine} is not in PORTED"
        );
    }
}

/// A pass that cuts a row's redraw before its shapes are copied probes the band as it was (T48):
/// at pass 4 976 of the route a creature on the row below the view turns at a ledge the row's
/// shapes then fill. An engine copying the shapes before the pass walks on.
#[test]
#[ignore = "full: 4 990 frames on the original twice, about 25 s"]
fn a_row_cut_before_its_shapes_keeps_the_band_for_the_pass() {
    let mut c = case("route-W1-1");
    c.frames = 4990;
    c.compare.picture = false;
    c.exclude.clear();
    c.goal = Some(Goal::All {
        all: vec![("lives".into(), LIVES as u32)],
        by_frame: c.sync_frame + c.frames,
    });
    let mut g = Watched::new(Shapes::Measured);
    let v = run_with(&c, &mut g, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v.outcome, Outcome::Same, "{}", head(&v));
    let mut g = Watched::new(Shapes::CopiedEarly);
    let v = run_with(&c, &mut g, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    match &v.outcome {
        Outcome::Differs {
            pass: 4976, var, ..
        } if var == "creature_slots+$1c" => {}
        other => panic!("{other}\n{}", head(&v)),
    }
}
