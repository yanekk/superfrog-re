//! AmigaDOS OFS (`DOS\0`) volumes: the root directory's listing and its files' bytes. The block
//! walking is `amiga-ffs`'s; this module is the crate's own interface over it, so callers keep one
//! API and one error style whatever reads the blocks underneath.

use std::collections::BTreeSet;
use std::fmt;

use amiga_ffs::read::{Entry, EntryKind, Error, Volume};
use amiga_ffs::{BlockSource, Variant};

use crate::ADF_SIZE;
use crate::error::ReadError;

const BLOCK: usize = 512;
const DOS0: [u8; 4] = [b'D', b'O', b'S', 0];

/// A parsed OFS volume over the bytes of a whole ADF.
#[derive(Debug, Clone)]
pub struct OfsImage<'a> {
    bytes: &'a [u8],
    volume: String,
    files: Vec<(String, u32)>,
}

impl<'a> OfsImage<'a> {
    /// Parse an OFS (`DOS\0`) image. `bytes` is a whole ADF; anything else is a [`ReadError`], as
    /// is a root directory with a damaged block. File contents are checked when they are read.
    ///
    /// The boot block's checksum is deliberately not checked: it only says whether a disk boots,
    /// and a data disk that never boots is still a good volume.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ReadError> {
        if bytes.len() < ADF_SIZE {
            return Err(ReadError::Truncated { at: bytes.len() });
        }
        if bytes.len() > ADF_SIZE {
            return Err(ReadError::Malformed {
                at: ADF_SIZE,
                what: format!("{} bytes is not a double-density ADF", bytes.len()),
            });
        }
        if bytes[..4] != DOS0 {
            return Err(ReadError::Malformed {
                at: 0,
                what: format!("dos type {:02x?} is not OFS (DOS\\0)", &bytes[..4]),
            });
        }
        let mut volume = open(bytes)?;
        let root = volume.root_lba();
        let name = latin1(&volume.root().name);
        let mut files = Vec::new();
        walk(&mut volume, root, "", &mut BTreeSet::from([root]), &mut files)?;
        Ok(OfsImage {
            bytes,
            volume: name,
            files,
        })
    }

    /// The volume name from the root block.
    pub fn volume_name(&self) -> &str {
        &self.volume
    }

    /// Every file on the volume: path and size in bytes, in directory order. A file in a subdirectory
    /// is `dir/name`, listed where its directory falls. Directory order is hash-table slot by slot,
    /// each slot's chain in turn; not alphabetical, and `xdftool list` sorts, so compare as sets.
    pub fn files(&self) -> impl Iterator<Item = (&str, u32)> {
        self.files.iter().map(|(n, s)| (n.as_str(), *s))
    }

    /// One file's contents by path, each part matched case-insensitively like dos.library.
    ///
    /// Owned, because every OFS data block carries its own 24-byte header: a file of more than one
    /// block is never contiguous in the image, so there is no slice of it to return.
    pub fn read_file(&self, name: &str) -> Result<Vec<u8>, ReadError> {
        let mut volume = open(self.bytes)?;
        let root_at = volume.root_lba() as usize * BLOCK;
        let missing = || ReadError::Malformed {
            at: root_at,
            what: format!("no file named {name:?} on the volume"),
        };
        let Some(raw) = name.chars().map(|c| u8::try_from(c).ok()).collect::<Option<Vec<u8>>>()
        else {
            return Err(missing()); // a name Latin-1 cannot spell cannot be on the disk
        };
        let root = volume.root_lba();
        let entry: Entry = match volume.lookup_path(root, &raw) {
            Ok(Some(entry)) => entry,
            Ok(None) | Err(Error::NameTooLong { .. }) => return Err(missing()),
            Err(e) => return Err(read_error(e)),
        };
        if entry.kind != EntryKind::File {
            return Err(ReadError::Malformed {
                at: entry.lba as usize * BLOCK,
                what: format!("{name:?} is not a plain file ({:?})", entry.kind),
            });
        }
        volume.read_file(entry.lba).map_err(read_error)
    }
}

fn open(bytes: &[u8]) -> Result<Volume<Image<'_>>, ReadError> {
    Volume::open(Image(bytes), Some(Variant::Ofs)).map_err(read_error)
}

/// Collect the files under directory `dir`. `seen` holds the directories on the way here and before:
/// the library guards each directory's own chains, but a subdirectory entry pointing back up would
/// otherwise recurse until the stack ran out.
fn walk(
    volume: &mut Volume<Image<'_>>,
    dir: u64,
    prefix: &str,
    seen: &mut BTreeSet<u64>,
    files: &mut Vec<(String, u32)>,
) -> Result<(), ReadError> {
    for entry in volume.read_dir(dir).map_err(read_error)? {
        let path = format!("{prefix}{}", latin1(&entry.name));
        match entry.kind {
            EntryKind::File => files.push((path, entry.byte_size)),
            EntryKind::Directory => {
                if !seen.insert(entry.lba) {
                    return Err(ReadError::Malformed {
                        at: entry.lba as usize * BLOCK,
                        what: format!("directory {path:?} is reached twice"),
                    });
                }
                walk(volume, entry.lba, &format!("{path}/"), seen, files)?;
            }
            // Links name files listed elsewhere, or paths on who knows what volume.
            _ => {}
        }
    }
    Ok(())
}

/// The names on disk are Latin-1, so every byte is one char and nothing is lost.
fn latin1(raw: &[u8]) -> String {
    raw.iter().map(|&b| char::from(b)).collect()
}

/// The image as `amiga-ffs` reads blocks.
struct Image<'a>(&'a [u8]);

/// A block pointer past the end of the image.
#[derive(Debug)]
struct PastTheEnd(u64);

impl fmt::Display for PastTheEnd {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "block {} is past the end of the image", self.0)
    }
}

impl BlockSource for Image<'_> {
    type Error = PastTheEnd;

    fn block_size(&self) -> usize {
        BLOCK
    }

    fn read_block(&mut self, lba: u64, buf: &mut [u8]) -> Result<(), PastTheEnd> {
        let block = usize::try_from(lba)
            .ok()
            .and_then(|l| l.checked_mul(BLOCK))
            .and_then(|at| self.0.get(at..at.checked_add(BLOCK)?))
            .ok_or(PastTheEnd(lba))?;
        buf.copy_from_slice(block);
        Ok(())
    }

    fn block_count(&self) -> Option<u64> {
        Some((self.0.len() / BLOCK) as u64)
    }
}

/// The library's refusal as this crate's error: the byte offset of the block it refused on.
fn read_error(e: Error<PastTheEnd>) -> ReadError {
    use Error::*;
    let lba = match &e {
        Io(PastTheEnd(lba)) => *lba,
        LbaOutOfRange { lba, .. }
        | Checksum { lba }
        | NotHeader { lba, .. }
        | WrongSecondaryType { lba, .. }
        | UnknownSecondaryType { lba, .. }
        | NotADirectory { lba, .. }
        | ChainCycle { lba }
        | ChainTooLong { lba }
        | WrongBlockType { lba, .. }
        | OwnKeyMismatch { lba, .. }
        | BlockOwnerMismatch { lba, .. }
        | DataPointerCount { lba, .. }
        | DataPointerHole { lba, .. }
        | FileSizeMismatch { lba, .. }
        | DataBlockSequence { lba, .. }
        | DataBlockSize { lba, .. }
        | NotALink { lba, .. }
        | LinkTargetMissing { lba }
        | DircacheRecordOverflow { lba, .. }
        | SoftLinkNotResolved { lba } => *lba,
        // The rest describe the volume as a whole, which starts at the boot block.
        _ => 0,
    };
    ReadError::Malformed {
        at: (lba as usize).saturating_mul(BLOCK),
        what: e.to_string(),
    }
}
