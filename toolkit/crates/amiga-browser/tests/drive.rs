//! The rig against made-up pages in `tests/fixtures/`. Chrome missing fails these tests with where
//! it looked; they never skip.

use amiga_browser::{Browser, Opts};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn launch(opts: Opts) -> Browser {
    Browser::launch(opts).unwrap_or_else(|e| panic!("amiga-browser: {e}"))
}

/// Processes still alive that belong to a session: in its group, or naming its profile (a helper
/// that left the group would still carry `--user-data-dir`).
fn leftovers(pgid: u32, profile: &Path) -> Vec<String> {
    let out = std::process::Command::new("ps")
        .args(["-axo", "pid=,pgid=,command="])
        .output()
        .expect("ps");
    let profile = profile.to_string_lossy().to_string();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|line| {
            let mut cols = line.split_whitespace();
            let _pid = cols.next();
            let group = cols.next().and_then(|g| g.parse::<u32>().ok());
            group == Some(pgid) || line.contains(&profile)
        })
        .map(str::to_string)
        .collect()
}

/// Helpers are reparented when Chrome dies and reaped a moment later.
fn assert_all_gone(pgid: u32, profile: &Path) {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let left = leftovers(pgid, profile);
        if left.is_empty() {
            break;
        }
        assert!(
            Instant::now() < until,
            "Chrome processes remain:\n{}",
            left.join("\n")
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    // SAFETY: signal 0 only asks whether the group exists.
    let alive = unsafe { libc::kill(-(pgid as i32), 0) } == 0;
    assert!(!alive, "Chrome's process group {pgid} still exists");
}

#[test]
fn a_page_opens_reads_its_text_and_a_click_changes_it() {
    let mut b = launch(Opts {
        limit: Duration::from_secs(20),
        width: 800,
        height: 600,
    });
    b.open(&fixture("button.html")).unwrap();
    assert_eq!(
        b.eval("document.getElementById('text').textContent")
            .unwrap(),
        "before"
    );
    assert_eq!(
        b.eval("[innerWidth, innerHeight]").unwrap(),
        serde_json::json!([800, 600])
    );
    b.click(160.0, 120.0).unwrap();
    b.wait_for(
        "document.getElementById('text').textContent === 'after'",
        Duration::from_secs(2),
    )
    .unwrap();
    b.scroll(400.0, 300.0, 0.0, 500.0).unwrap();
    b.wait_for("scrollY > 0", Duration::from_secs(2)).unwrap();
    b.viewport(1920, 1080).unwrap();
    assert_eq!(b.eval("innerWidth").unwrap(), 1920);
    let png = b.profile().join("shot.png");
    b.screenshot(&png).unwrap();
    let bytes = std::fs::read(&png).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    // The PNG header's width and height are the viewport's.
    assert_eq!(u32::from_be_bytes(bytes[16..20].try_into().unwrap()), 1920);
    assert_eq!(u32::from_be_bytes(bytes[20..24].try_into().unwrap()), 1080);
    assert!(
        b.eval("throw new Error('boom')")
            .unwrap_err()
            .contains("boom")
    );
    assert!(
        b.blocked_requests().is_empty(),
        "{:?}",
        b.blocked_requests()
    );
}

#[test]
fn a_font_from_the_network_is_blocked_and_listed() {
    let mut b = launch(Opts::default());
    b.open(&fixture("font.html")).unwrap();
    b.wait_for("document.fonts.status === 'loaded'", Duration::from_secs(5))
        .unwrap();
    let font = "https://fonts.example.com/far.woff2".to_string();
    assert_eq!(b.blocked_requests(), vec![font]);
    let failed = b
        .eval("Promise.all([...document.fonts].map(f => f.loaded.catch(() => 'failed')))")
        .unwrap();
    assert_eq!(failed, serde_json::json!(["failed"]));
}

#[test]
fn a_script_file_replaced_by_rename_is_read_again() {
    let dir = std::env::temp_dir().join(format!("amiga-browser-live-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::copy(fixture("live.html"), dir.join("live.html")).unwrap();
    let write = |n: u32| {
        let tmp = dir.join("live.js.tmp");
        std::fs::write(&tmp, format!("window.frame = {n};")).unwrap();
        std::fs::rename(&tmp, dir.join("live.js")).unwrap();
    };
    write(1);
    let mut b = launch(Opts::default());
    b.open(&dir.join("live.html")).unwrap();
    b.wait_for("window.frame === 1", Duration::from_secs(3))
        .unwrap();
    write(2);
    b.wait_for("window.frame === 2", Duration::from_secs(3))
        .unwrap();
    assert!(
        b.blocked_requests().is_empty(),
        "{:?}",
        b.blocked_requests()
    );
    drop(b);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_page_that_never_finishes_is_cut_off_at_the_limit() {
    let limit = Duration::from_secs(3);
    let mut b = launch(Opts {
        limit,
        ..Opts::default()
    });
    let (pid, profile) = (b.pid(), b.profile().to_path_buf());
    let started = Instant::now();
    let err = b.open(&fixture("hang.html")).unwrap_err();
    assert!(err.contains("time limit"), "{err}");
    assert!(
        started.elapsed() < limit + Duration::from_secs(2),
        "{:?}",
        started.elapsed()
    );
    assert_all_gone(pid, &profile);
    // Every later call reports the same.
    assert!(b.eval("1").unwrap_err().contains("time limit"));
    drop(b);
    assert!(
        !profile.exists(),
        "profile {} left behind",
        profile.display()
    );
}

#[test]
fn dropping_the_browser_leaves_no_chrome_process_or_profile() {
    let mut b = launch(Opts::default());
    b.open(&fixture("button.html")).unwrap();
    let (pid, profile) = (b.pid(), b.profile().to_path_buf());
    assert!(
        !leftovers(pid, &profile).is_empty(),
        "Chrome not found by its group before drop"
    );
    drop(b);
    // SAFETY: signal 0 only asks whether the process exists.
    assert_ne!(
        unsafe { libc::kill(pid as i32, 0) },
        0,
        "Chrome {pid} still alive"
    );
    assert_all_gone(pid, &profile);
    assert!(
        !profile.exists(),
        "profile {} left behind",
        profile.display()
    );
}
