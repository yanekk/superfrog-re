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
pub mod reference;
pub mod report;
pub mod script;
pub mod sync;

pub use case::{Case, Compare, cases_dir, scripts_dir};
pub use report::{Divergence, PictureDiff};
pub use script::InputScript;
pub use sync::SyncPoint;

use std::sync::{Arc, OnceLock};

use sf_assets::GameData;
use sf_core::layout::{AmigaMemory, LAYOUT, Ram, VarId};
use sf_core::{DisplayData, Game, Picture, Scene, Start, View};

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
    mut before_step: impl FnMut(u32, &mut Game),
) -> Result<Outcome, Divergence> {
    let synced = sync::reach(&case.sync, &case.script, case.frames + LOAD_SEARCH)
        .unwrap_or_else(|e| panic!("{}: {e}", case.name));
    let mut amiga = synced.amiga;
    let mut prev = synced.memory.clone();
    let start = match &case.sync {
        SyncPoint::PowerOn => Start::PowerOn {
            seed: synced.seed.unwrap_or_default(),
        },
        SyncPoint::AtFrame { .. } => Start::Resume {
            memory: synced.memory,
            cpu: sync::core_cpu(&amiga.cpu()),
        },
        _ => Start::Transplant(synced.memory),
    };
    let display = display_data();
    let mut game = Game::with_display(display.clone(), start);
    let mut outcome = Outcome {
        frames: case.frames,
        ..Outcome::default()
    };
    // The original's frame, counted from the sync point: ahead of the engine's after a load.
    let mut of = 0u32;
    // Jobs the engine did at once that the original's CPU may still be running.
    // Each with whether the original has been seen in it yet.
    let mut jobs: Vec<(sf_core::Job, bool)> = Vec::new();
    for f in 0..case.frames {
        let input = case.script.input_at(of);
        let mut theirs = amiga
            .step(input)
            .unwrap_or_else(|e| panic!("{}: oracle, frame {of}: {e}", case.name));
        of += 1;
        let mut mem = sync::core_memory(amiga.memory());
        let pacing = sync::pacing(&amiga.cpu(), &mem);
        before_step(f, &mut game);
        let ours = game.step_paced(input, pacing);
        if let Some(resume) = ours.load {
            // The engine loaded at once; the original is compared once its load is over.
            let mut n = 0;
            while !sync::main_chain(&amiga.cpu(), &mem)
                .iter()
                .take(4)
                .any(|&a| a == resume)
            {
                n += 1;
                assert!(
                    n < LOAD_SEARCH,
                    "{}: the original did not reach ${resume:x} within {LOAD_SEARCH} frames of the engine's load (frame {f})",
                    case.name
                );
                theirs = amiga
                    .step(case.script.input_at(of))
                    .unwrap_or_else(|e| panic!("{}: oracle, frame {of}: {e}", case.name));
                of += 1;
                mem = sync::core_memory(amiga.memory());
            }
            // What runs on by itself while the original loads is taken from it.
            for &(addr, len) in sf_core::front::RESYNC_AFTER_LOAD.iter().chain(&ours.resync) {
                let bytes: Vec<u8> = (addr..addr + len).map(|a| mem.byte(a)).collect();
                game.poke(addr, &bytes);
            }
        }
        // Memory a job is still writing in the original is compared once the job is over (owner,
        // 2026-10-12).
        // A job is pending until the original has been in it and left it (it may not have begun
        // yet when the engine, which does all at once, reports it).
        jobs.extend(ours.jobs.iter().map(|j| (j.clone(), false)));
        let chain = sync::main_chain(&amiga.cpu(), &mem);
        jobs.retain_mut(|(j, seen)| {
            let inside = chain.contains(&j.inside);
            let keep = inside || !*seen;
            *seen |= inside;
            keep
        });
        let masked: Vec<(u32, u32)> = jobs.iter().flat_map(|(j, _)| j.writes.clone()).collect();
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
        // On a frame the engine loaded on, what runs by itself (the keyboard's every-other-frame
        // poll) cannot be in step before the re-sync: its Paula writes are not compared.
        let loaded_case;
        let case_here = if ours.load.is_some() && case.compare.paula {
            loaded_case = Case {
                compare: Compare {
                    paula: false,
                    ..case.compare
                },
                ..case.clone()
            };
            &loaded_case
        } else {
            case
        };
        let d = compare::frame(
            case_here,
            f,
            &game.probe(),
            &mem,
            &ours,
            &theirs,
            rendered.as_ref(),
            &masked,
        );
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
            });
            d.unported = unported;
            d.oracle_frame = (of != f + 1).then_some(of - 1);
            return Err(d);
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
