//! Shell, test tooling: end-to-end tests driving the built app (DESIGN §5, End to end).
//!
//! The driver is `amiga_e2e` (toolkit-extract T08); this crate gives it Superfrog's app, the
//! [`superfrog`] spec, and keeps every test under `tests/`. [`App::launch`] makes a temporary home
//! with the owner's disks imported into it. Then either layer drives the real `superfrog` binary:
//!
//! - the script layer, [`App::run_script`]: the app runs a script of host events (they enter where
//!   winit's are translated) and writes every frame, the mixed audio and the inputs it saw;
//! - the OS layer ([`LaunchOpts::os`], `amiga_e2e`'s `os.rs`): real key events through CGEvent,
//!   real menu clicks through the Accessibility API and a capture of the real window. It needs
//!   Accessibility and Screen Recording for the terminal, and its tests run only under
//!   `SF_E2E_OS=1` (README.md).
//!
//! Every launch passes `--home`, `--max-frames` and `--timeout`, and puts the app in its own
//! process group, so a run always ends and leaves nothing behind (DESIGN §5.2).

pub use amiga_e2e::{
    AppSpec, Captured, Flags, Image, Key, LaunchOpts, MenuItemState, Mods, Shot, group_alive, os_permissions,
    read_wav,
};

use std::ffi::OsString;
use std::io;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

/// Superfrog's app as the driver launches it.
pub fn superfrog() -> &'static AppSpec {
    static SPEC: OnceLock<AppSpec> = OnceLock::new();
    SPEC.get_or_init(|| AppSpec {
        package: "sf-app",
        binary: "superfrog",
        workspace: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        flags: Flags {
            home: "--home",
            capture: "--capture",
            max_frames: "--max-frames",
            timeout: "--timeout",
            scale: "--scale",
            script: "--script",
        },
        clear_env: vec!["SUPERFROG_HOME"],
        keep_env: "SF_E2E_KEEP",
        scratch: "sf-e2e",
        provision,
        // sf-app's single-copy lock message.
        already_running: "already running",
    })
}

/// The `superfrog` binary, built once per test process.
pub fn binary() -> &'static Path {
    amiga_e2e::binary(superfrog())
}

/// The app under test, in its own temporary home: [`amiga_e2e::App`] on Superfrog's spec, with
/// Superfrog's disks. Every driver method is reached through it. The home is removed when this is
/// dropped, unless `SF_E2E_KEEP=1`.
pub struct App(amiga_e2e::App);

impl Deref for App {
    type Target = amiga_e2e::App;
    fn deref(&self) -> &amiga_e2e::App {
        &self.0
    }
}

impl DerefMut for App {
    fn deref_mut(&mut self) -> &mut amiga_e2e::App {
        &mut self.0
    }
}

impl App {
    /// A temporary home with the real disks imported into it.
    pub fn launch(opts: LaunchOpts) -> App {
        App(amiga_e2e::App::launch(superfrog(), opts))
    }

    /// An empty temporary home: a first launch. Its runs go through the import. (The harness's
    /// own files, the scripts, captures and stderr, are `capture-*` in it.)
    pub fn launch_empty(opts: LaunchOpts) -> App {
        App(amiga_e2e::App::launch_empty(superfrog(), opts))
    }

    /// Copies of the owner's four disk images in `<home>/<folder>`, by path in disk order, for
    /// a script's `pick` (the copies are under the home, so a test may move or damage them).
    pub fn disks_in_home(&self, folder: &str) -> Vec<PathBuf> {
        let src = sf_conform::disks::disk_dir().unwrap_or_else(|e| panic!("{e}"));
        let dir = self.home.join(folder);
        std::fs::create_dir_all(&dir).expect("disk folder");
        let mut names: Vec<_> = std::fs::read_dir(&src)
            .expect("the disk folder")
            .flatten()
            .map(|e| e.file_name())
            .filter(|n| n.to_string_lossy().starts_with("Superfrog") && n.to_string_lossy().ends_with(".zip"))
            .collect();
        names.sort();
        assert_eq!(names.len(), 4, "four Superfrog zips in {}", src.display());
        names
            .iter()
            .map(|n| {
                std::fs::copy(src.join(n), dir.join(n)).expect("copying a disk");
                dir.join(n)
            })
            .collect()
    }
}

/// Fills a fresh home: the imported cache, copied from the seed.
fn provision(home: &Path) -> io::Result<()> {
    std::fs::create_dir_all(home.join("cache"))?;
    let seed = seed_cache();
    std::fs::copy(seed, home.join("cache").join(seed.file_name().unwrap())).map_err(|e| {
        io::Error::new(e.kind(), format!("copying the imported cache into {}: {e}", home.display()))
    })?;
    Ok(())
}

/// The asset cache imported from the owner's disks by the real binary, kept between runs in
/// `~/.cache/superfrog-re/e2e/` (derived from the disks, so outside the repository: DESIGN §2.3).
/// Each launch copies it into its own home; the app re-imports when it is stale.
fn seed_cache() -> &'static Path {
    static SEED: OnceLock<PathBuf> = OnceLock::new();
    SEED.get_or_init(|| {
        let disks = sf_conform::disks::disk_dir().unwrap_or_else(|e| panic!("{e}"));
        let home = PathBuf::from(std::env::var_os("HOME").expect("HOME"))
            .join(".cache/superfrog-re/e2e");
        // Scripted, so the app opens no native panel that would wait for a person; and another
        // test process (another worktree's) may hold the seed home's lock: wait it out.
        std::fs::create_dir_all(&home).expect("the seed home");
        let script = home.join("seed.script");
        std::fs::write(&script, "").expect("the seed script");
        let args: Vec<OsString> = vec![
            "--home".into(),
            home.clone().into(),
            "--cache-from-disks".into(),
            disks.into(),
            "--script".into(),
            script.into(),
            "--max-frames".into(),
            "0".into(),
            "--timeout".into(),
            "120".into(),
        ];
        let (status, err) = amiga_e2e::run_waiting_for_lock(superfrog(), &args, Duration::from_secs(150));
        assert!(status.success(), "importing the disks failed ({status}): {err}");
        // The file this build reads, by name: an older format's file may sit beside it, and
        // `assets-v9.bin` sorts after `assets-v10.bin`.
        let file = home.join(format!("cache/assets-v{}.bin", sf_assets::CACHE_FORMAT));
        assert!(file.is_file(), "the import wrote no {}", file.display());
        file
    })
}
