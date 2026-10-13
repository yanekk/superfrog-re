//! Shell, test tooling: the conformance runner (DESIGN §2.1, §4).
//!
//! A [`Case`] names a sync point, an input script, a frame count and what to compare. [`run`]
//! brings the original (on the oracle) and the engine to the sync point, feeds both the same input
//! every frame and compares them; it returns the first frame that differs as a [`Divergence`]
//! naming the first variable, the first pixel (with a diff PNG) and the first Paula write.
//!
//! The CLI (`cargo run -q --release -p sf-conform -- run <case.toml> | all | record <case.toml>`)
//! is in `main.rs`; how to write scripts and cases is in `engine/conform/README.md`.

pub mod case;
pub mod compare;
pub mod disks;
pub mod freeze;
pub mod reference;
pub mod report;
pub mod routine;
pub mod script;
pub mod sync;

pub use case::{Case, Compare, Excluded, Exclusion, cases_dir, scripts_dir};
pub use report::{Divergence, PictureDiff};
pub use script::InputScript;
pub use sync::SyncPoint;

use std::sync::{Arc, OnceLock};

use sf_assets::GameData;
use sf_core::layout::{AmigaMemory, LAYOUT, Ram, VarId};
use sf_core::{DisplayData, FrameOut, Game, LoadEnd, Picture, Scene, Start, View};

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
    /// Frames after such a lag on which the variables still differed, so the engine was re-synced
    /// from the original's memory (DESIGN §2.1), with the first variable that differed. Always
    /// empty for a case with `resync = false`: there each is a divergence.
    pub resynced: Vec<(u32, String)>,
    /// Frames whose picture matched only with the rows of the engine's beam races left out
    /// (`FrameOut::beam_races`, owner 2026-10-13).
    pub raced: Vec<u32>,
}

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
/// uses to plant a divergence.
pub fn run_with(
    case: &Case,
    before_step: impl FnMut(u32, &mut Game),
) -> Result<Outcome, Divergence> {
    run_inner(case, before_step, None)
}

/// [`run`], going on past each divergence: every frame that differs, in order (for finding what
/// is left in a long case; the engine is not re-synced, so later ones may follow from earlier).
pub fn scan(case: &Case) -> Vec<Divergence> {
    let mut found = Vec::new();
    let _ = run_inner(case, |_, _| {}, Some(&mut found));
    found
}

fn run_inner(
    case: &Case,
    mut before_step: impl FnMut(u32, &mut Game),
    mut found: Option<&mut Vec<Divergence>>,
) -> Result<Outcome, Divergence> {
    let frozen = case.freeze();
    let synced = sync::reach_frozen(&case.sync, &case.script, case.frames + LOAD_SEARCH, frozen)
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
    let mut game = Game::with_display(display.clone(), start);
    if frog_only {
        game.frog_only();
    }
    let mut outcome = Outcome {
        frames: case.frames,
        ..Outcome::default()
    };
    let mut after_lag = false;
    // The original's frame, counted from the sync point: ahead of the engine's after a load.
    let mut of = 0u32;
    // Jobs the engine did at once that the original's CPU may still be running.
    // Each with whether the original has been seen in it yet, and frames since reported.
    let mut jobs: Vec<(sf_core::Job, bool, u32)> = Vec::new();
    // From the end sequence on, whenever the original asks for a disk it goes in, as a player
    // would. Other cases keep the drive as their sync point left it: inserting a disk changes the
    // timing of what follows (power-on's RNG seed among it).
    let mut disks = (case.sync == SyncPoint::EndSequence).then(|| sync::Disks::at(&prev));
    // The interrupts the oracle's last step took (one past its end is the next frame's).
    let mut irqs = Vec::new();
    // The original's last handler, taken too late to work in its step (it counts in the next).
    let mut late = false;
    for f in 0..case.frames {
        let input = case.script.input_at(of);
        frozen.apply(&mut amiga, &display);
        let mut theirs = sync::step_inserting(&mut amiga, input)
            .unwrap_or_else(|e| panic!("{}: oracle, frame {of}: {e}", case.name));
        let mut entry = sync::handler_entry(&irqs, &amiga.irqs());
        let handlers;
        (handlers, late) = sync::handlers(late, &irqs, &amiga.irqs(), &theirs.paula);
        irqs = amiga.irqs();
        if let Some(d) = disks.as_mut() {
            d.serve(&mut amiga);
        }
        of += 1;
        let mut mem = sync::core_memory(amiga.memory());
        let mut pacing = sync::pacing(&amiga.cpu(), &mem, &theirs.paula);
        before_step(f, &mut game);
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
            game.set_blank_merged(Some(handlers < amiga.frames_run()));
            ours = game.step_paced(input, pacing);
            ours.paula.splice(0..0, held);
            if let Some(end) = ours.load {
                // The engine loaded at once; the original is compared once its load is over.
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
                    theirs = sync::step_inserting(&mut amiga, case.script.input_at(of))
                        .unwrap_or_else(|e| panic!("{}: oracle, frame {of}: {e}", case.name));
                    entry = sync::handler_entry(&irqs, &amiga.irqs());
                    irqs = amiga.irqs();
                    if let Some(d) = disks.as_mut() {
                        d.serve(&mut amiga);
                    }
                    of += 1;
                    prev_mem = std::mem::replace(&mut mem, sync::core_memory(amiga.memory()));
                }
                // The schedule as the original is once its load is over.
                pacing = sync::pacing(&amiga.cpu(), &mem, &theirs.paula);
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
                    } else {
                        take(&mut g, &prev_mem, before.runs_during_load());
                    }
                    g.set_load_line(line);
                    g.set_handler_entry(entry);
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
                    let masked: Vec<(u32, u32)> = jobs
                        .iter()
                        .flat_map(|(j, _, _)| j.writes.clone())
                        .chain(ours.jobs.iter().flat_map(|j| j.writes.clone()))
                        .collect();
                    let fits = |g: &Game, out: &FrameOut| {
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
        if stepped && sync::mid_pass(&amiga.cpu(), &mem) {
            outcome.straddled.push(f);
            after_lag = true;
            prev = mem;
            continue;
        }
        if stepped && after_lag {
            after_lag = false;
            if case.compare.vars
                && let Some((v, theirs, ours)) =
                    compare::vars(game.layout(), &game.probe(), &mem, &[])
            {
                if !case.resync {
                    let d = Divergence {
                        frame: f,
                        first_var: Some((v, theirs, ours)),
                        picture_diff: None,
                        paula_diff: None,
                        render_diff: None,
                        oracle_frame: (of != f + 1).then_some(of - 1),
                        unported: Vec::new(),
                        var_detail: compare::var_diffs(game.layout(), &game.probe(), &mem, v.id)
                            .into_iter()
                            .take(40)
                            .collect(),
                        resync: true,
                    };
                    match found.as_mut() {
                        Some(all) => all.push(d),
                        None => return Err(d),
                    }
                }
                game = Game::with_display(display.clone(), Start::Transplant(mem.clone()));
                if frog_only {
                    game.frog_only();
                }
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
        // Memory a job is still writing in the original is compared once the job is over (owner,
        // 2026-10-12).
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
        let mut rendered = None;
        if case.compare.render {
            let g = Game::with_display(display.clone(), Start::Transplant(mem.clone()));
            let r = g.render(View::default());
            if g.scene() != Scene::Level {
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
        let d = compare::frame(
            case,
            f,
            game.layout(),
            &game.probe(),
            &mem,
            &ours,
            &theirs,
            rendered.as_ref(),
            &masked,
        );
        if d.as_ref().is_none_or(|d| d.picture_diff.is_none())
            && case.compare.picture
            && compare::raced_only(&ours, &theirs.picture, compare::picture_rows_from(case))
        {
            outcome.raced.push(f);
        }
        let unported = game.unported().to_vec();
        if d.is_some() || !unported.is_empty() {
            let mut d = d.unwrap_or(Divergence {
                frame: f,
                first_var: None,
                picture_diff: None,
                paula_diff: None,
                render_diff: None,
                oracle_frame: None,
                unported: Vec::new(),
                var_detail: Vec::new(),
                resync: false,
            });
            d.unported = unported;
            d.oracle_frame = (of != f + 1).then_some(of - 1);
            match found.as_mut() {
                Some(all) => all.push(d),
                None => return Err(d),
            }
        }
        prev = mem;
    }
    Ok(outcome)
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
