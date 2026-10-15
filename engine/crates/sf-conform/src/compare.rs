//! One frame of the engine against one frame of the original (DESIGN §2.1).

use std::ops::Range;

use sf_core::layout::{AmigaMemory, VarDef, VarId};
use sf_core::{FrameOut, Picture};
use sf_oracle::{FrameCapture, Rgb12Frame};
use sf_paula::PaulaWrite;

use crate::case::Case;
use crate::report::{self, Divergence};

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
    let skip_panel = case.excludes(crate::case::Excluded::Panel);
    let first_var = c
        .vars
        .then(|| {
            vars_except(layout, probe, theirs_mem, masked, |d| {
                (skip_panel && d.id == VarId::PanelBitmap)
                    || case
                        .only_vars
                        .as_ref()
                        .is_some_and(|only| !only.iter().any(|n| n == d.name))
            })
        })
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
        resync: false,
        lag: None,
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
/// compared: memory a job of the original's CPU has not finished writing (DESIGN §2.1).
pub fn vars(
    layout: &[VarDef],
    probe: &[(VarId, u32)],
    theirs: &AmigaMemory,
    masked: &[(u32, u32)],
) -> Option<(VarDef, u32, u32)> {
    vars_except(layout, probe, theirs, masked, |_| false)
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

/// Where two pictures first differ, before the PNG is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelMismatch {
    /// (x, y, original's colour, engine's colour), row-major first; None when only the window
    /// differs.
    pub first: Option<(u16, u16, u16, u16)>,
    /// The display windows, when they differ: (original's, engine's) as `left,top widthxheight`.
    pub window: Option<(String, String)>,
    /// Rows that differ: (first, last, how many pixels in all).
    pub rows: Option<(u16, u16, u32)>,
}

fn window(left: i32, top: u16, w: u16, h: u16) -> String {
    format!("{left:#x},{top:#x} {w}x{h}")
}

/// The first pixel, row-major, at which the engine's picture differs from the original's, over the
/// area both cover; and whether the windows themselves differ.
pub fn picture(ours: &Picture, theirs: &Rgb12Frame) -> Option<PixelMismatch> {
    picture_from(ours, theirs, 0)
}

/// [`picture`] from row `from` down.
pub fn picture_from(ours: &Picture, theirs: &Rgb12Frame, from: u16) -> Option<PixelMismatch> {
    picture_except(ours, theirs, std::slice::from_ref(&(0..from)))
}

/// The picture rows of a frame's beam races (`FrameOut::beam_races`, raster lines) in a picture
/// whose first row is raster line `top`.
pub fn race_rows(races: &[(u16, u16)], top: u16) -> Vec<Range<u16>> {
    races
        .iter()
        .map(|&(a, b)| a.saturating_sub(top)..b.saturating_sub(top))
        .collect()
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

/// [`picture`], leaving out the picture rows in `rows`.
pub fn picture_except(
    ours: &Picture,
    theirs: &Rgb12Frame,
    rows: &[Range<u16>],
) -> Option<PixelMismatch> {
    let a = window(theirs.left as i32, theirs.top, theirs.width, theirs.height);
    let b = window(ours.left, ours.top, ours.width, ours.height);
    let win = (a != b).then_some((a, b));
    let (w, h) = (ours.width.min(theirs.width), ours.height.min(theirs.height));
    let colours = sf_core::to_rgb12(ours);
    let differing: Vec<(u16, u16, u16, u16)> = (0..h)
        .filter(|y| !rows.iter().any(|r| r.contains(y)))
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| {
            let o = colours[y as usize * ours.width as usize + x as usize];
            (x, y, theirs.pixel(x, y), o)
        })
        .filter(|p| p.2 != p.3)
        .collect();
    let first = differing.first().copied();
    let rows = first.map(|f| {
        let last = differing.last().map_or(f.1, |l| l.1);
        (f.1, last, differing.len() as u32)
    });
    (first.is_some() || win.is_some()).then_some(PixelMismatch {
        first,
        window: win,
        rows,
    })
}

/// The first Paula write that differs in register or value (beam positions are the original's
/// alone; the engine is not cycle-timed), or a missing or extra write.
pub fn paula(ours: &[PaulaWrite], theirs: &[PaulaWrite]) -> Option<String> {
    let k = ours
        .iter()
        .zip(theirs)
        .position(|(a, b)| a.reg_value() != b.reg_value())
        .or((ours.len() != theirs.len()).then(|| ours.len().min(theirs.len())))?;
    let show = |w: Option<&PaulaWrite>| match w {
        Some(w) => format!("{} = ${:04x}", reg_name(w.reg), w.value),
        None => "nothing".to_string(),
    };
    let at = theirs
        .get(k)
        .map(|w| format!(" (original's beam at line {}, clock {})", w.vpos, w.hpos))
        .unwrap_or_default();
    let list = |ws: &[PaulaWrite]| {
        ws.iter()
            .map(|w| format!("{}=${:04x}", reg_name(w.reg), w.value))
            .collect::<Vec<_>>()
            .join(" ")
    };
    let mut text = format!(
        "write #{k}: original {}, engine {}{at}; {} writes in the original, {} in the engine",
        show(theirs.get(k)),
        show(ours.get(k)),
        theirs.len(),
        ours.len()
    );
    if theirs.len().max(ours.len()) <= 40 {
        text += &format!(
            "\n    original: {}\n    engine:   {}",
            list(theirs),
            list(ours)
        );
    } else {
        let from = k.saturating_sub(2);
        let around = |ws: &[PaulaWrite]| list(&ws[from.min(ws.len())..(from + 6).min(ws.len())]);
        text += &format!(
            "\n    original from #{from}: {}\n    engine from #{from}: {}",
            around(theirs),
            around(ours)
        );
    }
    Some(text)
}

/// A Paula register's name from its offset from `$DFF000`.
pub fn reg_name(reg: u16) -> String {
    match reg {
        0x96 => "DMACON".into(),
        0x9A => "INTENA".into(),
        0x9C => "INTREQ".into(),
        0x9E => "ADKCON".into(),
        0xA0..=0xDF => {
            let ch = (reg - 0xA0) / 0x10;
            let r = match (reg - 0xA0) % 0x10 {
                0x0 => "LCH",
                0x2 => "LCL",
                0x4 => "LEN",
                0x6 => "PER",
                0x8 => "VOL",
                0xA => "DAT",
                _ => "?",
            };
            format!("AUD{ch}{r}")
        }
        _ => format!("${reg:03x}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(reg: u16, value: u16) -> PaulaWrite {
        PaulaWrite {
            reg,
            value,
            hpos: 0,
            vpos: 0,
        }
    }

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
    fn picture_except_ignores_exactly_the_given_rows() {
        let mut theirs = frame(0x123);
        assert_eq!(picture_except(&ours(), &theirs, &[]), None);
        // Rows 2 and 3 differ.
        for x in 0..4 {
            theirs.pixels[2 * 4 + x] = 0xf00;
            theirs.pixels[3 * 4 + x] = 0xf00;
        }
        assert_eq!(picture_except(&ours(), &theirs, &[2..4]), None);
        assert_eq!(
            picture_except(&ours(), &theirs, &[1..3]).unwrap().first,
            Some((0, 3, 0xf00, 0x123))
        );
        assert_eq!(
            picture_except(&ours(), &theirs, &[]).unwrap().rows,
            Some((2, 3, 8))
        );
        // A differing pixel outside the rows still diverges.
        theirs.pixels[5 * 4 + 1] = 0x0f0;
        assert_eq!(
            picture_except(&ours(), &theirs, &[2..4]).unwrap().first,
            Some((1, 5, 0x0f0, 0x123))
        );
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

    #[test]
    fn paula_names_the_first_differing_write_and_its_index() {
        let theirs = [w(0x96, 0x8201), w(0xA8, 64), w(0xB8, 40)];
        assert_eq!(paula(&theirs, &theirs), None);
        let ours = [w(0x96, 0x8201), w(0xA8, 63), w(0xB8, 40)];
        let d = paula(&ours, &theirs).unwrap();
        assert!(
            d.starts_with("write #1: original AUD0VOL = $0040, engine AUD0VOL = $003f"),
            "{d}"
        );
        let d = paula(&ours[..1], &theirs).unwrap();
        assert!(
            d.starts_with("write #1: original AUD0VOL = $0040, engine nothing"),
            "{d}"
        );
        // Beam position alone is not a difference.
        let mut moved = theirs;
        moved[2].hpos = 99;
        assert_eq!(paula(&moved, &theirs), None);
        assert_eq!(reg_name(0xD6), "AUD3PER");
    }
}
