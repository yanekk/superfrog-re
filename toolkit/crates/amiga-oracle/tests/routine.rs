//! The routine runner on its own: hand-assembled 68000 code and synthetic hunk executables, with no
//! ROM and no disk. Needs `toolkit/tools/setup/oracle.sh`'s build. A game's own routines are tested
//! in its own crate.

use amiga_oracle::*;

const STACK: u32 = 0x30_0000;

fn empty() -> Cpu68k {
    Cpu68k::new(AmigaMemory {
        chip: vec![0; 512 * 1024],
        slow: vec![0; 512 * 1024],
    })
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
fn the_stack_pointer_keeps_all_32_bits() {
    // rts: returns at once, a7 back where it was, high byte and all.
    let mut cpu = with_code(&[0x4E75]);
    for a7 in [0, 0x8030_0000] {
        let mut r = Regs::default();
        r.a[7] = a7;
        let got = cpu.call(0x2000, r, 10).unwrap();
        assert_eq!((got.regs.a[7], got.instr), (a7, 1), "a7 = {a7:08x}");
    }
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


// ---------------------------------------------------------------------------------------------
// `load_exe` reads its blocks through `amiga_disk::hunk` since the toolkit move; the loader it
// replaced, kept here word for word, is the reference it is compared with over a sweep of files.
// ---------------------------------------------------------------------------------------------

fn reference_load(cpu: &mut Cpu68k, exe: &[u8], at: u32) -> Result<HunkMap, HunkError> {
    struct Reader<'a> {
        exe: &'a [u8],
        pos: usize,
    }
    impl Reader<'_> {
        fn bytes(&mut self, n: usize) -> Result<&[u8], HunkError> {
            let s = self
                .exe
                .get(self.pos..self.pos.saturating_add(n))
                .ok_or(HunkError::Truncated { at: self.pos })?;
            self.pos += n;
            Ok(s)
        }
        fn long(&mut self) -> Result<u32, HunkError> {
            Ok(u32::from_be_bytes(self.bytes(4)?.try_into().unwrap()))
        }
        fn skip(&mut self, longs: u32) -> Result<(), HunkError> {
            self.bytes(longs as usize * 4).map(|_| ())
        }
    }
    fn hunk(hunks: &mut [Hunk], cur: usize, at: usize) -> Result<&mut Hunk, HunkError> {
        hunks.get_mut(cur).ok_or(HunkError::Malformed {
            at,
            why: "more hunks than the header declares",
        })
    }
    let mut r = Reader { exe, pos: 0 };
    if r.long()? != 0x3F3 {
        return Err(HunkError::NotAnExecutable);
    }
    loop {
        let n = r.long()?;
        if n == 0 {
            break;
        }
        r.skip(n)?;
    }
    let count = r.long()? as usize;
    if count == 0 || count > 0x10000 {
        return Err(HunkError::Malformed {
            at: r.pos - 4,
            why: "hunk count",
        });
    }
    r.skip(2)?;
    let mut hunks = Vec::with_capacity(count);
    let mut base = at as u64;
    for _ in 0..count {
        let size = r.long()?.wrapping_mul(4);
        hunks.push(Hunk {
            base: base as u32,
            size,
            kind: HunkKind::Absent,
        });
        base += size as u64;
    }
    if base > MEM_SIZE as u64 {
        return Err(HunkError::TooBig);
    }
    let mut cur = 0;
    while r.pos < exe.len() {
        let at = r.pos;
        match r.long()? {
            0x3E7 | 0x3E8 => {
                let n = r.long()?;
                r.skip(n)?;
            }
            kind @ (0x3E9 | 0x3EA) => {
                let h = hunk(&mut hunks, cur, at)?;
                let n = r.long()?;
                if n == 0 || n as u64 * 4 > h.size as u64 {
                    return Err(HunkError::Malformed {
                        at,
                        why: "code/data length",
                    });
                }
                let body = r.bytes(n as usize * 4)?;
                let (b, size) = (h.base, h.size);
                h.kind = if kind == 0x3E9 {
                    HunkKind::Code
                } else {
                    HunkKind::Data
                };
                cpu.write(b, body);
                cpu.memory_mut()[(b + n * 4) as usize..(b + size) as usize].fill(0);
            }
            0x3EB => {
                let h = hunk(&mut hunks, cur, at)?;
                let n = r.long()?;
                if n == 0 || h.base as u64 + n as u64 * 4 > MEM_SIZE as u64 {
                    return Err(HunkError::Malformed {
                        at,
                        why: "bss length",
                    });
                }
                h.kind = HunkKind::Bss;
                let b = h.base as usize;
                cpu.memory_mut()[b..b + n as usize * 4].fill(0);
            }
            0x3EC => {
                let h = *hunk(&mut hunks, cur, at)?;
                loop {
                    let n = r.long()?;
                    if n == 0 {
                        break;
                    }
                    let target = r.long()? as usize;
                    let delta = hunks
                        .get(target)
                        .ok_or(HunkError::Malformed {
                            at,
                            why: "relocation to a missing hunk",
                        })?
                        .base;
                    for _ in 0..n {
                        let off = r.long()?;
                        if off as u64 + 4 > h.size as u64 {
                            return Err(HunkError::Malformed {
                                at,
                                why: "relocation outside its hunk",
                            });
                        }
                        let a = (h.base + off) as usize;
                        let m = cpu.memory_mut();
                        let v = u32::from_be_bytes([m[a], m[a + 1], m[a + 2], m[a + 3]]);
                        m[a..a + 4].copy_from_slice(&v.wrapping_add(delta).to_be_bytes());
                    }
                }
            }
            0x3ED..=0x3F1 => break,
            0x3F2 => {
                cur += 1;
                if cur == count {
                    break;
                }
            }
            _ => {
                return Err(HunkError::Malformed {
                    at,
                    why: "unknown block type",
                });
            }
        }
    }
    Ok(HunkMap { hunks })
}

fn longs(l: &[u32]) -> Vec<u8> {
    l.iter().flat_map(|v| v.to_be_bytes()).collect()
}

/// The sweep's starting files: the two-hunk executables, one with a unit, a name and a resident
/// library name, one stopping at a debug block, one running off its end before its last END, one
/// with two relocation groups.
fn sweep_files() -> Vec<Vec<u8>> {
    let mut named = vec![0x3F3, 1, 0x6C69_6200, 0, 1, 0, 0, 2];
    named.extend([0x3E7, 1, 0x7531_0000, 0x3E8, 1, 0x6E31_0000]);
    named.extend([0x3E9, 2, 0x4E71_4E75, 0x0000_0000, 0x3F2]);
    let mut debug = vec![0x3F3, 0, 2, 0, 1, 2, 1];
    debug.extend([0x3E9, 1, 0x4E75_4E71, 0x3F1, 1, 0xFFFF_FFFF, 0x3F2]);
    let mut short = vec![0x3F3, 0, 2, 0, 1, 1, 3];
    short.extend([0x3E9, 1, 0x4E75_4E71, 0x3F2, 0x3EA, 1, 0x1234_5678]);
    let mut groups = vec![0x3F3, 0, 2, 0, 1, 4, 2];
    groups.extend([0x3E9, 4, 0, 4, 8, 0x4E75_0000, 0x3EC, 1, 0, 0, 2, 1, 4, 8, 0, 0x3F2]);
    groups.extend([0x3EA, 2, 1, 2, 0x3F2]);
    vec![
        two_hunk_exe(2),
        two_hunk_exe(4),
        longs(&named),
        longs(&debug),
        longs(&short),
        longs(&groups),
    ]
}

/// One load: the outcome, and the memory digest when it loaded (a refused file's partial writes
/// are not compared: the caller drops the CPU).
fn load_with(
    f: fn(&mut Cpu68k, &[u8], u32) -> Result<HunkMap, HunkError>,
    cpu: &mut Cpu68k,
    exe: &[u8],
) -> (Result<HunkMap, HunkError>, Option<Vec<u8>>) {
    cpu.memory_mut()[0x1F000..0x30000].fill(0xA5);
    let r = f(cpu, exe, 0x20000);
    let mem = r.is_ok().then(|| cpu.read(0x1F000, 0x11000).to_vec());
    (r, mem)
}

#[test]
fn load_exe_matches_the_loader_it_replaced_over_a_sweep() {
    let (mut a, mut b) = (empty(), empty());
    let mut cases = 0;
    let mut differ = Vec::new();
    let mut check = |exe: &[u8], what: &str| {
        cases += 1;
        let new = load_with(|c, e, at| c.load_exe(e, at), &mut a, exe);
        let old = load_with(reference_load, &mut b, exe);
        // One difference is expected: the reader measures a whole RELOC32 block before the
        // loader checks its groups, so a block both malformed and cut short is reported cut short.
        let reloc_cut = matches!(
            (&new.0, &old.0),
            (
                Err(HunkError::Truncated { .. }),
                Err(HunkError::Malformed {
                    why: "relocation to a missing hunk" | "relocation outside its hunk",
                    ..
                })
            )
        );
        if new != old && !reloc_cut {
            differ.push(format!("{what}: new {:?}, old {:?}", new.0, old.0));
        }
    };
    for file in sweep_files() {
        check(&file, "whole");
        for len in 0..file.len() {
            check(&file[..len], &format!("cut at {len}"));
        }
        for i in 0..file.len() {
            for v in [0x00, 0xFF, file[i] ^ 0x01, file[i] ^ 0x40, file[i] ^ 0x80] {
                let mut m = file.clone();
                m[i] = v;
                check(&m, &format!("byte {i} = {v:#04x}"));
                // A changed byte in a file also cut short: the error each reports first.
                check(&m[..m.len() - 3], &format!("byte {i} = {v:#04x}, cut 3"));
            }
        }
    }
    assert!(cases > 5_000, "{cases}");
    assert!(differ.is_empty(), "{} of {cases} differ:\n{}", differ.len(), differ.join("\n"));
}
