//! What can be wrong with a file a reader is handed: where in it, and what.

use std::fmt;

/// A file a reader refused. `at` is a byte offset from the start of the bytes handed to the reader
/// (for a chunk nested in an IFF `FORM`, still from the start of the file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// The data ends before a structure it starts: `at` is the byte offset reached.
    Truncated { at: usize },
    /// The bytes are not the format: what was found, near `at`.
    Malformed { at: usize, what: String },
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::Truncated { at } => write!(f, "cut short at byte {at}"),
            ReadError::Malformed { at, what } => write!(f, "byte {at}: {what}"),
        }
    }
}

impl std::error::Error for ReadError {}
