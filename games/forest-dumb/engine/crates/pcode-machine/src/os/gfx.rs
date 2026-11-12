//! graphics.library: the calls a program makes at start and on its way out of the OS.

use pcode_rt::{D0, Regs, Stop};

use super::{a, layout};
use crate::Machine;
use crate::mem::ROM_BASE;

pub const OPEN_FONT: i32 = -72;
pub const CLOSE_FONT: i32 = -78;
pub const FREE_SPRITE: i32 = -414;

/// The one font in ROM the machine answers, by `OpenFont`'s name.
pub const TOPAZ: &str = "topaz.font";

/// The answer to graphics' vector `lvo`, or `None` for one the machine does not answer.
pub(crate) fn call(
    m: &mut Machine,
    lvo: i32,
    r: &mut Regs,
    site: &str,
) -> Option<Result<(), Stop>> {
    Some(match lvo {
        OPEN_FONT => open_font(m, r, site),
        // Nothing a program can see changes: the font stays in ROM.
        CLOSE_FONT => Ok(()),
        // Frees sprite d0 in GfxBase's reservation bits, which nothing here reads (a program that
        // took the hardware never asked for one).
        FREE_SPRITE => Ok(()),
        _ => return None,
    })
}

/// `OpenFont(a0 = TextAttr)`: topaz 8, the ROM font, in `d0`; any other stops, since its
/// TextFont would be a guess.
fn open_font(m: &Machine, r: &mut Regs, site: &str) -> Result<(), Stop> {
    let attr = a(r, 0);
    let (Some(name_at), Some(size)) = (m.peek_l(attr), m.peek_w(attr.wrapping_add(4))) else {
        return Err(Stop::Os(format!(
            "OpenFont's TextAttr at ${attr:06x} is outside memory, at {site}"
        )));
    };
    let name = m.c_string(name_at, "a font name", site)?;
    if !name.eq_ignore_ascii_case(TOPAZ.as_bytes()) || size != 8 {
        return Err(Stop::Os(format!(
            "OpenFont of {:?} size {size}, not topaz 8, at {site}",
            String::from_utf8_lossy(&name)
        )));
    }
    r.set(D0, 4, (ROM_BASE + layout::TOPAZ) as u64);
    Ok(())
}
