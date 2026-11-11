//! `Cpu68k::step`: one instruction alone on Moira, with its condition codes, the bytes it wrote and
//! its cycles. Hand-assembled 68000 code in empty memory; no ROM, no disk.

use amiga_oracle::*;

const AT: u32 = 0x2000;
const STACK: u32 = 0x30_0000;

// The condition-code bits of SR.
const C: u16 = 1;
const V: u16 = 2;
const Z: u16 = 4;
const N: u16 = 8;
const X: u16 = 16;

fn with_code(code: &[u16]) -> Cpu68k {
    let mut cpu = Cpu68k::new(AmigaMemory {
        chip: vec![0; 512 * 1024],
        slow: vec![0; 512 * 1024],
    });
    let bytes: Vec<u8> = code.iter().flat_map(|w| w.to_be_bytes()).collect();
    cpu.write(AT, &bytes);
    cpu
}

fn regs() -> Regs {
    let mut r = Regs::default();
    r.a[7] = STACK;
    r
}

#[test]
fn moveq_minus_one_sets_n_and_clears_z_in_four_cycles() {
    let mut cpu = with_code(&[0x70FF]); // moveq #-1,d0
    let mut r = regs();
    r.sr |= Z | V | C | X;
    let s = cpu.step(AT, r).unwrap();
    assert_eq!(s.regs.d[0], 0xFFFF_FFFF);
    assert_eq!(s.regs.sr & 0x1F, N | X, "N set; Z, V, C cleared; X untouched");
    assert_eq!(s.pc, AT + 2);
    assert_eq!(s.cycles, 4);
    assert!(s.writes.is_empty());
}

#[test]
fn add_long_with_overflow_sets_v_and_c_and_x_follows_c() {
    let mut cpu = with_code(&[0xD081]); // add.l d1,d0
    let mut r = regs();
    r.d[0] = 0x8000_0000;
    r.d[1] = 0x8000_0000;
    let s = cpu.step(AT, r).unwrap();
    assert_eq!(s.regs.d[0], 0);
    assert_eq!(s.regs.sr & 0x1F, X | Z | V | C);
    assert_eq!(s.cycles, 8);

    // No carry out: C clears, and X, set before, clears with it.
    r.d[0] = 0x7FFF_FFFF;
    r.d[1] = 1;
    r.sr |= X | C;
    let s = cpu.step(AT, r).unwrap();
    assert_eq!(s.regs.d[0], 0x8000_0000);
    assert_eq!(s.regs.sr & 0x1F, N | V);
}

#[test]
fn move_word_post_increment_writes_two_bytes_and_steps_a0() {
    let mut cpu = with_code(&[0x30C0]); // move.w d0,(a0)+
    let mut r = regs();
    r.d[0] = 0x1234_ABCD;
    r.a[0] = 0x4000;
    let s = cpu.step(AT, r).unwrap();
    assert_eq!(s.writes, vec![(0x4000, 0, 0xAB), (0x4001, 0, 0xCD)]);
    assert_eq!(s.regs.a[0], 0x4002);
    assert_eq!(s.regs.sr & 0x1F, N);
    assert_eq!(s.cycles, 8);
    assert_eq!(cpu.read(0x4000, 2), &[0, 0], "memory is put back after a step");
}

#[test]
fn bne_taken_and_not_taken_differ_in_pc_and_cycles() {
    let mut cpu = with_code(&[0x6610]); // bne.s *+$12
    let mut r = regs();
    let taken = cpu.step(AT, r).unwrap();
    assert_eq!(taken.pc, AT + 2 + 0x10);
    assert_eq!(taken.cycles, 10);
    r.sr |= Z;
    let not_taken = cpu.step(AT, r).unwrap();
    assert_eq!(not_taken.pc, AT + 2);
    assert_eq!(not_taken.cycles, 8);
    assert_eq!(not_taken.regs.sr, r.sr, "a branch leaves the flags alone");
}

#[test]
fn an_illegal_opcode_is_named_at_its_address() {
    for op in [0x4AFC, 0xA000, 0xF000] {
        let mut cpu = with_code(&[op]);
        assert_eq!(
            cpu.step(AT, regs()),
            Err(CallError::IllegalInstruction { pc: AT }),
            "opcode {op:04x}"
        );
        assert!(
            cpu.read(STACK - 8, 8).iter().all(|&b| b == 0),
            "the exception frame is put back"
        );
    }
}

#[test]
fn other_exceptions_are_reported_not_followed() {
    let mut cpu = with_code(&[0x4E41]); // trap #1
    assert_eq!(
        cpu.step(AT, regs()),
        Err(CallError::Exception { pc: AT, vector: 33 })
    );
    let mut cpu = with_code(&[0x80C1]); // divu.w d1,d0 with d1 = 0
    assert_eq!(
        cpu.step(AT, regs()),
        Err(CallError::Exception { pc: AT, vector: 5 })
    );
}

#[test]
fn move_usp_reads_and_writes_the_user_stack_pointer() {
    let mut cpu = with_code(&[0x4E68, 0x4E61]); // move usp,a0 ; move a1,usp
    let s = cpu.step_with_other_sp(AT, regs(), 0x1234).unwrap();
    assert_eq!(s.regs.a[0], 0x1234);
    assert_eq!((s.usp, s.ssp), (0x1234, STACK));
    let mut r = regs();
    r.a[1] = 0x5678;
    let s = cpu.step_with_other_sp(AT + 2, r, 0x1234).unwrap();
    assert_eq!((s.usp, s.ssp, s.regs.a[7]), (0x5678, STACK, STACK));
    // Plain `step` starts both stack pointers at a7.
    let s = cpu.step(AT, regs()).unwrap();
    assert_eq!(s.regs.a[0], STACK);
}

#[test]
fn rte_to_user_mode_switches_the_stack_pointer() {
    let mut cpu = with_code(&[0x4E73]); // rte
    // The frame: SR (user, Z), then the return PC.
    cpu.write(STACK, &[0x00, 0x04, 0x00, 0x00, 0x30, 0x00]);
    let s = cpu.step_with_other_sp(AT, regs(), 0x8000).unwrap();
    assert_eq!(s.pc, 0x3000);
    assert_eq!(s.regs.sr, Z);
    assert_eq!(s.regs.a[7], 0x8000, "a7 is now the user stack pointer");
    assert_eq!((s.usp, s.ssp), (0x8000, STACK + 6));
    assert_eq!(s.cycles, 20);
}

#[test]
fn hardware_mode_holds_for_a_step() {
    let mut cpu = with_code(&[0x33C0, 0x00DF, 0xF180]); // move.w d0,$dff180
    let mut r = regs();
    r.d[0] = 0x0F00;
    assert_eq!(
        cpu.step(AT, r),
        Err(CallError::CustomChipAccess { pc: AT, addr: 0xDF_F180 })
    );
    cpu.set_hardware(true);
    let s = cpu.step(AT, r).unwrap();
    assert_eq!(s.hw_writes, vec![(0x180, 0x0F00)]);
    assert_eq!(s.pc, AT + 6);
}

#[test]
fn a_step_between_traced_calls_leaves_the_trace_alone() {
    // move.b (a0),d0 ; rts
    let mut cpu = with_code(&[0x1010, 0x4E75]);
    cpu.set_trace(true);
    let mut r = regs();
    r.a[0] = 0x5000;
    cpu.call(AT, r, 10).unwrap();
    assert_eq!(cpu.last_trace().reads.first(), Some(&0x5000));
    r.a[0] = 0x6000;
    cpu.step(AT, r).unwrap();
    r.a[0] = 0x7000;
    cpu.call(AT, r, 10).unwrap();
    let reads = cpu.last_trace().reads;
    assert!(reads.contains(&0x7000));
    assert!(!reads.contains(&0x5000) && !reads.contains(&0x6000), "{reads:x?}");
}
