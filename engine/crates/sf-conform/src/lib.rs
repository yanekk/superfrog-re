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
use sf_core::layout::{AmigaMemory, LAYOUT, VarId};
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
    let frozen = case.excludes(case::Excluded::Objects);
    let synced = sync::reach_frozen(&case.sync, &case.script, case.frames, frozen)
        .unwrap_or_else(|e| panic!("{}: {e}", case.name));
    let mut amiga = synced.amiga;
    let mut prev = synced.memory.clone();
    let start = match &case.sync {
        SyncPoint::PowerOn => Start::PowerOn {
            seed: synced.seed.unwrap_or_default(),
        },
        _ => Start::Transplant(synced.memory),
    };
    let display = display_data();
    let mut game = Game::with_display(display.clone(), start);
    let mut outcome = Outcome {
        frames: case.frames,
        ..Outcome::default()
    };
    for f in 0..case.frames {
        let input = case.script.input_at(f);
        if frozen {
            freeze::freeze_objects(&mut amiga, &display);
        }
        let theirs = amiga
            .step(input)
            .unwrap_or_else(|e| panic!("{}: oracle, frame {f}: {e}", case.name));
        let mem = sync::core_memory(amiga.memory());
        let pacing = sync::pacing(&amiga.cpu(), &mem);
        before_step(f, &mut game);
        let ours = game.step_paced(input, pacing);
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
            &game.probe(),
            &mem,
            &ours,
            &theirs,
            rendered.as_ref(),
        );
        if let Some(d) = d {
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
