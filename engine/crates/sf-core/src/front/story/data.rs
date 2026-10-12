//! What the story intro reads from the disks: disk 3's boot program (`d3_c02`), the intro program
//! (`d3_c04`) and its chip hunks (`d3_c03`), each relocated where the original's loaders put them,
//! and the four picture banks (`d3_c05`..`d3_c08`) as its own loader leaves them.

use sf_assets::{ExeId, ExeImage, GameData};

use crate::front::data::relocate;

/// Where disk 3's bootblock loads the boot program `d3_c02` (measured on the oracle).
pub const BOOT_BASE: u32 = 0x519d0;
/// The intro's hunks as `d3_c02`'s relocator places them: hunk 0 where the game's goes, hunk 1
/// after it, hunks 2 and 3 in chip RAM from `$400` (measured on the oracle).
pub const INTRO_H0: u32 = 0xc0_0fa0;
pub const INTRO_H1: u32 = 0xc5_3100;
pub const INTRO_H2: u32 = 0x400;
pub const INTRO_H3: u32 = 0x1_8fd4;
/// The BSS hunks' sizes, from the intro's header.
pub const INTRO_H1_BYTES: u32 = 0x2_1340;
pub const INTRO_H3_BYTES: u32 = 0x6_26b0;
const INTRO_BASES: [u32; 4] = [INTRO_H0, INTRO_H1, INTRO_H2, INTRO_H3];

/// The story's disk content.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoryData {
    /// `d3_c02`'s hunk 0 as loaded at [`BOOT_BASE`].
    pub boot: Vec<u8>,
    /// `d3_c04`'s hunk 0 as loaded at [`INTRO_H0`].
    pub intro: Vec<u8>,
    /// Hunk 2 (`d3_c03`'s data block) as loaded at [`INTRO_H2`].
    pub chip: Vec<u8>,
    /// The picture banks `d3_c05`..`d3_c08`, as the intro's `load_file` leaves them (unpacked).
    pub banks: [Vec<u8>; 4],
    /// The sector spans `intro_file_table` names, raw: what `load_file` (h0+$c3fa) reads.
    pub spans: [Vec<u8>; 4],
}

impl StoryData {
    pub fn new(data: &GameData) -> StoryData {
        let exe = |id| data.exes.iter().find(|e: &&ExeImage| e.id == id);
        let boot = exe(ExeId::Disk3Boot)
            .map(|e| relocate(e, 0, &[BOOT_BASE]))
            .unwrap_or_default();
        let intro = exe(ExeId::Intro)
            .map(|e| relocate(e, 0, &INTRO_BASES))
            .unwrap_or_default();
        let chip = data
            .raw
            .get("intro-sound")
            .map(|b| data_hunk(b, &INTRO_BASES))
            .unwrap_or_default();
        let bank = |n: usize| {
            data.raw
                .get(&format!("intro-gfx-{n}"))
                .cloned()
                .unwrap_or_default()
        };
        let span = |n: usize| {
            data.raw
                .get(&format!("intro-span-{n}"))
                .cloned()
                .unwrap_or_default()
        };
        StoryData {
            boot,
            intro,
            chip,
            banks: [bank(1), bank(2), bank(3), bank(4)],
            spans: [span(1), span(2), span(3), span(4)],
        }
    }
}

/// A `HUNK_DATA` block (`$3ea`, length in longs, the data) and the `HUNK_RELOC32` groups that
/// follow it, relocated with `bases`: how `d3_c03` carries the intro's hunk 2 (its hunk 3, BSS,
/// follows and is left out). Anything malformed gives what was read so far.
fn data_hunk(b: &[u8], bases: &[u32]) -> Vec<u8> {
    let long = |at: usize| {
        b.get(at..at + 4)
            .map(|x| u32::from_be_bytes([x[0], x[1], x[2], x[3]]))
    };
    if long(0) != Some(0x3ea) {
        return Vec::new();
    }
    let n = long(4).unwrap_or(0) as usize * 4;
    let mut out = b.get(8..8 + n).unwrap_or(&[]).to_vec();
    let mut at = 8 + n;
    if long(at) == Some(0x3ec) {
        at += 4;
        while let Some(count) = long(at).filter(|&c| c != 0) {
            let base = long(at + 4)
                .and_then(|t| bases.get(t as usize))
                .copied()
                .unwrap_or(0);
            for k in 0..count as usize {
                let Some(o) = long(at + 8 + 4 * k) else {
                    return out;
                };
                let o = o as usize;
                if let Some(x) = out.get_mut(o..o + 4) {
                    let v = u32::from_be_bytes([x[0], x[1], x[2], x[3]]).wrapping_add(base);
                    x.copy_from_slice(&v.to_be_bytes());
                }
            }
            at += 8 + 4 * count as usize;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_data_hunk_is_relocated_by_its_reloc32_groups() {
        let mut b = Vec::new();
        for l in [0x3ea, 2, 0x10, 0x20, 0x3ec, 1, 2, 4, 0, 0x3f2] {
            b.extend(u32::to_be_bytes(l));
        }
        let out = data_hunk(&b, &[0, 0, 0x400, 0]);
        assert_eq!(out, [0, 0, 0, 0x10, 0, 0, 4, 0x20]);
        assert!(data_hunk(&[0; 8], &[]).is_empty());
    }
}
