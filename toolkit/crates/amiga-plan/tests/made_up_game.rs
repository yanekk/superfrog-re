//! A made-up grid game solved by the planner, with no disk: the toolkit's planner is enough on its
//! own for a game that implements `PlanGame`, as the dry fit shows for the engine crates.
//!
//! The game: the player moves one cell a frame in the joystick's direction, walls stop it, a pit
//! costs a life, a switch opens a gate, and the exit counts only once the collectible is taken.
//! The exit is behind the gate and the collectible up a winding corridor, so a route must press the
//! switch, take the collectible, and come back down without falling in the pit.

use std::sync::Arc;
use std::time::Duration;

use amiga_plan::run::{RunOptions, plan_engine};
use amiga_plan::shorten::replay_exit;
use amiga_plan::walk::WalkCompass;
use amiga_plan::{Cell, CellKey, Compass, Facts, FrameInput, Joy, Options, PlanGame};

const LEVEL: [&str; 7] = [
    "###########",
    "#S..#....c#",
    "#.#.#.###.#",
    "#.#...#...#",
    "#.#####.###",
    "#s..x...GE#",
    "###########",
];

#[derive(Clone)]
struct Grid {
    map: Arc<Vec<Vec<u8>>>,
    at: Cell,
    taken: bool,
    pressed: bool,
    playing: bool,
    lives: i16,
    dying: bool,
}

impl Grid {
    fn new() -> (Grid, Facts) {
        let map: Vec<Vec<u8>> = LEVEL.iter().map(|r| r.bytes().collect()).collect();
        let find = |b: u8| -> Vec<Cell> {
            (0..map.len())
                .flat_map(|r| (0..map[r].len()).map(move |c| (c, r)))
                .filter(|&(c, r)| map[r][c] == b)
                .map(|(c, r)| (c as u16, r as u16))
                .collect()
        };
        let facts = Facts {
            cols: map[0].len() as u16,
            rows: map.len() as u16,
            exits: find(b'E'),
            collectibles: find(b'c'),
            gates: vec![find(b'G')],
        };
        let g = Grid {
            at: find(b'S')[0],
            map: Arc::new(map),
            taken: false,
            pressed: false,
            playing: true,
            lives: 3,
            dying: false,
        };
        (g, facts)
    }

    fn tile(&self, (c, r): Cell) -> u8 {
        self.map[r as usize][c as usize]
    }

    fn wall(&self, c: Cell) -> bool {
        match self.tile(c) {
            b'#' => true,
            b'G' => !self.pressed,
            _ => false,
        }
    }
}

impl PlanGame for Grid {
    fn step(&mut self, i: FrameInput) {
        if !self.playing || self.dying {
            return;
        }
        let (c, r) = self.at;
        let j = i.joy;
        let to = if j.0 & Joy::RIGHT.0 != 0 {
            (c + 1, r)
        } else if j.0 & Joy::LEFT.0 != 0 {
            (c - 1, r)
        } else if j.0 & Joy::UP.0 != 0 {
            (c, r - 1)
        } else if j.0 & Joy::DOWN.0 != 0 {
            (c, r + 1)
        } else {
            return;
        };
        if self.wall(to) {
            return;
        }
        self.at = to;
        match self.tile(to) {
            b'x' => {
                self.lives -= 1;
                self.dying = true;
            }
            b's' => self.pressed = true,
            b'c' => self.taken = true,
            b'E' if self.taken => self.playing = false,
            _ => {}
        }
    }
    fn state_bytes(&self) -> u64 {
        std::mem::size_of::<Grid>() as u64
    }
    fn cell(&self) -> Cell {
        self.at
    }
    fn lives(&self) -> i16 {
        self.lives
    }
    fn dying(&self) -> bool {
        self.dying
    }
    fn health(&self) -> u16 {
        1
    }
    fn playing(&self) -> bool {
        self.playing
    }
    fn collectibles_left(&self) -> u16 {
        u16::from(!self.taken)
    }
    fn held_flag(&self) -> bool {
        false
    }
    fn landed(&self) -> bool {
        true
    }
    fn open(&self, c: Cell) -> bool {
        !self.wall(c) && self.tile(c) != b'x'
    }
    fn collectible_live(&self, c: Cell) -> bool {
        self.tile(c) == b'c' && !self.taken
    }
    fn pressed(&self, _: &Facts) -> u32 {
        u32::from(self.pressed)
    }
    const ACTS: &'static [Joy] = &[Joy::NONE, Joy::RIGHT, Joy::LEFT, Joy::UP, Joy::DOWN];
}

fn run(threads: usize) -> RunOptions {
    RunOptions {
        threads,
        budget: Duration::from_secs(120),
        max_rounds: Some(2_000),
        ..RunOptions::default()
    }
}

/// Plans the grid to its exit with `compass` on `threads` threads, and checks the route on a fresh
/// game: the exit reached with the collectible and the switch, no life lost.
fn solve(compass: Compass, threads: usize) -> Vec<FrameInput> {
    let (start, facts) = Grid::new();
    let opt = Options {
        compass,
        ..Options::default()
    };
    let route = match plan_engine(start.clone(), facts, opt, &run(threads)) {
        Ok(r) => r,
        Err((_, why)) => panic!("no route: {why:?}"),
    };
    let mut g = start.clone();
    for &x in &route.inputs {
        g.step(x);
    }
    assert!(!g.playing, "the route ends at the exit");
    assert!(g.taken && g.pressed);
    assert_eq!(g.lives, 3, "no life lost");
    let mut f = 0;
    assert_eq!(
        replay_exit(&start, &route.inputs, &mut f),
        Some((route.inputs.len(), 0))
    );
    route.inputs
}

#[test]
fn the_grid_is_solved_with_the_straight_compass() {
    solve(Compass::Straight, 2);
}

#[test]
fn the_grid_is_solved_with_the_walk_compass() {
    solve(Compass::Walk, 2);
}

/// The same seed gives the same route on any thread count.
#[test]
fn the_route_is_the_same_on_any_threads() {
    assert_eq!(solve(Compass::Walk, 1), solve(Compass::Walk, 4));
}

/// The walk compass reads the exit as sealed behind the shut gate, and open once its switch is
/// pressed.
#[test]
fn the_walk_compass_opens_the_gate_with_its_switch() {
    let (start, facts) = Grid::new();
    let mut w = WalkCompass::new(&start, &facts);
    let key = |switches| CellKey {
        col: 3,
        row: 2,
        band: 0,
        switches,
        wings: false,
        via: 0,
    };
    assert_eq!(w.exit_steps(&key(0)), None, "the exit is shut in");
    assert!(w.exit_steps(&key(1)).is_some());
}
