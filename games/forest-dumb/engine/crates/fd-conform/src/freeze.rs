//! What a level case freezes in the original because the engine does not model it yet
//! (plans/forest-dumb-level-1 DESIGN §2.2): objects, once T00 and T21 have measured their entry
//! points. A frozen routine is patched to return at once (`rts`) in the original's code at the sync
//! point, so the original plays the level without it, as Superfrog's `freeze.rs` does.
//!
//! The list is empty until T21 names the object routines; a case asking to freeze before then is
//! an error rather than a run that silently freezes nothing.

use amiga_oracle::Amiga;

use crate::HUNK_BASES;

/// One routine frozen: its entry in the main program, and the task that ports it and so removes
/// it from this list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frozen {
    pub hunk: u8,
    pub offset: u32,
    pub what: &'static str,
    pub task: &'static str,
}

/// Every routine a `freeze = true` case returns from at once. T21 fills it.
pub const FROZEN: &[Frozen] = &[];

/// `rts`.
const RTS: [u8; 2] = [0x4e, 0x75];

/// Patch `list` into the original, or say why a case cannot freeze.
pub fn apply(am: &mut Amiga, list: &[Frozen]) -> Result<(), String> {
    if list.is_empty() {
        return Err(
            "freeze = true, but freeze.rs lists nothing to freeze yet: T21 measures \
                    the object entry points"
                .to_string(),
        );
    }
    for f in list {
        am.poke(HUNK_BASES[f.hunk as usize] + f.offset, &RTS);
    }
    Ok(())
}
