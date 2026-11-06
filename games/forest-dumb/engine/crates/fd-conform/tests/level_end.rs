//! The level's end against the original (plans/forest-dumb-level-1 T26, DESIGN §2.5): the case
//! `level-end` (the cheat on, HELP, `main_loop`'s check, `next_world`), compared up to the step the
//! original runs `next_world` in and stopped there by name; the runner's scene checks; the status
//! bar the check skips; and a recording through "Level 1 complete" and the restart.
//!
//! HELP with `cheat_flag` on reaches the level's end without collecting W1-1's 282 items; it
//! counts only when read by a pass `main_loop`'s check follows, since `char_object_cells` (T22)
//! clears `level_exit_touched` each pass. Needs the disks and Kickstart, as every case does.

use std::path::Path;

use amiga_chips::Picture;
use amiga_oracle::{FrameInput, Joy};
use fd_conform::{AFTER_NEXT_WORLD, Case, Engine, Options, Outcome, Verdict, run_with};
use fd_core::level_end::LevelEnd;
use fd_core::recording::{LevelId, recorder};
use fd_core::{Event, Game, Mem, Scene, Start, Transplant, Var};

/// HELP's raw key code.
const HELP: u8 = 0x5f;

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

#[test]
fn the_level_end_matches_the_original_up_to_next_world_and_stops_there_by_name() {
    let case = load("level-end");
    let mut g = game();
    let v = run(&case, &mut g);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    // Measured: HELP pressed in step 6523 is read by pass 224's help_key; main_loop's check after
    // it runs next_world in step 6525 (frame 6525's), where the comparison ends.
    assert_eq!(v.level_end, Some(6525), "{v}");
    assert_eq!(v.goal_met, Some(6525));
    let events: Vec<Event> = v.events.iter().map(|(_, e)| *e).collect();
    assert_eq!(events, [Event::LevelEnd], "{v}");
    assert_eq!(*g.scene(), Scene::LevelEnd(LevelEnd { world: 1 }));
    assert_eq!(g.state().world_index, 1);
    // Every pass up to the end compared, variables and picture; none after, by name.
    assert_eq!((v.passes, v.pictures, v.unseen), (123, 123, 0), "{v}");
    assert!(case.frames > 6525 - case.sync_frame + 1, "frames were left");
    assert!(v.to_string().contains(AFTER_NEXT_WORLD), "{v}");
    let hits = |name: &str| {
        v.coverage
            .hits
            .iter()
            .find(|(p, _)| *p == name)
            .map_or(0, |(_, n)| *n)
    };
    assert_eq!(hits("help_key"), 124, "{v}");
}

/// fd-core's game with its level end changed one way.
struct Changed {
    game: Game,
    /// The cheat off after the transplant: HELP does nothing, the level never ends.
    no_cheat: bool,
    /// Claims the level's end once this many passes have run.
    early: Option<u32>,
    /// Reports `world_index` one past the game's once the level has ended: a `next_world` that
    /// moves to the wrong world.
    world_off: bool,
    passes: u32,
}

impl Transplant for Changed {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.game.transplant(mem);
        if self.no_cheat {
            self.game.set_cheat(false);
        }
    }
    fn probe(&self, v: &Var) -> u32 {
        let off = self.world_off && self.game.finished() && v.name == "world_index";
        self.game.probe(v) + off as u32
    }
}

impl Engine for Changed {
    fn freeze(&mut self) {
        Engine::freeze(&mut self.game);
    }
    fn pass(&mut self, input: FrameInput) {
        self.passes += 1;
        self.game.step(input)
    }
    fn picture(&self) -> Option<Picture> {
        Some(self.game.picture())
    }
    fn off_play(&mut self, input: FrameInput) {
        self.game.step(input)
    }
    fn level_end(&self) -> bool {
        self.game.finished() || self.early.is_some_and(|k| self.passes >= k)
    }
}

#[test]
fn an_engine_that_misses_the_end_or_finds_it_early_fails() {
    let mut case = load("level-end");
    // What HELP changes with the cheat on would fail first; here the scene check is what fails.
    let help = [
        "cheat_flag",
        "last_key_byte",
        "level_exit_touched",
        "items_left",
    ];
    case.vars.retain(|v| !help.contains(&v.as_str()));
    let v = run(
        &case,
        &mut Changed {
            game: game(),
            no_cheat: true,
            early: None,
            world_off: false,
            passes: 0,
        },
    );
    assert_eq!(
        v.outcome,
        Outcome::SceneDiffers {
            frame: 6525,
            why: "the original ran `next_world`; the engine did not find the level's end".into()
        },
        "{v}"
    );
    let v = run(
        &case,
        &mut Changed {
            game: game(),
            no_cheat: false,
            early: Some(10),
            world_off: false,
            passes: 0,
        },
    );
    let Outcome::SceneDiffers { why, .. } = &v.outcome else {
        panic!("{v}")
    };
    assert_eq!(
        why,
        "the engine found the level's end; the original ran a pass"
    );
}

/// The runner holds `world_index` in the step both sides run `next_world`, which no pass end
/// compares: an engine that ends the level in the right step on the wrong world fails there.
#[test]
fn an_engine_that_moves_to_the_wrong_world_fails_at_next_world() {
    let case = load("level-end");
    let v = run(
        &case,
        &mut Changed {
            game: game(),
            no_cheat: false,
            early: None,
            world_off: true,
            passes: 0,
        },
    );
    let Outcome::Differs {
        frame,
        var,
        engine,
        oracle,
        ..
    } = &v.outcome
    else {
        panic!("{v}")
    };
    assert_eq!(
        (*frame, var.as_str(), *engine, *oracle),
        (6525, "world_index", 2, 1),
        "{v}"
    );
    assert_eq!(v.level_end, None, "{v}");
}

/// A game with the cheat on, run to `irq_count` 63: HELP in its next step is read by pass 64's
/// `help_key`, whose exit touched with no items left `main_loop`'s check after that pass finds.
/// HELP any earlier is forgotten: the next pass's `char_object_cells` clears `level_exit_touched`
/// (T22), as the original's does.
fn ending() -> Game {
    let mut g = game();
    g.set_cheat(true);
    while g.state().irq_count < 63 {
        g.step(FrameInput::NONE);
    }
    g
}

/// `main_loop` branches to `next_world` (`h0+$286`..`$298`) before `bsr h0_6a96`, its status bar
/// redraw, so the frame of the pass its check finds the level's end after keeps the bar's old
/// numbers. The runner never compares that frame (the comparison ends in the next step), so this
/// holds the engine to the listing, as `death.rs` does for the game over: the countdown loses one at
/// pass 63, and pass 64's frame still shows 999 when the level has ended, 998 when not.
#[test]
fn the_frame_main_loop_finds_the_level_end_after_keeps_the_old_status_bar() {
    let (mut end, mut on) = (ending(), game());
    while on.state().irq_count < end.state().irq_count {
        on.step(FrameInput::NONE);
    }
    let before = end.picture();
    end.step(FrameInput::key(HELP));
    on.step(FrameInput::NONE);
    assert_eq!(*end.scene(), Scene::Level);
    assert_eq!(
        (end.state().irq_count, end.state().level_exit_touched),
        (64, 1)
    );
    assert_eq!((end.energy(), on.energy()), (998, 998));
    let digits = |p: &Picture| -> Vec<u16> {
        let rgb = amiga_chips::to_rgb12(p);
        (223..231)
            .flat_map(|y| (144..168).map(move |x| (y, x)))
            .map(|(y, x)| rgb[y * p.width as usize + x])
            .collect()
    };
    let (shown, redrawn) = (digits(&end.picture()), digits(&on.picture()));
    assert_ne!(shown, redrawn, "998 is drawn when the level goes on");
    assert_eq!(shown, digits(&before), "the bar as pass 63 left it");
    // The step after: next_world, the play colours black, then the message.
    end.step(FrameInput::NONE);
    assert_eq!(end.take_events().last(), Some(&Event::LevelEnd));
    let black = end.picture();
    let play_rows = 194 * black.width as usize;
    assert!(
        amiga_chips::to_rgb12(&black)[..play_rows]
            .iter()
            .all(|&c| c == 0),
        "h0+$500 clears the play area's colours"
    );
}

#[test]
fn a_recording_runs_through_level_1_complete_and_the_restart_and_replays_alike() {
    let data = fd_conform::game_data().unwrap_or_else(|e| panic!("{e}"));
    // Right for a while, then fire held (play reads no fire) through HELP read by pass 64 (an
    // earlier one is cleared by the next pass's `char_object_cells`, T22); the check after it ends
    // the level, the message ignores keys and the held fire, fire pressed restarts, then right
    // again until Stop.
    let mut inputs = vec![FrameInput::joy(Joy::RIGHT); 40];
    inputs.extend(vec![FrameInput::joy(Joy::FIRE); 22]);
    let help_and_fire = FrameInput {
        joy: Joy::FIRE,
        ..FrameInput::key(HELP)
    };
    inputs.extend([help_and_fire; 3]);
    inputs.extend(vec![FrameInput::joy(Joy::FIRE); 8]);
    inputs.extend(vec![FrameInput::joy(Joy::LEFT | Joy::UP); 10]);
    inputs.extend(vec![FrameInput::NONE; 5]);
    inputs.extend([FrameInput::joy(Joy::FIRE); 3]);
    inputs.extend(vec![FrameInput::joy(Joy::RIGHT); 40]);
    // The cheat as the title's code would set it: the recording holds inputs only, so a replay
    // starts its game the same way (`Game::set_cheat`).
    let cheat_game = || {
        let mut g = Game::new(data.clone(), Start::Level1);
        g.set_cheat(true);
        g
    };
    let mut r = recorder(LevelId::W1_1, fd_tools::ENGINE_BUILD);
    let mut g = cheat_game();
    let mut events = Vec::new();
    let (mut ended, mut restarted) = (None, None);
    for (k, &i) in inputs.iter().enumerate() {
        r.step(i);
        g.step(i);
        for e in g.take_events() {
            match e {
                Event::LevelEnd => ended = Some(k),
                Event::Restart => restarted = Some(k),
                _ => {}
            }
            events.push(e);
        }
    }
    assert_eq!(events, [Event::LevelEnd, Event::Restart]);
    // Step k is pass k + 2: the check after pass 64 (step 62) ends the level in step 63; fire
    // pressed first after it was let go, in step 88, restarts.
    assert_eq!((ended, restarted), (Some(63), Some(88)));
    let rec = r
        .finish(g.location(), "through level 1 complete".into())
        .expect("recording");
    assert_eq!(rec.inputs, inputs, "the recording ran on until Stop");
    let replay = || {
        let mut game = cheat_game();
        let last = amiga_record::replay(&mut game, &rec.inputs, |_, _, _| {});
        (game, last)
    };
    let ((a, a_last), (b, b_last)) = (replay(), replay());
    assert_eq!(a, g);
    assert_eq!(a, b);
    assert_eq!(
        amiga_chips::to_rgb12(&a_last),
        amiga_chips::to_rgb12(&b_last)
    );
    // From the restart on, the game is a new game: lives full, score zero.
    let mut fresh = Game::new(data, Start::Level1);
    for &i in &inputs[restarted.unwrap() + 1..] {
        fresh.step(i);
    }
    assert_eq!(*a.scene(), Scene::Level);
    assert_eq!((a.lives(), a.state().score), (32, 0));
    assert_eq!(a, fresh);
}
