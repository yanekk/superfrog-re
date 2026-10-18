//! T23: the first-run import and the launch decision, through the real binary (DESIGN §2.2,
//! §2.11). The script layer answers the disk picker and the missing-disks message (`pick`,
//! `answer`); every question the app asked is in the capture's `prompts.txt`.

mod common;

use common::{app_files, cache_data, cache_file, headless};
use sf_core::input::FrameInput;
use sf_e2e::{App, Image, LaunchOpts};

fn pick(paths: &[&std::path::PathBuf]) -> String {
    let p: Vec<String> = paths.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    format!("pick {}\n", p.join("; "))
}

/// The `missing:` lines of a run's prompts.
fn missing(prompts: &str) -> Vec<&str> {
    prompts.lines().filter(|l| l.starts_with("missing: ")).collect()
}

#[test]
fn a_first_launch_asks_for_the_disks_imports_the_four_zips_and_reaches_the_title() {
    // The engine alone: the first frame the title waits for the player.
    let data = cache_data(&App::launch(LaunchOpts::default()).home);
    let mut g = headless(&data);
    let title = (1..3000u64)
        .find(|_| {
            g.step(FrameInput::NONE);
            g.title_waits()
        })
        .expect("the title within 3000 frames");

    let frames = title + 10;
    let mut app = App::launch_empty(LaunchOpts {
        max_frames: frames,
        timeout_s: 120,
        ..Default::default()
    });
    assert_eq!(app_files(&app.home), Vec::<String>::new(), "the home starts empty");
    let d = app.disks_in_home("disks");
    let c = app.run_script_unread(&pick(&d.iter().collect::<Vec<_>>()));
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(
        c.prompts,
        "picker: The game data has not been imported yet. Choose your four Superfrog disk images \
         to import them.\n",
        "the import prompt, once"
    );
    assert_eq!(cache_data(&app.home), data, "the cache is the disks' game data");
    let remembered = std::fs::read_to_string(app.home.join("disks.toml")).unwrap();
    for p in &d {
        assert!(remembered.contains(&*p.to_string_lossy()), "{remembered}");
    }
    let mut g = headless(&data);
    let mut want = None;
    for _ in 1..=frames {
        want = Some(g.step(FrameInput::NONE).picture);
    }
    let last = c.read_frame(frames);
    assert_eq!(last, Image::of_picture(&want.unwrap()), "the app shows the engine's title");
    assert!(common::not_black(&last), "the title is not black");
}

#[test]
fn a_cancelled_picker_names_every_disk_and_quit_writes_nothing() {
    let mut app = App::launch_empty(LaunchOpts::default());
    let c = app.run_script("pick cancel\nanswer quit\n");
    assert_eq!(c.exit.code(), Some(0), "Quit is a clean exit: {}", c.stderr);
    let m = missing(&c.prompts);
    assert_eq!(m.len(), 1, "{}", c.prompts);
    assert!(m[0].starts_with("missing: No disk images were chosen."), "{}", m[0]);
    for n in 1..=4 {
        assert!(m[0].contains(&format!("Superfrog Disk {n} of 4 is missing")), "{}", m[0]);
    }
    assert!(m[0].ends_with("-> quit"));
    assert!(c.frames.is_empty(), "no game ran");
    assert_eq!(app_files(&app.home), Vec::<String>::new(), "Quit wrote nothing");
}

#[test]
fn two_disks_name_the_other_two_and_choose_disks_reopens_the_picker() {
    let mut app = App::launch_empty(LaunchOpts {
        max_frames: 5,
        ..Default::default()
    });
    let d = app.disks_in_home("disks");
    // Two disks; "Choose Disks…"; then all four.
    let script = format!("{}answer choose\n{}", pick(&[&d[0], &d[1]]), pick(&d.iter().collect::<Vec<_>>()));
    let c = app.run_script(&script);
    assert!(c.exit.success(), "{}", c.stderr);
    let kinds: Vec<&str> = c.prompts.lines().map(|l| l.split(':').next().unwrap()).collect();
    assert_eq!(kinds, ["picker", "missing", "picker"], "{}", c.prompts);
    let m = missing(&c.prompts)[0];
    assert!(m.contains("Disk 3 of 4 is missing") && m.contains("Disk 4 of 4 is missing"), "{m}");
    assert!(!m.contains("Disk 1 of 4") && !m.contains("Disk 2 of 4"), "{m}");
    assert!(m.ends_with("-> choose"), "{m}");
    assert!(cache_file(&app.home).is_file(), "the second pick imported");
    assert_eq!(c.frames.len(), 6, "and the game ran");
}

#[test]
fn a_wrong_disk_or_a_missing_one_is_named_by_slot_and_nothing_reaches_the_cache() {
    let mut app = App::launch_empty(LaunchOpts::default());
    let d = app.disks_in_home("disks");
    // Disk 2 offered again in disk 3's place, under another name.
    let again = app.home.join("disks/copy of disk 2.zip");
    std::fs::copy(&d[1], &again).unwrap();
    // A file the right size that is no Superfrog disk.
    let blank = app.home.join("disks/blank.adf");
    std::fs::write(&blank, vec![0u8; 901_120]).unwrap();
    let script = format!(
        "{}answer choose\n{}answer choose\n{}answer quit\n",
        pick(&[&d[0], &d[1], &again, &d[3]]),
        pick(&[&d[0], &d[1], &blank, &d[3]]),
        pick(&[&d[0], &d[1], &d[3]]),
    );
    let c = app.run_script(&script);
    assert!(c.exit.success(), "{}", c.stderr);
    let m = missing(&c.prompts);
    assert_eq!(m.len(), 3, "{}", c.prompts);
    assert!(m[0].contains("Superfrog Disk 2 of 4 was chosen more than once"), "{}", m[0]);
    assert!(m[0].contains("Superfrog Disk 3 of 4 is missing"), "{}", m[0]);
    assert!(m[1].contains("“blank.adf” is not one of the four Superfrog disks"), "{}", m[1]);
    assert!(m[1].contains("Superfrog Disk 3 of 4 is missing"), "{}", m[1]);
    assert!(m[2].contains("Superfrog Disk 3 of 4 is missing"), "{}", m[2]);
    assert!(!m[2].contains("Disk 1 of 4") && !m[2].contains("Disk 4 of 4"), "{}", m[2]);
    assert!(!app.home.join("cache").exists(), "nothing was written to the cache");
    assert!(!app.home.join("disks.toml").exists());
}

#[test]
fn a_damaged_cache_is_imported_again_silently_until_the_disks_are_moved() {
    let mut app = App::launch_empty(LaunchOpts {
        max_frames: 5,
        ..Default::default()
    });
    let d = app.disks_in_home("disks");
    let c = app.run_script(&pick(&d.iter().collect::<Vec<_>>()));
    assert!(c.exit.success(), "{}", c.stderr);
    let good = std::fs::read(cache_file(&app.home)).unwrap();

    // Damaged: re-imported from the remembered disks, with no question.
    let mut bad = good.clone();
    bad.truncate(bad.len() / 2);
    std::fs::write(cache_file(&app.home), &bad).unwrap();
    let c = app.run_script("");
    assert!(c.exit.success(), "{}", c.stderr);
    assert_eq!(c.prompts, "", "the re-import asked nothing");
    assert!(c.stderr.contains("is being imported from your Superfrog disks"), "{}", c.stderr);
    assert_eq!(cache_data(&app.home), sf_assets::cache_decode(&good).unwrap());
    assert_eq!(c.frames.len(), 6);

    // The disks moved away: the next damaged cache asks for them again.
    std::fs::rename(app.home.join("disks"), app.home.join("moved")).unwrap();
    std::fs::write(cache_file(&app.home), &bad).unwrap();
    let c = app.run_script("answer quit\n");
    assert!(c.exit.success(), "{}", c.stderr);
    let first = c.prompts.lines().next().unwrap_or("");
    assert!(first.starts_with("picker: "), "{}", c.prompts);
    assert!(first.contains("the disk images used last time are gone or have changed"), "{first}");
    assert!(first.contains("Superfrog Disk 1 of 4 is missing"), "{first}");
    assert_eq!(std::fs::read(cache_file(&app.home)).unwrap(), bad, "Quit left the cache alone");
}
