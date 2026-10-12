//! The front end against the original (T20): the level codes against the game's own
//! `password_check` on the routine oracle, every code there is. The frame cases (`front-*`) are in
//! `engine/conform/cases/`.

use std::sync::OnceLock;

use sf_core::front::password::{self, Password, Progress};
use sf_disk::{DiskSlot, Offered, carve, decompress, identify};
use sf_oracle::{Cpu68k, Regs};

fn game_exe() -> Vec<u8> {
    let files: Vec<Offered> = sf_conform::disks::read_disk_files()
        .unwrap_or_else(|e| panic!("{e}"))
        .into_iter()
        .map(|(name, bytes)| Offered { name, bytes })
        .collect();
    let set = identify(&files).unwrap_or_else(|e| panic!("{e:?}"));
    let chunk = carve(set.disk(DiskSlot::D1))
        .into_iter()
        .find(|c| c.index == 3)
        .unwrap();
    decompress(&chunk.raw).unwrap()
}

struct Original {
    cpu: Cpu68k,
    check: u32,
    buf: u32,
    world: u32,
    level: u32,
}

fn original() -> &'static std::sync::Mutex<Original> {
    static O: OnceLock<std::sync::Mutex<Original>> = OnceLock::new();
    O.get_or_init(|| {
        let mut cpu = Cpu68k::new(sf_oracle::AmigaMemory {
            chip: vec![0; 512 * 1024],
            slow: vec![0; 512 * 1024],
        });
        let map = cpu.load_exe(&game_exe(), 0x1_0000).unwrap();
        std::sync::Mutex::new(Original {
            check: map.addr(0, 0x2432),
            buf: map.addr(0, 0x242a),
            world: map.addr(0, 0xa),
            level: map.addr(0, 0xe),
            cpu,
        })
    })
}

/// `password_check` on the original: where the typed code starts the game, `None` when it leaves
/// the start as it was (both zero, as the entry sets them).
fn original_check(o: &mut Original, code: &[u8; 6]) -> Option<Progress> {
    o.cpu.write(o.buf, code);
    o.cpu.write(o.world, &[0; 8]);
    let mut regs = Regs::default();
    regs.a[7] = 0x8_0000;
    let got = o
        .cpu
        .call(o.check, regs, 10_000)
        .expect("password_check returns");
    let long = |a| u32::from_be_bytes(got.bytes_after(&o.cpu, a, 4).try_into().unwrap());
    let (w, l) = (long(o.world), long(o.level));
    (w != 0 || l != 0).then_some(Progress {
        world: w as u8,
        level: l as u8,
    })
}

#[test]
fn every_six_digit_code_starts_where_the_original_starts_it() {
    let table = password::table(&sf_conform::display_data().front);
    assert_eq!(table.len(), 28, "the table has 28 codes");
    let mut o = original().lock().unwrap();
    let mut known = 0;
    for n in 0..1_000_000u32 {
        let digits: [u8; 6] =
            std::array::from_fn(|k| b'0' + (n / 10u32.pow(5 - k as u32) % 10) as u8);
        let ours = password::decode_password(&table, &Password(digits));
        let theirs = original_check(&mut o, &digits);
        assert_eq!(ours, theirs, "code {}", String::from_utf8_lossy(&digits));
        known += ours.is_some() as u32;
    }
    assert!(known >= 24, "{known} codes start somewhere");
}

#[test]
fn each_world_and_level_has_a_code_that_starts_it() {
    let table = password::table(&sf_conform::display_data().front);
    for world in 1..=6u8 {
        for level in 1..=4u8 {
            let p = Progress { world, level };
            let code =
                password::password_for(&table, &p).unwrap_or_else(|| panic!("no code for {p:?}"));
            assert_eq!(password::decode_password(&table, &code), Some(p));
        }
    }
    let bonus = Progress { world: 7, level: 1 };
    let code = password::password_for(&table, &bonus).expect("the bonus level has codes");
    assert_eq!(password::decode_password(&table, &code), Some(bonus));
}

/// Power-on to the title, frame for frame (the case `front-boot`), in the test block.
#[test]
fn power_on_to_the_title_matches_the_original() {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("front-boot.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    if let Err(d) = sf_conform::run(&case) {
        panic!("{d}");
    }
}

/// The level-code entry, frame for frame (the case `front-password`).
#[test]
fn the_level_code_entry_matches_the_original() {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("front-password.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    if let Err(d) = sf_conform::run(&case) {
        panic!("{d}");
    }
}

/// Fire skipping the title's intro, held on into the fade-in (the case `front-skip-blink`): the
/// restore blit's length under the six-plane display decides the frame the fade-in starts on.
#[test]
fn fire_skipping_the_title_intro_matches_the_original() {
    let case = sf_conform::Case::load(&sf_conform::cases_dir().join("front-skip-blink.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    if let Err(d) = sf_conform::run(&case) {
        panic!("{d}");
    }
}
