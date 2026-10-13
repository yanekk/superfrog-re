//! One frame of a level, in the original's order (docs/08 "Main loop and interrupt"): the
//! vertical-blank interrupt (`vbl_handler`, h0+$11bf8), the copper interrupt the scroller asks for
//! (the same handler's other half), and, every second frame, the main loop's 25 Hz pass
//! (`level_frame_loop`, h0+$aa6).

use crate::frog;
use crate::layout::h0;
use crate::level::beam::{self, Cost};
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
    c.spend(Cost::VblEntry);
    let b = &mut c.bus;
    if b.w(LOGIC_RUNNING) != 0 {
        b.set_w(VBL_COUNT, b.w(VBL_COUNT).wrapping_add(1));
        scroll::scroll_latch(c);
        c.bus.set_w(PASS_FLAG, 1);
        let at = c.paula.len();
        c.bus.st.audio.stream_tick(c.sound, c.paula);
        c.audio_spent(at, Cost::AudioStop, Cost::AudioWrite);
        c.spend(Cost::FrogEntry);
        frog::anim::frog_level_entry(c);
        let b = &c.bus;
        if b.w(camera::SHOOTER_MODE) != 0 || b.w(camera::SPRITES_HIDDEN) == 0 {
            c.spend(Cost::Demo);
            frog::physics::demo_record(c);
            frog::physics::demo_play(c);
            c.spend(Cost::FrogTiles);
            collision::frog_tiles(c);
            c.spend(Cost::FrogAir);
            frog::physics::frog_air_anim(c);
            c.spend(Cost::FrogAnim);
            frog::anim::frog_anim_tick(c);
            c.spend(Cost::FrogWalk);
            frog::physics::frog_walk_input(c);
            c.spend(Cost::FrogInput);
            frog::anim::frog_input_anim(c);
            c.spend(Cost::FrogSprites);
            frog::anim::place_frog_sprites(c);
            c.spend(Cost::ClockTick);
            clock::level_clock_tick(c);
            c.spend(Cost::FrogMove);
            frog::physics::frog_move(c);
            c.spend(Cost::Camera);
            camera::camera_follow(c);
        }
        let at = c.paula.len();
        c.spend(Cost::Music);
        c.bus.st.audio.music_tick(c.sound, c.paula);
        c.audio_spent(at, Cost::AudioStop, Cost::AudioWrite);
        scroll::scroller(c);
        spawn::scan_edges(c);
    } else {
        c.bus.st.audio.stream_tick(c.sound, c.paula);
    }
    frog::anim::frog_exit_walk(c);
    debug_assert_ne!(c.bus.w(LOGIC_RUNNING), 0, "only a level's vertical blank is ported");
    let at = c.paula.len();
    c.bus.st.audio.music_dma_tick(c.sound, c.paula);
    c.bus.st.audio.sfx_tick(c.sound, c.paula);
    c.audio_spent(at, Cost::AudioStop, Cost::AudioWrite);
    c.spend(Cost::VblTail);
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
    // The copper interrupt the scroller requested (INTREQ `$8010`) runs as the handler returns,
    // or, when the pass's blit has it masked, once the pass unmasks it.
    if c.bus.w(LOGIC_RUNNING) != 0 {
        c.request_copper();
    }
}

/// The copper interrupt (`vbl_handler`'s other half, h0+$11d1a): the scroller's queued blits.
pub fn copper_irq(c: &mut LevelCtx) {
    // An interrupt: nothing in it is the main program's.
    let outer = c.frames.as_mut().map(|f| std::mem::replace(&mut f.in_vbl, true));
    scroll::copper_irq_blits(c);
    c.custom(0x9c, 0x0010);
    if let (Some(f), Some(was)) = (c.frames.as_mut(), outer) {
        f.in_vbl = was;
    }
}

/// The level's main program for one step on the beam (DESIGN §2.1; T36): `level_frame_loop`
/// waiting for its pass, the vertical blank coming at the frame's start, the frame ending where
/// `pacing` says ([`crate::Pacing::Late`]: in the middle of a pass, which is then pending into the
/// next step) or once the main loop waits again. With a pass `pending`, the pass runs again from
/// its start, the frames it ran through ending where they did, each next one's vertical blank
/// coming there, then the step's own. The frame's writes to the display's memory are logged for
/// its picture.
pub fn main_program(c: &mut LevelCtx, pending: Option<beam::Pending>, pacing: crate::Pacing) {
    let own = beam::FrameSpec {
        hw: c.hw,
        cut: match pacing {
            crate::Pacing::OnTime => None,
            crate::Pacing::Late {
                writes,
                line,
                hpos,
                phase,
            } => Some(beam::Cut {
                writes,
                line,
                hpos,
                phase,
            }),
        },
    };
    let mut f = beam::Frames::default();
    let resume = pending.map(|p| {
        f.specs = p.frames.clone();
        *c.bus.st = (*p.start).clone();
        f.at = Some(0);
        f.pre = p.pre;
        f.pass_start = Some((p.start, p.clock, p.hw, p.pre));
        (p.clock, p.hw)
    });
    f.specs.push(own);
    c.frames = Some(Box::new(f));
    match resume {
        Some((clock, hw)) => {
            c.clock = clock;
            c.bus.line = clock.line;
            c.hw = hw;
            let f = c.frames.as_mut().expect("frames");
            f.from = c.paula.len();
            f.pass_frames = vec![f.specs[0]];
            run_pass(c, true);
        }
        None => {
            c.clock = beam::Clock {
                line: beam::FRAME_LINES,
                into: 0,
            };
            c.frame_end();
        }
    }
    while !c.frames.as_ref().is_some_and(|f| f.done) {
        if pass_due(c) {
            run_pass(c, false);
        } else {
            // Waiting for the next vertical blank.
            if c.clock.line < beam::FRAME_LINES {
                c.clock = beam::Clock {
                    line: beam::FRAME_LINES,
                    into: 0,
                };
            }
            c.frames.as_mut().expect("frames").past_end = false;
            c.frame_end();
        }
    }
}

/// One pass of the main loop, interruptible at its frame's end ([`LevelCtx::frame_end`]).
fn run_pass(c: &mut LevelCtx, resumed: bool) {
    let writes = c.frame_writes_now();
    let f = c.frames.as_mut().expect("frames");
    if !resumed {
        let spec = f.specs[f.at.expect("a frame")];
        f.pass_frames = vec![spec];
        // A pass its frame may end in the middle of is kept as it begins.
        let may_cut = f.specs[f.at.expect("a frame")..].iter().any(|s| s.cut.is_some());
        f.pass_start = may_cut.then(|| (Box::new(c.bus.st.clone()), c.clock, c.hw, writes));
    }
    f.in_pass = true;
    pass(c);
    let f = c.frames.as_mut().expect("frames");
    f.in_pass = false;
}

/// Whether the main loop's pass is due: the vertical blank has counted two since the last.
pub fn pass_due(c: &LevelCtx) -> bool {
    let b = &c.bus;
    b.w(LOGIC_RUNNING) != 0 && crate::level::n_clear(b.w(VBL_COUNT).wrapping_sub(2))
}

/// The 25 Hz pass (`level_frame_loop`'s body, h0+$ab8).
pub fn pass(c: &mut LevelCtx) {
    c.phase = 0;
    c.spend(Cost::PassEntry);
    c.bus.set_w(VBL_COUNT, 0);
    c.spend(Cost::Spud);
    crate::items::powerups::spud_update(c);
    c.phase = 1;
    crate::level::objects::obj_run_all(c);
    c.phase = 2;
    crate::level::objects::obj_build_hitboxes(c);
    c.phase = 3;
    crate::level::objects::collide_all(c);
    c.phase = 4;
    crate::level::objects::build_bob_draw_list(c);
    c.phase = 5;
    crate::level::objects::bob_draw(c);
    c.phase = 6;
    c.spend(Cost::AfterDraw);
    pass::buffer_flip(c);
    c.phase = 7;
    pass::tile_anim_draw(c);
    c.phase = 8;
    c.spend(Cost::Panel);
    crate::level::panel::panel_update(c);
    c.phase = 9;
    c.spend(Cost::PassTail);
    pass::pause_check(c);
    c.phase = 10;
    pass::passage_step(c);
    c.phase = 11;
    crate::level::panel::extra_life(c);
    c.phase = 12;
    crate::level::death::time_up_check(c);
    c.phase = 13;
    let b = &mut c.bus;
    let ls = b.l(collision::LEVEL_STATE);
    if crate::level::collision::n_clear_l(ls.wrapping_add(1)) {
        b.set_l(collision::LEVEL_STATE, ls.wrapping_sub(1));
    }
}
