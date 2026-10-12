//! One frame of a level, in the original's order (docs/08 "Main loop and interrupt"): the
//! vertical-blank interrupt (`vbl_handler`, h0+$11bf8), the copper interrupt the scroller asks for
//! (the same handler's other half), and, every second frame, the main loop's 25 Hz pass
//! (`level_frame_loop`, h0+$aa6).

use crate::frog;
use crate::layout::h0;
use crate::level::{LevelCtx, camera, clock, collision, fade, pass, scroll, spawn};

/// `logic_running` (h0+$9d6), `vbl_count` (h0+$9d8), `vbl_pass_flag` (h0+$9da).
const LOGIC_RUNNING: u32 = h0(0x9d6);
const VBL_COUNT: u32 = h0(0x9d8);
const PASS_FLAG: u32 = h0(0x9da);
/// `display_timer` (h0+$123d4).
const DISPLAY_TIMER: u32 = h0(0x123d4);
/// `screen_fades_off` (h0+$9e0), `screen_fade_toggle` (h0+$3ca0): outside a level the title's
/// screens step the fades only every second frame.
const FADES_OFF: u32 = h0(0x9e0);
const FADE_TOGGLE: u32 = h0(0x3ca0);

/// The vertical blank in a level (h0+$11bf8 with `logic_running` set), then the copper
/// interrupt it requested.
pub fn vbl(c: &mut LevelCtx) {
    let b = &mut c.bus;
    if b.w(LOGIC_RUNNING) != 0 {
        b.set_w(VBL_COUNT, b.w(VBL_COUNT).wrapping_add(1));
        scroll::scroll_latch(c);
        c.bus.set_w(PASS_FLAG, 1);
        c.audio.stream_tick(c.sound, c.paula);
        frog::anim::frog_level_entry(c);
        let b = &c.bus;
        if b.w(camera::SHOOTER_MODE) != 0 || b.w(camera::SPRITES_HIDDEN) == 0 {
            frog::physics::demo_record(c);
            frog::physics::demo_play(c);
            collision::frog_tiles(c);
            frog::physics::frog_air_anim(c);
            frog::anim::frog_anim_tick(c);
            frog::physics::frog_walk_input(c);
            frog::anim::frog_input_anim(c);
            frog::anim::place_frog_sprites(c);
            clock::level_clock_tick(c);
            frog::physics::frog_move(c);
            camera::camera_follow(c);
        }
        c.audio.music_tick(c.sound, c.paula);
        scroll::scroller(c);
        spawn::scan_edges(c);
    } else {
        c.audio.stream_tick(c.sound, c.paula);
    }
    frog::anim::frog_exit_walk(c);
    debug_assert_ne!(c.bus.w(LOGIC_RUNNING), 0, "only a level's vertical blank is ported");
    c.audio.music_dma_tick(c.sound, c.paula);
    c.audio.sfx_tick(c.sound, c.paula);
    // (progress_map_vbl, h0+$3f22: the progress map's, off in a level.)
    clock::kbd_poll(c);
    let b = &mut c.bus;
    let t = b.w(DISPLAY_TIMER);
    if crate::level::n_clear(t) {
        b.set_w(DISPLAY_TIMER, t.wrapping_sub(1));
    }
    if b.w(FADES_OFF) == 0 || b.w(FADE_TOGGLE) != 0 {
        fade::fades_tick(c);
    }
    // (h0+$3b22: the title's, off in a level.)
    c.custom(0x9c, 0x0020);
    // The copper interrupt the scroller requested (INTREQ `$8010`) runs as the handler returns.
    if c.bus.w(LOGIC_RUNNING) != 0 {
        scroll::copper_irq_blits(c);
        c.custom(0x9c, 0x0010);
    }
}

/// Whether the main loop's pass is due: the vertical blank has counted two since the last.
pub fn pass_due(c: &LevelCtx) -> bool {
    let b = &c.bus;
    b.w(LOGIC_RUNNING) != 0 && crate::level::n_clear(b.w(VBL_COUNT).wrapping_sub(2))
}

/// The 25 Hz pass (`level_frame_loop`'s body, h0+$ab8).
pub fn pass(c: &mut LevelCtx) {
    c.bus.set_w(VBL_COUNT, 0);
    crate::level::objects::spud_update(c);
    crate::level::objects::obj_run_all(c);
    crate::level::objects::obj_build_hitboxes(c);
    crate::level::objects::collide_all(c);
    crate::level::objects::build_bob_draw_list(c);
    crate::level::objects::bob_draw(c);
    pass::buffer_flip(c);
    pass::tile_anim_draw(c);
    crate::level::panel::panel_update(c);
    pass::pause_check(c);
    pass::passage_step(c);
    crate::level::panel::extra_life(c);
    pass::time_up_check(c);
    let b = &mut c.bus;
    let ls = b.l(collision::LEVEL_STATE);
    if crate::level::collision::n_clear_l(ls.wrapping_add(1)) {
        b.set_l(collision::LEVEL_STATE, ls.wrapping_sub(1));
    }
}
