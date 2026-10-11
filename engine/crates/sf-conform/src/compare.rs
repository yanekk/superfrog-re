//! One frame of the engine against one frame of the original (DESIGN §2.1).

use sf_core::layout::{AmigaMemory, LAYOUT, VarDef, VarId};
use sf_core::{FrameOut, Picture};
use sf_oracle::{FrameCapture, Rgb12Frame};
use sf_paula::PaulaWrite;

use crate::case::Case;
use crate::report::{self, Divergence};

/// The case's comparisons on frame `frame`: `None` when all match.
pub fn frame(
    case: &Case,
    frame: u32,
    probe: &[(VarId, u32)],
    theirs_mem: &AmigaMemory,
    ours: &FrameOut,
    theirs: &FrameCapture,
) -> Option<Divergence> {
    let c = case.compare;
    let first_var = c.vars.then(|| vars(probe, theirs_mem)).flatten();
    let picture_diff = c
        .picture
        .then(|| picture(&ours.picture, &theirs.picture))
        .flatten()
        .map(|d| report::write_diff_png(&case.name, frame, &ours.picture, &theirs.picture, d));
    let paula_diff = c.paula.then(|| paula(&ours.paula, &theirs.paula)).flatten();
    if first_var.is_none() && picture_diff.is_none() && paula_diff.is_none() {
        return None;
    }
    Some(Divergence {
        frame,
        first_var,
        picture_diff,
        paula_diff,
    })
}

/// The first `LAYOUT` piece whose engine value differs from the original's memory: (piece,
/// original's value, engine's value).
pub fn vars(probe: &[(VarId, u32)], theirs: &AmigaMemory) -> Option<(VarDef, u32, u32)> {
    let pieces: Vec<VarDef> = LAYOUT.iter().flat_map(|d| d.pieces()).collect();
    assert_eq!(
        pieces.len(),
        probe.len(),
        "probe and LAYOUT disagree on the number of pieces"
    );
    pieces
        .into_iter()
        .zip(probe)
        .map(|(p, &(id, ours))| {
            assert_eq!(p.id, id, "probe out of LAYOUT order");
            (p, p.read(theirs), ours)
        })
        .find(|(_, theirs, ours)| theirs != ours)
}

/// Where two pictures first differ, before the PNG is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelMismatch {
    /// (x, y, original's colour, engine's colour), row-major first; None when only the window
    /// differs.
    pub first: Option<(u16, u16, u16, u16)>,
    /// The display windows, when they differ: (original's, engine's) as `left,top widthxheight`.
    pub window: Option<(String, String)>,
}

fn window(left: u16, top: u16, w: u16, h: u16) -> String {
    format!("{left:#x},{top:#x} {w}x{h}")
}

/// The first pixel, row-major, at which the engine's picture differs from the original's, over the
/// area both cover; and whether the windows themselves differ.
pub fn picture(ours: &Picture, theirs: &Rgb12Frame) -> Option<PixelMismatch> {
    let a = window(theirs.left, theirs.top, theirs.width, theirs.height);
    let b = window(ours.left, ours.top, ours.width, ours.height);
    let win = (a != b).then_some((a, b));
    let (w, h) = (ours.width.min(theirs.width), ours.height.min(theirs.height));
    let first = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .map(|(x, y)| (x, y, theirs.pixel(x, y), ours.rgb12(x, y)))
        .find(|p| p.2 != p.3);
    (first.is_some() || win.is_some()).then_some(PixelMismatch { first, window: win })
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
    Some(format!(
        "write #{k}: original {}, engine {}{at}; {} writes in the original, {} in the engine",
        show(theirs.get(k)),
        show(ours.get(k)),
        theirs.len(),
        ours.len()
    ))
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
