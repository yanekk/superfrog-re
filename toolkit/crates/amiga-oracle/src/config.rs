//! What the oracle runs (a Kickstart and any number of disks) and how it fails.

use std::ffi::OsString;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The frame cap when none is given: no oracle run is unbounded.
pub const DEFAULT_MAX_FRAMES: u32 = 20_000;

/// The Kickstart looked for under `~/amiga-roms/` when `AMIGA_KICKSTART` is not set: 1.3 (34.5).
const KICKSTART: &str = "kick1.3.rom";

#[derive(Clone, PartialEq, Eq)]
pub struct OracleConfig {
    /// A 256 or 512 KB Kickstart image. Opened read-only, never copied.
    pub kickstart: PathBuf,
    /// The disk images (raw ADF bytes) the run may insert, by index: [`OracleConfig::boot`] and
    /// [`crate::Amiga::insert_disk`] name them by their place here. The caller reads and unpacks
    /// them; the order is part of the trace cache key.
    pub disks: Vec<Vec<u8>>,
    /// The disk in df0 at power-on, or `None` for an empty drive (the Kickstart asks for a disk).
    pub boot: Option<usize>,
    /// How the trace cache key names a boot disk: empty for the caller's usual boot, which then
    /// adds nothing to the key, else ` boot <label>` is added. An empty df0 is ` boot none`.
    pub boot_label: String,
    /// [`crate::Amiga::step`] refuses to run frame `max_frames` and beyond.
    pub max_frames: u32,
    /// Drives connected, df0 first: 1 (the A500 as sold, the default) to 4. Drives past df0 start
    /// empty.
    pub drives: u8,
    /// Where [`crate::cached_trace`] keeps traces; [`default_cache`]`.join("traces")` unless the
    /// caller keeps its own.
    pub cache: PathBuf,
}

impl fmt::Debug for OracleConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OracleConfig")
            .field("kickstart", &self.kickstart)
            .field(
                "disks",
                &self.disks.iter().map(Vec::len).collect::<Vec<_>>(),
            )
            .field("boot", &self.boot)
            .field("boot_label", &self.boot_label)
            .field("max_frames", &self.max_frames)
            .field("drives", &self.drives)
            .field("cache", &self.cache)
            .finish()
    }
}

impl OracleConfig {
    /// One drive, booting the first disk (none if `disks` is empty), traces under
    /// [`default_cache`].
    pub fn new(kickstart: PathBuf, disks: Vec<Vec<u8>>, max_frames: u32) -> OracleConfig {
        let boot = (!disks.is_empty()).then_some(0);
        OracleConfig {
            kickstart,
            disks,
            boot,
            boot_label: String::new(),
            max_frames,
            drives: 1,
            cache: default_cache().join("traces"),
        }
    }

    /// The same set-up with `drives` drives connected (1..=4).
    pub fn with_drives(self, drives: u8) -> OracleConfig {
        assert!(
            (1..=4).contains(&drives),
            "an Amiga has 1 to 4 floppy drives"
        );
        OracleConfig { drives, ..self }
    }

    /// The same set-up powered on with disk `boot` in df0 (`None`: empty), named `label` in the
    /// trace key (see [`OracleConfig::boot_label`]).
    pub fn with_boot(self, boot: Option<usize>, label: &str) -> OracleConfig {
        OracleConfig {
            boot,
            boot_label: label.to_string(),
            ..self
        }
    }

    /// The text the trace cache key adds after the disks' digests: nothing for one drive and the
    /// usual boot, so the keys a caller had before stay what they were.
    pub(crate) fn setup_key(&self) -> String {
        let mut text = String::new();
        if self.drives != 1 {
            text += &format!(" drives {}", self.drives);
        }
        match self.boot {
            None => text += " boot none",
            Some(_) if !self.boot_label.is_empty() => text += &format!(" boot {}", self.boot_label),
            Some(_) => {}
        }
        text
    }

    /// Read the Kickstart, read-only.
    pub(crate) fn rom(&self) -> Result<Vec<u8>, OracleError> {
        read_only(&self.kickstart)
    }
}

/// `AMIGA_KICKSTART`, else `~/amiga-roms/kick1.3.rom`; an error names the path it looked at and
/// how to point it elsewhere.
pub fn kickstart_path() -> Result<PathBuf, OracleError> {
    find_kickstart(
        std::env::var_os("AMIGA_KICKSTART"),
        std::env::var_os("HOME"),
    )
}

/// [`kickstart_path`] with the two variables passed in. A set `AMIGA_KICKSTART` is the only place
/// looked: a wrong override is an error, not a silent fall back to another ROM.
pub(crate) fn find_kickstart(
    var: Option<OsString>,
    home: Option<OsString>,
) -> Result<PathBuf, OracleError> {
    let (path, from) = match var.filter(|v| !v.is_empty()) {
        Some(p) => (PathBuf::from(p), "AMIGA_KICKSTART"),
        None => (
            PathBuf::from(home.unwrap_or_default())
                .join("amiga-roms")
                .join(KICKSTART),
            "the default, AMIGA_KICKSTART not set",
        ),
    };
    if path.is_file() {
        return Ok(path);
    }
    Err(OracleError::Setup(format!(
        "Kickstart not found; looked at {} ({from}). Set AMIGA_KICKSTART to a Kickstart 1.3 (34.5) image",
        path.display()
    )))
}

/// `AMIGA_TOOLKIT_CACHE`, else `~/.cache/amiga-toolkit`: the toolkit's own cache root, for a caller
/// that keeps none of its own.
pub fn default_cache() -> PathBuf {
    std::env::var_os("AMIGA_TOOLKIT_CACHE")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache/amiga-toolkit")
        })
}

/// A file's bytes, opened read-only; the error names the file.
pub fn read_only(path: &Path) -> Result<Vec<u8>, OracleError> {
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|mut f| f.read_to_end(&mut bytes))
        .map_err(|e| OracleError::Setup(format!("{}: {e}", path.display())))?;
    Ok(bytes)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OracleError {
    /// The ROM, the disks or the emulator could not be set up.
    Setup(String),
    /// A step was asked for at or past `max_frames`.
    FrameCap { max_frames: u32 },
    /// The emulator failed while running (a CPU halt, a corrupt snapshot).
    Emulator(String),
    /// A drive that is not connected.
    NoDrive(u8),
    /// The trace cache could not be read or written.
    Cache(String),
}

impl fmt::Display for OracleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OracleError::Setup(m) => write!(f, "oracle setup: {m}"),
            OracleError::FrameCap { max_frames } => {
                write!(
                    f,
                    "oracle frame cap reached: max_frames is {max_frames}; raise it to run longer"
                )
            }
            OracleError::Emulator(m) => write!(f, "oracle emulator: {m}"),
            OracleError::NoDrive(d) => write!(
                f,
                "oracle: no drive df{d} (connect it with OracleConfig::with_drives)"
            ),
            OracleError::Cache(m) => write!(f, "oracle trace cache: {m}"),
        }
    }
}

impl std::error::Error for OracleError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> OracleConfig {
        OracleConfig::new(PathBuf::from("/rom"), vec![vec![1], vec![2]], 10)
    }

    #[test]
    fn the_usual_set_up_adds_nothing_to_the_key() {
        assert_eq!(cfg().setup_key(), "");
        assert_eq!(cfg().with_drives(2).setup_key(), " drives 2");
        assert_eq!(cfg().with_boot(Some(1), "D3").setup_key(), " boot D3");
        assert_eq!(cfg().with_boot(None, "").setup_key(), " boot none");
        assert_eq!(
            cfg().with_drives(2).with_boot(None, "x").setup_key(),
            " drives 2 boot none"
        );
    }

    #[test]
    fn no_disks_powers_on_empty() {
        let c = OracleConfig::new(PathBuf::from("/rom"), Vec::new(), 10);
        assert_eq!(c.boot, None);
        assert_eq!(cfg().boot, Some(0));
    }

    #[test]
    fn a_missing_kickstart_names_where_it_looked() {
        let scratch = std::env::temp_dir().join("amiga-oracle-no-such-dir");
        let e = find_kickstart(
            Some(scratch.join("kick.rom").into()),
            Some("/home/x".into()),
        )
        .unwrap_err()
        .to_string();
        assert!(
            e.contains(&scratch.join("kick.rom").display().to_string()),
            "{e}"
        );
        assert!(e.contains("AMIGA_KICKSTART"), "{e}");
        let e = find_kickstart(None, Some(scratch.clone().into()))
            .unwrap_err()
            .to_string();
        assert!(
            e.contains(&scratch.join("amiga-roms/kick1.3.rom").display().to_string()),
            "{e}"
        );
        // An empty variable is unset.
        let e = find_kickstart(Some("".into()), Some(scratch.into()))
            .unwrap_err()
            .to_string();
        assert!(e.contains("kick1.3.rom"), "{e}");
    }
}
