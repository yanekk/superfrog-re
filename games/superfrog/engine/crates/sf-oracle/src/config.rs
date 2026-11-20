//! What the oracle runs for Superfrog (the owner's Kickstart and four disks), turned into the
//! toolkit's set-up ([`amiga_oracle::OracleConfig`]) for each run.

use std::path::{Path, PathBuf};
use std::process::Command;

use amiga_oracle::{OracleError, read_only};
use sf_disk::{DiskSet, DiskSlot, Offered};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleConfig {
    /// A 256 or 512 KB Kickstart image. Opened read-only, never copied (DESIGN §5).
    pub kickstart: PathBuf,
    /// The four disks, zipped or raw `.adf`, in any order: they are identified by SHA-1.
    pub disks: [PathBuf; 4],
    /// [`crate::Amiga::step`] refuses to run frame `max_frames` and beyond.
    pub max_frames: u32,
    /// Drives connected, df0 first: 1 (the A500 as sold, the default) to 4. Drives past df0 start
    /// empty; W6-2..4's first map set needs one of df1–df3 (games/superfrog/docs/12 "Reaching every level").
    pub drives: u8,
    /// The disk in df0 at power-on: disk 1 (the game, the default) or disk 3, whose own boot plays
    /// the story intro before it asks for disk 1 (games/superfrog/docs/08).
    pub boot: DiskSlot,
}

impl OracleConfig {
    /// The owner's ROM and disks where DESIGN §5 says they are: `SUPERFROG_KICKSTART`, else the
    /// toolkit's [`amiga_oracle::kickstart_path`] (`AMIGA_KICKSTART`, else
    /// `~/amiga-roms/kick1.3.rom`, the 1.3 (34.5) that boots the crack to the title, FINDINGS
    /// 2026-10-11); `SUPERFROG_DISKS`, else `games/superfrog/` in the main checkout, else its root.
    pub fn locate(max_frames: u32) -> Result<OracleConfig, OracleError> {
        let kickstart = match std::env::var_os("SUPERFROG_KICKSTART").filter(|v| !v.is_empty()) {
            Some(p) => {
                let p = PathBuf::from(p);
                if !p.is_file() {
                    return Err(OracleError::Setup(format!(
                        "Kickstart not found at {} (SUPERFROG_KICKSTART); set it to Kickstart 1.3 (34.5)",
                        p.display()
                    )));
                }
                p
            }
            // Then the toolkit's own lookup (DESIGN §2.5): `AMIGA_KICKSTART`, else the default.
            None => amiga_oracle::kickstart_path().map_err(|e| match e {
                OracleError::Setup(m) => {
                    OracleError::Setup(format!("{m}, or SUPERFROG_KICKSTART to override both"))
                }
                e => e,
            })?,
        };
        let dir = match std::env::var_os("SUPERFROG_DISKS").filter(|v| !v.is_empty()) {
            Some(d) => PathBuf::from(d),
            None => {
                let root = main_checkout()?;
                let game = root.join("games/superfrog");
                if holds_a_disk(&game) { game } else { root }
            }
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
                        "Superfrog disk {n} of 4 not found in {} (looked in SUPERFROG_DISKS, games/superfrog/ in the \
                         main checkout, then its root); put the four disk images there or point SUPERFROG_DISKS \
                         at their folder",
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

/// Whether `dir` holds any of the four disks, so a game folder without them falls through.
fn holds_a_disk(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.filter_map(|e| e.ok()).any(|e| {
            let name = e.file_name().to_string_lossy().to_lowercase();
            (name.ends_with(".zip") || name.ends_with(".adf")) && name.contains(" of 4")
        })
    })
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
