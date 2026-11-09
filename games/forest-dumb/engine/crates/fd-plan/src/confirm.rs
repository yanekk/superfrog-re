//! A planned route played on the original (shell: the oracle). The route runs as a level case from
//! the level sync point with creature starts frozen (`freeze.rs`, T79), as T11's stretches do; items,
//! bonuses and the exit are taken (T22) and springs and animated cells run (T47) on both sides; the
//! case's engine leaves out what it freezes (`fd_conform::engine_for`). The
//! case compares the character's, the camera's and the status variables on every pass, and its goal
//! is where that engine ends.
//!
//! Where the two first differ is where the modelled part of the route ends: contact with the
//! playfield and the lives it costs are modelled since T70. Creatures stay frozen here on both sides
//! (`freeze.rs`'s T79 entry, the engine starting none either since T46) until T79 runs them live: a
//! redraw the next pass cuts short starts one a pass later in the original, which the case runner
//! hands the engine since T75.

use std::sync::Arc;

use amiga_input::FrameInput;
use fd_conform::{Case, Inputs, Options, Verdict, run_inputs, sync};
use fd_core::{Game, GameData, Start};

/// The variables compared: T11's stretches' (`conform/cases/stretch-*.toml`).
pub const VARS: &str = r#""char_x", "char_y", "camera_x", "camera_y", "char_screen_x", "char_walk_t", "char_facing", "char_probe_1", "char_probe_2", "char_anim_ptr", "joy_left", "joy_right", "joy_up", "joy_down", "input_log_index", "char_jump_3b28", "char_jump_41b4", "char_jump_41b6", "char_dying_t", "char_dying", "char_rising", "char_falling", "char_probe_3", "char_probe_4", "char_probe_5", "char_probe_6", "char_slide_t", "char_jumped", "char_jump_t", "char_run_left", "char_run_right", "char_samples_0", "char_samples_4", "char_samples_8", "char_samples_c", "char_update_t", "char_unhurt_t", "edge_latch_right", "edge_latch_left", "edge_latch_up", "edge_latch_down", "edge_at_right_x", "edge_at_right_y", "edge_at_left_x", "edge_at_left_y", "edge_at_up_x", "edge_at_up_y", "edge_at_down_x", "edge_at_down_y", "lives", "hits_taken", "items_left", "score", "irq_count", "countdown", "countdown_tick""#;

/// Oracle frames run past the route's last step, for the original's last passes to end in.
pub const SLACK: u32 = 8;

/// The case that plays `inputs` (a step each) from the level sync point, then [`SLACK`] frames idle;
/// the goal the engine's `char_x` and `char_y` after the route.
pub fn case(data: &Arc<GameData>, inputs: &[FrameInput]) -> Result<Case, String> {
    case_excluding(data, inputs, &[])
}

/// [`case`], with each `(variable, task)` in `exclude` left out of the compared variables and
/// excluded, named with the task that removes it.
pub fn case_excluding(
    data: &Arc<GameData>,
    inputs: &[FrameInput],
    exclude: &[(&str, &str)],
) -> Result<Case, String> {
    // The original ends its passes about one behind the frames run (measured, T20 review: 599 of a
    // 600-step route by its 600th frame), so a case of exactly the route's frames can miss the goal
    // on the last pass. The slack plays idle past the route ([`crate::script`] lets go).
    let frames = inputs.len() as u32 + SLACK;
    let vars = VARS
        .split(", ")
        .filter(|v| !exclude.iter().any(|(e, _)| v.trim_matches('"') == *e))
        .collect::<Vec<_>>()
        .join(", ");
    let excluded = if exclude.is_empty() {
        String::new()
    } else {
        let list: Vec<String> = exclude
            .iter()
            .map(|(v, task)| format!("{{ what = \"var {v}\", task = \"{task}\" }}"))
            .collect();
        format!("exclude = [{}]\n", list.join(", "))
    };
    let text = |goal: &str| {
        format!(
            "sync = {{ script = \"{script}\", frame = {sync} }}\n\
             frames = {frames}\n\
             compare = [\"vars\"]\n\
             vars = [{vars}]\n\
             goal = {goal}\n\
             freeze = true\n\
             {excluded}",
            script = sync::LEVEL_1_SCRIPT,
            sync = sync::LEVEL_1_SYNC,
        )
    };
    // The goal is where the case's own engine ends, frozen as the case freezes the original, so it
    // is played on `engine_for` the case without its goal first.
    let ahead = Case::parse("planned-route", &text("{ idle = true }"))?;
    let mut g = fd_conform::engine_for(&ahead, data.clone());
    for &i in inputs {
        g.step(i);
        g.take_events();
    }
    let (x, y) = (g.state().char_x, g.state().char_y);
    let by = sync::LEVEL_1_SYNC + frames;
    Case::parse(
        "planned-route",
        &text(&format!(
            "{{ all = [{{ var = \"char_x\", value = {x} }}, {{ var = \"char_y\", value = {y} }}], by_frame = {by} }}"
        )),
    )
}

/// `inputs` played on the original and on the engine, compared pass by pass.
pub fn confirm(data: &Arc<GameData>, inputs: &[FrameInput]) -> Result<Verdict, String> {
    confirm_excluding(data, inputs, &[])
}

/// [`confirm`] with [`case_excluding`]'s exclusions.
pub fn confirm_excluding(
    data: &Arc<GameData>,
    inputs: &[FrameInput],
    exclude: &[(&str, &str)],
) -> Result<Verdict, String> {
    let case = case_excluding(data, inputs, exclude)?;
    let after = crate::script(inputs);
    let inputs = Inputs::with_after(&case, after)?;
    let mut game = Game::new(data.clone(), Start::Level1);
    run_inputs(&case, &inputs, &mut game, &Options::default())
}
