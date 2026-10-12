//! Sound read from the disks (DESIGN §2.3): the music modules, the sound-effect banks, the samples
//! the game streams on channel 3 and the replayer's own tables, all as the game loads them
//! (docs/11-sound.md).
//!
//! Files are read the way the game's `load_file` reads them: through its `file_table` (`h0+$cbfe`
//! in the game executable `d1_c03`), a start sector and a sector count on one disk, then
//! `ATN!`-decompressed. Reading whole sectors is what makes W1's music (`d1_c16`) decode: its
//! trailer lies in the sector after the carver's last one (docs/03).

use std::fmt;

use sf_disk::{AtnError, DiskSet, DiskSlot, carve, decompress};

/// Where the bootstrap puts the game's hunk 0 (docs/08): `h0+$x` is `H0 + x`.
pub const H0: u32 = 0xC0_0FA0;
/// Where hunk 3 sits in chip RAM (docs/08).
pub const H3: u32 = 0x77FC;
/// Chip address an effect bank (`LnFX`) is loaded at, `h3+$64c70`; the title loads its module here too.
pub const BANK_ADDR: u32 = H3 + 0x64c70;
/// Chip buffer the streamed sample is copied into, 160 bytes a frame, `h3+$64b70`.
pub const STREAM_BUFFER: u32 = H3 + 0x64b70;

/// `file_table` in hunk 0: entries of three words (start sector, sector count, disk index 0..3).
const FILE_TABLE: usize = 0xcbfe;
const FILE_COUNT: usize = 95;
const SECTOR: usize = 512;

/// `sample_play`'s eight 16-byte entries: data pointer (into hunk 0), byte length, volume.
const STREAM_TABLE: usize = 0x1491e;
pub const STREAM_SAMPLES: usize = 8;

/// A music module, by the file the game loads it from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TuneId {
    /// `MUSC`: the title, menus, tables and credits.
    Title,
    /// `LnMU` for world 1..=6.
    World(u8),
    /// `LBMU`: the bonus level.
    Bonus,
    /// `LFMU`: the slot machine.
    Slot,
}

impl TuneId {
    pub const ALL: [TuneId; 9] = [
        TuneId::Title,
        TuneId::World(1),
        TuneId::World(2),
        TuneId::World(3),
        TuneId::World(4),
        TuneId::World(5),
        TuneId::World(6),
        TuneId::Bonus,
        TuneId::Slot,
    ];

    /// The `file_table` entry the game loads it from.
    fn file(self) -> usize {
        match self {
            TuneId::Title => 11,
            TuneId::World(1) => 22,
            TuneId::World(2) => 32,
            TuneId::World(3) => 53,
            TuneId::World(4) => 63,
            TuneId::World(5) => 73,
            TuneId::World(6) => 90,
            TuneId::World(w) => panic!("there is no world {w}"),
            TuneId::Bonus => 84,
            TuneId::Slot => 21,
        }
    }
}

/// A sound-effect bank (`LnFX`), by the file the game loads it from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BankId {
    /// `L1FX` from disk 1: worlds 1 and 2.
    Worlds12,
    /// `L1FX` from disk 2 (the same name): worlds 3 to 6.
    Worlds36,
    /// `LBFX`: the bonus level.
    Bonus,
    /// `LFFX`: the slot machine.
    Slot,
}

impl BankId {
    pub const ALL: [BankId; 4] = [
        BankId::Worlds12,
        BankId::Worlds36,
        BankId::Bonus,
        BankId::Slot,
    ];

    fn file(self) -> usize {
        match self {
            BankId::Worlds12 => 16,
            BankId::Worlds36 => 47,
            BankId::Bonus => 83,
            BankId::Slot => 20,
        }
    }
}

/// One sample `sample_play` streams on channel 3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamSample {
    /// The data's address in RAM (hunk 0, slow RAM): where the streamer copies from.
    pub addr: u32,
    /// The sample bytes (the length field's worth; the streamer copies 160 at a time and may read
    /// past the end, into whatever follows it in hunk 0).
    pub bytes: Vec<u8>,
    /// The length field, in bytes.
    pub len: u32,
    /// The volume word.
    pub volume: u16,
}

/// Everything the audio driver reads that comes from the disks.
#[derive(Clone, PartialEq, Eq)]
pub struct SoundData {
    /// The game's hunk 0 as in the file: the replayer's tables (arpeggio steps `$13548`, periods
    /// `$135a2`, finetune offsets `$13f5e`, vibrato `$140bc`) and the driver's power-on variables.
    h0: Vec<u8>,
    tunes: Vec<(TuneId, Vec<u8>)>,
    banks: Vec<(BankId, Vec<u8>)>,
    pub stream: Vec<StreamSample>,
}

impl fmt::Debug for SoundData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SoundData")
            .field(
                "tunes",
                &self
                    .tunes
                    .iter()
                    .map(|t| (t.0, t.1.len()))
                    .collect::<Vec<_>>(),
            )
            .field(
                "banks",
                &self
                    .banks
                    .iter()
                    .map(|b| (b.0, b.1.len()))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SoundError {
    /// The game executable `d1_c03` is not on disk 1 or will not decompress.
    NoGame,
    /// `d1_c03` is not a hunk executable whose first hunk holds the tables.
    BadGame,
    /// A `file_table` entry points outside its disk.
    FileOutsideDisk { file: usize },
    /// A file did not decompress.
    Atn { file: usize, err: AtnError },
    /// A module without the `P41A` tag.
    NotAModule(TuneId),
}

impl fmt::Display for SoundError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SoundError::NoGame => write!(
                f,
                "the game executable (disk 1 chunk 3) is missing or damaged"
            ),
            SoundError::BadGame => write!(
                f,
                "the game executable (disk 1 chunk 3) is not the expected program"
            ),
            SoundError::FileOutsideDisk { file } => {
                write!(f, "file table entry {file} points outside its disk")
            }
            SoundError::Atn { file, err } => {
                write!(f, "file table entry {file} does not decompress: {err}")
            }
            SoundError::NotAModule(t) => write!(f, "{t:?} is not a P41A module"),
        }
    }
}

impl std::error::Error for SoundError {}

/// The game executable's first hunk as it is in the file: code and data, not relocated.
pub fn game_hunk0(disks: &DiskSet) -> Result<Vec<u8>, SoundError> {
    let chunk = carve(disks.disk(DiskSlot::D1))
        .into_iter()
        .find(|c| c.index == 3)
        .ok_or(SoundError::NoGame)?;
    let exe = decompress(&chunk.raw).map_err(|_| SoundError::NoGame)?;
    first_hunk(&exe).ok_or(SoundError::BadGame)
}

/// The first HUNK_CODE/HUNK_DATA block of a hunk executable.
fn first_hunk(exe: &[u8]) -> Option<Vec<u8>> {
    let long = |i: usize| -> Option<usize> {
        Some(u32::from_be_bytes(exe.get(i * 4..i * 4 + 4)?.try_into().ok()?) as usize)
    };
    if long(0)? != 0x3F3 {
        return None;
    }
    let mut i = 1;
    while long(i)? != 0 {
        i += long(i)? + 1;
    }
    let (first, last) = (long(i + 2)?, long(i + 3)?);
    i += 4 + (last - first + 1);
    loop {
        match long(i)? {
            0x3E9 | 0x3EA => {
                let n = long(i + 1)?;
                return exe.get((i + 2) * 4..(i + 2 + n) * 4).map(<[u8]>::to_vec);
            }
            0x3E7 | 0x3E8 => i += 2 + long(i + 1)?,
            _ => return None,
        }
    }
}

fn be16(b: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([b[at], b[at + 1]])
}

fn be32(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

/// File `file` of the game's `file_table`, read as `load_file` reads it: `count` sectors from
/// `sector` on disk `index + 1`, then decompressed.
pub fn load_file(disks: &DiskSet, h0: &[u8], file: usize) -> Result<Vec<u8>, SoundError> {
    assert!(file < FILE_COUNT, "the file table has {FILE_COUNT} entries");
    let e = FILE_TABLE + file * 6;
    let (sector, count, disk) = (
        be16(h0, e) as usize,
        be16(h0, e + 2) as usize,
        be16(h0, e + 4) as usize,
    );
    let adf = &disks
        .disks
        .get(disk)
        .ok_or(SoundError::FileOutsideDisk { file })?
        .bytes;
    let bytes = adf
        .get(sector * SECTOR..(sector + count) * SECTOR)
        .ok_or(SoundError::FileOutsideDisk { file })?;
    decompress(bytes).map_err(|err| SoundError::Atn { file, err })
}

impl SoundData {
    /// Decode every module, effect bank, streamed sample and replayer table from the disks.
    pub fn from_disks(disks: &DiskSet) -> Result<SoundData, SoundError> {
        let h0 = game_hunk0(disks)?;
        SoundData::from_hunk0(disks, &h0)
    }

    pub fn from_hunk0(disks: &DiskSet, h0: &[u8]) -> Result<SoundData, SoundError> {
        if h0.len() < STREAM_TABLE + STREAM_SAMPLES * 16 {
            return Err(SoundError::BadGame);
        }
        let mut tunes = Vec::new();
        for t in TuneId::ALL {
            let m = load_file(disks, h0, t.file())?;
            if m.get(..4) != Some(b"P41A") {
                return Err(SoundError::NotAModule(t));
            }
            tunes.push((t, m));
        }
        let mut banks = Vec::new();
        for b in BankId::ALL {
            banks.push((b, load_file(disks, h0, b.file())?));
        }
        let mut stream = Vec::new();
        for k in 0..STREAM_SAMPLES {
            let e = STREAM_TABLE + k * 16;
            // The pointer is a hunk-0 offset in the file; the loader relocates it by hunk 0's base.
            let (off, len) = (be32(h0, e) as usize, be32(h0, e + 4));
            let bytes = h0
                .get(off..off + len as usize)
                .ok_or(SoundError::BadGame)?
                .to_vec();
            stream.push(StreamSample {
                addr: H0 + off as u32,
                bytes,
                len,
                volume: be16(h0, e + 10),
            });
        }
        Ok(SoundData {
            h0: h0.to_vec(),
            tunes,
            banks,
            stream,
        })
    }

    /// The same, from decoded [`crate::GameData`] (the asset cache) instead of the disks.
    pub fn from_game(data: &crate::GameData) -> Result<SoundData, SoundError> {
        let h0 = data
            .exes
            .iter()
            .find(|e| e.id == crate::ExeId::Game)
            .and_then(|e| e.hunks.first())
            .map(|h| h.data.clone())
            .ok_or(SoundError::NoGame)?;
        if h0.len() < STREAM_TABLE + STREAM_SAMPLES * 16 {
            return Err(SoundError::BadGame);
        }
        let raw = |name: &str| data.raw.get(name).cloned().ok_or(SoundError::NoGame);
        let mut tunes = Vec::new();
        for t in TuneId::ALL {
            let name = match t {
                TuneId::Title => "MUSC".to_string(),
                TuneId::World(n) => format!("L{n}MU"),
                TuneId::Bonus => "LBMU".to_string(),
                TuneId::Slot => "LFMU".to_string(),
            };
            tunes.push((t, raw(&name)?));
        }
        let mut banks = Vec::new();
        for b in BankId::ALL {
            let name = match b {
                BankId::Worlds12 => "L1FX",
                BankId::Worlds36 => "L1FX-2",
                BankId::Bonus => "LBFX",
                BankId::Slot => "LFFX",
            };
            banks.push((b, raw(name)?));
        }
        let mut stream = Vec::new();
        for k in 0..STREAM_SAMPLES {
            let e = STREAM_TABLE + k * 16;
            let (off, len) = (be32(&h0, e) as usize, be32(&h0, e + 4));
            let bytes = h0
                .get(off..off + len as usize)
                .ok_or(SoundError::BadGame)?
                .to_vec();
            stream.push(StreamSample {
                addr: H0 + off as u32,
                bytes,
                len,
                volume: be16(&h0, e + 10),
            });
        }
        Ok(SoundData {
            h0,
            tunes,
            banks,
            stream,
        })
    }

    /// Whether this holds every tune and bank (not an empty stand-in a test built).
    pub fn is_complete(&self) -> bool {
        self.tunes.len() == TuneId::ALL.len() && self.banks.len() == BankId::ALL.len()
    }

    /// The module bytes, as on disk (unrelocated).
    pub fn tune(&self, t: TuneId) -> &[u8] {
        &self
            .tunes
            .iter()
            .find(|x| x.0 == t)
            .expect("every tune is decoded")
            .1
    }

    pub fn bank(&self, b: BankId) -> &[u8] {
        &self
            .banks
            .iter()
            .find(|x| x.0 == b)
            .expect("every bank is decoded")
            .1
    }

    /// The byte at `h0+$off` as in the file, 0 past its end. An index past a table's end reads on
    /// into whatever follows it, as the original does.
    pub fn h0_byte(&self, off: u32) -> u8 {
        self.h0.get(off as usize).copied().unwrap_or(0)
    }

    /// The word at `h0+$off`.
    pub fn h0_word(&self, off: u32) -> u16 {
        u16::from_be_bytes([self.h0_byte(off), self.h0_byte(off + 1)])
    }

    /// `len` bytes from `h0+$off`.
    pub fn h0_bytes(&self, off: u32, len: usize) -> Vec<u8> {
        (0..len as u32).map(|i| self.h0_byte(off + i)).collect()
    }

    /// For tests: data built by hand. `h0` stands in for the game's hunk 0.
    pub fn from_parts(
        h0: Vec<u8>,
        tunes: Vec<(TuneId, Vec<u8>)>,
        banks: Vec<(BankId, Vec<u8>)>,
        stream: Vec<StreamSample>,
    ) -> SoundData {
        SoundData {
            h0,
            tunes,
            banks,
            stream,
        }
    }
}
