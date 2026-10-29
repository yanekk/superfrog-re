//! A bounds-checked big-endian reader over a piece of a file, whose errors give file offsets.

use crate::error::ReadError;

#[derive(Clone, Copy)]
pub(crate) struct Reader<'a> {
    pub data: &'a [u8],
    /// Where `data` starts in the file, so a nested piece reports offsets in the whole file.
    pub base: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Reader<'a> {
        Reader { data, base: 0 }
    }

    /// `len` bytes from `at` as a reader of their own.
    pub fn sub(&self, at: usize, len: usize) -> Result<Reader<'a>, ReadError> {
        Ok(Reader {
            data: self.bytes(at, len)?,
            base: self.base + at,
        })
    }

    pub fn bad(&self, at: usize, what: impl Into<String>) -> ReadError {
        ReadError::Malformed {
            at: self.base + at,
            what: what.into(),
        }
    }

    pub fn bytes(&self, at: usize, len: usize) -> Result<&'a [u8], ReadError> {
        at.checked_add(len)
            .and_then(|end| self.data.get(at..end))
            .ok_or(ReadError::Truncated {
                at: self.base + self.data.len().min(at),
            })
    }

    pub fn u8(&self, at: usize) -> Result<u8, ReadError> {
        Ok(self.bytes(at, 1)?[0])
    }

    pub fn u16(&self, at: usize) -> Result<u16, ReadError> {
        let b = self.bytes(at, 2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    pub fn u32(&self, at: usize) -> Result<u32, ReadError> {
        let b = self.bytes(at, 4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
}
