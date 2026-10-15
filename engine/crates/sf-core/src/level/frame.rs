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
pub(crate) const VBL_COUNT: u32 = h0(0x9d8);
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
            c.spend(Cost::FrogAnimTail);
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
    if c.bus.w(LOGIC_RUNNING) == 0 {
        // Paused (T191): h0+$11aa6, the mouse pointer and its sprite; the music; and h0+$12a14,
        // a wait until eight lines on from the one it reads.
        crate::front::vbl::pointer(c.bus.st);
        let at = c.paula.len();
        c.spend(Cost::Music);
        c.bus.st.audio.music_tick(c.sound, c.paula);
        c.audio_spent(at, Cost::AudioStop, Cost::AudioWrite);
        c.wait_lines(8);
    }
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
            crate::Pacing::Unpaced | crate::Pacing::OnTime => None,
            crate::Pacing::Late {
                writes,
                line,
                hpos,
                phase,
                step,
                blanks,
            } => Some(beam::Cut {
                writes,
                line,
                hpos,
                phase,
                step,
                blanks,
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
            c.bus.at = clock.nominal();
            c.hw = hw;
            let f = c.frames.as_mut().expect("frames");
            f.from = c.paula.len();
            f.pass_frames = vec![f.specs[0]];
            run_pass(c, true);
            after_pass(c);
        }
        None => {
            c.clock = beam::Clock {
                line: beam::FRAME_LINES,
                into: 0,
            };
            c.frame_end();
        }
    }
    // A pass has just returned in this step, with no vertical blank since.
    let mut ran = resume.is_some();
    while !c.frames.as_ref().is_some_and(|f| f.done) {
        if c.bus.st.pause.is_some() && unpause(c) {
            ran = true;
        } else if pass_due(c) && !(ran && waits_out(c, pacing)) {
            run_pass(c, false);
            after_pass(c);
            ran = true;
        } else {
            ran = false;
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

/// A pass due straight after another (the last ran past two vertical blanks) waits for the next
/// frame: the step's own frame is on time, so the original ended it in `level_frame_loop`'s wait,
/// and has made every write of its frame already, so it never began the pass (it ended the frame
/// on the wait's last check, h0+$aae).
fn waits_out(c: &LevelCtx, pacing: crate::Pacing) -> bool {
    pacing == crate::Pacing::OnTime
        && c.frames.as_ref().is_some_and(|f| matches!(f.current(), Some((true, None))))
        && !c.schedule.is_empty()
        && c.frame_writes_now() as usize >= c.schedule.len()
}

/// After a pass: when `level_frame_loop` has returned, the rest of the frame is `top_loop`'s
/// (T37); in the bonus level after a death, the frog back on the screen and the loop again (T79).
fn after_pass(c: &mut LevelCtx) {
    if c.bus.w(LOGIC_RUNNING) != 0 || c.bus.st.pause.is_some() {
        return;
    }
    if crate::bonus::frame::goes_on(&c.bus) {
        crate::bonus::frame::back_on_screen(c);
    } else {
        c.loop_over();
    }
}

/// A paused pass (`pause`, h0+$7442): on to the pause's end, then the rest of the pass
/// (T191). False while the pause goes on to the frame's end.
fn unpause(c: &mut LevelCtx) -> bool {
    if !pass::pause_run(c) {
        return false;
    }
    let f = c.frames.as_mut().expect("frames");
    f.in_pass = true;
    pass_tail(c);
    c.frames.as_mut().expect("frames").in_pass = false;
    after_pass(c);
    true
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

/// `player_died` (h0+$82c), `abort_flag` (h0+$9de), `game_full_mode` (h0+$6), `key_down`
/// (h0+$16078).
const PLAYER_DIED: u32 = h0(0x82c);
const ABORT_FLAG: u32 = h0(0x9de);
const GAME_FULL_MODE: u32 = h0(0x6);
const KEY_DOWN: u32 = h0(0x16078);

/// The 25 Hz pass (`level_frame_loop`'s body, h0+$ab8). The loop ends after it (`logic_running`
/// cleared, h0+$b5e) when `level_state` has counted down to 0 (the exit), the frog's death has
/// played out (`player_died`), or ESC is down (`abort_flag`; in the attract demo, fire too).
pub fn pass(c: &mut LevelCtx) {
    c.enter_phase(0);
    c.spend(Cost::PassEntry);
    c.bus.set_w(VBL_COUNT, 0);
    c.spend(Cost::Spud);
    crate::items::powerups::spud_update(c);
    c.enter_phase(1);
    crate::level::objects::obj_run_all(c);
    c.enter_phase(2);
    crate::level::objects::obj_build_hitboxes(c);
    c.enter_phase(3);
    crate::level::objects::collide_all(c);
    c.enter_phase(4);
    crate::level::objects::build_bob_draw_list(c);
    c.enter_phase(5);
    crate::level::objects::bob_draw(c);
    c.enter_phase(6);
    c.spend(Cost::AfterDraw);
    pass::buffer_flip(c);
    c.enter_phase(7);
    pass::tile_anim_draw(c);
    c.enter_phase(8);
    c.spend(Cost::Panel);
    crate::level::panel::panel_update(c);
    c.enter_phase(9);
    c.spend(Cost::PassTail);
    if pass::pause_check(c) {
        // Paused: the rest of the pass waits for its end ([`unpause`]).
        return;
    }
    pass_tail(c);
}

/// The pass after the pause check (`level_frame_loop`'s calls from h0+$af6): the secret passage,
/// the extra life, the time, then whether the loop is over.
fn pass_tail(c: &mut LevelCtx) {
    c.enter_phase(10);
    pass::passage_step(c);
    c.enter_phase(11);
    crate::level::panel::extra_life(c);
    c.enter_phase(12);
    crate::level::death::time_up_check(c);
    c.enter_phase(13);
    let b = &mut c.bus;
    let ls = b.l(collision::LEVEL_STATE);
    if crate::level::collision::n_clear_l(ls.wrapping_add(1)) {
        b.set_l(collision::LEVEL_STATE, ls.wrapping_sub(1));
    }
    let over = b.l(collision::LEVEL_STATE) == 0 || b.w(PLAYER_DIED) != 0 || {
        let demo_stop = b.l(crate::level::ATTRACT_MODE) != 0 && c.hw.fire;
        let esc = b.b(KEY_DOWN) == 0x45;
        if demo_stop || esc {
            b.set_w(ABORT_FLAG, 1);
            debug_assert_ne!(b.w(GAME_FULL_MODE), 0, "a game run from a disk loader exits to it");
        }
        demo_stop || esc
    };
    if over {
        b.set_w(LOGIC_RUNNING, 0);
    }
    c.pass_returns();
}
