//! What a divergence says, and the diff PNG beside it (drawn by `amiga_conform::report`).
//!
//! Diff images go to `SF_CONFORM_OUT`, else `<temp dir>/sf-conform/`: they show the original's
//! picture, which is derived from the disks, so they never go in the repository (DESIGN §2.3).

use std::fmt;
use std::path::PathBuf;

use sf_core::Picture;
use sf_core::layout::VarDef;
use sf_oracle::Rgb12Frame;

use crate::compare::PixelMismatch;

// The diff picture's half moved to the toolkit (toolkit-extract T06); kept at this path for its
// callers.
pub use amiga_conform::report::{PictureDiff, png};

/// The first frame at which the engine and the original differ, and how.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Divergence {
    /// Counted from the sync point.
    pub frame: u32,
    /// The first `LAYOUT` variable that differs: (its piece, the original's value, the engine's).
    pub first_var: Option<(VarDef, u32, u32)>,
    pub picture_diff: Option<PictureDiff>,
    pub paula_diff: Option<String>,
    /// The picture rendered from the original's memory at this frame, where it differs.
    pub render_diff: Option<PictureDiff>,
    /// The original's frame compared, counted from the sync point, when it is not `frame`: the
    /// original spent frames loading that the engine does at once (DESIGN §2.1).
    pub oracle_frame: Option<u32>,
    /// Paths of the original the engine reached and does not follow yet.
    pub unported: Vec<&'static str>,
    /// How the engine's last level ended, once one has ([`sf_core::Game::level_outcome`]).
    pub level_outcome: Option<sf_core::level::LevelOutcome>,
    /// Every differing piece of the first differing variable (up to 40).
    pub var_detail: Vec<String>,
    /// The variables differed on the frame after a lag, where the runner would re-sync the
    /// engine, and the case forbids that (`resync = false`).
    pub resync: bool,
    /// The original ended more frames in a row mid-pass than the case's limit: (the run's first
    /// frame, the limit). Nothing was compared from that frame to `frame` (T120).
    pub lag: Option<(u32, u32)>,
    /// Faithful (DESIGN §2.1 check 2): why the picture's difference is past the tolerance.
    pub picture_note: Option<String>,
    /// Faithful (check 3): the sound event that differs.
    pub sound_diff: Option<String>,
    /// Faithful (check 1): the sides ended more than a pass apart: (the original's passes, the
    /// engine's).
    pub passes_apart: Option<(u64, u64)>,
    /// Check 4: where the engine run alone and the original part ([`crate::milestones`]).
    pub milestone: Option<crate::milestones::MilestoneDiff>,
    /// With `milestone`: the first frame the engine run alone ended with a gameplay variable
    /// unlike the original's (both at a pass's end), and which: where to look for the cause.
    pub alone_first_var: Option<(u32, String)>,
}

impl Divergence {
    /// Nothing differing yet, at `frame`.
    pub fn at(frame: u32) -> Divergence {
        Divergence {
            frame,
            ..Divergence::default()
        }
    }
}

impl fmt::Display for Divergence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.oracle_frame {
            Some(o) => writeln!(
                f,
                "diverged at frame {} (the original's frame {o})",
                self.frame
            )?,
            None => writeln!(f, "diverged at frame {}", self.frame)?,
        }
        if self.resync {
            writeln!(
                f,
                "  after a lag; the runner would re-sync here, and the case sets resync = false"
            )?;
        }
        if let Some((from, limit)) = self.lag {
            writeln!(
                f,
                "  the original ended every frame mid-pass from frame {from}: {} in a row, past the limit of {limit}, so nothing was compared",
                self.frame - from + 1
            )?;
        }
        if let Some((theirs, ours)) = self.passes_apart {
            writeln!(
                f,
                "  passes: the original has ended {theirs}, the engine {ours}: more than one apart"
            )?;
        }
        if let Some(m) = &self.milestone {
            writeln!(f, "  plays alone (check 4): {m}")?;
            if let Some((frame, v)) = &self.alone_first_var {
                writeln!(f, "    the engine alone first differs at frame {frame}: {v}")?;
            }
        }
        for u in &self.unported {
            writeln!(f, "  the engine reached a path it does not follow yet: {u}")?;
        }
        if let Some((d, theirs, ours)) = &self.first_var {
            writeln!(
                f,
                "  variable {} at ${:06x} ({:?}): original ${theirs:x}, engine ${ours:x}",
                d.name, d.addr, d.width
            )?;
            if self.var_detail.len() > 1 {
                writeln!(
                    f,
                    "    all of it that differs (original/engine): {}",
                    self.var_detail.join(" ")
                )?;
            }
        }
        for (what, p) in [
            ("picture", &self.picture_diff),
            ("render from the original's memory", &self.render_diff),
        ] {
            let Some(p) = p else { continue };
            if let Some((x, y, theirs, ours)) = p.mismatch.first {
                writeln!(
                    f,
                    "  {what}: first pixel ({x}, {y}): original ${theirs:03x}, engine ${ours:03x}"
                )?;
            }
            if let Some((a, b, n)) = p.mismatch.rows {
                writeln!(f, "  {what}: {n} pixels differ, on rows {a} to {b}")?;
            }
            if let Some((theirs, ours)) = &p.mismatch.window {
                writeln!(f, "  display window: original {theirs}, engine {ours}")?;
            }
            if let (Some(why), "picture") = (&self.picture_note, what) {
                writeln!(f, "  {what}: past the tolerance: {why}")?;
            }
            writeln!(f, "  diff image: {}", p.path.display())?;
        }
        if let Some(p) = &self.paula_diff {
            writeln!(f, "  paula: {p}")?;
        }
        if let Some(s) = &self.sound_diff {
            writeln!(f, "  sound: {s}")?;
        }
        Ok(())
    }
}

/// Where diff images go.
pub fn out_dir() -> PathBuf {
    std::env::var_os("SF_CONFORM_OUT")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("sf-conform"))
}

/// Write `<out>/<case>-frame<N>.png` ([`amiga_conform::report::write_diff_png`] into [`out_dir`]).
pub fn write_diff_png(
    case: &str,
    frame: u32,
    ours: &Picture,
    theirs: &Rgb12Frame,
    mismatch: PixelMismatch,
) -> PictureDiff {
    amiga_conform::report::write_diff_png(&out_dir(), case, frame, ours, theirs, mismatch)
}
