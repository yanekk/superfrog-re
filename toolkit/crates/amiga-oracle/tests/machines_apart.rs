//! Game-free: setting a machine up leaves one running on another thread alone. Needs the Kickstart
//! (`AMIGA_KICKSTART`, else `~/amiga-roms/kick1.3.rom`) and `toolkit/tools/setup/oracle.sh`'s build.
//!
//! The core keeps one table of a line's disk, audio and sprite DMA slots for every machine in the
//! process, and wiped and refilled it in every machine's set-up. A running machine copies its
//! line's slots from it twice a frame while sprite DMA is on, so one stepped on another thread
//! meanwhile could copy an empty line: the CPU then gets bus cycles it should not, and the run is
//! a few instructions early from there on. Two runs of one original compared in parallel with
//! others differed that way about once in several hundred (learned on: forest-dumb T85). Core
//! patch 0007 fills the table once.
//!
//! The wipe lasts about a microsecond, so two runs compared here would almost never meet it. The
//! test reads the table itself instead, without a pause, while machines are set up.

use std::sync::atomic::{AtomicBool, Ordering};

use amiga_oracle::*;

/// Machines set up while the table is watched.
const MACHINES: u32 = 60;

fn set_up() {
    let rom = kickstart_path().unwrap_or_else(|e| panic!("{e}"));
    drop(Amiga::power_on(&OracleConfig::new(rom, Vec::new(), 1)).unwrap());
}

#[test]
fn setting_a_machine_up_never_wipes_the_table_running_machines_share() {
    // The first machine fills it.
    set_up();
    assert!(
        das_table_whole(),
        "the first machine's set-up left the table unfilled"
    );

    let done = AtomicBool::new(false);
    let (looks, wiped) = std::thread::scope(|s| {
        let watcher = s.spawn(|| {
            let (mut looks, mut wiped) = (0u64, 0u64);
            while !done.load(Ordering::Relaxed) {
                looks += 1;
                wiped += !das_table_whole() as u64;
            }
            (looks, wiped)
        });
        for _ in 0..MACHINES {
            set_up();
        }
        done.store(true, Ordering::Relaxed);
        watcher.join().unwrap()
    });
    // A look takes well under the wipe's microsecond; fewer looks than this and the watcher was
    // starved, so the test saw nothing.
    assert!(
        looks > 100_000 * MACHINES as u64,
        "only {looks} looks over {MACHINES} set-ups"
    );
    assert_eq!(wiped, 0, "of {looks} looks, over {MACHINES} set-ups");
}
