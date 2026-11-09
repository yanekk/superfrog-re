//! The camera's whole-run cases (plans/forest-dumb-level-1 T43): fd-core's `Game` against the original
//! on scripted runs that scroll both ways and reach W1-1's four ends, comparing the camera, the
//! character and the edge-redraw hand-off (`edge_latch_*`, `edge_at_*`) on every pass
//! (`conform/cases/camera-*.toml`). Needs the disks and the Kickstart: absent, it fails naming where it
//! looked.

use std::path::Path;

use amiga_chips::Picture;
use amiga_oracle::FrameInput;
use fd_conform::{Case, Coverage, Engine, Inputs, Live, Options, Outcome, Verdict, reach_case};
use fd_conform::{game_data, run_with};
use fd_core::level::collision::{BEFORE, CollisionPlane, KEPT, MAP};
use fd_core::{Event, Game, Mem, PORTED, Start, Transplant, Var, layout_var};

const CASES: [&str; 3] = [
    "camera-top-left",
    "camera-right-left",
    "camera-bottom-right",
];

/// The routines T09 registered (`fd-core/src/ported.rs`, "T09, the camera").
const T09: [&str; 10] = [
    "camera_update",
    "h0_4e9a",
    "h0_4ea6",
    "h0_4eb4",
    "h0_4ec2",
    "h0_513c",
    "h0_51fc",
    "h0_526a",
    "h0_52d8",
    "h0_5346",
];

fn case(name: &str) -> Case {
    Case::load(&Path::new("cases").join(format!("{name}.toml"))).unwrap_or_else(|e| panic!("{e}"))
}

fn game() -> Game {
    Game::new(game_data().unwrap_or_else(|e| panic!("{e}")), Start::Level1)
}

/// `case` on fd-core's `Game`, and the game as the run left it.
fn run(case: &Case) -> (Verdict, Game) {
    let mut g = game();
    let v = run_with(case, &mut g, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    (v, g)
}

fn hits(c: &Coverage, name: &str) -> u32 {
    c.hits
        .iter()
        .find(|(n, _)| *n == name)
        .map_or(0, |(_, h)| *h)
}

#[test]
fn each_camera_case_passes_on_the_level_engine_and_reaches_its_goal() {
    let mut reached = vec![0u32; T09.len()];
    for name in CASES {
        let c = case(name);
        assert!(c.goal.is_some(), "{name} states no goal");
        let (v, _) = run(&c);
        assert_eq!(v.outcome, Outcome::Same, "{v}");
        assert!(v.goal_met.is_some(), "{v}");
        // A step now and then ends two passes; nearly every pass is compared. Since T47 the
        // animated cells' blits lengthen the original's passes: 24 of 780 unseen in camera-right-left.
        assert!(v.passes + v.unseen + 1 >= c.frames - 5, "{v}");
        assert!(v.unseen * 25 < v.passes, "{v}");
        for (n, r) in T09.iter().zip(&mut reached) {
            *r += hits(&v.coverage, n);
        }
    }
    // Every routine T09 ported ran on the original in these cases (DESIGN §4.1).
    let none: Vec<&str> = T09
        .iter()
        .zip(&reached)
        .filter(|(_, h)| **h == 0)
        .map(|(n, _)| *n)
        .collect();
    assert!(none.is_empty(), "never reached: {none:?}");
    // And each T09 entry here is the register's, so a routine added to T09 is not left out.
    for n in T09 {
        assert!(PORTED.iter().any(|p| p.name == n), "{n} is not registered");
    }
}

/// The camera's extremes on the original over `case`, sampled at every frame's end.
fn extremes(c: &Case) -> ([u32; 2], [u32; 2], u32) {
    let inputs = Inputs::load(c).unwrap_or_else(|e| panic!("{e}"));
    let mut am = reach_case(c, 0).unwrap_or_else(|e| panic!("{e}"));
    let var = |n: &str| layout_var(n).unwrap();
    let (cx, cy, x) = (var("camera_x"), var("camera_y"), var("char_x"));
    let (mut lo, mut hi, mut far) = ([u32::MAX; 2], [0u32; 2], 0);
    for f in 0..c.frames {
        am.step(inputs.at(c.sync_frame + f))
            .unwrap_or_else(|e| panic!("{e}"));
        let at = [Live(&am).value(cx), Live(&am).value(cy)];
        for i in 0..2 {
            lo[i] = lo[i].min(at[i]);
            hi[i] = hi[i].max(at[i]);
        }
        far = far.max(Live(&am).value(x));
    }
    (lo, hi, far)
}

#[test]
fn the_cases_reach_the_scroll_limits_at_all_four_level_ends() {
    // Left and top: camera 0, the clamp's lower ends (`h0_513c`, the scrolls' `tst.w; bpl`).
    let (lo, _, _) = extremes(&case("camera-top-left"));
    assert_eq!(lo, [0, 0]);
    // Bottom: camera_y's clamp, $53f. Right: the character against the wall at column 229, the
    // camera at $dc9. camera_x's own clamp, $17cf, is past column 400 and so never reached in W1-1.
    let (lo, hi, far) = extremes(&case("camera-bottom-right"));
    assert_eq!(hi, [0xdc9, 0x53f]);
    assert_eq!(lo[0], 0);
    assert_eq!(far >> 4, 229, "char_x {far:#x}");
    // The cases compare camera_x and camera_y on every pass, so the engine reaches the same.
    for name in ["camera-top-left", "camera-bottom-right"] {
        assert!(case(name).vars.iter().any(|v| v == "camera_y"), "{name}");
    }
}

/// `camera-bottom-right` cut where the character has stood at the right wall for 150 frames.
fn at_the_right_wall() -> Case {
    Case {
        frames: 1580,
        goal: None,
        exclude: Vec::new(),
        ..case("camera-bottom-right")
    }
}

#[test]
fn past_column_240_the_engine_draws_exactly_the_filler_the_original_does() {
    // At the wall the right edge was last redrawn from a camera_x in $dc0..$dcf: `redraw_edge_d`
    // reads map column (x >> 4) + 21 (its `$fc0` and `$7e0` offsets) = 241, past the 240 W1-1's shape map holds, so both sides read what follows it
    // in the level buffers. The engine reads it from the same layout (`LevelImage`); here its
    // collision_map equals the original's, those columns included, and no column past 241 is ever
    // drawn because the character never passes column 229 on either side (char_x is compared every
    // pass in the case).
    let c = at_the_right_wall();
    let (v, g) = run(&c);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    let edge_x = g.probe(layout_var("edge_at_right_x").unwrap());
    assert_eq!((edge_x >> 4) + 21, 241, "edge_at_right_x {edge_x:#x}");

    let inputs = Inputs::load(&c).unwrap_or_else(|e| panic!("{e}"));
    let mut am = reach_case(&c, 0).unwrap_or_else(|e| panic!("{e}"));
    for f in 0..c.frames {
        am.step(inputs.at(c.sync_frame + f))
            .unwrap_or_else(|e| panic!("{e}"));
    }
    let theirs = Live(&am).read(3, MAP - BEFORE, (BEFORE + KEPT) as usize);
    let differing = |ours: &CollisionPlane| -> Vec<u32> {
        CollisionPlane::kept_offsets()
            .zip(&theirs)
            .filter(|&(at, b)| ours.kept_byte(at) != *b)
            .map(|(at, _)| at)
            .collect()
    };
    // The comparison sees something: the level start's plane is far from the wall's.
    assert!(differing(game().plane()).len() > 1000);
    let differ = differing(g.plane());
    assert!(
        differ.is_empty(),
        "{} byte(s) of collision_map differ, first at {:+#x}",
        differ.len(),
        differ[0] as i32
    );
}

/// The level engine with its camera stopping one pixel short of camera_y's clamp: what a case must
/// catch.
struct ShortClamp(Game);

impl Transplant for ShortClamp {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.0.transplant(mem);
    }
    fn probe(&self, v: &Var) -> u32 {
        let n = self.0.probe(v);
        if v.name == "camera_y" {
            n.min(0x53e)
        } else {
            n
        }
    }
}

impl Engine for ShortClamp {
    fn freeze(&mut self) {
        Engine::freeze(&mut self.0);
    }
    fn pass(&mut self, input: FrameInput) {
        self.0.pass(input);
    }
    fn picture(&self) -> Option<Picture> {
        None
    }
    fn events(&mut self) -> Vec<Event> {
        self.0.events()
    }
    fn foreground_cut(&mut self, at: Option<fd_core::level::EdgesCut>) {
        self.0.foreground_cut(at);
    }
}

#[test]
fn a_camera_one_pixel_short_of_the_bottom_clamp_fails_the_case() {
    let c = case("camera-bottom-right");
    let v = run_with(&c, &mut ShortClamp(game()), &Options::default())
        .unwrap_or_else(|e| panic!("{e}"));
    match &v.outcome {
        Outcome::Differs {
            var,
            engine,
            oracle,
            ..
        } => {
            assert_eq!(
                (var.as_str(), *engine, *oracle),
                ("camera_y", 0x53e, 0x53f),
                "{v}"
            )
        }
        other => panic!("expected camera_y to differ, got {other:?}"),
    }
}
