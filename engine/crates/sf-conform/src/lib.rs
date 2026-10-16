//! Shell, test tooling: the conformance runner (DESIGN §2.1, §4).
//!
//! A [`Case`] names a sync point, an input script, a frame count and what to compare. [`run`]
//! brings the original (on the oracle) and the engine to the sync point, feeds both the same input
//! every frame and judges them by the faithful checks (DESIGN §2.1 checks 1–3: state pass by pass,
//! picture within [`Tolerance`], sound as events), or frame-exact with [`Mode::Strict`]; it returns
//! the first failure as a [`Divergence`] naming the first variable, the first pixel (with a diff
//! PNG), the first sound event or Paula write.
//!
//! The CLI (`cargo run -q --release -p sf-conform -- [--strict] run <case.toml> | all | record <case.toml>`)
//! is in `main.rs`; how to write scripts and cases is in `engine/conform/README.md`.

pub mod case;
pub mod compare;
pub mod disks;
pub mod freeze;
pub mod milestones;
pub mod reference;
pub mod report;
pub mod routine;
pub mod script;
pub mod sound_events;
pub mod sync;

pub use case::{Alone, Case, Compare, Excluded, Exclusion, LagLimit, cases_dir, scripts_dir};
pub use compare::{Mode, Tolerance};
pub use milestones::{FrameState, Milestone, MilestoneDiff, Milestones};
pub use report::{Divergence, PictureDiff};
pub use script::InputScript;
pub use sound_events::{SoundEvent, sound_events};
pub use sync::SyncPoint;

use std::collections::VecDeque;
use std::sync::{Arc, OnceLock};

use sf_assets::GameData;
use sf_core::input::FrameInput;
use sf_core::layout::{AmigaMemory, LAYOUT, Ram, VarDef, VarId, def, h0};
use sf_core::{DisplayData, FrameOut, Game, LoadEnd, Pacing, Picture, Start, View};

/// The owner's disks decoded, once per process.
pub fn game_data() -> &'static GameData {
    static DATA: OnceLock<GameData> = OnceLock::new();
    DATA.get_or_init(|| {
        let files: Vec<sf_disk::Offered> = disks::read_disk_files()
            .unwrap_or_else(|e| panic!("{e}"))
            .into_iter()
            .map(|(name, bytes)| sf_disk::Offered { name, bytes })
            .collect();
        let set = sf_disk::identify(&files).unwrap_or_else(|e| panic!("{e:?}"));
        let chunks = sf_assets::unpack(&set).unwrap_or_else(|e| panic!("{e}"));
        sf_assets::decode(&chunks).unwrap_or_else(|e| panic!("{e}"))
    })
}

/// What the display reads from the disks, built once per process.
pub fn display_data() -> Arc<DisplayData> {
    static DISPLAY: OnceLock<Arc<DisplayData>> = OnceLock::new();
    DISPLAY
        .get_or_init(|| Arc::new(DisplayData::new(game_data())))
        .clone()
}

/// The most frames the original may spend on one load the engine did at once.
const LOAD_SEARCH: u32 = 20_000;

/// What a green run compared: the frames, and for a `render` case the frames it left out, each
/// with the reason.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    pub frames: u32,
    /// Frames whose transplanted render was compared against the original's picture.
    pub rendered: u32,
    pub skipped: Vec<(u32, &'static str)>,
    /// Frames the original ended in the middle of a level's pass (lag): not compared, nor the
    /// next frame's picture and Paula writes, which carry the rest of that pass.
    pub straddled: Vec<u32>,
    /// Frames of play on which the original's pass was late at the vertical blank
    /// (`Pacing::Late`), its load frames not counted: the slowdown the engine does not reproduce
    /// in normal play, measured (DESIGN §2.1; per level in docs/08 "Lag", T181).
    pub late: u32,
    /// Frames after such a lag on which the variables still differed, so the engine was re-synced
    /// from the original's memory (DESIGN §2.1), with the first variable that differed. Always
    /// empty for a case with `resync = false`: there each is a divergence.
    pub resynced: Vec<(u32, String)>,
    /// Frames whose picture matched only with the rows of the engine's beam races left out
    /// (`FrameOut::beam_races`, owner 2026-10-13).
    pub raced: Vec<u32>,
    /// Frames that reached the comparison (every frame but the lag's: `straddled` and the frame
    /// after each lag, which only checks the variables for a re-sync).
    pub compared: u32,
    /// The longest run of `straddled` frames in a row: (its first frame, its length); `(0, 0)`
    /// when there was none. Past [`LAG_LIMIT`] (or the case's `lag-limit`) the case fails (T120).
    pub longest_mid_pass: (u32, u32),
    /// The longest run of frames left out for lag, the frame after each lag among them (only
    /// the variables are checked there): reported, not limited.
    pub longest_skip: (u32, u32),
    /// How the case was judged.
    pub mode: Mode,
    /// Faithful (DESIGN §2.1 check 2): frames whose picture difference was tolerated, as a
    /// colour step, as fewer than 64 pixels, and as a glitch of at most 2 frames.
    pub picture_colour_step: Vec<u32>,
    pub picture_few_pixels: Vec<u32>,
    pub picture_glitches: Vec<u32>,
    /// Faithful (check 3): sound events matched a frame or more apart, and events whose match
    /// could only have come after the case's end.
    pub sound_shifts: u32,
    pub sound_cut_off: u32,
    /// Faithful (check 3): period and volume changes left unmatched as their channel stopped
    /// within the tolerance.
    pub sound_trailing: u32,
    /// Faithful (check 1): frames on which one side had ended a pass the other had not, and
    /// how many of those passes' states were compared a frame apart (each side's at its own
    /// pass's end; the others' ends were not seen, the next pass running).
    pub pass_skews: Vec<u32>,
    pub pass_held: u32,
    /// Check 4 (DESIGN §2.1): the original's milestones, each of which the engine run alone met,
    /// when it ran.
    pub milestones: Option<Vec<(u64, Milestone)>>,
}

/// The most frames in a row the original may end mid-pass before a case fails: the original
/// never finishing its pass leaves nothing compared (T120). Fitted to the cases (2026-10-15):
/// every green one is at 97 or under but complete-w4-3 (261) and enemies-w6-1 (133), pinned; the
/// old `items-upper-w2-1` hang was 468.
pub const LAG_LIMIT: u32 = 100;

/// Why a `render` case leaves a frame out.
pub const LATE_REDRAW: &str = "late redraw: the original rewrote the copper list, the panel or \
    the shown buffer after the beam passed it, so the end-of-frame memory is newer than the picture (T14 onward \
    must still match this frame by stepping)";
pub const NOT_A_LEVEL: &str = "not a level screen: the original is between levels (T20/T21)";

/// Run `case`: `Ok` when every compared frame matched. Panics when the oracle cannot be set up
/// (no Kickstart, no disks, a sync point that cannot be reached), with the reason.
pub fn run(case: &Case) -> Result<Outcome, Divergence> {
    run_with(case, |_, _| {})
}

/// [`run`], calling `before_step(frame, game)` before the engine steps each frame: the hook a test
/// uses to plant a divergence. With [`Alone::Only`] the engine is check 4's, run alone.
pub fn run_with(
    case: &Case,
    before_step: impl FnMut(u32, &mut Game),
) -> Result<Outcome, Divergence> {
    run_inner(case, before_step, &mut |_, _, _| {}, None)
}

/// [`run`], calling `after_step(frame, ours, theirs)` with both sides' Paula writes of each frame
/// compared: the hook a test uses to check what the comparison leaves out (a write's beam).
pub fn run_observing(
    case: &Case,
    mut after_step: impl FnMut(u32, &[sf_paula::PaulaWrite], &[sf_paula::PaulaWrite]),
) -> Result<Outcome, Divergence> {
    run_inner(case, |_, _| {}, &mut after_step, None)
}

/// [`run`] with both hooks: `before_step(frame, game)` and `after_step(frame, ours, theirs)`.
pub fn run_hooked(
    case: &Case,
    before_step: impl FnMut(u32, &mut Game),
    mut after_step: impl FnMut(u32, &[sf_paula::PaulaWrite], &[sf_paula::PaulaWrite]),
) -> Result<Outcome, Divergence> {
    run_inner(case, before_step, &mut after_step, None)
}

/// [`run`], going on past each divergence: every frame that differs, in order (for finding what
/// is left in a long case; the engine is not re-synced, so later ones may follow from earlier).
pub fn scan(case: &Case) -> Vec<Divergence> {
    scan_counting(case).0
}

/// [`scan`], with what the run compared.
pub fn scan_counting(case: &Case) -> (Vec<Divergence>, Outcome) {
    let mut found = Vec::new();
    let o = run_inner(case, |_, _| {}, &mut |_, _, _| {}, Some(&mut found))
        .expect("a scan collects its divergences");
    (found, o)
}

fn run_inner(
    case: &Case,
    mut before_step: impl FnMut(u32, &mut Game),
    after_step: &mut dyn FnMut(u32, &[sf_paula::PaulaWrite], &[sf_paula::PaulaWrite]),
    found: Option<&mut Vec<Divergence>>,
) -> Result<Outcome, Divergence> {
    // `run --alone`: checks 1–3 run on, their failures set aside, and check 4 ends the run.
    let mut set_aside = Vec::new();
    let only_alone = case.alone == Alone::Only;
    let mut found = match found {
        None if only_alone => Some(&mut set_aside),
        found => found,
    };
    let frozen = case.freeze();
    // The oracle's frame cap: the case's frames, what its loads take (the whole disk-3 intro's
    // spend more than one search's worth, T56) and one load search more.
    let cap = 2 * case.frames + LOAD_SEARCH;
    let synced = sync::reach_frozen(&case.sync, &case.script, cap, frozen)
        .unwrap_or_else(|e| panic!("{}: {e}", case.name));
    let mut amiga = synced.amiga;
    let mut prev = synced.memory.clone();
    let start = match &case.sync {
        SyncPoint::PowerOn => Start::PowerOn {
            seed: synced.seed.unwrap_or_default(),
        },
        SyncPoint::PowerOnDisk3 => Start::Disk3 {
            seed: synced.seed.unwrap_or_default(),
        },
        SyncPoint::AtFrame { .. } | SyncPoint::EndSequence | SyncPoint::Slot { .. } => {
            Start::Resume {
                memory: synced.memory,
                cpu: sync::core_cpu(&amiga.cpu()),
            }
        }
        _ => Start::Transplant(synced.memory),
    };
    let display = display_data();
    let frog_only = frozen == freeze::Freeze::Objects;
    let mut game = Game::with_display(display.clone(), start.clone());
    if frog_only {
        game.frog_only();
    }
    // Check 4: the engine from the same start, given only the inputs, the original's pacing and
    // the seeds it read from its clock.
    let mut alone = (case.mode == Mode::Faithful && case.alone != Alone::Off).then(|| {
        let mut g = Game::with_display(display.clone(), start);
        if frog_only {
            g.frog_only();
        }
        AloneRun::new(g, case.sync == SyncPoint::PowerOnDisk3)
    });
    // The original's frames loading under a timed screen since the engine's load ([`timed_screen`]).
    let mut under_screen: Vec<(FrameInput, milestones::FrameState)> = Vec::new();
    // COP1LC as the original's CPU last wrote it (from the start's as the engine has it).
    let mut cop = game.cop1lc();
    let mut outcome = Outcome {
        frames: case.frames,
        ..Outcome::default()
    };
    let mut after_lag = false;
    let mut lag = LagRun::new(case.lag_limit.as_ref().map_or(LAG_LIMIT, |l| l.frames));
    // The original's frame, counted from the sync point: ahead of the engine's after a load.
    let mut of = 0u32;
    // Jobs the engine did at once that the original's CPU may still be running.
    // Each with whether the original has been seen in it yet, and frames since reported.
    let mut jobs: Vec<(sf_core::Job, bool, u32)> = Vec::new();
    // Whenever the original asks for a disk it goes in, as a player would (`sync::Disks`).
    let mut disks = Some(sync::Disks::at(&prev));
    // The interrupts the oracle's last step took (one past its end is the next frame's).
    let mut irqs = Vec::new();
    // The original's last handler, taken too late to work in its step (it counts in the next).
    let mut late = false;
    // Faithful (DESIGN §2.1): the picture's tolerated frames, both sides' sound events, and the
    // state matched by pass, with the side that ended a pass first held.
    let faithful = case.mode == Mode::Faithful;
    let tol = Tolerance::OWNER;
    let mut pictures = compare::PictureJudge::new(tol, case.frames);
    let mut sound = sound_events::SoundMatch::new(tol.sound_frames);
    let mut paula_theirs = sound_events::Paula::default();
    let mut paula_ours = sound_events::Paula::default();
    let mut passes = compare::PassMatch::default();
    let mut theirs_clock = sf_core::PassClock::default();
    let mut held: Option<Held> = None;
    for f in 0..case.frames {
        let input = case.script.input_at(of);
        frozen.apply(&mut amiga, &display);
        let resumes = sync::intro_resumes(&amiga);
        let mut theirs = sync::step_inserting(&mut amiga, input)
            .unwrap_or_else(|e| panic!("{}: oracle, frame {of}: {e}", case.name));
        cop = sync::cop1lc(cop, &amiga.cpu_writes());
        let mut entry = sync::handler_entry(&irqs, &amiga.irqs());
        let mut handler_end = sync::handler_end(&theirs.paula, resumes);
        let handlers;
        (handlers, late) = sync::handlers(late, &irqs, &amiga.irqs(), &theirs.paula);
        irqs = amiga.irqs();
        if let Some(d) = disks.as_mut() {
            d.serve(&mut amiga);
        }
        of += 1;
        let mut mem = sync::core_memory(amiga.memory());
        let mut pacing = sync::pacing(&amiga.cpu(), &mem, &theirs.paula);
        // Under `--alone` the engine judged is the one run alone, so a planted divergence goes there.
        match alone.as_mut() {
            Some(a) if only_alone => before_step(f, &mut a.game),
            _ => before_step(f, &mut game),
        }
        // A case comparing only fresh renders does not step the engine (its scenes may not model
        // what the original runs there yet).
        let stepped = case.compare.vars || case.compare.picture || case.compare.paula;
        let mut ours = FrameOut::default();
        if stepped {
            // The original's step ran two video frames (a blit held its CPU through the first's
            // end and the whole second): the engine runs both, compared together (owner, T50).
            // Each engine frame has a handler as long as the original ran one for it; the rest
            // merge into the next blank (T55).
            let mut held = Vec::new();
            if amiga.frames_run() == 2 {
                game.set_blank_merged(Some(handlers == 0));
                held = game.step_paced(input, pacing).paula;
            }
            game.anchor_writes(&theirs.paula.iter().map(|w| (w.vpos, w.hpos)).collect::<Vec<_>>());
            game.anchor_blits(&amiga.blit_starts());
            let before = game.clone();
            game.set_handler_entry(entry);
            game.set_handler_end(handler_end);
            game.set_blank_merged(Some(handlers < amiga.frames_run()));
            ours = game.step_paced(input, pacing);
            ours.paula.splice(0..0, held);
            if let Some(end) = ours.load {
                // The engine loaded at once; the original is compared once its load is over.
                under_screen.clear();
                let mut n = 0;
                let mut seen = false;
                let mut prev_mem = mem.clone();
                loop {
                    let chain = sync::main_chain(&amiga.cpu(), &mem);
                    let over = match end {
                        LoadEnd::Reached(a) => chain.iter().take(4).any(|&x| x == a),
                        LoadEnd::Left(a) => {
                            let inside = chain.contains(&a);
                            seen |= inside;
                            seen && !inside
                        }
                    };
                    if over {
                        break;
                    }
                    n += 1;
                    assert!(
                        n < LOAD_SEARCH,
                        "{}: the original's load ({end:x?}) was not over within {LOAD_SEARCH} frames of the engine's (frame {f})",
                        case.name
                    );
                    frozen.apply(&mut amiga, &display);
                    let resumes = sync::intro_resumes(&amiga);
                    // A load is not modelled frame by frame: what it plays is not compared, but
                    // leaves Paula as it is when the engine's frame comes.
                    paula_theirs.frame(&theirs.paula);
                    theirs = sync::step_inserting(&mut amiga, case.script.input_at(of))
                        .unwrap_or_else(|e| panic!("{}: oracle, frame {of}: {e}", case.name));
                    cop = sync::cop1lc(cop, &amiga.cpu_writes());
                    entry = sync::handler_entry(&irqs, &amiga.irqs());
                    handler_end = sync::handler_end(&theirs.paula, resumes);
                    irqs = amiga.irqs();
                    if let Some(d) = disks.as_mut() {
                        d.serve(&mut amiga);
                    }
                    of += 1;
                    prev_mem = std::mem::replace(&mut mem, sync::core_memory(amiga.memory()));
                    // A frame the original loads under a screen it times itself counts as play
                    // for check 4 (owner, 2026-10-15).
                    if alone.is_some() && cop.is_some_and(timed_screen) {
                        under_screen.push((
                            case.script.input_at(of - 1),
                            milestones::FrameState::read(
                                &|a, n| (a..a + n).map(|x| mem.byte(x)).collect(),
                                cop,
                            ),
                        ));
                    }
                }
                // The schedule as the original is once its load is over.
                pacing = sync::pacing(&amiga.cpu(), &mem, &theirs.paula);
                paula_ours = paula_theirs.clone();
                // The engine's frame again, with what runs on by itself while the original loads
                // (the keyboard's poll, the music) as the original had it just before this frame,
                // then what the load left that the engine does not model, as it is after it.
                let take = |g: &mut Game, from: &AmigaMemory, runs: &[(u32, u32)]| {
                    for &(addr, len) in runs {
                        let bytes: Vec<u8> = (addr..addr + len).map(|a| from.byte(a)).collect();
                        g.poke(addr, &bytes);
                    }
                };
                let redo = |line: Option<u16>| {
                    let mut g = before.clone();
                    if before.story().is_some() {
                        // The intro puts them in place at its load, after what came before it.
                        let runs = before
                            .runs_during_load()
                            .iter()
                            .map(|&(a, n)| (a, (a..a + n).map(|x| prev_mem.byte(x)).collect()))
                            .collect();
                        g.set_load_state(runs);
                        // Or the intro hands over, and the game's own load is the one that ended.
                        let runs = sf_core::front::RUNS_DURING_LOAD
                            .iter()
                            .map(|&(a, n)| (a, (a..a + n).map(|x| prev_mem.byte(x)).collect()))
                            .collect();
                        g.set_handover_state(runs);
                    } else {
                        let runs = before
                            .runs_during_load()
                            .iter()
                            .map(|&(a, n)| (a, (a..a + n).map(|x| prev_mem.byte(x)).collect()))
                            .collect();
                        g.set_load_state(runs);
                    }
                    g.set_load_line(line);
                    g.set_handler_entry(entry);
                    g.set_handler_end(handler_end);
                    let out = g.step_paced(input, pacing);
                    let leaves = g.load_leaves();
                    take(&mut g, &mem, leaves);
                    take(&mut g, &mem, &out.resync);
                    (g, out)
                };
                (game, ours) = redo(None);
                // The line the original's load ended on, which decides where the work after it
                // falls (DESIGN §2.1): the first one on which the engine's frame matches the
                // original's.
                {
                    // Each try's own jobs are masked: a fade set up only when the load ends
                    // earlier is one only that try has.
                    let fits = |g: &Game, out: &FrameOut| {
                        let masked: Vec<(u32, u32)> = jobs
                            .iter()
                            .flat_map(|(j, _, _)| j.writes.clone())
                            .chain(out.jobs.iter().flat_map(|j| j.writes.clone()))
                            .collect();
                        compare::frame(
                            case,
                            f,
                            g.layout(),
                            &g.probe(),
                            &mem,
                            out,
                            &theirs,
                            None,
                            &masked,
                        )
                        .is_none()
                    };
                    if !fits(&game, &ours) {
                        for line in (0..sf_core::front::FRAME_LINES).step_by(2) {
                            let (g, out) = redo(Some(line));
                            if fits(&g, &out) {
                                (game, ours) = (g, out);
                                break;
                            }
                        }
                    }
                }
            }
        }
        if matches!(pacing, Pacing::Late { .. }) {
            outcome.late += 1;
        }
        if let Some(a) = alone.as_mut() {
            let twice = amiga.frames_run() == 2;
            let skipped = std::mem::take(&mut under_screen);
            a.frame(case, f, input, pacing, twice, (&amiga, &mem), cop, &ours.resync, skipped);
        }
        let theirs_mid = stepped && sync::mid_pass(&amiga.cpu(), &mem);
        let mut step = compare::PassStep::default();
        if faithful && stepped {
            let vbl_count = (mem.byte(VBL_COUNT) as u16) << 8 | mem.byte(VBL_COUNT + 1) as u16;
            let ended = theirs_clock.frame(
                sync::in_level_wait(&amiga.cpu(), &mem),
                theirs_mid,
                vbl_count,
            );
            step = passes.frame(f, ended, game.passes_ended());
            if let Some(apart) = step.apart {
                passes.level();
                held = None;
                emit(
                    &mut found,
                    Divergence {
                        passes_apart: Some(apart),
                        ..frame_divergence(f, of, &game)
                    },
                )?;
            }
            // Sound is a stream: every frame's writes count, a lag's among them.
            if case.compare.paula {
                let t = paula_theirs.frame(&theirs.paula);
                let o = paula_ours.frame(&ours.paula);
                if let Err(e) = sound.frame(f, &t, &o) {
                    emit(
                        &mut found,
                        Divergence {
                            sound_diff: Some(e),
                            ..frame_divergence(f, of, &game)
                        },
                    )?;
                }
            }
        }
        if theirs_mid {
            outcome.straddled.push(f);
            after_lag = true;
            if let Some(from) = lag.left_out(f, true, &mut outcome) {
                let d = lag_divergence(f, of, from, lag.limit, &game);
                emit(&mut found, d)?;
            }
            if faithful {
                let masked: Vec<(u32, u32)> =
                    jobs.iter().flat_map(|(j, _, _)| j.writes.clone()).collect();
                if let Some(v) = pass_state(
                    case,
                    step,
                    &mut passes,
                    &mut held,
                    &game,
                    &mem,
                    &masked,
                    !theirs_mid,
                    &mut outcome.pass_held,
                ) {
                    emit(&mut found, state_divergence(f, of, &game, &mem, v))?;
                }
            }
            prev = mem;
            continue;
        }
        // Memory a job is still writing in the original is compared once the job is over (owner,
        // 2026-10-12), after a lag too (T106: W1-1's exit fade set up from a late pass's end).
        // A job is pending until the original has been in it and left it (it may not have begun
        // yet when the engine, which does all at once, reports it).
        // One the original is never seen in at a frame's end it finished within a frame: it is
        // dropped once it is two frames old.
        jobs.extend(ours.jobs.iter().map(|j| (j.clone(), false, 0)));
        let chain = sync::main_chain(&amiga.cpu(), &mem);
        jobs.retain_mut(|(j, seen, age)| {
            let inside = j.inside.iter().any(|a| chain.contains(a));
            let keep = inside || (!*seen && *age < 2);
            *seen |= inside;
            *age += 1;
            keep
        });
        let masked: Vec<(u32, u32)> = jobs.iter().flat_map(|(j, _, _)| j.writes.clone()).collect();
        if stepped && after_lag {
            after_lag = false;
            lag.left_out(f, false, &mut outcome);
            // Faithful: a re-sync forbidden fails on a gameplay variable only (one the pass
            // matching compares); the engine is re-synced on any.
            let gameplay = faithful
                .then(|| {
                    pass_state(
                        case,
                        step,
                        &mut passes,
                        &mut held,
                        &game,
                        &mem,
                        &masked,
                        !theirs_mid,
                        &mut outcome.pass_held,
                    )
                })
                .flatten();
            if case.compare.vars
                && let Some((v, theirs, ours)) =
                    compare::vars(game.layout(), &game.probe(), &mem, &masked)
            {
                let fails = if faithful {
                    gameplay
                } else {
                    Some((v, theirs, ours))
                };
                if !case.resync
                    && let Some(v) = fails
                {
                    let d = Divergence {
                        resync: true,
                        ..state_divergence(f, of, &game, &mem, v)
                    };
                    emit(&mut found, d)?;
                }
                game = Game::with_display(display.clone(), Start::Transplant(mem.clone()));
                if frog_only {
                    game.frog_only();
                }
                passes.level();
                held = None;
                paula_ours = paula_theirs.clone();
                outcome.resynced.push((
                    f,
                    format!(
                        "{} at ${:06x}: original ${theirs:x}, engine ${ours:x}",
                        v.name, v.addr
                    ),
                ));
            }
            prev = mem;
            continue;
        }
        lag.compared(&mut outcome);
        let mut rendered = None;
        if case.compare.render {
            let g = Game::with_display(display.clone(), Start::Transplant(mem.clone()));
            let r = g.render(View::default());
            if !g.scene().is_level() {
                outcome.skipped.push((f, NOT_A_LEVEL));
            } else if compare::picture(&r, &theirs.picture).is_some()
                && late_redraw(&display, &mem, &prev, &r, &theirs.picture)
            {
                outcome.skipped.push((f, LATE_REDRAW));
            } else {
                outcome.rendered += 1;
                rendered = Some(r);
            }
        }
        after_step(f, &ours.paula, &theirs.paula);
        let d = if faithful {
            let state = pass_state(
                case,
                step,
                &mut passes,
                &mut held,
                &game,
                &mem,
                &masked,
                !theirs_mid,
                &mut outcome.pass_held,
            );
            faithful_frame(
                case,
                f,
                &game,
                &mem,
                state,
                &ours,
                &theirs,
                rendered.as_ref(),
                &mut pictures,
            )
        } else {
            compare::frame(
                case,
                f,
                game.layout(),
                &game.probe(),
                &mem,
                &ours,
                &theirs,
                rendered.as_ref(),
                &masked,
            )
        };
        if d.as_ref().is_none_or(|d| d.picture_diff.is_none())
            && case.compare.picture
            && compare::raced_only(&ours, &theirs.picture, compare::picture_rows_from(case))
        {
            outcome.raced.push(f);
        }
        let unported = game.unported().to_vec();
        if d.is_some() || !unported.is_empty() {
            let mut d = d.unwrap_or(Divergence::at(f));
            d.unported = unported;
            d.level_outcome = game.level_outcome();
            d.oracle_frame = (of != f + 1).then_some(of - 1);
            emit(&mut found, d)?;
        }
        prev = mem;
    }
    outcome.mode = case.mode;
    outcome.picture_colour_step = pictures.colour_step;
    outcome.picture_few_pixels = pictures.few_pixels;
    outcome.picture_glitches = pictures.glitches;
    outcome.sound_shifts = sound.shifts;
    outcome.sound_cut_off = sound.cut_off();
    outcome.sound_trailing = sound.trailing;
    outcome.pass_skews = passes.skews;
    if let Some(a) = alone {
        let alone_first_var = a.first_var.clone();
        let last = case.frames.saturating_sub(1) + a.extra;
        match a.judge(last) {
            Ok(met) => outcome.milestones = Some(met),
            Err(d) => {
                let d = Divergence {
                    milestone: Some(d.clone()),
                    alone_first_var,
                    ..Divergence::at(d.frame as u32)
                };
                if only_alone {
                    return Err(d);
                }
                emit(&mut found, d)?;
            }
        }
    }
    Ok(outcome)
}

/// Check 4's engine, run alone beside the comparison (DESIGN §2.1; T182), and both sides'
/// milestones frame by frame of play (the original's loads, which the runner skips, not counted).
struct AloneRun {
    game: Game,
    theirs: Milestones,
    ours: Milestones,
    /// The seeds the original read from its clock (`rng_seed_from_cia` at the end sequence's
    /// end), as the runner's engine met them, and how many the engine run alone has asked for and
    /// not had: the engine keeps no clock, so they count as inputs.
    seeds: VecDeque<Vec<u8>>,
    wanted: u32,
    /// The engine panicked: the frame of play, and the message. It steps no more.
    panicked: Option<(u32, String)>,
    /// For the report: the first frame of play at whose end, both sides at a pass's end, a
    /// gameplay variable differed, and which (DESIGN §2.1 check 1's comparison, never judged here).
    first_var: Option<(u32, String)>,
    /// A disk-3 power-on: the original is in the story (its boot, then the intro) until the game
    /// program owns the vertical blank.
    disk3: bool,
    /// Frames of play beyond the runner's: the original's load frames under a screen it times
    /// itself, which count as play (owner, 2026-10-15; [`timed_screen`]).
    extra: u32,
}

/// The copper lists of the screens whose own timers run on while the original loads under them
/// (T183): the world's loading picture (h2+$6750), the progress map (h2+$69f0), the
/// end-of-world picture (h2+$6494). The original's frames spent loading under one count as
/// play for check 4, the engine run alone showing the screen meanwhile.
pub fn timed_screen(cop1lc: u32) -> bool {
    use sf_core::layout::h2;
    [h2(0x6750), h2(0x69f0), h2(0x6494)].contains(&cop1lc)
}

impl AloneRun {
    fn new(game: Game, disk3: bool) -> AloneRun {
        AloneRun {
            game,
            theirs: Milestones::default(),
            ours: Milestones::default(),
            seeds: VecDeque::new(),
            wanted: 0,
            panicked: None,
            first_var: None,
            disk3,
            extra: 0,
        }
    }

    /// Frame of play `f`: the engine steps on `input` and `pacing` (twice when the original's
    /// step ran two video frames), and each side's state at its end is read. `mem` and `cop1lc`
    /// are the original's; `resync`, what the runner's engine took from the original after a load
    /// this frame, says whether it was a clock's seed.
    #[allow(clippy::too_many_arguments)]
    fn frame(
        &mut self,
        case: &Case,
        f: u32,
        input: FrameInput,
        pacing: Pacing,
        twice: bool,
        (amiga, mem): (&sf_oracle::Amiga, &AmigaMemory),
        cop1lc: Option<u32>,
        resync: &[(u32, u32)],
        skipped: Vec<(FrameInput, FrameState)>,
    ) {
        let rng = def(VarId::RngState);
        let len = rng.width.len();
        if resync.iter().any(|&(a, _)| a == rng.addr) {
            self.seeds
                .push_back((rng.addr..rng.addr + len).map(|a| mem.byte(a)).collect());
        }
        if self.panicked.is_none() {
            let g = &mut self.game;
            let stepped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if twice {
                    g.step_paced(input, pacing);
                }
                g.step_paced(input, pacing)
            }));
            match stepped {
                Ok(out) => {
                    if out.resync.iter().any(|&(a, _)| a == rng.addr) {
                        self.wanted += 1;
                    }
                }
                Err(e) => {
                    let why = e
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_default();
                    self.panicked = Some((f, why));
                }
            }
            while self.wanted > 0
                && let Some(seed) = self.seeds.pop_front()
            {
                self.game.poke(rng.addr, &seed);
                self.wanted -= 1;
            }
        }
        let g = &self.game;
        let ours = match g.story() {
            Some(_) => FrameState::story(),
            None => FrameState {
                mid_pass: g.mid_pass(),
                ..FrameState::read(&|a, n| g.state().read(a, n), g.cop1lc())
            },
        };
        self.disk3 &= !sync::game_owns_vbl(mem);
        let theirs = if self.disk3 || sync::in_story(mem) {
            FrameState::story()
        } else {
            FrameState {
                mid_pass: sync::mid_pass(&amiga.cpu(), mem),
                ..FrameState::read(&|a, n| (a..a + n).map(|x| mem.byte(x)).collect(), cop1lc)
            }
        };
        if self.first_var.is_none()
            && !ours.story
            && !theirs.story
            && !g.mid_pass()
            && !sync::mid_pass(&amiga.cpu(), mem)
            && case.compare.vars
            && let Some((v, t, o)) = compare::gameplay_vars(case, g.layout(), &g.probe(), mem, &[])
        {
            self.first_var = Some((
                f,
                format!("{} at ${:06x}: original ${t:x}, engine ${o:x}", v.name, v.addr),
            ));
        }
        let at = (f + self.extra) as u64;
        if skipped.is_empty() {
            self.ours.frame(at, ours);
            self.theirs.frame(at, theirs);
            return;
        }
        // The engine's load frame stands for the first the original loaded under its screen;
        // the engine shows the screen through the rest, on their inputs, then this frame's.
        self.ours.frame(at, ours);
        let inputs: Vec<FrameInput> = skipped.iter().skip(1).map(|s| s.0).chain([input]).collect();
        let k = skipped.len() as u64;
        for (i, (_, st)) in skipped.into_iter().enumerate() {
            self.theirs.frame(at + i as u64, st);
        }
        self.theirs.frame(at + k, theirs);
        for (i, inp) in inputs.into_iter().enumerate() {
            if self.panicked.is_none() {
                let g = &mut self.game;
                let stepped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    g.step_paced(inp, Pacing::OnTime);
                }));
                if let Err(e) = stepped {
                    let why = e
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_default();
                    self.panicked = Some((f, why));
                }
            }
            let g = &self.game;
            let st = FrameState::read(&|a, n| g.state().read(a, n), g.cop1lc());
            self.ours.frame(at + 1 + i as u64, st);
        }
        self.extra += k as u32;
    }

    /// Check 4 at the run's last frame of play: the milestones met, or where they part.
    fn judge(mut self, last: u32) -> Result<Vec<(u64, Milestone)>, MilestoneDiff> {
        if let Some((f, why)) = self.panicked {
            return Err(MilestoneDiff {
                frame: f as u64,
                theirs: None,
                ours: None,
                why: format!("the engine run alone panicked: {why}"),
            });
        }
        let theirs = self.theirs.end(last as u64).to_vec();
        let ours = self.ours.end(last as u64);
        milestones::judge(&theirs, ours, milestones::MILESTONE_FRAMES)?;
        Ok(theirs)
    }
}

/// `vbl_count` (h0+$9d8).
const VBL_COUNT: u32 = h0(0x9d8);

/// A side's state held at the end of a pass the other side has not ended yet (DESIGN §2.1
/// check 1).
enum Held {
    /// The original's memory, and what of it a job was still writing.
    Theirs(Box<AmigaMemory>, Vec<(u32, u32)>),
    /// The engine's probe, and the layout it is in.
    Ours(&'static [VarDef], Vec<(VarId, u32)>),
}

/// Check 1 on a frame, as the pass matching's `step` asks: the first gameplay variable that
/// differs between the two sides' states at the end of the same pass, comparing what is held
/// with a side's state now, or both sides' now; then holding a side's state if asked.
/// A side's state is seen at a pass's end unless the frame ended in the middle of the next pass
/// (`theirs_seen`; the engine's, [`Game::mid_pass`]).
#[allow(clippy::too_many_arguments)]
fn pass_state(
    case: &Case,
    step: compare::PassStep,
    passes: &mut compare::PassMatch,
    held: &mut Option<Held>,
    game: &Game,
    mem: &AmigaMemory,
    masked: &[(u32, u32)],
    theirs_seen: bool,
    compared_held: &mut u32,
) -> Option<(VarDef, u32, u32)> {
    if !case.compare.vars {
        return None;
    }
    let ours_seen = !game.mid_pass();
    let mut first = None;
    if step.compare_held {
        first = match held.take() {
            Some(Held::Theirs(m, hm)) if ours_seen => {
                *compared_held += 1;
                compare::gameplay_vars(case, game.layout(), &game.probe(), &m, &hm)
            }
            Some(Held::Ours(layout, probe)) if theirs_seen => {
                *compared_held += 1;
                compare::gameplay_vars(case, layout, &probe, mem, masked)
            }
            _ => None,
        };
    }
    if step.compare_now && theirs_seen && ours_seen && first.is_none() {
        first = compare::gameplay_vars(case, game.layout(), &game.probe(), mem, masked);
    }
    match step.hold {
        Some(compare::Side::Theirs) if theirs_seen => {
            *held = Some(Held::Theirs(Box::new(mem.clone()), masked.to_vec()));
        }
        Some(compare::Side::Ours) if ours_seen => {
            *held = Some(Held::Ours(game.layout(), game.probe()));
        }
        Some(_) => {
            *held = None;
            passes.drop_held();
        }
        None => {}
    }
    first
}

/// A divergence at frame `f`, with what the engine says of where it is.
fn frame_divergence(f: u32, of: u32, game: &Game) -> Divergence {
    Divergence {
        oracle_frame: (of != f + 1).then_some(of - 1),
        unported: game.unported().to_vec(),
        level_outcome: game.level_outcome(),
        ..Divergence::at(f)
    }
}

/// A divergence at frame `f` in variable piece `v`: (piece, the original's, the engine's).
fn state_divergence(
    f: u32,
    of: u32,
    game: &Game,
    mem: &AmigaMemory,
    v: (VarDef, u32, u32),
) -> Divergence {
    Divergence {
        first_var: Some(v),
        var_detail: compare::var_diffs(game.layout(), &game.probe(), mem, v.0.id)
            .into_iter()
            .take(40)
            .collect(),
        ..frame_divergence(f, of, game)
    }
}

/// Checks 1 and 2 on a compared frame (DESIGN §2.1): `state`, what [`pass_state`] found, and
/// the picture judged within the tolerance. A `render` comparison stays exact.
#[allow(clippy::too_many_arguments)]
fn faithful_frame(
    case: &Case,
    f: u32,
    game: &Game,
    mem: &AmigaMemory,
    state: Option<(VarDef, u32, u32)>,
    ours: &FrameOut,
    theirs: &sf_oracle::FrameCapture,
    rendered: Option<&Picture>,
    pictures: &mut compare::PictureJudge,
) -> Option<Divergence> {
    let render_diff = rendered.and_then(|r| {
        compare::picture(r, &theirs.picture).map(|d| {
            report::write_diff_png(&format!("{}-render", case.name), f, r, &theirs.picture, d)
        })
    });
    let from_row = compare::picture_rows_from(case);
    let mut picture_note = None;
    if case.compare.picture
        && !case.picture_excluded(f)
        && !compare::raced_only(ours, &theirs.picture, from_row)
    {
        let class = compare::classify(
            &ours.picture,
            &theirs.picture,
            std::slice::from_ref(&(0..from_row)),
            &Tolerance::OWNER,
        );
        picture_note = pictures.frame(f, class).err();
    }
    let picture_diff = picture_note.as_ref().and_then(|_| {
        compare::picture_from(&ours.picture, &theirs.picture, from_row)
            .map(|d| report::write_diff_png(&case.name, f, &ours.picture, &theirs.picture, d))
    });
    if state.is_none() && picture_note.is_none() && render_diff.is_none() {
        return None;
    }
    Some(Divergence {
        first_var: state,
        var_detail: state
            .map(|(d, _, _)| {
                compare::var_diffs(game.layout(), &game.probe(), mem, d.id)
                    .into_iter()
                    .take(40)
                    .collect()
            })
            .unwrap_or_default(),
        picture_diff,
        picture_note,
        render_diff,
        ..Divergence::at(f)
    })
}

/// A divergence found: kept by a scan, else the end of the run.
fn emit(found: &mut Option<&mut Vec<Divergence>>, d: Divergence) -> Result<(), Divergence> {
    match found.as_mut() {
        Some(all) => {
            all.push(d);
            Ok(())
        }
        None => Err(d),
    }
}

/// The runs of frames left out for lag going on now, against the case's limit.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LagRun {
    /// Frames in a row the original ended mid-pass: (the first, how many).
    mid_pass: (u32, u32),
    /// Frames in a row left out, the frame after each lag among them: (the first, how many).
    skipped: (u32, u32),
    limit: u32,
}

impl LagRun {
    fn new(limit: u32) -> LagRun {
        LagRun {
            mid_pass: (0, 0),
            skipped: (0, 0),
            limit,
        }
    }

    /// Frame `f` left out: the original ended it mid-pass (`mid_pass`), or it is the frame after
    /// such a one. The run's first frame, once the mid-pass run passes the limit (on that frame
    /// only, so `scan` reports a run once).
    fn left_out(&mut self, f: u32, mid_pass: bool, outcome: &mut Outcome) -> Option<u32> {
        let grow = |run: &mut (u32, u32)| {
            *run = if run.1 == 0 {
                (f, 1)
            } else {
                (run.0, run.1 + 1)
            };
        };
        grow(&mut self.skipped);
        if self.skipped.1 > outcome.longest_skip.1 {
            outcome.longest_skip = self.skipped;
        }
        if !mid_pass {
            self.mid_pass.1 = 0;
            return None;
        }
        grow(&mut self.mid_pass);
        if self.mid_pass.1 > outcome.longest_mid_pass.1 {
            outcome.longest_mid_pass = self.mid_pass;
        }
        (self.mid_pass.1 == self.limit + 1).then_some(self.mid_pass.0)
    }

    /// A frame compared: both runs end.
    fn compared(&mut self, outcome: &mut Outcome) {
        outcome.compared += 1;
        self.mid_pass.1 = 0;
        self.skipped.1 = 0;
    }
}

/// The divergence for a mid-pass run from `from` that passed `limit` at frame `f`.
fn lag_divergence(f: u32, of: u32, from: u32, limit: u32, game: &Game) -> Divergence {
    Divergence {
        lag: Some((from, limit)),
        ..frame_divergence(f, of, game)
    }
}

/// The original's clean background buffer (h3+$35860): what a bob's erase restores from.
const CLEAN_BUFFER: u32 = sf_core::layout::h3(0x35860);

/// Whether a render that differs is explained by a late redraw: the copper list, the panel or a
/// display buffer changed during the frame, and every pixel that differs lies where that change
/// shows, i.e. where some alternative (the frame before's copper, panel or buffers, or, mid-erase,
/// the clean background) draws differently from the end of the frame. The alternative need not
/// draw the original's pixel itself: the beam saw a mix of old and new that no single alternative
/// reproduces (10 of the 25 render cases have such a frame). So a renderer defect confined to
/// what changed this frame is left out here, not caught; stepping conformance (T14 on) catches it.
fn late_redraw(
    display: &Arc<DisplayData>,
    mem: &AmigaMemory,
    prev: &AmigaMemory,
    ours: &Picture,
    theirs: &sf_oracle::Rgb12Frame,
) -> bool {
    let regions: Vec<(usize, usize, bool)> = LAYOUT
        .iter()
        .filter_map(|d| match d.id {
            VarId::PanelBitmap | VarId::LevelCopper => {
                Some((d.addr as usize, d.width.len() as usize, false))
            }
            VarId::PlayfieldBuffer0 | VarId::PlayfieldBuffer1 => {
                Some((d.addr as usize, d.width.len() as usize, true))
            }
            _ => None,
        })
        .collect();
    if regions
        .iter()
        .all(|&(a, n, _)| mem.chip[a..a + n] == prev.chip[a..a + n])
    {
        return false;
    }
    // One alternative per region taken from the frame before (the rest as now: the vertical
    // blank's own copper writes come before the beam), and the buffers as the clean background.
    let mut alternatives: Vec<AmigaMemory> = Vec::new();
    let mut erased = mem.clone();
    for &(a, n, buffer) in &regions {
        if mem.chip[a..a + n] != prev.chip[a..a + n] {
            let mut before = mem.clone();
            before.chip[a..a + n].copy_from_slice(&prev.chip[a..a + n]);
            alternatives.push(before);
        }
        if buffer {
            let c = CLEAN_BUFFER as usize;
            erased.chip.copy_within(c..c + n, a);
        }
    }
    alternatives.push(erased);
    // The vertical blank latches the scroll first (h0+$1700a) and writes the sprites' and the
    // palette's copper words later, which on a busy frame lands after the copper ran them: the
    // copper list with its scroll as now and every other changed move as the frame before.
    if let Some(d) = LAYOUT.iter().find(|d| d.id == VarId::LevelCopper) {
        let mut late = mem.clone();
        let a = d.addr as usize;
        for at in (a..a + d.width.len() as usize).step_by(4) {
            let reg = u16::from_be_bytes([mem.chip[at], mem.chip[at + 1]]);
            let scroll = reg & 1 == 1 || (0xe0..0xf8).contains(&reg) || reg == 0x102;
            if !scroll {
                late.chip[at + 2..at + 4].copy_from_slice(&prev.chip[at + 2..at + 4]);
            }
        }
        alternatives.push(late);
    }
    let render = |m: AmigaMemory| {
        Game::with_display(display.clone(), Start::Transplant(m)).render(View::default())
    };
    let pictures: Vec<Picture> = std::iter::once(ours.clone())
        .chain(alternatives.into_iter().map(render))
        .collect();
    if pictures
        .iter()
        .any(|p| (p.width, p.height) != (theirs.width, theirs.height))
    {
        return false;
    }
    let colours: Vec<Vec<u16>> = pictures.iter().map(sf_core::to_rgb12).collect();
    // A differing pixel counts as late only where what changed during the frame shows: some
    // alternative draws it differently from the end of the frame.
    theirs
        .pixels
        .iter()
        .enumerate()
        .all(|(i, &t)| t == colours[0][i] || colours[1..].iter().any(|c| c[i] != colours[0][i]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feed `frames` (true: mid-pass, false: the frame after a lag, None: compared) and return
    /// the frames at which the limit was passed, with the run's first frame.
    fn feed(limit: u32, frames: &[Option<bool>]) -> (Vec<(u32, u32)>, Outcome) {
        let mut lag = LagRun::new(limit);
        let mut o = Outcome::default();
        let mut failed = Vec::new();
        for (f, k) in frames.iter().enumerate() {
            let f = f as u32;
            match k {
                Some(mid) => {
                    if let Some(from) = lag.left_out(f, *mid, &mut o) {
                        failed.push((f, from));
                    }
                }
                None => lag.compared(&mut o),
            }
        }
        (failed, o)
    }

    /// The old `items-upper-w2-1` (commit `4789f2e`): compared to ~1012, then the original held
    /// in its `frog_tiles` wall loop, mid-pass to the end of its 1 480 frames (T85 review, T105).
    #[test]
    fn an_original_held_mid_pass_past_the_limit_fails() {
        let mut trace = vec![None; 1012];
        trace.extend(vec![Some(true); 468]);
        let (failed, o) = feed(LAG_LIMIT, &trace);
        assert_eq!(failed, vec![(1012 + LAG_LIMIT, 1012)]);
        assert_eq!(o.longest_mid_pass, (1012, 468));
        assert_eq!(o.compared, 1012);
    }

    #[test]
    fn a_mid_pass_run_at_the_limit_passes() {
        let mut trace = vec![None; 10];
        trace.extend(vec![Some(true); LAG_LIMIT as usize]);
        trace.push(Some(false));
        trace.extend(vec![None; 10]);
        let (failed, o) = feed(LAG_LIMIT, &trace);
        assert!(failed.is_empty());
        assert_eq!(o.longest_mid_pass, (10, LAG_LIMIT));
        assert_eq!(o.longest_skip, (10, LAG_LIMIT + 1));
        assert_eq!(o.compared, 20);
    }

    /// A pass longer than a frame, lagging every other frame: the frames after each lag break
    /// the mid-pass run (the variables are checked there) but not the run left out.
    #[test]
    fn alternating_lag_is_reported_not_limited() {
        let trace: Vec<Option<bool>> = (0..400).map(|f| Some(f % 2 == 0)).collect();
        let (failed, o) = feed(LAG_LIMIT, &trace);
        assert!(failed.is_empty());
        assert_eq!(o.longest_mid_pass.1, 1);
        assert_eq!(o.longest_skip, (0, 400));
        assert_eq!(o.compared, 0);
    }

    /// One failure per run, so `scan` reports it once; a later run past the limit again.
    #[test]
    fn each_run_past_the_limit_fails_once() {
        let mut trace = vec![Some(true); 5];
        trace.push(None);
        trace.extend(vec![Some(true); 4]);
        let (failed, _) = feed(3, &trace);
        assert_eq!(failed, vec![(3, 0), (9, 6)]);
    }

    /// DESIGN §2.1 check 1: a pass ending a frame later on one side is compared with the state
    /// the other side held at its own pass's end, not with that side's state now (T180 review: a
    /// mutation comparing now survived every test).
    #[test]
    fn a_held_pass_end_is_compared_with_the_other_side_s_pass_end() {
        let case = Case::parse("state", "sync = \"power-on\"\nframes = 1").unwrap();
        let x = (sf_core::layout::def(VarId::FrogX).addr - sf_core::layout::SLOW_BASE) as usize;
        let mem = |frog_x: u8| {
            let mut m = AmigaMemory {
                chip: vec![0; 512 * 1024],
                slow: vec![0; 512 * 1024],
            };
            m.slow[x + 1] = frog_x;
            m
        };
        let at_end = |frog_x| Game::new(&GameData::default(), Start::Transplant(mem(frog_x)));
        let step = compare::PassStep {
            compare_held: true,
            ..compare::PassStep::default()
        };
        let judge = |held: Held, game: &Game, now: &AmigaMemory| {
            let mut held = Some(held);
            let mut n = 0;
            let v = pass_state(
                &case,
                step,
                &mut compare::PassMatch::default(),
                &mut held,
                game,
                now,
                &[],
                true,
                &mut n,
            );
            assert_eq!(n, 1);
            v.map(|(d, t, o)| (d.name, t, o))
        };
        // The original ended pass n with the frog at 5, the engine a frame later at 5; the
        // original has moved on to 6 since.
        let held = |frog_x| Held::Theirs(Box::new(mem(frog_x)), Vec::new());
        assert_eq!(judge(held(5), &at_end(5), &mem(6)), None);
        assert_eq!(judge(held(4), &at_end(5), &mem(5)), Some(("frog_x", 4, 5)));
        // The engine ended it first, at 5, and is at 6 when the original ends it at 5.
        let g = at_end(5);
        let ours = || Held::Ours(g.layout(), g.probe());
        assert_eq!(judge(ours(), &at_end(6), &mem(5)), None);
        assert_eq!(judge(ours(), &at_end(5), &mem(4)), Some(("frog_x", 4, 5)));
    }
}
