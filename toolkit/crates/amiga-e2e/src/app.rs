//! Launching the real binary: its temporary home, the script layer, and waiting for it to end.

use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use amiga_app::capture;

use crate::{AppSpec, Image};

/// How a launch runs. Every run is bounded: `max_frames` and `timeout_s` are always passed.
#[derive(Debug, Clone)]
pub struct LaunchOpts {
    pub max_frames: u64,
    pub timeout_s: u64,
    /// The scale flag; `None` leaves the app to its remembered setting.
    pub scale: Option<u32>,
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
            scale: Some(1),
            os: false,
            extra_args: Vec::new(),
        }
    }
}

/// What a run left behind.
#[derive(Debug)]
pub struct Captured {
    /// Frame N is the picture after step N; frame 0 the start. Empty after
    /// [`App::run_script_unread`].
    pub frames: Vec<Image>,
    pub audio_wav: PathBuf,
    pub exit: ExitStatus,
    /// `N joy keys` for each step N whose input was not empty (`amiga_app::capture`).
    pub inputs: String,
    pub stderr: String,
    /// Every question the launch asked, one a line.
    pub prompts: String,
    /// The capture folder.
    pub dir: PathBuf,
}

impl Captured {
    /// Frame `n` as the app wrote it (frame 0 is the start, frame N the picture after step N).
    pub fn read_frame(&self, n: u64) -> Image {
        Image::read_png(&self.dir.join(capture::frame_file(n))).unwrap_or_else(|e| panic!("{e}"))
    }

    /// The window as presented at the script's `shot` at step `n`, and what the windows file says
    /// of it.
    pub fn read_shot(&self, n: u64) -> (Image, Shot) {
        let img = Image::read_png(&self.dir.join(capture::window_file(n))).unwrap_or_else(|e| panic!("{e}"));
        let text = std::fs::read_to_string(self.dir.join(capture::WINDOWS)).unwrap_or_default();
        let line = text
            .lines()
            .find(|l| l.split(' ').next() == Some(&n.to_string()))
            .unwrap_or_else(|| panic!("no shot at step {n} in {}: {text:?}", capture::WINDOWS));
        let v: Vec<f64> = line.split(' ').skip(1).map(|v| v.parse().unwrap()).collect();
        let shot = Shot {
            frame: v[0] as u64,
            surface: (v[1] as u32, v[2] as u32),
            scale_factor: v[3],
            viewport: (v[4] as u32, v[5] as u32, v[6] as u32, v[7] as u32),
        };
        (img, shot)
    }
}

/// What the app said it presented at a `shot`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shot {
    /// The frame the window showed (the newest stepped when it was drawn).
    pub frame: u64,
    /// The surface's size in its pixels.
    pub surface: (u32, u32),
    pub scale_factor: f64,
    pub viewport: (u32, u32, u32, u32),
}

/// The app under test, in its own temporary home. The home is removed when this is dropped,
/// unless the spec's keep variable is `1`.
pub struct App {
    pub spec: &'static AppSpec,
    pub opts: LaunchOpts,
    pub home: PathBuf,
    runs: u32,
    /// The running app, for the OS layer.
    pub(crate) child: Option<Child>,
}

impl App {
    /// A temporary home the spec's `provision` has filled.
    pub fn launch(spec: &'static AppSpec, opts: LaunchOpts) -> App {
        let home = temp_home(spec);
        std::fs::create_dir_all(&home).expect("temp home");
        (spec.provision)(&home).unwrap_or_else(|e| panic!("provisioning {}: {e}", home.display()));
        App::in_home(spec, opts, home)
    }

    /// An empty temporary home: a first launch. (The harness's own files, the scripts, captures
    /// and stderr, are `capture-*` in it.)
    pub fn launch_empty(spec: &'static AppSpec, opts: LaunchOpts) -> App {
        let home = temp_home(spec);
        std::fs::create_dir_all(&home).expect("temp home");
        App::in_home(spec, opts, home)
    }

    /// Start the app on this home for the OS layer to drive, as [`LaunchOpts::os`] does at launch.
    pub fn start_os(&mut self) {
        assert!(self.child.is_none(), "the app is already running");
        let dir = self.capture_dir();
        self.child = Some(self.spawn(None, &dir));
    }

    /// Run `script` in the background, for a test that acts while it runs; [`App::finish`] waits.
    pub fn start_script(&mut self, script: &str) {
        assert!(self.child.is_none(), "the app is already running");
        let dir = self.capture_dir();
        let path = dir.with_extension("script");
        std::fs::write(&path, script).expect("writing the script");
        self.child = Some(self.spawn(Some(&path), &dir));
    }

    /// Kill the running app's whole process group, as `kill -9` would, and wait for it.
    pub fn kill(&mut self) {
        let mut c = self.child.take().expect("no app is running");
        kill_group(c.id());
        let _ = c.wait();
    }

    fn in_home(spec: &'static AppSpec, opts: LaunchOpts, home: PathBuf) -> App {
        let mut app = App {
            spec,
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
        self.wait(child, &dir, true)
    }

    /// The script layer for a long run: as [`App::run_script`], but `frames` is left empty and the
    /// test reads the frames it needs by number ([`Captured::read_frame`]), so thousands of them
    /// are not held in memory at once.
    pub fn run_script_unread(&mut self, script: &str) -> Captured {
        let dir = self.capture_dir();
        let path = dir.with_extension("script");
        std::fs::write(&path, script).expect("writing the script");
        let child = self.spawn(Some(&path), &dir);
        self.wait(child, &dir, false)
    }

    /// Wait for the app the OS layer is driving to end by itself, and return what it captured.
    pub fn finish(&mut self) -> Captured {
        let child = self.child.take().expect("no app is running: launch with `os: true`");
        let dir = self.capture_dir_of(self.runs - 1);
        self.wait(child, &dir, true)
    }

    /// [`App::finish`] without reading the frames (see [`App::run_script_unread`]).
    pub fn finish_unread(&mut self) -> Captured {
        let child = self.child.take().expect("no app is running");
        let dir = self.capture_dir_of(self.runs - 1);
        self.wait(child, &dir, false)
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
        let flags = &self.spec.flags;
        let bin = binary(self.spec);
        let mut cmd = Command::new(bin);
        cmd.arg(flags.home)
            .arg(&self.home)
            .arg(flags.capture)
            .arg(capture)
            .arg(flags.max_frames)
            .arg(self.opts.max_frames.to_string())
            .arg(flags.timeout)
            .arg(self.opts.timeout_s.to_string());
        if let Some(s) = self.opts.scale {
            cmd.arg(flags.scale).arg(s.to_string());
        }
        if let Some(s) = script {
            cmd.arg(flags.script).arg(s);
        }
        cmd.args(&self.opts.extra_args);
        for v in &self.spec.clear_env {
            cmd.env_remove(v);
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(
                std::fs::File::create(capture.with_extension("stderr")).expect("stderr file"),
            ))
            // Its own process group: the app's timeout stops the whole group, and so does the
            // harness's own guard below.
            .process_group(0);
        cmd.spawn()
            .unwrap_or_else(|e| panic!("launching {}: {e}", bin.display()))
    }

    fn wait(&self, mut child: Child, dir: &Path, read_frames: bool) -> Captured {
        let exit = wait_bounded(
            &mut child,
            Duration::from_secs(self.opts.timeout_s + 30),
        );
        let stderr = std::fs::read_to_string(dir.with_extension("stderr")).unwrap_or_default();
        let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect())
            .unwrap_or_default();
        names.retain(|p| read_frames && p.file_name().is_some_and(|n| is_frame_file(&n.to_string_lossy())));
        names.sort();
        let frames = names
            .iter()
            .map(|p| Image::read_png(p).unwrap_or_else(|e| panic!("{e}")))
            .collect();
        Captured {
            frames,
            audio_wav: dir.join(capture::AUDIO),
            exit,
            inputs: std::fs::read_to_string(dir.join(capture::INPUTS)).unwrap_or_default(),
            prompts: std::fs::read_to_string(dir.join(capture::PROMPTS)).unwrap_or_default(),
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
        if std::env::var_os(self.spec.keep_env).is_none_or(|v| v != "1") {
            let _ = std::fs::remove_dir_all(&self.home);
        }
    }
}

/// Is `name` a captured frame's file? Matched on [`capture::frame_file`]'s own prefix, so the two
/// cannot drift apart.
fn is_frame_file(name: &str) -> bool {
    let first = capture::frame_file(0);
    let prefix = &first[..first.find('0').unwrap_or(first.len())];
    name.starts_with(prefix)
}

/// A fresh temporary home's path, removed if it was left from before.
fn temp_home(spec: &AppSpec) -> PathBuf {
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let home = std::env::temp_dir().join(format!("{}-{}-{n}", spec.scratch, std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    home
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

/// Run the spec's binary with `args` alone (none of the flags a launch always passes), in its own
/// process group, stopped after `limit`. Another test process (another worktree's) may hold the
/// lock of the home `args` name: while its stderr says [`AppSpec::already_running`], it is run
/// again, for up to 300 s. Returns its exit status and stderr.
pub fn run_waiting_for_lock(spec: &AppSpec, args: &[OsString], limit: Duration) -> (ExitStatus, String) {
    let start = Instant::now();
    loop {
        let mut child = Command::new(binary(spec))
            .args(args)
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .expect("launching the app");
        let status = wait_bounded(&mut child, limit);
        let mut err = String::new();
        if let Some(mut e) = child.stderr.take() {
            use std::io::Read;
            let _ = e.read_to_string(&mut err);
        }
        let held = err.contains(spec.already_running);
        if status.success() || !held || start.elapsed() > Duration::from_secs(300) {
            return (status, err);
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}

/// The spec's binary, built once per test process into the same target folder as the tests
/// (Cargo builds a package's binary only for that package's own tests).
pub fn binary(spec: &AppSpec) -> &'static Path {
    // Held across the build, so two threads asking at once build once.
    static BUILT: Mutex<Vec<(&'static str, &'static str, &'static Path)>> = Mutex::new(Vec::new());
    let mut built = BUILT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(&(_, _, p)) = built.iter().find(|(pkg, bin, _)| *pkg == spec.package && *bin == spec.binary) {
        return p;
    }
    let exe = std::env::current_exe().expect("test executable");
    // target/<profile>/deps/<test> → target/<profile>
    let profile_dir = exe.parent().and_then(Path::parent).expect("target folder").to_path_buf();
    let target = profile_dir.parent().expect("target folder");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut cmd = Command::new(cargo);
    cmd.current_dir(&spec.workspace)
        .args(["build", "-q", "-p", spec.package, "--bin", spec.binary])
        .env("CARGO_TARGET_DIR", target)
        .env("CARGO_TERM_COLOR", "never");
    if profile_dir.file_name().is_some_and(|n| n == "release") {
        cmd.arg("--release");
    }
    let out = cmd.output().expect("running cargo build");
    assert!(
        out.status.success(),
        "cargo build -p {} failed:\n{}",
        spec.package,
        String::from_utf8_lossy(&out.stderr)
    );
    let path: &'static Path = Box::leak(profile_dir.join(spec.binary).into_boxed_path());
    built.push((spec.package, spec.binary, path));
    path
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_files_are_told_from_the_capture_s_other_files() {
        assert!(is_frame_file(&capture::frame_file(0)));
        assert!(is_frame_file(&capture::frame_file(12345)));
        for other in [capture::window_file(3).as_str(), capture::AUDIO, capture::INPUTS, capture::WINDOWS] {
            assert!(!is_frame_file(other), "{other}");
        }
    }

    #[test]
    fn a_wav_reads_back_as_its_rate_and_stereo_samples() {
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF\0\0\0\0WAVEfmt ");
        b.extend_from_slice(&[0; 8]);
        b.extend_from_slice(&48_000u32.to_le_bytes());
        b.extend_from_slice(&[0; 8]);
        b.extend_from_slice(b"data\0\0\0\0");
        b.extend_from_slice(&[1, 0, 0xFF, 0xFF, 0, 0x80, 0xFF, 0x7F]);
        let path = std::env::temp_dir().join(format!("amiga-e2e-wav-{}.wav", std::process::id()));
        std::fs::write(&path, &b).unwrap();
        let (rate, samples) = read_wav(&path);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(rate, 48_000);
        assert_eq!(samples, [[1, -1], [i16::MIN, i16::MAX]]);
    }
}
