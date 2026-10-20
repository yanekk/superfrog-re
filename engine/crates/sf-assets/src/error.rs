//! What can be wrong with a chunk: always named by the chunk and the game file it holds.

use std::fmt;

use crate::gamedata::ChunkId;

/// A chunk that could not be decoded. `chunk` is the disk chunk the content came from (for content
/// embedded in an executable, the executable's chunk); `what` names the content, by the game's own
/// file name where it has one (`L1BM tiles`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetError {
    pub chunk: ChunkId,
    pub what: String,
    pub why: Why,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// The chunk was not among those handed to `decode`.
    Missing,
    /// The chunk's size is not the one its format has.
    Size { expected: usize, got: usize },
    /// The data ends before a structure it starts: `at` is the byte offset reached.
    Truncated { at: usize },
    /// The bytes are not the format: what was found where.
    Malformed(String),
    /// The chunk does not decompress, even with the next sector appended.
    Atn(sf_disk::AtnError),
}

impl AssetError {
    pub(crate) fn new(chunk: ChunkId, what: impl Into<String>, why: Why) -> AssetError {
        AssetError {
            chunk,
            what: what.into(),
            why,
        }
    }
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}): ", self.chunk, self.what)?;
        match &self.why {
            Why::Missing => write!(f, "chunk missing"),
            Why::Size { expected, got } => write!(f, "{got} bytes, the format has {expected}"),
            Why::Truncated { at } => write!(f, "cut short at byte {at}"),
            Why::Malformed(m) => write!(f, "{m}"),
            Why::Atn(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for AssetError {}

/// A bounds-checked big-endian reader over one piece of content, whose errors name it.
pub(crate) struct Reader<'a> {
    pub data: &'a [u8],
    pub chunk: ChunkId,
    pub what: String,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8], chunk: ChunkId, what: &str) -> Reader<'a> {
        Reader {
            data,
            chunk,
            what: what.to_string(),
        }
    }

    pub fn err(&self, why: Why) -> AssetError {
        AssetError::new(self.chunk, self.what.as_str(), why)
    }

    pub fn bytes(&self, at: usize, len: usize) -> Result<&'a [u8], AssetError> {
        at.checked_add(len)
            .and_then(|end| self.data.get(at..end))
            .ok_or_else(|| {
                self.err(Why::Truncated {
                    at: self.data.len().min(at),
                })
            })
    }

    pub fn u8(&self, at: usize) -> Result<u8, AssetError> {
        Ok(self.bytes(at, 1)?[0])
    }

    pub fn u16(&self, at: usize) -> Result<u16, AssetError> {
        let b = self.bytes(at, 2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    pub fn u32(&self, at: usize) -> Result<u32, AssetError> {
        let b = self.bytes(at, 4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn size(&self, expected: usize) -> Result<(), AssetError> {
        if self.data.len() == expected {
            Ok(())
        } else {
            Err(self.err(Why::Size {
                expected,
                got: self.data.len(),
            }))
        }
    }
}
