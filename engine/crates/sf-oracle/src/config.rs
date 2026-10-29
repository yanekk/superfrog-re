//! What the oracle runs for Superfrog (the owner's Kickstart and four disks), turned into the
//! toolkit's set-up ([`amiga_oracle::OracleConfig`]) for each run.

use std::path::{Path, PathBuf};
use std::process::Command;

use amiga_oracle::{OracleError, read_only};
use sf_disk::{DiskSet, DiskSlot, Offered};

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
    /// The disk in df0 at power-on: disk 1 (the game, the default) or disk 3, whose own boot plays
    /// the story intro before it asks for disk 1 (docs/08).
    pub boot: DiskSlot,
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
            boot: DiskSlot::D1,
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

    /// The same set-up powered on with `slot`'s disk in df0.
    pub fn with_boot(self, boot: DiskSlot) -> OracleConfig {
        OracleConfig { boot, ..self }
    }

    /// The toolkit's set-up for this one: the four disks identified and in slot order (so disk
    /// `n` is index `n - 1`, and the trace key's disk digests are in the order they always were),
    /// disk 1 booting without a label and any other boot labelled `D<n>` (the key text this crate
    /// wrote before the move), traces under [`trace_cache`]. Every file is opened read-only.
    pub fn generic(&self) -> Result<amiga_oracle::OracleConfig, OracleError> {
        let set = self.identify()?;
        let disks = set.disks.into_iter().map(|d| d.bytes).collect();
        let label = if self.boot == DiskSlot::D1 {
            String::new()
        } else {
            format!("{:?}", self.boot)
        };
        Ok(amiga_oracle::OracleConfig {
            kickstart: self.kickstart.clone(),
            disks,
            boot: Some(self.boot.index()),
            boot_label: label,
            max_frames: self.max_frames,
            drives: self.drives,
            cache: trace_cache(),
        })
    }

    /// Identify the four disks. Every file is opened read-only.
    fn identify(&self) -> Result<DiskSet, OracleError> {
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
        Ok(set)
    }
}

/// `~/.cache/superfrog-re/traces`, `SUPERFROG_CACHE` replacing `~/.cache/superfrog-re` (DESIGN
/// §3.5): where Superfrog's traces have always been.
pub fn trace_cache() -> PathBuf {
    std::env::var_os("SUPERFROG_CACHE")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".cache/superfrog-re")
        })
        .join("traces")
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
