//! The numeric tables the game's code reads, each decoded from the game executable (`d1_c03`) at
//! the offset its doc comment names (DESIGN §2.3: from the disks, never from our source).
//!
//! The pattern for every mechanic task: one typed field here, read in [`Tables::read`] from
//! `ExeImage` at a named hunk offset, the offset in the field's doc comment and in
//! `re/symbols.toml`. Addresses inside a table are kept as the file has them: unrelocated, so they
//! read as offsets into hunk 0 (`h0+$x`).

use crate::error::{AssetError, Why};
use crate::exe::ExeImage;

/// One `file_table` entry (docs/08 "Files, disks and level composition").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileEntry {
    pub start_sector: u16,
    pub sectors: u16,
    /// 0..3; `disk_ids` gives the disk ID each index needs (disks 2 and 4 share sector numbers).
    pub disk: u16,
}

/// One `password_table` entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PasswordEntry {
    /// Six ASCII digits.
    pub digits: [u8; 6],
    /// The routine that starts the game there, as an `h0` offset.
    pub handler: u32,
}

/// One entry of the high-score table the game starts with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HiscoreEntry {
    pub score: u32,
    /// The second long of the entry (zero in the shipped table; inferred: the name pointer).
    pub rest: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tables {
    /// `file_table`, d1_c03 h0+$cbfe: 95 entries of 6 bytes.
    pub file_table: Vec<FileEntry>,
    /// `disk_id_table`, d1_c03 h0+$b11e: the 4-byte ID (at disk offset `$dbffc`) per disk index.
    pub disk_ids: [[u8; 4]; 4],
    /// `password_table`, d1_c03 h0+$24dc: 28 entries of 10 bytes, six digits then the handler
    /// long. Entries 0–23 start W1L1..W6L4, 24–27 the bonus level (docs/08).
    pub passwords: Vec<PasswordEntry>,
    /// `start_resume_table`, d1_c03 h0+$bde: per world (7 rows of $14 bytes) the world's entry
    /// and the resume points for levels 1..4, as `h0` offsets.
    pub start_resume: Vec<[u32; 5]>,
    /// `hiscore_table`, d1_c03 h0+$2f90: 10 entries of 8 bytes (inferred layout).
    pub hiscores: Vec<HiscoreEntry>,
    /// `obj_flag_jumptab`, d1_c03 h0+$178ec: 7 worlds × 64 `bra.w`, one per map-cell marker
    /// (docs/09). Kept as each branch's target, an `h0` offset.
    pub obj_marker_stubs: Vec<[u32; 64]>,
    /// `obj_templates`, d1_c03 h0+$1c6e2: 160 object templates of $ae bytes (docs/09 "The slot
    /// record"), as stored. Their routine pointers are unrelocated `h0` offsets.
    pub obj_templates: Vec<Vec<u8>>,
}

impl Tables {
    pub(crate) fn read(game: &ExeImage) -> Result<Tables, AssetError> {
        let r = game.reader(0, "game tables");
        let file_table = (0..95)
            .map(|k| {
                let at = 0xcbfe + 6 * k;
                Ok(FileEntry {
                    start_sector: r.u16(at)?,
                    sectors: r.u16(at + 2)?,
                    disk: r.u16(at + 4)?,
                })
            })
            .collect::<Result<_, AssetError>>()?;
        let mut disk_ids = [[0u8; 4]; 4];
        for (k, id) in disk_ids.iter_mut().enumerate() {
            id.copy_from_slice(r.bytes(0xb11e + 4 * k, 4)?);
        }
        let passwords = (0..28)
            .map(|k| {
                let at = 0x24dc + 10 * k;
                let digits: [u8; 6] = r.bytes(at, 6)?.try_into().unwrap();
                if !digits.iter().all(u8::is_ascii_digit) {
                    return Err(r.err(Why::Malformed(format!("password {k} is not six digits"))));
                }
                Ok(PasswordEntry {
                    digits,
                    handler: r.u32(at + 6)?,
                })
            })
            .collect::<Result<_, AssetError>>()?;
        let start_resume = (0..7)
            .map(|w| {
                let mut row = [0u32; 5];
                for (k, v) in row.iter_mut().enumerate() {
                    *v = r.u32(0xbde + 0x14 * w + 4 * k)?;
                }
                Ok(row)
            })
            .collect::<Result<_, AssetError>>()?;
        let hiscores = (0..10)
            .map(|k| {
                Ok(HiscoreEntry {
                    score: r.u32(0x2f90 + 8 * k)?,
                    rest: r.u32(0x2f94 + 8 * k)?,
                })
            })
            .collect::<Result<_, AssetError>>()?;
        let obj_marker_stubs = (0..7)
            .map(|w| {
                let mut row = [0u32; 64];
                for (m, target) in row.iter_mut().enumerate() {
                    let at = 0x178ec + 0x100 * w + 4 * m;
                    let (op, disp) = (r.u16(at)?, r.u16(at + 2)? as i16);
                    if op != 0x6000 {
                        return Err(r.err(Why::Malformed(format!(
                            "obj_flag_jumptab world {w} marker {m}: {op:#06x} is not bra.w"
                        ))));
                    }
                    *target = (at as i64 + 2 + disp as i64) as u32;
                }
                Ok(row)
            })
            .collect::<Result<_, AssetError>>()?;
        let obj_templates = (0..160)
            .map(|k| Ok(r.bytes(0x1c6e2 + 0xae * k, 0xae)?.to_vec()))
            .collect::<Result<_, AssetError>>()?;
        Ok(Tables {
            file_table,
            disk_ids,
            passwords,
            start_resume,
            hiscores,
            obj_marker_stubs,
            obj_templates,
        })
    }
}
