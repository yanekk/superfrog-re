//! What ends the game, against the original (plans/forest-dumb-level-1 T23, DESIGN §2.4): the cases
//! `death-esc` (P pauses, Esc gives up) and `death-countdown` (the countdown runs out), each through
//! the game over screen to where the original enters its high scores and the engine restarts the
//! level; the runner's scene checks; and a recording through game over and the restart.
//!
//! A life lost (`char_hit_check`, contact on the spikes) and the last one ending the game are T70's:
//! the engine models no contact yet. Needs the disks and Kickstart, as every case does.

use std::path::Path;

use amiga_chips::Picture;
use amiga_oracle::{FrameInput, Joy};
use fd_conform::{Case, Engine, Options, Outcome, Verdict, run_with};
use fd_core::recording::{LevelId, recorder};
use fd_core::{Event, Game, Mem, Scene, Start, Transplant, Var};

fn load(name: &str) -> Case {
    Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap_or_else(|e| panic!("{e}"))
}

fn game() -> Game {
    Game::new(
        fd_conform::game_data().unwrap_or_else(|e| panic!("{e}")),
        Start::Level1,
    )
}

fn run(case: &Case, engine: &mut dyn Engine) -> Verdict {
    let v = run_with(case, engine, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    println!("{v}");
    v
}

fn hits(v: &Verdict, name: &str) -> u32 {
    v.coverage
        .hits
        .iter()
        .find(|(p, _)| *p == name)
        .map_or(0, |(_, n)| *n)
}

#[test]
fn giving_up_shows_the_original_game_over_screen_until_fire_is_let_go() {
    let case = load("death-esc");
    let v = run(&case, &mut game());
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    // Measured: Esc let go in step 6483 gives up in that pass; main_loop's check after pass 192 (frame
    // 6492) shows the screen in the next step; fire held from step 6540 (the 47th of the screen) and
    // let go in 6543 ends it there.
    assert_eq!(v.game_over, Some((6493, Some(6544))), "{v}");
    assert_eq!(v.goal_met, Some(6544));
    let events: Vec<Event> = v.events.iter().map(|(_, e)| *e).collect();
    assert_eq!(events, [Event::GameOver, Event::Restart], "{v}");
    // Every frame of the screen but the one that switched to it was compared.
    assert!(v.pictures > 80 + (6544 - 6493 - 1), "{v}");
    // P paused 40 passes (the paused branch runs play_keys, not countdown_step).
    assert_eq!(hits(&v, "play_keys"), 92, "{v}");
    assert_eq!(hits(&v, "esc_key"), 92, "{v}");
    assert_eq!(hits(&v, "countdown_step"), 52, "{v}");
}

#[test]
#[ignore = "about 63 000 frames: the full set"]
fn the_countdown_running_out_ends_the_game_as_the_originals() {
    let case = load("death-countdown");
    let v = run(&case, &mut game());
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    // Measured (release, 140 s): every pass from 6400, the countdown's digits down to 000.
    assert_eq!(v.game_over, Some((69245, Some(69284))), "{v}");
    assert!(v.passes > 62_800, "{v}");
}

/// fd-core's game with its game over screen changed one way.
struct Changed {
    game: Game,
    /// Never shows the screen: steps on as play.
    no_screen: bool,
    /// Holds fire for the screen's first 22 steps, so leaves it as soon as fire counts.
    fire: bool,
    /// Steps on the screen so far.
    screen_steps: u32,
}

impl Transplant for Changed {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.game.transplant(mem)
    }
    fn probe(&self, v: &Var) -> u32 {
        self.game.probe(v)
    }
}

impl Engine for Changed {
    fn pass(&mut self, input: FrameInput) {
        self.game.step(input)
    }
    fn picture(&self) -> Option<Picture> {
        Some(self.game.picture())
    }
    fn off_play(&mut self, mut input: FrameInput) {
        if self.no_screen {
            return;
        }
        if self.fire && self.screen_steps <= 21 {
            input.joy = input.joy | Joy::FIRE;
        }
        self.screen_steps += 1;
        self.game.step(input)
    }
    fn game_over(&self) -> bool {
        matches!(self.game.scene(), Scene::GameOver(_))
    }
}

fn changed(no_screen: bool, fire: bool) -> Changed {
    Changed {
        game: game(),
        no_screen,
        fire,
        screen_steps: 0,
    }
}

#[test]
fn an_engine_that_misses_the_screen_or_leaves_it_early_fails() {
    let case = load("death-esc");
    let v = run(&case, &mut changed(true, false));
    assert_eq!(
        v.outcome,
        Outcome::SceneDiffers {
            frame: 6493,
            why: "the original went to the game over screen; the engine did not".into()
        },
        "{v}"
    );
    // Fire held from the screen's first step counts at its 21st and is let go at its 22nd: the
    // original waits for the script's fire, 25 steps later.
    let v = run(&case, &mut changed(false, true));
    let Outcome::SceneDiffers { frame, why } = &v.outcome else {
        panic!("{v}")
    };
    assert_eq!(
        (*frame, why.as_str()),
        (
            6515,
            "the engine left the game over screen; the original did not"
        )
    );
}

#[test]
fn a_recording_runs_through_game_over_and_the_restart_and_replays_alike() {
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    // Right for a while, Esc, the screen, fire, then play again from the start.
    let mut inputs = vec![FrameInput::joy(Joy::RIGHT); 60];
    inputs.extend([FrameInput::key(0x45); 3]);
    inputs.extend(vec![FrameInput::NONE; 60]);
    inputs.extend([FrameInput::joy(Joy::FIRE); 3]);
    inputs.extend(vec![FrameInput::joy(Joy::RIGHT); 40]);
    let mut r = recorder(LevelId::W1_1, fd_tools::ENGINE_BUILD);
    let mut g = Game::new(data.clone(), Start::Level1);
    let mut events = Vec::new();
    let mut restarted = None;
    for (k, &i) in inputs.iter().enumerate() {
        r.step(i);
        g.step(i);
        for e in g.take_events() {
            if e == Event::Restart {
                restarted = Some(k);
            }
            events.push(e);
        }
    }
    assert_eq!(events, [Event::GameOver, Event::Restart]);
    let rec = r
        .finish(g.location(), "through game over".into())
        .expect("recording");
    let a = fd_conform::bug::replay(data.clone(), &rec, |_, _, _| {});
    let b = fd_conform::bug::replay(data.clone(), &rec, |_, _, _| {});
    assert_eq!(a.game, g);
    assert_eq!(a.game, b.game);
    assert_eq!(to_rgb(&a.last), to_rgb(&b.last));
    // Fire let go restarts the level in that step, as a new game: the rest of the recording plays
    // on from there with lives full and score zero.
    let k = restarted.unwrap();
    assert_eq!(
        inputs[k],
        FrameInput::joy(Joy::RIGHT),
        "the step fire is let go in"
    );
    let mut fresh = Game::new(data, Start::Level1);
    for &i in &inputs[k + 1..] {
        fresh.step(i);
    }
    assert_eq!(*a.game.scene(), Scene::Level);
    assert_eq!((a.game.lives(), a.game.state().score), (32, 0));
    assert_eq!(a.game, fresh);
}

fn to_rgb(p: &Picture) -> Vec<u16> {
    amiga_chips::to_rgb12(p)
}
