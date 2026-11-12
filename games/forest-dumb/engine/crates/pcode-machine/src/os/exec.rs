//! exec.library: opening and closing the libraries the machine answers.

use pcode_rt::{D0, Regs, Stop};

use super::{a, d, library_by_base, library_by_name};
use crate::Machine;

pub const OLD_OPEN_LIBRARY: i32 = -408;
pub const CLOSE_LIBRARY: i32 = -414;
pub const OPEN_LIBRARY: i32 = -552;

/// The newest library version Kickstart 1.3 has: `OpenLibrary` asking for more gets 0, as there.
pub const VERSION: u32 = 34;

/// The answer to exec's vector `lvo`, or `None` for one the machine does not answer.
pub(crate) fn call(
    m: &mut Machine,
    lvo: i32,
    r: &mut Regs,
    site: &str,
) -> Option<Result<(), Stop>> {
    Some(match lvo {
        OLD_OPEN_LIBRARY => open(m, r, None, site),
        OPEN_LIBRARY => open(m, r, Some(d(r, 0)), site),
        // Libraries are never expunged here, so closing one changes nothing; a base that is no
        // library's would be the program's bug, and the OS would not notice it either.
        CLOSE_LIBRARY => match library_by_base(a(r, 1)) {
            Some(_) => Ok(()),
            None => Err(Stop::Os(format!(
                "CloseLibrary of ${:08x}, no library's base, at {site}",
                a(r, 1)
            ))),
        },
        _ => return None,
    })
}

/// `OldOpenLibrary(a1 = name)`, or `OpenLibrary(a1 = name, d0 = version)`: the base in `d0`. A
/// library the machine does not answer stops rather than returning 0, which would be a guess: on
/// an Amiga it would open, and the program would go on to call it.
fn open(m: &Machine, r: &mut Regs, version: Option<u32>, site: &str) -> Result<(), Stop> {
    let name = m.c_string(a(r, 1), "a library name", site)?;
    let Some(lib) = library_by_name(&name) else {
        return Err(Stop::Os(format!(
            "OpenLibrary of {:?}, a library the machine does not answer, at {site}",
            String::from_utf8_lossy(&name)
        )));
    };
    let base = match version {
        Some(v) if v > VERSION => 0,
        _ => lib.base(),
    };
    r.set(D0, 4, base as u64);
    Ok(())
}
