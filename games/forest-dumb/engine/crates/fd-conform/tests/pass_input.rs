//! Each pass gets the input the original read for it (T67). While the camera scrolls a pass
//! usually runs past its step and `read_joystick` reads JOY1DAT in the next step, so the runner
//! pairs a pass's joystick with the step its read ran in (T31's counter), not the step it started
//! in. `character-run-slide-buttons` changes up twice mid-scroll: it passes on fd-core's `Game` and
//! fails when the engine is fed the old pairing. Keys keep the start step's, as measured here. Needs
//! the disks and the Kickstart: absent, it fails naming where it looked.

use std::path::Path;

use amiga_oracle::{FrameInput, KeySet};
use fd_conform::{
    Case, Engine, Inputs, Options, OracleMem, Outcome, PASS_END, reach_case_counting, run_with,
    sync,
};
use fd_core::{Event, Game, Mem, Start, Transplant, Var, layout_var};

fn load(name: &str) -> Case {
    Case::load(Path::new(&format!("cases/{name}.toml"))).unwrap_or_else(|e| panic!("{e}"))
}

fn game() -> Game {
    Game::new(
        fd_conform::game_data().unwrap_or_else(|e| panic!("{e}")),
        Start::Level1,
    )
}

#[test]
fn up_pressed_and_released_mid_scroll_matches_and_reaches_the_jump() {
    let case = load("character-run-slide-buttons");
    let v = run_with(&case, &mut game(), &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v.outcome, Outcome::Same, "{v}");
    assert!(v.goal_met.is_some(), "{v}");
    assert!(v.passes + v.unseen + 1 >= case.frames - 1, "{v}");
}

/// `Game` fed the runner's pairing before T67: pass `n`'s whole input from the step in which
/// `irq_count` reached `n`. The count is 1 at the level sync point and rises once a step
/// (docs/09-play-measured.md), so that is the step starting at frame `LEVEL_1_SYNC + n - 2`.
struct StartStep {
    game: Game,
    inputs: Inputs,
    next: u32,
}

impl Transplant for StartStep {
    fn transplant(&mut self, mem: &dyn Mem) {
        self.next = mem.value(layout_var("irq_count").unwrap()) + 1;
        self.game.transplant(mem);
    }

    fn probe(&self, v: &Var) -> u32 {
        self.game.probe(v)
    }
}

impl Engine for StartStep {
    fn freeze(&mut self) {
        Engine::freeze(&mut self.game);
    }
    fn pass(&mut self, _read: FrameInput) {
        let input = self.inputs.at(sync::LEVEL_1_SYNC + self.next - 2);
        self.next += 1;
        self.game.pass(input);
    }

    fn picture(&self) -> Option<amiga_chips::Picture> {
        None
    }

    fn events(&mut self) -> Vec<Event> {
        self.game.take_events()
    }
}

#[test]
fn the_old_pairing_fails_where_up_changes_mid_scroll() {
    let case = load("character-run-slide-buttons");
    let mut engine = StartStep {
        game: game(),
        inputs: Inputs::load(&case).unwrap_or_else(|e| panic!("{e}")),
        next: 0,
    };
    let v = run_with(&case, &mut engine, &Options::default()).unwrap_or_else(|e| panic!("{e}"));
    match &v.outcome {
        // Up pressed at the step starting 6480: the old pairing gives it to the pass that started
        // there, which read the joystick a step later; or the release at 6490 likewise.
        Outcome::Differs { frame, .. } => {
            assert!((6480..=6492).contains(frame), "{v}")
        }
        other => panic!("the old pairing passed: {other}\n{v}"),
    }
}

/// The keys a pass gets, measured. P is held for six steps from frame `from`, while standing or
/// sliding; `play_keys` sets `pause_key_down` on seeing it. The first pass whose end shows it set is
/// the pass the runner pairs with P, and the pass before it is paired without P: `last_key_byte`
/// holds the byte before the level-3 interrupt of the step the key goes down in. The pairing also
/// counts every pass through the pause that follows the release (paused passes run no
/// `read_joystick`), with `input_log_index` agreeing.
///
/// Every pass whose end is seen is paired with P exactly when its `pause_key_down` is set. With P
/// held 6455 to 6460 the reads lag a step from 6459 (T67 review): pass 161 starts in 6460 with P and
/// reads the joystick in 6461 without it, and its end shows P down, so keys are the start step's
/// and not the read step's. From 6470 every pass reads in its start step and cannot tell the two.
#[test]
fn keys_reach_the_pass_started_in_their_step_standing_or_sliding_and_a_pause_pairs() {
    const P: u8 = 0x19;
    let slide = load("character-run-slide");
    // The slide's script, which counts from its own sync frame 6400.
    let slide_inputs = Inputs::load(&slide).unwrap_or_else(|e| panic!("{e}"));
    for (what, from) in [("standing", 6340u32), ("sliding", 6455), ("sliding", 6470)] {
        let end = from + 40;
        // A case from the level sync point itself, so the pairing runs over every step.
        let case = Case {
            sync_frame: sync::LEVEL_1_SYNC,
            frames: end - sync::LEVEL_1_SYNC,
            ..slide.clone()
        };
        let at = |f: u32| FrameInput {
            joy: if what == "sliding" {
                slide_inputs.at(f).joy
            } else {
                FrameInput::NONE.joy
            },
            keys: if (from..from + 6).contains(&f) {
                KeySet::key(P)
            } else {
                KeySet::NONE
            },
        };
        let (mut am, mut read) =
            reach_case_counting(&case, 4, &[]).unwrap_or_else(|e| panic!("{e}"));
        am.watch_write(Some(PASS_END));
        let (irq, paused) = (
            layout_var("irq_count").unwrap(),
            layout_var("paused").unwrap(),
        );
        let (mut first_down, mut paused_passes) = (None, 0);
        // (pass, whether its end shows pause_key_down) for every pass end seen.
        let mut seen: Vec<(u32, bool)> = Vec::new();
        // paired[k] is pass k + 2's: pass 1 read the joystick before the level sync point.
        let mut paired: Vec<FrameInput> = Vec::new();
        for f in sync::LEVEL_1_SYNC..end {
            am.step(at(f)).unwrap_or_else(|e| panic!("{e}"));
            read.step(&am, at(f))
                .unwrap_or_else(|e| panic!("{what}: {e}"));
            if let Some(m) = am.watched_memory() {
                let o = OracleMem(&m);
                // pause_key_down, the word after `paused`.
                let key_down = o.read(0, paused.offset + 2, 2) != [0, 0];
                seen.push((o.value(irq), key_down));
                if o.value(paused) != 0 {
                    paused_passes += 1;
                }
                if key_down && first_down.is_none() {
                    first_down = Some(o.value(irq));
                }
            }
            while let Some(i) = read.take(paired.len() as u32 + 2) {
                paired.push(i);
            }
        }
        let n = first_down.unwrap_or_else(|| panic!("{what}: P never seen"));
        let keys = |pass: u32| paired[(pass - 2) as usize].keys;
        assert!(keys(n).contains(P), "{what}: pass {n} is not paired with P");
        assert!(!keys(n - 1).contains(P), "{what}: pass {} is", n - 1);
        for &(pass, down) in seen.iter().filter(|(p, _)| *p >= 2) {
            assert_eq!(
                keys(pass).contains(P),
                down,
                "{what} from {from}: pass {pass} paired with P is not its pause_key_down"
            );
        }
        assert!(paused_passes > 5, "{what}: the release never paused");
        // Every pass up to the last step's was paired, the paused ones included.
        assert!(
            paired.len() as u32 + 3 >= end - sync::LEVEL_1_SYNC,
            "{what}: {}",
            paired.len()
        );
    }
}
