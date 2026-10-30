//! A planner child: run in its own process group, killed as a group past its timeout (DESIGN §5.2:
//! budget + 5 min), and never outliving the benchmark (§5.2, "a killed benchmark leaves no
//! emulator running").
//!
//! The benchmark can be killed in ways it cannot catch (SIGKILL, a closed terminal), so each child
//! gets a guard: a small shell loop in the child's own group that checks once a second whether the
//! benchmark is still alive and, when it is not, kills the group, itself included. On a normal end
//! the benchmark kills the group itself, which takes the guard with it.

use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use std::os::unix::process::CommandExt;

/// What a child's run came to.
#[derive(Debug, Clone)]
pub struct ChildRun {
    /// `None` when it was killed for its timeout.
    pub status: Option<ExitStatus>,
    pub timed_out: bool,
    pub wall: Duration,
}

/// `$1` the process to outlive no longer than, `$2` the group to kill.
const GUARD: &str = "while kill -0 \"$1\" 2>/dev/null; do sleep 1; done; kill -9 -\"$2\" 2>/dev/null";

/// Kills its group when dropped (a panic in the benchmark included).
struct Group(i32);

impl Drop for Group {
    fn drop(&mut self) {
        // SAFETY: killpg only sends a signal; a group already gone returns an error, ignored.
        unsafe {
            libc::killpg(self.0, libc::SIGKILL);
        }
    }
}

/// Runs `cmd` to its end or `timeout`, whichever comes first. `watch` is the process whose end
/// kills the child (this process when `None`; a test passes a stand-in).
pub fn run(mut cmd: Command, timeout: Duration, watch: Option<u32>) -> Result<ChildRun, String> {
    let t0 = Instant::now();
    cmd.process_group(0);
    let mut child = cmd.spawn().map_err(|e| format!("{cmd:?}: {e}"))?;
    let group = Group(child.id() as i32);
    let mut guard = guard(watch.unwrap_or_else(std::process::id), group.0);
    let mut timed_out = false;
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(s) => break Some(s),
            None if t0.elapsed() >= timeout => {
                timed_out = true;
                // SAFETY: as in `Group::drop`.
                unsafe {
                    libc::killpg(group.0, libc::SIGKILL);
                }
                let _ = child.wait();
                break None;
            }
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    };
    let wall = t0.elapsed();
    drop(group);
    if let Some(g) = guard.as_mut() {
        let _ = g.wait();
    }
    Ok(ChildRun {
        status,
        timed_out,
        wall,
    })
}

/// The guard for group `pgid`, in that group. `None` if the group is already gone (the child
/// ended before the guard could join it): there is nothing left to guard.
fn guard(watch: u32, pgid: i32) -> Option<Child> {
    Command::new("/bin/sh")
        .args(["-c", GUARD, "sf-bench-guard", &watch.to_string(), &pgid.to_string()])
        .process_group(pgid)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()
}

/// Whether any process of group `pgid` is alive.
pub fn group_alive(pgid: i32) -> bool {
    // SAFETY: signal 0 only checks the group exists.
    unsafe { libc::killpg(pgid, 0) == 0 }
}
