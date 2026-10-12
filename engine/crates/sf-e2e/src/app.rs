//! Launching the real binary: its temporary home, the script layer, and waiting for it to end.

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::Image;

/// How a launch runs. Every run is bounded: `max_frames` and `timeout_s` are always passed.
#[derive(Debug, Clone)]
pub struct LaunchOpts {
    pub max_frames: u64,
    pub timeout_s: u64,
    pub scale: u32,
    /// Start the app at launch, unscripted, for the OS layer to drive.
    pub os: bool,
    /// Further arguments, after the ones the harness always passes.
    pub extra_args: Vec<String>,
}

impl Default for LaunchOpts {
    fn default() -> LaunchOpts {
        LaunchOpts {
            max_frames: 100,
            timeout_s: 60,
            scale: 1,
            os: false,
            extra_args: Vec::new(),
        }
    }
}

/// What a run left behind.
#[derive(Debug)]
pub struct Captured {
    /// Frame N is the picture after step N; frame 0 the start.
    pub frames: Vec<Image>,
    pub audio_wav: PathBuf,
    pub exit: ExitStatus,
    /// `N joy keys` for each step N whose input was not empty (sf-app `capture.rs`).
    pub inputs: String,
    pub stderr: String,
    /// The capture folder.
    pub dir: PathBuf,
}

/// The app under test, in its own temporary home. The home is removed when this is dropped,
/// unless `SF_E2E_KEEP=1`.
pub struct App {
    pub opts: LaunchOpts,
    pub home: PathBuf,
    runs: u32,
    /// The running app, for the OS layer.
    pub(crate) child: Option<Child>,
}

impl App {
    /// A temporary home with the real disks imported into it.
    pub fn launch(opts: LaunchOpts) -> App {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let home = std::env::temp_dir().join(format!("sf-e2e-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join("cache")).expect("temp home");
        let seed = seed_cache();
        std::fs::copy(seed, home.join("cache").join(seed.file_name().unwrap()))
            .unwrap_or_else(|e| panic!("copying the imported cache into {}: {e}", home.display()));
        let mut app = App {
            opts,
            home,
            runs: 0,
            child: None,
        };
        if app.opts.os {
            let dir = app.capture_dir();
            app.child = Some(app.spawn(None, &dir));
        }
        app
    }

    /// The script layer: run `script` to the frame cap and return what the app captured.
    pub fn run_script(&mut self, script: &str) -> Captured {
        let dir = self.capture_dir();
        let path = dir.with_extension("script");
        std::fs::write(&path, script).expect("writing the script");
        let child = self.spawn(Some(&path), &dir);
        self.wait(child, &dir)
    }

    /// Wait for the app the OS layer is driving to end by itself, and return what it captured.
    pub fn finish(&mut self) -> Captured {
        let child = self.child.take().expect("no app is running: launch with `os: true`");
        let dir = self.capture_dir_of(self.runs - 1);
        self.wait(child, &dir)
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.as_ref().map(Child::id)
    }

    fn capture_dir(&mut self) -> PathBuf {
        self.runs += 1;
        self.capture_dir_of(self.runs - 1)
    }

    fn capture_dir_of(&self, run: u32) -> PathBuf {
        self.home.join(format!("capture-{run}"))
    }

    fn spawn(&self, script: Option<&Path>, capture: &Path) -> Child {
        let mut cmd = Command::new(binary());
        cmd.arg("--home")
            .arg(&self.home)
            .arg("--capture")
            .arg(capture)
            .arg("--max-frames")
            .arg(self.opts.max_frames.to_string())
            .arg("--timeout")
            .arg(self.opts.timeout_s.to_string())
            .arg("--scale")
            .arg(self.opts.scale.to_string());
        if let Some(s) = script {
            cmd.arg("--script").arg(s);
        }
        cmd.args(&self.opts.extra_args)
            .env_remove("SUPERFROG_HOME")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(
                std::fs::File::create(capture.with_extension("stderr")).expect("stderr file"),
            ))
            // Its own process group: the app's --timeout stops the whole group, and so does the
            // harness's own guard below.
            .process_group(0);
        cmd.spawn()
            .unwrap_or_else(|e| panic!("launching {}: {e}", binary().display()))
    }

    fn wait(&self, mut child: Child, dir: &Path) -> Captured {
        let exit = wait_bounded(
            &mut child,
            Duration::from_secs(self.opts.timeout_s + 30),
        );
        let stderr = std::fs::read_to_string(dir.with_extension("stderr")).unwrap_or_default();
        let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect())
            .unwrap_or_default();
        names.retain(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("frame-")));
        names.sort();
        let frames = names
            .iter()
            .map(|p| Image::read_png(p).unwrap_or_else(|e| panic!("{e}")))
            .collect();
        Captured {
            frames,
            audio_wav: dir.join("audio.wav"),
            exit,
            inputs: std::fs::read_to_string(dir.join("inputs.txt")).unwrap_or_default(),
            stderr,
            dir: dir.to_path_buf(),
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(mut c) = self.child.take() {
            kill_group(c.id());
            let _ = c.wait();
        }
        if std::env::var_os("SF_E2E_KEEP").is_none_or(|v| v != "1") {
            let _ = std::fs::remove_dir_all(&self.home);
        }
    }
}

/// Wait for `child` at most `limit`; past it, kill its process group and panic.
pub(crate) fn wait_bounded(child: &mut Child, limit: Duration) -> ExitStatus {
    let start = Instant::now();
    loop {
        if let Some(s) = child.try_wait().expect("waiting for the app") {
            return s;
        }
        if start.elapsed() > limit {
            kill_group(child.id());
            let _ = child.wait();
            panic!("the app ran past {limit:?} and was killed by the harness");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

pub(crate) fn kill_group(pid: u32) {
    // SAFETY: a signal to the process group the harness made for this child.
    unsafe {
        libc::killpg(pid as i32, libc::SIGKILL);
    }
}

/// Does any process remain in the group `pgid`?
pub fn group_alive(pgid: u32) -> bool {
    // SAFETY: signal 0 only checks that the group exists.
    unsafe { libc::killpg(pgid as i32, 0) == 0 }
}

/// The `superfrog` binary, built once per test process into the same target folder as the tests
/// (Cargo builds a package's binary only for that package's own tests).
pub fn binary() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let exe = std::env::current_exe().expect("test executable");
        // target/<profile>/deps/<test> → target/<profile>
        let profile_dir = exe.parent().and_then(Path::parent).expect("target folder").to_path_buf();
        let target = profile_dir.parent().expect("target folder");
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let mut cmd = Command::new(cargo);
        cmd.current_dir(&workspace)
            .args(["build", "-q", "-p", "sf-app", "--bin", "superfrog"])
            .env("CARGO_TARGET_DIR", target)
            .env("CARGO_TERM_COLOR", "never");
        if profile_dir.file_name().is_some_and(|n| n == "release") {
            cmd.arg("--release");
        }
        let out = cmd.output().expect("running cargo build");
        assert!(
            out.status.success(),
            "cargo build -p sf-app failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        profile_dir.join("superfrog")
    })
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
        let mut child = Command::new(binary())
            .arg("--home")
            .arg(&home)
            .arg("--cache-from-disks")
            .arg(&disks)
            .args(["--max-frames", "0", "--timeout", "120"])
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .expect("launching the import");
        let status = wait_bounded(&mut child, Duration::from_secs(150));
        let mut err = String::new();
        if let Some(mut e) = child.stderr.take() {
            use std::io::Read;
            let _ = e.read_to_string(&mut err);
        }
        assert!(status.success(), "importing the disks failed ({status}): {err}");
        // The file this build reads, by name: an older format's file may sit beside it, and
        // `assets-v9.bin` sorts after `assets-v10.bin`.
        let file = home.join(format!("cache/assets-v{}.bin", sf_assets::CACHE_FORMAT));
        assert!(file.is_file(), "the import wrote no {}", file.display());
        file
    })
}

/// The 16-bit stereo samples of a WAV the app wrote, and its rate.
pub fn read_wav(path: &Path) -> (u32, Vec<[i16; 2]>) {
    let b = std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    assert!(b.len() >= 44 && &b[..4] == b"RIFF" && &b[36..40] == b"data", "not the app's WAV");
    let rate = u32::from_le_bytes(b[24..28].try_into().unwrap());
    let samples = b[44..]
        .chunks_exact(4)
        .map(|s| {
            [
                i16::from_le_bytes([s[0], s[1]]),
                i16::from_le_bytes([s[2], s[3]]),
            ]
        })
        .collect();
    (rate, samples)
}
