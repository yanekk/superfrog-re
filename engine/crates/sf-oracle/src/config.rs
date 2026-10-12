//! What the oracle runs (the owner's Kickstart and disks) and how it fails.

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use sf_disk::{DiskSet, DiskSlot, Offered};

/// The frame cap when none is given (DESIGN §5.2): no oracle run is unbounded.
pub const DEFAULT_MAX_FRAMES: u32 = 20_000;

/// The Kickstart that boots the crack to the title (FINDINGS 2026-10-11): 1.3 (34.5).
const KICKSTART: &str = "kick1.3.rom";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleConfig {
    /// A 256 or 512 KB Kickstart image. Opened read-only, never copied (DESIGN §5).
    pub kickstart: PathBuf,
    /// The four disks, zipped or raw `.adf`, in any order: they are identified by SHA-1.
    pub disks: [PathBuf; 4],
    /// [`crate::Amiga::step`] refuses to run frame `max_frames` and beyond.
    pub max_frames: u32,
    /// Drives connected, df0 first: 1 (the A500 as sold, the default) to 4. Drives past df0 start
    /// empty; W6-2..4's first map set needs one of df1–df3 (docs/12 "Reaching every level").
    pub drives: u8,
}

impl OracleConfig {
    /// The owner's ROM and disks where DESIGN §5 says they are: `SUPERFROG_KICKSTART`, else
    /// `~/amiga-roms/kick1.3.rom`; `SUPERFROG_DISKS`, else the main checkout's root.
    pub fn locate(max_frames: u32) -> Result<OracleConfig, OracleError> {
        let kickstart = match std::env::var_os("SUPERFROG_KICKSTART").filter(|v| !v.is_empty()) {
            Some(p) => PathBuf::from(p),
            None => PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join("amiga-roms")
                .join(KICKSTART),
        };
        if !kickstart.is_file() {
            return Err(OracleError::Setup(format!(
                "Kickstart not found at {}; set SUPERFROG_KICKSTART to Kickstart 1.3 (34.5)",
                kickstart.display()
            )));
        }
        let dir = match std::env::var_os("SUPERFROG_DISKS").filter(|v| !v.is_empty()) {
            Some(d) => PathBuf::from(d),
            None => main_checkout()?,
        };
        let mut found: Vec<PathBuf> = Vec::new();
        for n in 1..=4 {
            let want = format!("disk {n} of 4");
            let hit = std::fs::read_dir(&dir)
                .map_err(|e| OracleError::Setup(format!("{}: {e}", dir.display())))?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    let name = p
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase();
                    (name.ends_with(".zip") || name.ends_with(".adf")) && name.contains(&want)
                })
                .min();
            match hit {
                Some(p) => found.push(p),
                None => {
                    return Err(OracleError::Setup(format!(
                        "Superfrog disk {n} of 4 not found in {}; put the four disk images there or point \
                         SUPERFROG_DISKS at their folder",
                        dir.display()
                    )));
                }
            }
        }
        let disks: [PathBuf; 4] = found.try_into().expect("four disks");
        Ok(OracleConfig {
            kickstart,
            disks,
            max_frames,
            drives: 1,
        })
    }

    /// The same set-up with `drives` drives connected (1..=4).
    pub fn with_drives(self, drives: u8) -> OracleConfig {
        assert!(
            (1..=4).contains(&drives),
            "an Amiga has 1 to 4 floppy drives"
        );
        OracleConfig { drives, ..self }
    }

    /// Read the Kickstart and identify the four disks. Every file is opened read-only.
    pub(crate) fn load(&self) -> Result<(Vec<u8>, DiskSet), OracleError> {
        let rom = read_only(&self.kickstart)?;
        let mut offered = Vec::new();
        for p in &self.disks {
            offered.push(Offered {
                name: p.display().to_string(),
                bytes: read_only(p)?,
            });
        }
        let set = sf_disk::identify(&offered).map_err(|errs| {
            OracleError::Setup(
                errs.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        })?;
        Ok((rom, set))
    }
}

fn read_only(path: &Path) -> Result<Vec<u8>, OracleError> {
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|mut f| f.read_to_end(&mut bytes))
        .map_err(|e| OracleError::Setup(format!("{}: {e}", path.display())))?;
    Ok(bytes)
}

/// `<git common dir>/..`: the main checkout, whichever worktree this runs in.
fn main_checkout() -> Result<PathBuf, OracleError> {
    let out = Command::new("git")
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|e| OracleError::Setup(format!("running git: {e}")))?;
    let common = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    common.parent().map(Path::to_path_buf).ok_or_else(|| {
        OracleError::Setup("cannot find the main checkout; set SUPERFROG_DISKS".into())
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OracleError {
    /// The ROM, the disks or the emulator could not be set up.
    Setup(String),
    /// A step was asked for at or past `max_frames` (DESIGN §5.2).
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

/// Slot `slot`'s disk in the set.
pub(crate) fn disk(set: &DiskSet, slot: DiskSlot) -> &[u8] {
    &set.disk(slot).bytes
}
