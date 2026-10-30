//! `locate` reads `SUPERFROG_KICKSTART` first, then the toolkit's `AMIGA_KICKSTART`, then
//! `~/amiga-roms/kick1.3.rom` (toolkit-extract DESIGN §2.5). Its own test binary: it sets both
//! variables, which no other test in this process may see. Needs the owner's Kickstart and disks.

use sf_oracle::*;

#[test]
fn superfrog_kickstart_comes_first_then_the_toolkits_lookup() {
    let real = OracleConfig::locate(1).unwrap_or_else(|e| panic!("{e}")).kickstart;
    let scratch = std::env::temp_dir().join(format!("sf-oracle-kick-{}", std::process::id()));
    let (amiga, sf) = (scratch.join("amiga.rom"), scratch.join("sf.rom"));
    // SAFETY: the only test in this binary, so no other thread reads the environment meanwhile.
    unsafe {
        std::env::remove_var("SUPERFROG_KICKSTART");
        std::env::set_var("AMIGA_KICKSTART", &amiga);
    }
    let e = OracleConfig::locate(1).unwrap_err().to_string();
    assert!(e.contains(&amiga.display().to_string()), "{e}");
    assert!(e.contains("AMIGA_KICKSTART") && e.contains("SUPERFROG_KICKSTART"), "{e}");

    unsafe { std::env::set_var("AMIGA_KICKSTART", &real) };
    assert_eq!(OracleConfig::locate(1).unwrap().kickstart, real);

    // SUPERFROG_KICKSTART wins over a good AMIGA_KICKSTART, and a wrong one is an error.
    unsafe { std::env::set_var("SUPERFROG_KICKSTART", &sf) };
    let e = OracleConfig::locate(1).unwrap_err().to_string();
    assert!(e.contains(&sf.display().to_string()) && e.contains("SUPERFROG_KICKSTART"), "{e}");
    unsafe { std::env::set_var("SUPERFROG_KICKSTART", &real) };
    unsafe { std::env::set_var("AMIGA_KICKSTART", &amiga) };
    assert_eq!(OracleConfig::locate(1).unwrap().kickstart, real);
}
