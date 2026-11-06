//! Shell, test tooling: end-to-end tests driving the built `fd-app` (DESIGN §5, End to end).
//!
//! The driver is `amiga_e2e`; this crate gives it Forest Dumb's app, [`spec`], and keeps every
//! test under `tests/`. [`App::launch`] makes a temporary home and passes `--disks` with the
//! owner's zips, which `fd-tools` finds. The script layer runs in the test sets; the OS layer
//! (real keys, menu clicks, window captures) needs Accessibility and Screen Recording and runs only
//! under `FD_E2E_OS=1`.
//!
//! Every launch passes `--home`, `--max-frames` and `--timeout`, and puts the app in its own
//! process group, so a run always ends and leaves nothing behind (DESIGN §5.2).

pub use amiga_e2e::{
    AppSpec, Captured, Flags, Image, Key, LaunchOpts, Mods, Shot, group_alive, os_permissions,
};

use std::io;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Forest Dumb's app as the driver launches it.
pub fn spec() -> &'static AppSpec {
    static SPEC: OnceLock<AppSpec> = OnceLock::new();
    SPEC.get_or_init(|| AppSpec {
        package: "fd-app",
        binary: "fd-app",
        workspace: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        flags: Flags {
            home: "--home",
            capture: "--capture",
            max_frames: "--max-frames",
            timeout: "--timeout",
            scale: "--scale",
            script: "--script",
        },
        clear_env: vec!["FOREST_DUMB_HOME"],
        keep_env: "FD_E2E_KEEP",
        scratch: "fd-e2e",
        provision,
        // amiga_app::lock's message.
        already_running: "already running",
    })
}

/// The `fd-app` binary, built once per test process.
pub fn binary() -> &'static Path {
    amiga_e2e::binary(spec())
}

/// The `fd-conform` binary, built once per test process as the app is: what replays a bug zip
/// (`fd-conform bug replay`, T14).
pub fn conform_binary() -> &'static Path {
    static SPEC: OnceLock<AppSpec> = OnceLock::new();
    let app = spec();
    amiga_e2e::binary(SPEC.get_or_init(|| AppSpec {
        package: "fd-conform",
        binary: "fd-conform",
        workspace: app.workspace.clone(),
        // Unused: the binary is only built here, never launched as an app.
        flags: Flags {
            home: "",
            capture: "",
            max_frames: "",
            timeout: "",
            scale: "",
            script: "",
        },
        clear_env: Vec::new(),
        keep_env: app.keep_env,
        scratch: app.scratch,
        provision,
        already_running: "",
    }))
}

/// The folder holding the owner's two zips; a test that cannot find them fails naming where it
/// looked, never skips.
pub fn disks() -> PathBuf {
    fd_tools::disk_dir().unwrap_or_else(|e| panic!("{e}"))
}

/// The app under test in its own temporary home: [`amiga_e2e::App`] on [`spec`]. Removed when
/// dropped, unless `FD_E2E_KEEP=1`.
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
    /// A temporary home, every launch reading the owner's disks (`--disks`).
    pub fn launch(mut opts: LaunchOpts) -> App {
        opts.extra_args
            .splice(0..0, ["--disks".to_string(), disks().display().to_string()]);
        App(amiga_e2e::App::launch(spec(), opts))
    }

    /// As [`App::launch`], the disks left to the arguments `opts` passes (a missing-disks test).
    pub fn launch_bare(opts: LaunchOpts) -> App {
        App(amiga_e2e::App::launch(spec(), opts))
    }
}

/// A fresh home needs nothing: there is no cache, the disks are read at every launch.
fn provision(_: &Path) -> io::Result<()> {
    Ok(())
}
