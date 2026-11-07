//! T13: missing or wrong disks (DESIGN §2.9, §2.12): a plain message naming where the app looked, and a
//! non-zero exit, before any window opens.

use fd_e2e::{App, LaunchOpts};

#[test]
fn a_disks_folder_without_the_zips_is_named_and_the_app_exits_1() {
    let empty = std::env::temp_dir().join(format!("fd-e2e-no-disks-{}", std::process::id()));
    std::fs::create_dir_all(&empty).unwrap();
    let mut app = App::launch_bare(LaunchOpts {
        max_frames: 10,
        extra_args: vec!["--disks".into(), empty.display().to_string()],
        ..Default::default()
    });
    let c = app.run_script("");
    std::fs::remove_dir_all(&empty).unwrap();
    assert_eq!(c.exit.code(), Some(1), "{}", c.stderr);
    assert!(c.stderr.contains("zips not found"), "{}", c.stderr);
    assert!(
        c.stderr.contains(&empty.display().to_string()),
        "names where it looked: {}",
        c.stderr
    );
    assert!(!c.stderr.contains("playing W1-1"), "{}", c.stderr);
    assert!(!c.stderr.contains("panicked"), "{}", c.stderr);
}

#[test]
fn damaged_zips_are_named_and_the_app_exits_1() {
    let dir = std::env::temp_dir().join(format!("fd-e2e-bad-disks-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for n in [1, 2] {
        std::fs::write(
            dir.join(format!("Forest Dumb Forever!!! (Disk {n} of 2)[cr WT].zip")),
            b"not a zip",
        )
        .unwrap();
    }
    let mut app = App::launch_bare(LaunchOpts {
        max_frames: 10,
        extra_args: vec!["--disks".into(), dir.display().to_string()],
        ..Default::default()
    });
    let c = app.run_script("");
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(c.exit.code(), Some(1), "{}", c.stderr);
    assert!(
        c.stderr.contains("(Disk 1 of 2)[cr WT].zip"),
        "names the zip: {}",
        c.stderr
    );
    assert!(
        !c.stderr.contains("playing W1-1") && !c.stderr.contains("panicked"),
        "{}",
        c.stderr
    );
}
