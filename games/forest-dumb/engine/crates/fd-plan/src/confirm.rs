//! A planned route played on the original (shell: the oracle). The route runs as a level case from
//! the level sync point with what the engine does not model frozen (T21's `freeze.rs`: creatures,
//! animated cells, springs), as T11's stretches do; items, bonuses and the exit are taken on both
//! sides (T22). The case compares the character's, the camera's and the status variables on every
//! pass, and its goal is where the engine ends.
//!
//! Where the two first differ is where the modelled part of the route ends: a contact pass
//! (`hits_taken`, T70, T24) is the original's and not yet the engine's.

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
    let mut g = Game::new(data.clone(), Start::Level1);
    for &i in inputs {
        g.step(i);
        g.take_events();
    }
    let (x, y) = (g.state().char_x, g.state().char_y);
    // The original ends its passes about one behind the frames run (measured, T20 review: 599 of a
    // 600-step route by its 600th frame), so a case of exactly the route's frames can miss the goal
    // on the last pass. The slack plays idle past the route ([`crate::script`] lets go).
    let frames = inputs.len() as u32 + SLACK;
    let text = format!(
        "sync = {{ script = \"{script}\", frame = {sync} }}\n\
         frames = {frames}\n\
         compare = [\"vars\"]\n\
         vars = [{VARS}]\n\
         goal = {{ all = [{{ var = \"char_x\", value = {x} }}, {{ var = \"char_y\", value = {y} }}], by_frame = {by} }}\n\
         freeze = true\n",
        script = sync::LEVEL_1_SCRIPT,
        sync = sync::LEVEL_1_SYNC,
        by = sync::LEVEL_1_SYNC + frames,
    );
    Case::parse("planned-route", &text)
}

/// `inputs` played on the original and on the engine, compared pass by pass.
pub fn confirm(data: &Arc<GameData>, inputs: &[FrameInput]) -> Result<Verdict, String> {
    let case = case(data, inputs)?;
    let after = crate::script(inputs);
    let inputs = Inputs::with_after(&case, after)?;
    let mut game = Game::new(data.clone(), Start::Level1);
    run_inputs(&case, &inputs, &mut game, &Options::default())
}
