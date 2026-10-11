//! The routine oracle (T06): one routine of the original run alone on Moira. The ATN! decompressor
//! from the owner's disk 1 is the first real routine and a cross-check on `sf_disk::decompress`
//! (T04); the rest use hand-assembled 68000 code. Needs `tools/setup/oracle.sh` (the test block's
//! setup) and, for the cross-check, the four disks (DESIGN §5).

use std::sync::OnceLock;

use sf_disk::{DiskSet, DiskSlot, Offered, carve, decompress, identify};
use sf_oracle::*;

/// Where the cross-check puts things, as `tools/atn.py` does.
const CODE: u32 = 0x1000;
const SRC: u32 = 0x40_0000;
const DST: u32 = 0x60_0000;
const STACK: u32 = 0x30_0000;
/// `atn_decompress` in the stage-1.5 loader, disk 1 bytes `0x400..0x1600` (re/symbols.toml).
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

fn words(code: &[u16]) -> Vec<u8> {
    code.iter().flat_map(|w| w.to_be_bytes()).collect()
}

/// `code` at 0x2000 in empty memory.
fn with_code(code: &[u16]) -> Cpu68k {
    let mut cpu = empty();
    cpu.write(0x2000, &words(code));
    cpu
}

fn stack_regs() -> Regs {
    let mut r = Regs::default();
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
fn registers_go_in_and_come_out() {
    // add.l d1,d0 ; move.l a0,(a1) ; moveq #0,d2 ; rts   (moveq #0 sets Z)
    let mut cpu = with_code(&[0xD081, 0x2288, 0x7400, 0x4E75]);
    let mut r = stack_regs();
    r.d[0] = 40;
    r.d[1] = 2;
    r.d[2] = 7;
    r.a[0] = 0x1234_5678;
    r.a[1] = 0x8000;
    let got = cpu.call(0x2000, r, 100).unwrap();
    assert_eq!(got.regs.d[0], 42);
    assert_eq!(got.regs.d[2], 0);
    assert_eq!(got.regs.a[7], STACK);
    assert_eq!(got.regs.sr, 0x2704);
    assert_eq!(got.instr, 4);
    assert_eq!(
        got.writes,
        vec![
            (0x8000, 0, 0x12),
            (0x8001, 0, 0x34),
            (0x8002, 0, 0x56),
            (0x8003, 0, 0x78)
        ]
    );
    assert_eq!(
        got.bytes_after(&cpu, 0x7FFF, 6),
        [0, 0x12, 0x34, 0x56, 0x78, 0]
    );
    assert_eq!(cpu.read(0x8000, 4), [0; 4]);
}

#[test]
fn a_byte_rewritten_with_its_own_value_is_not_a_write() {
    // move.b #$55,$8000.l ; move.b #$aa,$8001.l ; rts — with $8000 already $55
    let mut cpu = with_code(&[
        0x13FC, 0x0055, 0x0000, 0x8000, 0x13FC, 0x00AA, 0x0000, 0x8001, 0x4E75,
    ]);
    cpu.write(0x8000, &[0x55]);
    let got = cpu.call(0x2000, stack_regs(), 100).unwrap();
    assert_eq!(got.writes, vec![(0x8001, 0, 0xAA)]);
}

#[test]
fn max_instr_stops_an_infinite_loop() {
    // loop: bra.s loop
    let mut cpu = with_code(&[0x60FE]);
    assert_eq!(
        cpu.call(0x2000, stack_regs(), 10_000),
        Err(CallError::Timeout)
    );
    // A routine that returns in exactly max_instr instructions is not a timeout.
    let mut cpu = with_code(&[0x4E71, 0x4E75]);
    assert_eq!(cpu.call(0x2000, stack_regs(), 2).unwrap().instr, 2);
    assert_eq!(cpu.call(0x2000, stack_regs(), 1), Err(CallError::Timeout));
}

#[test]
fn a_timed_out_call_leaves_memory_as_it_was() {
    // loop: addq.l #1,$8000.l ; bra.s loop
    let mut cpu = with_code(&[0x52B9, 0x0000, 0x8000, 0x60F8]);
    let before = cpu.memory().to_vec();
    assert_eq!(
        cpu.call(0x2000, stack_regs(), 1_000),
        Err(CallError::Timeout)
    );
    assert!(cpu.memory() == &before[..]);
}

#[test]
fn custom_chip_and_cia_accesses_are_errors_with_the_pc() {
    let cases: [(&[u16], u32); 5] = [
        // nop ; move.w #$7fff,$dff09a.l      (INTENA)
        (&[0x4E71, 0x33FC, 0x7FFF, 0x00DF, 0xF09A, 0x4E75], 0xDFF09A),
        // nop ; move.w d0,$dff1fe.l          (the last custom register)
        (&[0x4E71, 0x33C0, 0x00DF, 0xF1FE, 0x4E75], 0xDFF1FE),
        // nop ; move.b d0,$bfe001.l          (CIA-A PRA)
        (&[0x4E71, 0x13C0, 0x00BF, 0xE001, 0x4E75], 0xBFE001),
        // nop ; tst.b $bfd100.l              (a read of CIA-B PRB)
        (&[0x4E71, 0x4A39, 0x00BF, 0xD100, 0x4E75], 0xBFD100),
        // nop ; move.w $dff006.l,d0          (a read of VHPOSR)
        (&[0x4E71, 0x3039, 0x00DF, 0xF006, 0x4E75], 0xDFF006),
    ];
    for (code, addr) in cases {
        let mut cpu = with_code(code);
        let before = cpu.memory().to_vec();
        assert_eq!(
            cpu.call(0x2000, stack_regs(), 100),
            Err(CallError::CustomChipAccess { pc: 0x2002, addr }),
            "{addr:06x}"
        );
        assert!(cpu.memory() == &before[..], "{addr:06x}: memory changed");
    }
    // Just past the custom range is ordinary memory here.
    let mut cpu = with_code(&[0x33C0, 0x00DF, 0xF200, 0x4E75]);
    let got = cpu.call(0x2000, stack_regs(), 100).unwrap();
    assert_eq!(got.instr, 2);
}

#[test]
fn illegal_instructions_and_other_exceptions_are_errors() {
    // nop ; illegal
    let mut cpu = with_code(&[0x4E71, 0x4AFC]);
    assert_eq!(
        cpu.call(0x2000, stack_regs(), 100),
        Err(CallError::IllegalInstruction { pc: 0x2002 })
    );
    // A-line
    let mut cpu = with_code(&[0xA000]);
    assert_eq!(
        cpu.call(0x2000, stack_regs(), 100),
        Err(CallError::IllegalInstruction { pc: 0x2000 })
    );
    // moveq #0,d1 ; divu d1,d0
    let mut cpu = with_code(&[0x7200, 0x80C1, 0x4E75]);
    assert_eq!(
        cpu.call(0x2000, stack_regs(), 100),
        Err(CallError::Exception {
            pc: 0x2002,
            vector: 5
        })
    );
}

/// A two-hunk executable: hunk 0 is code reading a long from hunk 1 by absolute address.
fn two_hunk_exe(declared: u32) -> Vec<u8> {
    let mut l: Vec<u32> = vec![0x3F3, 0, declared, 0, declared - 1];
    // Sizes: code 3 longs, data 2 longs (+ BSS of 4 longs and absent hunks when declared > 2).
    l.extend([3, 2, 4, 5].iter().take(declared as usize));
    // move.l (hunk1+4).l,d0 ; rts ; nop
    l.extend([0x3E9, 3, 0x2039_0000, 0x0004_4E75, 0x4E71_4E71]);
    l.extend([0x3EC, 1, 1, 2, 0]); // one reloc to hunk 1, at offset 2
    l.push(0x3F2);
    l.extend([0x3EA, 1, 0xDEAD_BEEF]); // data shorter than its hunk: the rest is cleared
    l.push(0x3F2);
    if declared > 2 {
        l.extend([0x3EB, 4, 0x3F2]);
    }
    l.iter().flat_map(|v| v.to_be_bytes()).collect()
}

#[test]
fn relocation_points_an_absolute_reference_at_the_relocated_target() {
    let mut cpu = empty();
    cpu.write(0x20008, &[0xFF; 32]);
    let map = cpu.load_exe(&two_hunk_exe(2), 0x20000).unwrap();
    assert_eq!(
        map.hunks,
        vec![
            Hunk {
                base: 0x20000,
                size: 12,
                kind: HunkKind::Code
            },
            Hunk {
                base: 0x2000C,
                size: 8,
                kind: HunkKind::Data
            },
        ]
    );
    // The operand at hunk 0 + 2 was hunk-relative 4; it now names hunk 1's second long.
    assert_eq!(cpu.read(0x20002, 4), 0x2_0010u32.to_be_bytes());
    assert_eq!(cpu.read(0x2000C, 8), [0xDE, 0xAD, 0xBE, 0xEF, 0, 0, 0, 0]);
    cpu.write(0x20010, &0x1234_5678u32.to_be_bytes());
    let got = cpu.call(map.addr(0, 0), stack_regs(), 100).unwrap();
    assert_eq!(got.regs.d[0], 0x1234_5678);
    // Loaded elsewhere, the same reference follows.
    let mut cpu = empty();
    let map = cpu.load_exe(&two_hunk_exe(2), 0x7_0000).unwrap();
    assert_eq!(cpu.read(0x7_0002, 4), (map.addr(1, 4)).to_be_bytes());
}

#[test]
fn absent_hunks_get_room_but_no_bytes() {
    let mut cpu = empty();
    cpu.write(0x20020, &[0xFF; 64]);
    let map = cpu.load_exe(&two_hunk_exe(4), 0x20000).unwrap();
    let kinds: Vec<_> = map.hunks.iter().map(|h| (h.base, h.size, h.kind)).collect();
    assert_eq!(
        kinds,
        vec![
            (0x20000, 12, HunkKind::Code),
            (0x2000C, 8, HunkKind::Data),
            (0x20014, 16, HunkKind::Bss),
            (0x20024, 20, HunkKind::Absent),
        ]
    );
    assert_eq!(map.end(), 0x20038);
    assert_eq!(cpu.read(0x20014, 16), [0; 16]);
    assert_eq!(
        cpu.read(0x20024, 4),
        [0xFF; 4],
        "an absent hunk is not cleared"
    );
}

#[test]
fn not_an_executable_and_truncated_files_are_refused() {
    let mut cpu = empty();
    assert_eq!(
        cpu.load_exe(b"ATN!xxxx", 0),
        Err(HunkError::NotAnExecutable)
    );
    let exe = two_hunk_exe(2);
    assert!(matches!(
        cpu.load_exe(&exe[..30], 0x20000),
        Err(HunkError::Truncated { .. })
    ));
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
