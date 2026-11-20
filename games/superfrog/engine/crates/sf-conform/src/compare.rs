//! One frame of the engine against one frame of the original (DESIGN §2.1): Superfrog's half, its
//! memory variables, its panel rows and its cases. The picture and sound judges are
//! `amiga_conform::compare`'s.

use sf_core::layout::{AmigaMemory, VarDef, VarId};
use sf_core::{FrameOut, Picture};
use sf_oracle::{FrameCapture, Rgb12Frame};

use crate::case::Case;
use crate::report::{self, Divergence};

// The game-free half moved to the toolkit (toolkit-extract T06); kept at this path for its callers.
pub use amiga_conform::compare::{
    Mode, PassMatch, PassStep, PictureClass, PictureJudge, PixelMismatch, Side, Tolerance, classify,
    paula, picture, picture_except, picture_from, race_rows, reg_name,
};

/// The case's comparisons on frame `frame`: `None` when all match.
#[allow(clippy::too_many_arguments)]
pub fn frame(
    case: &Case,
    frame: u32,
    layout: &[VarDef],
    probe: &[(VarId, u32)],
    theirs_mem: &AmigaMemory,
    ours: &FrameOut,
    theirs: &FrameCapture,
    rendered: Option<&Picture>,
    masked: &[(u32, u32)],
) -> Option<Divergence> {
    let c = case.compare;
    let render_diff = rendered
        .and_then(|r| picture(r, &theirs.picture).map(|d| (r, d)))
        .map(|(r, d)| {
            report::write_diff_png(
                &format!("{}-render", case.name),
                frame,
                r,
                &theirs.picture,
                d,
            )
        });
    let first_var = c
        .vars
        .then(|| vars_except(layout, probe, theirs_mem, masked, |d| case_skips(case, d)))
        .flatten();
    let from_row = picture_rows_from(case);
    let picture_diff = (c.picture && !case.picture_excluded(frame))
        .then(|| picture_from(&ours.picture, &theirs.picture, from_row))
        .flatten()
        .filter(|_| !raced_only(ours, &theirs.picture, from_row))
        .map(|d| report::write_diff_png(&case.name, frame, &ours.picture, &theirs.picture, d));
    let paula_diff = c.paula.then(|| paula(&ours.paula, &theirs.paula)).flatten();
    if first_var.is_none()
        && picture_diff.is_none()
        && paula_diff.is_none()
        && render_diff.is_none()
    {
        return None;
    }
    Some(Divergence {
        frame,
        first_var,
        picture_diff,
        paula_diff,
        render_diff,
        oracle_frame: None,
        unported: Vec::new(),
        level_outcome: None,
        var_detail: first_var
            .map(|(d, _, _)| {
                var_diffs(layout, probe, theirs_mem, d.id)
                    .into_iter()
                    .take(40)
                    .collect()
            })
            .unwrap_or_default(),
        ..Divergence::default()
    })
}

/// The first picture row `case` compares.
pub fn picture_rows_from(case: &Case) -> u16 {
    if case.excludes(crate::case::Excluded::Panel) {
        PANEL_ROWS
    } else {
        0
    }
}

/// The picture rows of the status panel and the two black lines under it (window lines `$27` to
/// `$35`), which a case excluding the panel does not compare.
pub const PANEL_ROWS: u16 = 0x36 - 0x27;

/// Every piece of `id`'s entry that differs, as `offset: original/engine`, for a report.
pub fn var_diffs(
    layout: &[VarDef],
    probe: &[(VarId, u32)],
    theirs: &AmigaMemory,
    id: VarId,
) -> Vec<String> {
    let pieces: Vec<VarDef> = layout.iter().flat_map(|d| d.pieces()).collect();
    let base = layout.iter().find(|d| d.id == id).map_or(0, |d| d.addr);
    pieces
        .into_iter()
        .zip(probe)
        .filter(|(p, _)| p.id == id)
        .filter_map(|(p, &(_, ours))| {
            let t = p.read(theirs);
            (t != ours).then(|| format!("+{:x}: {t:x}/{ours:x}", p.addr - base))
        })
        .collect()
}

/// The first `LAYOUT` piece whose engine value differs from the original's memory: (piece,
/// original's value, engine's value). Pieces inside a `masked` run (address, length) are not
/// compared: memory a job of the original's CPU has not finished writing (DESIGN §2.1). Nor is
/// a variable the original has no counterpart of ([`sf_core::layout::engine_only`]).
pub fn vars(
    layout: &[VarDef],
    probe: &[(VarId, u32)],
    theirs: &AmigaMemory,
    masked: &[(u32, u32)],
) -> Option<(VarDef, u32, u32)> {
    vars_except(layout, probe, theirs, masked, |d| sf_core::layout::engine_only(d.id))
}

/// [`vars`], leaving out the variables `skip` says.
pub fn vars_except(
    layout: &[VarDef],
    probe: &[(VarId, u32)],
    theirs: &AmigaMemory,
    masked: &[(u32, u32)],
    skip: impl Fn(&VarDef) -> bool,
) -> Option<(VarDef, u32, u32)> {
    let pieces: Vec<VarDef> = layout.iter().flat_map(|d| d.pieces()).collect();
    assert_eq!(
        pieces.len(),
        probe.len(),
        "probe and LAYOUT disagree on the number of pieces"
    );
    pieces
        .into_iter()
        .zip(probe)
        .filter(|(p, _)| {
            !masked
                .iter()
                .any(|&(a, n)| p.addr < a + n && a < p.addr + p.width.len())
        })
        .map(|(p, &(id, ours))| {
            assert_eq!(p.id, id, "probe out of LAYOUT order");
            (p, p.read(theirs), ours)
        })
        .find(|(p, theirs, ours)| theirs != ours && !skip(p))
}

/// Whether the engine's picture differs from the original's from row `from` down, but matches it
/// once the rows of the frame's beam races are left out: a frame conformance excuses and counts
/// (`Outcome::raced`).
pub fn raced_only(ours: &FrameOut, theirs: &Rgb12Frame, from: u16) -> bool {
    if ours.beam_races.is_empty() {
        return false;
    }
    let mut rows = race_rows(&ours.beam_races, ours.picture.top);
    rows.push(0..from);
    picture_from(&ours.picture, theirs, from).is_some()
        && picture_except(&ours.picture, theirs, &rows).is_none()
}

/// [`vars_except`] for the faithful state check: leaves out what `case` leaves out and every
/// `presentation` variable or piece (DESIGN §2.1 check 1).
pub fn gameplay_vars(
    case: &Case,
    layout: &[VarDef],
    probe: &[(VarId, u32)],
    theirs: &AmigaMemory,
    masked: &[(u32, u32)],
) -> Option<(VarDef, u32, u32)> {
    vars_except(layout, probe, theirs, masked, |d| {
        sf_core::layout::presentation_piece(d).is_some() || case_skips(case, d)
    })
}

/// Every gameplay variable piece that differs, as [`gameplay_vars`] judges the first.
pub fn gameplay_diffs(
    case: &Case,
    layout: &[VarDef],
    probe: &[(VarId, u32)],
    theirs: &AmigaMemory,
) -> Vec<(VarDef, u32, u32)> {
    let pieces: Vec<VarDef> = layout.iter().flat_map(|d| d.pieces()).collect();
    pieces
        .into_iter()
        .zip(probe)
        .map(|(p, &(_, ours))| (p, p.read(theirs), ours))
        .filter(|(p, theirs, ours)| {
            theirs != ours
                && sf_core::layout::presentation_piece(p).is_none()
                && !case_skips(case, p)
        })
        .collect()
}

/// Whether `case` leaves variable `d` out of its comparison (the panel, a `vars` list), or every
/// case does ([`sf_core::layout::engine_only`]).
pub fn case_skips(case: &Case, d: &VarDef) -> bool {
    sf_core::layout::engine_only(d.id)
        || (case.excludes(crate::case::Excluded::Panel) && d.id == VarId::PanelBitmap)
        || case
            .only_vars
            .as_ref()
            .is_some_and(|only| !only.iter().any(|n| n == d.name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(colour: u16) -> Rgb12Frame {
        Rgb12Frame {
            left: 0,
            top: 0x2c,
            width: 4,
            height: 6,
            pixels: vec![colour; 24],
            hires_detail: false,
        }
    }

    fn ours() -> Picture {
        let mut p = Picture {
            left: 0,
            top: 0x2c,
            width: 4,
            height: 6,
            pixels: vec![1; 24],
            palettes: vec![[0; 32]; 6],
            splits: Vec::new(),
        };
        p.palettes.iter_mut().for_each(|c| c[1] = 0x123);
        p
    }

    #[test]
    fn a_frame_differing_only_on_raced_lines_is_excused() {
        let mut out = FrameOut {
            picture: ours(),
            ..FrameOut::default()
        };
        let mut theirs = frame(0x123);
        theirs.pixels[2 * 4] = 0xf00;
        assert!(!raced_only(&out, &theirs, 0), "no races, no excuse");
        out.beam_races = vec![(0x2c + 2, 0x2c + 3)];
        assert!(raced_only(&out, &theirs, 0));
        assert!(!raced_only(&out, &frame(0x123), 0), "nothing to excuse");
        theirs.pixels[4] = 0xf00;
        assert!(!raced_only(&out, &theirs, 0), "row 1 is not raced");
        assert!(raced_only(&out, &theirs, 2), "row 1 is under the panel");
    }

    #[test]
    fn a_frame_differing_only_on_raced_lines_does_not_diverge() {
        let case = Case::parse(
            "raced",
            "sync = \"power-on\"\nframes = 1\ncompare = [\"picture\"]",
        )
        .unwrap();
        let out = FrameOut {
            picture: ours(),
            beam_races: vec![(0x2c + 2, 0x2c + 3)],
            ..FrameOut::default()
        };
        let mut picture = frame(0x123);
        picture.pixels[2 * 4] = 0xf00;
        let theirs = FrameCapture {
            picture,
            paula: Vec::new(),
            audio: Vec::new(),
        };
        let mem = AmigaMemory {
            chip: Vec::new(),
            slow: Vec::new(),
        };
        assert!(super::frame(&case, 0, &[], &[], &mem, &out, &theirs, None, &[]).is_none());
    }
}
