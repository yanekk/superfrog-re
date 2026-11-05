//! What a level case freezes in the original because the engine does not model it yet
//! (plans/forest-dumb-level-1 DESIGN §2.2): objects, at the entry points T21 measured
//! (`docs/10-object-system.md`). A frozen routine is patched to return at once (`rts`) in the
//! original's code at the sync point, so the original plays the level without it, as Superfrog's
//! `freeze.rs` does. `tests/object_claims.rs` shows that with these patched every object value of
//! W1-1 does nothing and the character moves as on an empty object map.
//!
//! Each entry names the task whose port removes it. What freezing leaves in place: the object map
//! itself, the per-pass creature loop over empty slots, and `char_hit_check`'s reaction to CLXDAT,
//! which the oracle never sets until T59.

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

/// Every routine a `freeze = true` case returns from at once.
pub const FROZEN: &[Frozen] = &[
    Frozen {
        hunk: 0,
        offset: 0x681c,
        what: "h0_681c: a creature value (1 to 6) in view takes a slot; returning leaves d0 non-zero, \
               so the cell stays and no creature ever exists",
        task: "T24",
    },
    Frozen {
        hunk: 0,
        offset: 0x6882,
        what: "h0_6882: a value $20-$3f in view joins h0_62b6's animated cells (items, water, \
               vanishing blocks), so none is drawn and no vanishing block writes collision_map",
        task: "T25",
    },
    Frozen {
        hunk: 0,
        offset: 0xa06,
        what: "h0_a06: the bottom edge redraw blits a bonus ($40+)",
        task: "T22",
    },
    Frozen {
        hunk: 0,
        offset: 0xce4,
        what: "h0_ce4: the top edge redraw blits a bonus ($40+)",
        task: "T22",
    },
    Frozen {
        hunk: 0,
        offset: 0x104c,
        what: "h0_104c: the right edge redraw blits a bonus ($40+)",
        task: "T22",
    },
    Frozen {
        hunk: 0,
        offset: 0x1390,
        what: "h0_1390: the left edge redraw blits a bonus ($40+)",
        task: "T22",
    },
    Frozen {
        hunk: 0,
        offset: 0x26ec,
        what: "h0_26ec: an item, bonus or the exit under the character is taken; the exit is \
               frozen with them, so a frozen case cannot end the level",
        task: "T22",
    },
    Frozen {
        hunk: 0,
        offset: 0x24ce,
        what: "h0_24ce: a spring ($10-$13) under the feet throws the character",
        task: "T25",
    },
];

/// `rts`.
const RTS: [u8; 2] = [0x4e, 0x75];

/// Patch `list` into the original, or say why a case cannot freeze.
pub fn apply(am: &mut Amiga, list: &[Frozen]) -> Result<(), String> {
    if list.is_empty() {
        return Err("freeze = true, but the list to freeze is empty".to_string());
    }
    for f in list {
        am.poke(HUNK_BASES[f.hunk as usize] + f.offset, &RTS);
    }
    Ok(())
}
