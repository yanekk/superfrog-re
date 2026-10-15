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

/// How a case is judged (DESIGN §2.1).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Mode {
    /// Checks 1–3: state pass by pass, picture within [`Tolerance`], sound as events. The
    /// default of `run`, `all` and `scan`.
    #[default]
    Faithful,
    /// Frame-exact, for diagnosis (`--strict`): every variable, pixel and Paula write each frame.
    Strict,
}

/// DESIGN §2.1's thresholds: the owner's bar. Not a per-case setting; a case cannot loosen them,
/// and a task that finds one wrong raises it with the owner rather than changing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tolerance {
    /// A pixel this many steps off in each of R, G and B or less is a colour step.
    pub colour_step: u8,
    /// A frame with fewer differing pixels than this is tolerated, however long it lasts.
    pub few_pixels: u32,
    /// Any other difference is tolerated for at most this many frames in a row…
    pub glitch_frames: u32,
    /// …and on at most this percentage of the case's frames.
    pub glitch_budget_pct: u32,
    /// A sound event matches the same event this many frames away or nearer.
    pub sound_frames: u32,
}

impl Tolerance {
    pub const OWNER: Tolerance = Tolerance {
        colour_step: 1,
        few_pixels: 64,
        glitch_frames: 2,
        glitch_budget_pct: 1,
        sound_frames: 2,
    };
}

impl Default for Tolerance {
    fn default() -> Tolerance {
        Tolerance::OWNER
    }
}

/// How a frame's picture differs from the original's (DESIGN §2.1 check 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PictureClass {
    Same,
    /// Every differing pixel at most [`Tolerance::colour_step`] off in each of R, G and B.
    ColourStep(u32),
    /// Fewer than [`Tolerance::few_pixels`] pixels differ.
    FewPixels(u32),
    /// Any other difference (the display window's among them): this many pixels.
    Glitch(u32),
}

/// [`PictureClass`] of the engine's picture against the original's, leaving out the picture rows
/// in `rows`.
pub fn classify(
    ours: &Picture,
    theirs: &Rgb12Frame,
    rows: &[Range<u16>],
    tol: &Tolerance,
) -> PictureClass {
    let windows_differ = (theirs.left as i32, theirs.top, theirs.width, theirs.height)
        != (ours.left, ours.top, ours.width, ours.height);
    let (w, h) = (ours.width.min(theirs.width), ours.height.min(theirs.height));
    let colours = sf_core::to_rgb12(ours);
    let (mut n, mut steps) = (0u32, true);
    for y in (0..h).filter(|y| !rows.iter().any(|r| r.contains(y))) {
        for x in 0..w {
            let (o, t) = (
                colours[y as usize * ours.width as usize + x as usize],
                theirs.pixel(x, y),
            );
            if o != t {
                n += 1;
                let off = |s: u16| ((o >> s & 15) as i16 - (t >> s & 15) as i16).unsigned_abs();
                steps &= [8, 4, 0].iter().all(|&s| off(s) <= tol.colour_step as u16);
            }
        }
    }
    match (windows_differ, n) {
        (false, 0) => PictureClass::Same,
        (false, n) if steps => PictureClass::ColourStep(n),
        (false, n) if n < tol.few_pixels => PictureClass::FewPixels(n),
        (_, n) => PictureClass::Glitch(n),
    }
}

/// Check 2 over a case: each frame's [`PictureClass`], the tolerated ones counted.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PictureJudge {
    tol: Tolerance,
    /// The case's frames, which the glitch budget is a percentage of.
    frames: u32,
    /// Glitch frames in a row now.
    run: u32,
    /// Frames tolerated as a colour step, as a few pixels, as a short glitch.
    pub colour_step: Vec<u32>,
    pub few_pixels: Vec<u32>,
    pub glitches: Vec<u32>,
}

impl PictureJudge {
    pub fn new(tol: Tolerance, frames: u32) -> PictureJudge {
        PictureJudge {
            tol,
            frames,
            ..PictureJudge::default()
        }
    }

    /// Frame `f` compared: why its difference is not tolerated, if it is not.
    pub fn frame(&mut self, f: u32, class: PictureClass) -> Result<(), String> {
        let n = match class {
            PictureClass::Glitch(n) => n,
            other => {
                self.run = 0;
                match other {
                    PictureClass::ColourStep(_) => self.colour_step.push(f),
                    PictureClass::FewPixels(_) => self.few_pixels.push(f),
                    _ => {}
                }
                return Ok(());
            }
        };
        self.run += 1;
        self.glitches.push(f);
        let (limit, pct) = (self.tol.glitch_frames, self.tol.glitch_budget_pct);
        if self.run > limit {
            return Err(format!(
                "{n} pixels differ, {} frames in a row past one colour step: more than {limit}",
                self.run
            ));
        }
        if self.glitches.len() as u64 * 100 > self.frames as u64 * pct as u64 {
            return Err(format!(
                "{n} pixels differ: the {}th frame past the tolerance, over {pct}% of the case's {} frames",
                self.glitches.len(),
                self.frames
            ));
        }
        Ok(())
    }
}

/// Check 1's timing: the state is compared at the end of each main-loop pass, the original's
/// `n`th with the engine's `n`th, so a pass ending a frame later on one side is not a difference.
/// The runner holds the state of the side that ended a pass first, and compares it once the other
/// side ends the same pass. A pass whose end is not seen (another began and was still running at
/// the frame's end) is not compared.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PassMatch {
    theirs: u64,
    ours: u64,
    /// The side whose state at the end of its pass `n` is held.
    held: Option<(Side, u64)>,
    /// Frames on which one side had ended more passes than the other.
    pub skews: Vec<u32>,
}

/// A side of a comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    Theirs,
    Ours,
}

/// What [`PassMatch::frame`] asks of the runner, in this order.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct PassStep {
    /// The other side has ended the pass held: compare its state now with the one held.
    pub compare_held: bool,
    /// Both sides ended the same pass now: compare now.
    pub compare_now: bool,
    /// This side ended a pass the other has not: hold its state now for when the other does.
    pub hold: Option<Side>,
    /// The sides are more than a pass apart: (the original's passes, the engine's).
    pub apart: Option<(u64, u64)>,
}

impl PassMatch {
    /// Frame `f` ended `theirs` passes on the original's side and `ours` on the engine's.
    pub fn frame(&mut self, f: u32, theirs: u32, ours: u32) -> PassStep {
        self.theirs += theirs as u64;
        self.ours += ours as u64;
        let (t, o) = (self.theirs, self.ours);
        if t != o {
            self.skews.push(f);
        }
        let mut step = PassStep::default();
        if t.abs_diff(o) > 1 {
            step.apart = Some((t, o));
            self.held = None;
            return step;
        }
        if let Some((side, n)) = self.held {
            let (count, ended) = match side {
                Side::Theirs => (o, ours),
                Side::Ours => (t, theirs),
            };
            if count >= n {
                // The other side's pass `n` ended now (or, past it, unseen).
                step.compare_held = count == n && ended > 0;
                self.held = None;
            }
        }
        let ahead = match t.cmp(&o) {
            std::cmp::Ordering::Greater => (theirs > 0).then_some((Side::Theirs, t)),
            std::cmp::Ordering::Less => (ours > 0).then_some((Side::Ours, o)),
            std::cmp::Ordering::Equal => {
                step.compare_now = theirs > 0 && ours > 0;
                None
            }
        };
        if let Some(h) = ahead {
            self.held = Some(h);
            step.hold = Some(h.0);
        }
        step
    }

    /// The side held can no longer be (its state at the pass's end was not seen).
    pub fn drop_held(&mut self) {
        self.held = None;
    }

    /// The engine re-synced from the original's memory: the sides start level again.
    pub fn level(&mut self) {
        self.ours = self.theirs;
        self.held = None;
    }
}

/// [`vars_except`] for the faithful state check: leaves out what `case` leaves out and every
/// `presentation` variable (DESIGN §2.1 check 1).
pub fn gameplay_vars(
    case: &Case,
    layout: &[VarDef],
    probe: &[(VarId, u32)],
    theirs: &AmigaMemory,
    masked: &[(u32, u32)],
) -> Option<(VarDef, u32, u32)> {
    vars_except(layout, probe, theirs, masked, |d| {
        sf_core::layout::presentation(d.id).is_some() || case_skips(case, d)
    })
}

/// Whether `case` leaves variable `d` out of its comparison (the panel, a `vars` list).
pub fn case_skips(case: &Case, d: &VarDef) -> bool {
    (case.excludes(crate::case::Excluded::Panel) && d.id == VarId::PanelBitmap)
        || case
            .only_vars
            .as_ref()
            .is_some_and(|only| !only.iter().any(|n| n == d.name))
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
