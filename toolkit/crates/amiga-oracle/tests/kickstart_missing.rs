//! A missing Kickstart fails with where it looked (owner, 2026-10-28). Its own test binary: it
//! sets `AMIGA_KICKSTART`, which no other test in this process may see.

use amiga_oracle::*;

#[test]
fn a_missing_kickstart_names_the_path_it_tried() {
    let scratch = std::env::temp_dir().join(format!("amiga-oracle-{}", std::process::id()));
    let rom = scratch.join("no-such-kick.rom");
    // SAFETY: the only test in this binary, so no other thread reads the environment meanwhile.
    unsafe { std::env::set_var("AMIGA_KICKSTART", &rom) };
    let e = kickstart_path().unwrap_err().to_string();
    assert!(e.contains(&rom.display().to_string()), "{e}");
    assert!(e.contains("AMIGA_KICKSTART"), "{e}");

    // A set-up naming a ROM that is not there fails at power-on, naming the file.
    let e = Amiga::power_on(&OracleConfig::new(rom.clone(), Vec::new(), 1))
        .err()
        .expect("no ROM, no machine")
        .to_string();
    assert!(e.contains(&rom.display().to_string()), "{e}");
}
