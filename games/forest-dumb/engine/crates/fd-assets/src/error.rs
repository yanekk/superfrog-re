//! What can be wrong with a file: always named by the file.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetError {
    /// The file's name on disk, as the game loads it (`_Mapa1-1`).
    pub file: String,
    pub why: Why,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Why {
    /// The file could not be had: the caller's reason (no such file, a bad disk).
    Missing(String),
    /// The file's size is not its format's.
    Size { expected: usize, got: usize },
    /// The size is not a whole number of the format's units (tiles, shapes, lines), or is none.
    NotWhole { unit: usize, got: usize },
    /// A value the format cannot hold, at byte `at` of the file.
    Malformed { at: usize, what: String },
}

impl AssetError {
    pub fn new(file: &str, why: Why) -> AssetError {
        AssetError {
            file: file.to_string(),
            why,
        }
    }

    /// For the file closure: `file` could not be read, for `reason`.
    pub fn missing(file: &str, reason: impl Into<String>) -> AssetError {
        AssetError::new(file, Why::Missing(reason.into()))
    }
}

impl fmt::Display for AssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.file)?;
        match &self.why {
            Why::Missing(m) => write!(f, "{m}"),
            Why::Size { expected, got } => write!(f, "{got} bytes, the format has {expected}"),
            Why::NotWhole { unit, got } => {
                write!(
                    f,
                    "{got} bytes, not a whole non-zero number of {unit}-byte units"
                )
            }
            Why::Malformed { at, what } => write!(f, "at byte {at}: {what}"),
        }
    }
}

impl std::error::Error for AssetError {}
