//! The routine oracle (T06) on the original's own code: the ATN! decompressor from the owner's disk
//! 1, a cross-check on `sf_disk::decompress` (T04), and the game's executables through `load_exe`.
//! The tests on hand-assembled code and synthetic executables moved with the runner to the toolkit
//! (`toolkit/crates/amiga-oracle/tests/routine.rs`, toolkit-extract T05). Needs
//! `toolkit/tools/setup/oracle.sh` and the four disks (DESIGN §5).

use std::sync::OnceLock;

use sf_disk::{DiskSet, DiskSlot, Offered, carve, decompress, identify};
use sf_oracle::*;

/// Where the cross-check puts things, as `games/superfrog/tools/atn.py` does.
const CODE: u32 = 0x1000;
const SRC: u32 = 0x40_0000;
const DST: u32 = 0x60_0000;
const STACK: u32 = 0x30_0000;
/// `atn_decompress` in the stage-1.5 loader, disk 1 bytes `0x400..0x1600` (games/superfrog/re/symbols.toml).
const STAGE15: std::ops::Range<usize> = 0x400..0x1600;
const ATN_DECOMPRESS: u32 = 0x2DA;

fn real_set() -> &'static DiskSet {
    static SET: OnceLock<DiskSet> = OnceLock::new();
    SET.get_or_init(|| {
        let files: Vec<Offered> = sf_conform::disks::read_disk_files()
            .unwrap_or_else(|e| panic!("{e}"))
            .into_iter()
            .map(|(name, bytes)| Offered { name, bytes })
            .collect();
        identify(&files).unwrap_or_else(|e| panic!("{e:?}"))
    })
}

fn empty() -> Cpu68k {
    Cpu68k::new(AmigaMemory {
        chip: vec![0; 512 * 1024],
        slow: vec![0; 512 * 1024],
    })
}

fn stage15_cpu() -> Cpu68k {
    let mut cpu = empty();
    cpu.write(CODE, &real_set().disk(DiskSlot::D1).bytes[STAGE15]);
    cpu
}

fn atn_regs() -> Regs {
    let mut r = Regs::default();
    r.a[0] = SRC;
    r.a[1] = DST;
    r.a[7] = STACK;
    r
}




#[test]
fn atn_routine_reproduces_sf_disk_on_every_chunk_it_decodes() {
    let mut cpu = stage15_cpu();
    let mut prev = 0;
    let mut checked = Vec::new();
    for slot in DiskSlot::ALL {
        for chunk in carve(real_set().disk(slot)) {
            let Ok(want) = decompress(&chunk.raw) else {
                continue;
            };
            let id = format!("d{}_c{:02}", slot.number(), chunk.index);
            cpu.write(SRC, &vec![0; prev]);
            cpu.write(SRC, &chunk.raw);
            prev = chunk.raw.len();
            let got = cpu
                .call(CODE + ATN_DECOMPRESS, atn_regs(), 200_000_000)
                .unwrap_or_else(|e| panic!("{id}: {e}"));
            assert_eq!(
                got.regs.d[0], 0xFFFF_FFFF,
                "{id}: the routine reports failure"
            );
            assert_eq!(got.regs.a[7], STACK, "{id}: stack not balanced");
            let out = got.bytes_after(&cpu, DST, want.len());
            assert!(
                out == want,
                "{id}: routine output differs from sf_disk::decompress"
            );
            checked.push(id);
        }
    }
    assert_eq!(checked.len(), 103, "{checked:?}");
}

#[test]
fn two_identical_calls_give_identical_results() {
    let mut cpu = stage15_cpu();
    let chunk = carve(real_set().disk(DiskSlot::D1)).remove(1);
    cpu.write(SRC, &chunk.raw);
    let before = cpu.memory().to_vec();
    let a = cpu
        .call(CODE + ATN_DECOMPRESS, atn_regs(), 200_000_000)
        .unwrap();
    assert!(cpu.memory() == &before[..], "the call left memory changed");
    let b = cpu
        .call(CODE + ATN_DECOMPRESS, atn_regs(), 200_000_000)
        .unwrap();
    assert_eq!(a, b);
    assert!(
        a.writes.len() as u32 >= chunk.decomp_size / 2,
        "{} writes",
        a.writes.len()
    );
    assert!(a.instr > 0);
}












#[test]
fn the_games_own_executables_load() {
    let set = real_set();
    for (slot, index, hunks) in [
        (DiskSlot::D1, 3, 4),
        (DiskSlot::D3, 2, 1),
        (DiskSlot::D3, 4, 4),
    ] {
        let chunk = carve(set.disk(slot))
            .into_iter()
            .find(|c| c.index == index)
            .unwrap();
        let exe = decompress(&chunk.raw).unwrap();
        let mut cpu = empty();
        let map = cpu
            .load_exe(&exe, 0x1000)
            .unwrap_or_else(|e| panic!("d{}_c{index:02}: {e}", slot.number()));
        assert_eq!(map.hunks.len(), hunks);
        assert_eq!(map.hunks[0].kind, HunkKind::Code);
    }
}
