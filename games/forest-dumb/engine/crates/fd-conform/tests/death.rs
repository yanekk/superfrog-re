//! What ends the game, against the original (plans/forest-dumb-level-1 T23, DESIGN §2.4): the cases
//! `death-esc` (P pauses, Esc gives up) and `death-countdown` (the countdown runs out), each through
//! the game over screen to where the original enters its high scores and the engine restarts the
//! level; the runner's scene checks; and a recording through game over and the restart.
//!
//! A life lost on the spikes (`death-pit`) and the last one ending the game (`death-spikes`) are
//! T70's: `char_hit_check` on CLXDAT the engine's own frame sets (`fd_core::render::contact`). Needs
//! the disks and Kickstart, as every case does.

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
    fn freeze(&mut self) {
        Engine::freeze(&mut self.game);
    }
    fn thaw(&mut self, offset: u32) {
        Engine::thaw(&mut self.game, offset);
    }
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

/// `main_loop` branches to `game_over_screen` (`h0+$264`..`$282`) before `bsr h0_6a96`, its status
/// bar redraw, so the frame of the pass its check finds the game over after keeps the bar's old
/// numbers. The runner never compares that frame (the screen switches in it), so this holds the
/// engine to the listing: the countdown loses one at pass 63, and pass 64's frame still shows 999
/// when Esc was given up between, 998 when not.
#[test]
fn the_frame_main_loop_finds_the_game_over_after_keeps_the_old_status_bar() {
    let (mut over, mut on) = (game(), game());
    let step = |g: &mut Game, i: FrameInput| {
        g.step(i);
        assert_eq!(*g.scene(), Scene::Level);
    };
    while over.state().irq_count < 40 {
        step(&mut over, FrameInput::NONE);
        step(&mut on, FrameInput::NONE);
    }
    for i in [FrameInput::key(0x45), FrameInput::NONE] {
        step(&mut over, i);
        step(&mut on, FrameInput::NONE);
    }
    assert_eq!(over.state().give_up, 1);
    let mut before = None;
    while over.state().irq_count < 64 {
        before = Some(over.picture());
        step(&mut over, FrameInput::NONE);
        step(&mut on, FrameInput::NONE);
    }
    assert_eq!((over.energy(), on.energy()), (998, 998));
    // The countdown's digits (`stretch-pit`'s old exclusion box, rows 223..231 cols 144..168).
    let digits = |p: &Picture| -> Vec<u16> {
        let rgb = to_rgb(p);
        (223..231)
            .flat_map(|y| (144..168).map(move |x| (y, x)))
            .map(|(y, x)| rgb[y * p.width as usize + x])
            .collect()
    };
    let (shown, redrawn) = (digits(&over.picture()), digits(&on.picture()));
    assert_ne!(shown, redrawn, "998 is drawn when the game goes on");
    assert_eq!(
        shown,
        digits(&before.unwrap()),
        "the bar as pass 63 left it"
    );
}

fn to_rgb(p: &Picture) -> Vec<u16> {
    amiga_chips::to_rgb12(p)
}

/// fd-core's game, watched after every pass: the lives, the place and `char_dying`.
struct Watched {
    game: Game,
    seen: Vec<(u16, u32, u32, u16)>,
}

impl Transplant for Watched {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.game.transplant(mem)
    }
    fn probe(&self, v: &Var) -> u32 {
        self.game.probe(v)
    }
}

impl Engine for Watched {
    fn freeze(&mut self) {
        Engine::freeze(&mut self.game);
    }
    fn thaw(&mut self, offset: u32) {
        Engine::thaw(&mut self.game, offset);
    }
    fn pass(&mut self, input: FrameInput) {
        self.game.step(input);
        let s = self.game.state();
        self.seen
            .push((s.lives, s.char_x, s.char_y, s.motion.dying));
    }
    fn picture(&self) -> Option<Picture> {
        Some(self.game.picture())
    }
    fn events(&mut self) -> Vec<Event> {
        self.game.take_events()
    }
    fn off_play(&mut self, input: FrameInput) {
        self.game.step(input)
    }
    fn game_over(&self) -> bool {
        matches!(self.game.scene(), Scene::GameOver(_))
    }
}

/// The slide drops into the spike pit; three passes of contact cost a life at frame 6524, then the
/// hurt frame, the hold and the blink, and the next life about 135 passes later: every pass's
/// variables (CLXDAT as `char_hit_check` copied it among them) and every frame's picture equal the
/// original's. A life lost moves nobody: the original has no respawn, and the engine's character
/// stays where it was hurt through the hold.
#[test]
fn falling_into_the_spike_pit_loses_a_life_as_the_originals() {
    let case = load("death-pit");
    let mut w = Watched {
        game: game(),
        seen: Vec::new(),
    };
    let v = run(&case, &mut w);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert!(v.events.is_empty(), "{v}");
    // 6661 with creatures frozen; live since T46, a pass ends a frame later.
    assert_eq!(v.goal_met, Some(6662), "the second life lost");
    assert!(v.passes > 320 && v.pictures > 320, "{v}");
    assert_eq!(v.tolerated, [0, 0, 0], "{v}");
    assert_eq!(hits(&v, "lives_minus_one"), 2, "{v}");
    assert!(hits(&v, "char_hit_check") > 320, "{v}");
    let lost: Vec<usize> = (1..w.seen.len())
        .filter(|&k| w.seen[k].0 < w.seen[k - 1].0)
        .collect();
    assert_eq!(lost.len(), 2, "{:?}", w.seen);
    for k in lost {
        let (_, x, y, dying) = w.seen[k];
        assert_eq!(dying, 1);
        let held = w.seen[k..].iter().take_while(|s| s.3 != 0);
        assert!(held.clone().count() > 20, "{:?}", &w.seen[k..]);
        for &(_, hx, hy, _) in held {
            assert_eq!((hx, hy), (x, y), "moved while dying");
        }
    }
}

/// Right held in the spike pit until all 32 lives are gone: each lost as the original loses it, then
/// `main_loop`'s check shows the game over screen, compared frame by frame, and fire ends it where the
/// original enters its high scores and the engine restarts (T23's runner end). Where pass `32k + 1`
/// cut `main_loop`'s redraw short on the original, the bar shows the engine's cut redraw (T71).
#[test]
fn the_last_life_lost_on_the_spikes_ends_the_game_as_the_originals() {
    let case = load("death-spikes");
    let v = run(&case, &mut game());
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    // 10716 with creatures frozen; live since T46, the last life goes a frame later.
    assert_eq!(v.game_over, Some((10717, Some(10774))), "{v}");
    let events: Vec<Event> = v.events.iter().map(|(_, e)| *e).collect();
    assert_eq!(events, [Event::GameOver, Event::Restart], "{v}");
    assert_eq!(hits(&v, "lives_minus_one"), 32, "{v}");
    assert!(v.passes > 4300, "{v}");
    assert!(v.cut_redraws > 0, "{v}");
}

/// fd-core's game with its cut redraw (T71) changed one way.
struct Cut {
    game: Game,
    how: CutHow,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CutHow {
    /// As fd-core draws it.
    Same,
    /// None: only the engine's own redraw.
    Without,
    /// One pixel of the bar's lives digits changed: neither the old nor the new bar.
    WrongBar,
    /// One pixel of the play area changed: a cut redraw may change the bar only.
    PlayArea,
    /// The engine's own picture wrong by one play-area pixel while pass 1441's cut is accepted, the
    /// cut redraw right: a cut redraw must not excuse a play area the engine drew wrong (T71 review).
    HidesPlayArea,
}

impl Transplant for Cut {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.game.transplant(mem)
    }
    fn probe(&self, v: &Var) -> u32 {
        self.game.probe(v)
    }
}

impl Engine for Cut {
    fn freeze(&mut self) {
        Engine::freeze(&mut self.game);
    }
    fn thaw(&mut self, offset: u32) {
        Engine::thaw(&mut self.game, offset);
    }
    fn pass(&mut self, input: FrameInput) {
        self.game.step(input)
    }
    fn picture(&self) -> Option<Picture> {
        let mut p = self.game.picture();
        let irq = fd_core::layout_var("irq_count").unwrap();
        if self.how == CutHow::HidesPlayArea && (1442..1472).contains(&self.game.probe(irq)) {
            let w = p.width as usize;
            p.pixels[10 * w + 10] ^= 1;
        }
        Some(p)
    }
    fn events(&mut self) -> Vec<Event> {
        self.game.take_events()
    }
    fn off_play(&mut self, input: FrameInput) {
        self.game.step(input)
    }
    fn game_over(&self) -> bool {
        matches!(self.game.scene(), Scene::GameOver(_))
    }
    fn cut_redraw(&self, cut: u8) -> Option<Picture> {
        let mut p = self.game.cut_redraw(cut)?;
        let w = p.width as usize;
        let at = match self.how {
            CutHow::Same | CutHow::HidesPlayArea => return Some(p),
            CutHow::Without => return None,
            CutHow::WrongBar => 228 * w + 90,
            CutHow::PlayArea => 10 * w + 10,
        };
        p.pixels[at] ^= 1;
        Some(p)
    }
}

/// `death-spikes` up to frame 7800: pass 1441 takes a life and cuts `main_loop`'s redraw short
/// before its lives read (measured, T71), so the original's bar shows the life gone 31 frames before
/// the engine's own redraw. The runner accepts the engine's cut redraw there, and nothing else: no
/// cut redraw, a bar matching neither the old nor the new numbers, or a play area changed fails, and
/// a cut redraw never excuses the engine's own play area.
#[test]
fn a_redraw_cut_short_is_accepted_on_the_bar_only_as_the_engine_draws_it() {
    let mut case = load("death-spikes");
    case.frames = 1400;
    case.goal = None;
    // Five runs of about 4 s, side by side.
    let runs: Vec<(CutHow, Verdict)> = std::thread::scope(|s| {
        let case = &case;
        let all = [
            CutHow::Same,
            CutHow::Without,
            CutHow::WrongBar,
            CutHow::PlayArea,
            CutHow::HidesPlayArea,
        ]
        .map(|how| s.spawn(move || (how, run(case, &mut Cut { game: game(), how }))));
        all.map(|h| h.join().unwrap()).into()
    });
    for (how, v) in runs {
        if how == CutHow::Same {
            assert_eq!(v.outcome, Outcome::Same, "{v}");
            assert!(v.cut_redraws >= 31, "{v}");
            continue;
        }
        let Outcome::PictureDiffers { frame, why, .. } = &v.outcome else {
            panic!("{how:?}: {v}");
        };
        assert!((7742..7775).contains(frame), "{how:?}: {v}");
        // "first at (x, y)": the bar's rows, or the play area's changed pixel.
        let y: u16 = why
            .split("first at (")
            .nth(1)
            .and_then(|r| r.split([',', ')']).nth(1))
            .and_then(|y| y.trim().parse().ok())
            .unwrap_or_else(|| panic!("{how:?}: {why}"));
        if how == CutHow::HidesPlayArea {
            assert_eq!(y, 10, "{how:?}: {why}");
            continue;
        }
        assert!(fd_conform::STATUS_ROWS.contains(&y), "{how:?}: {why}");
    }
}

/// fd-core's game as [`Watched`], thawing what the case thaws (T72).
struct Thawed(Watched);

impl Transplant for Thawed {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.0.transplant(mem)
    }
    fn probe(&self, v: &Var) -> u32 {
        self.0.probe(v)
    }
}

impl Engine for Thawed {
    fn freeze(&mut self) {
        self.0.freeze();
    }
    fn thaw(&mut self, offset: u32) {
        Engine::thaw(&mut self.0.game, offset);
    }
    fn pass(&mut self, input: FrameInput) {
        self.0.pass(input)
    }
    fn picture(&self) -> Option<Picture> {
        self.0.picture()
    }
    fn events(&mut self) -> Vec<Event> {
        self.0.events()
    }
}

/// The character drops into the pool at `W1-1:148,66` to `149,66`; since T47 no case freezes `h0_6882`,
/// so both sides draw the water (T25's `h0_62b6`): three contact passes in a row cost a life at frame 7062 and the next one
/// after the hold at 7204, every pass's variables and every frame's picture equal to the original's.
/// The character stays where the water hurt it.
#[test]
fn falling_into_the_water_loses_a_life_as_the_originals() {
    let case = load("death-water");
    assert!(case.freeze && case.thaw.is_empty());
    let mut w = Thawed(Watched {
        game: game(),
        seen: Vec::new(),
    });
    let v = run(&case, &mut w);
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert!(v.events.is_empty(), "{v}");
    assert_eq!(v.goal_met, Some(7204), "the second life lost");
    assert!(v.passes > 860 && v.pictures > 900, "{v}");
    assert_eq!(hits(&v, "lives_minus_one"), 2, "{v}");
    // The original adds the cells in view, the water among them, to the animated cells.
    assert!(hits(&v, "h0_6882") > 0, "{v}");
    let seen = &w.0.seen;
    let lost: Vec<usize> = (1..seen.len())
        .filter(|&k| seen[k].0 < seen[k - 1].0)
        .collect();
    assert_eq!(lost.len(), 2, "{seen:?}");
    for k in lost {
        let (_, x, _, dying) = seen[k];
        assert_eq!(dying, 1);
        assert!((148..=149).contains(&(x / 16)), "{x}: not over the pool");
    }
}
