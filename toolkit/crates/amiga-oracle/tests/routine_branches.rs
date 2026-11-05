//! The routine runner's trace and static walk (`Cpu68k::set_trace`, `Cpu68k::conditional_branches`)
//! on hand-assembled 68000 code, with no ROM and no disk. Needs `toolkit/tools/setup/oracle.sh`'s
//! build.

use std::collections::BTreeMap;

use amiga_oracle::*;

const STACK: u32 = 0x30_0000;

fn words(code: &[u16]) -> Vec<u8> {
    code.iter().flat_map(|w| w.to_be_bytes()).collect()
}

fn cpu_with(parts: &[(u32, &[u16])]) -> Cpu68k {
    let mut cpu = Cpu68k::new(AmigaMemory {
        chip: vec![0; 512 * 1024],
        slow: vec![],
    });
    for (at, code) in parts {
        cpu.write(*at, &words(code));
    }
    cpu
}

fn regs(d: [u32; 8], a0: u32) -> Regs {
    let mut r = Regs {
        d,
        ..Regs::default()
    };
    r.a[0] = a0;
    r.a[7] = STACK;
    r
}

/// Two branches in sequence and a third inside the first one's taken arm:
/// ```text
/// 2000 tst.l d0       2002 beq.s $2008     2004 moveq #1,d2   2006 bra.s $200e
/// 2008 tst.l d1       200a bne.s $200e     200c moveq #2,d2
/// 200e tst.l d3       2010 bmi.s $2014     2012 moveq #3,d4   2014 rts
/// ```
const NESTED: [u16; 11] = [
    0x4a80, 0x6704, 0x7401, 0x6006, 0x4a81, 0x6602, 0x7402, 0x4a83, 0x6b02, 0x7803, 0x4e75,
];

/// Per branch, over how many states it went [not taken, taken]: what a rig's coverage counts.
fn tally(cpu: &mut Cpu68k, entry: u32, states: &[[u32; 8]]) -> BTreeMap<u32, [u32; 2]> {
    let mut t: BTreeMap<u32, [u32; 2]> = cpu
        .conditional_branches(entry)
        .into_iter()
        .map(|a| (a, [0, 0]))
        .collect();
    cpu.set_trace(true);
    for d in states {
        cpu.call(entry, regs(*d, 0), 1000).unwrap();
        for b in cpu.last_trace().branches {
            let e = t.entry(b.at).or_default();
            e[0] += b.not_taken as u32;
            e[1] += b.taken as u32;
        }
    }
    cpu.set_trace(false);
    t
}

#[test]
fn the_walk_lists_every_conditional_branch_but_not_bra() {
    let cpu = cpu_with(&[(0x2000, &NESTED)]);
    assert_eq!(cpu.conditional_branches(0x2000), vec![0x2002, 0x200a, 0x2010]);
}

#[test]
fn states_setting_one_condition_leave_exactly_the_others_uncovered() {
    let mut cpu = cpu_with(&[(0x2000, &NESTED)]);
    let mut g = Gen::new(7);
    // d0 never zero: beq is never taken, so the branch in its arm never runs. d3 either sign.
    let states: Vec<[u32; 8]> = (0..50)
        .map(|_| {
            let mut d = [0; 8];
            d[0] = g.next() as u32 | 1;
            d[1] = g.next() as u32 | 1;
            d[3] = g.next() as u32;
            d
        })
        .collect();
    let t = tally(&mut cpu, 0x2000, &states);
    assert_eq!(t[&0x2002], [50, 0]);
    assert_eq!(t[&0x200a], [0, 0], "never executed: listed by the walk, both counts 0");
    let [not, taken] = t[&0x2010];
    assert!(not > 0 && taken > 0 && not + taken == 50, "{not} {taken}");
    let uncovered: Vec<u32> = t
        .iter()
        .filter(|(_, w)| w[0] == 0 || w[1] == 0)
        .map(|(a, _)| *a)
        .collect();
    assert_eq!(uncovered, vec![0x2002, 0x200a]);

    // Half the states with d0 zero reach the inner branch; d1 never zero takes it every time.
    let half: Vec<[u32; 8]> = states
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let mut d = *d;
            if i % 2 == 0 {
                d[0] = 0;
            }
            d
        })
        .collect();
    let t = tally(&mut cpu, 0x2000, &half);
    assert_eq!(t[&0x2002], [25, 25]);
    assert_eq!(t[&0x200a], [0, 25]);
}

#[test]
fn a_dbra_loop_goes_both_ways_in_one_call_and_its_exit_is_not_taken() {
    // moveq #3,d0; loop: dbra d0,loop; rts
    let mut cpu = cpu_with(&[(0x2000, &[0x7003, 0x51c8, 0xfffe, 0x4e75])]);
    assert_eq!(cpu.conditional_branches(0x2000), vec![0x2002]);
    cpu.set_trace(true);
    cpu.call(0x2000, regs([0; 8], 0), 100).unwrap();
    assert_eq!(
        cpu.last_trace().branches,
        vec![BranchWays {
            at: 0x2002,
            not_taken: true,
            taken: true
        }]
    );
    // moveq #0,d0; dbra d0,*; rts: d0 goes from 0 to -1 at once, so the dbra only exits.
    let mut cpu = cpu_with(&[(0x2000, &[0x7000, 0x51c8, 0xfffe, 0x4e75])]);
    cpu.set_trace(true);
    cpu.call(0x2000, regs([0; 8], 0), 100).unwrap();
    assert_eq!(
        cpu.last_trace().branches,
        vec![BranchWays {
            at: 0x2002,
            not_taken: true,
            taken: false
        }]
    );
}

#[test]
fn the_walk_follows_direct_calls_and_not_indirect_jumps() {
    // 3000 jsr $3100.l; 3006 jmp (a0)
    // 3100 tst.w d0; 3102 bne.s $3106; 3104 nop; 3106 rts
    // 3200 tst.w d1; 3202 beq.w $3206; 3206 rts   (reached only through a0)
    let mut cpu = cpu_with(&[
        (0x3000, &[0x4eb9, 0x0000, 0x3100, 0x4ed0]),
        (0x3100, &[0x4a40, 0x6602, 0x4e71, 0x4e75]),
        (0x3200, &[0x4a41, 0x6700, 0x0002, 0x4e75]),
    ]);
    assert_eq!(cpu.conditional_branches(0x3000), vec![0x3102]);
    // Executed, the indirect jump's branch shows up.
    cpu.set_trace(true);
    cpu.call(0x3000, regs([0; 8], 0x3200), 100).unwrap();
    let at: Vec<u32> = cpu.last_trace().branches.iter().map(|b| b.at).collect();
    assert_eq!(at, vec![0x3102, 0x3202]);
    // jsr (d16,pc) and jmp abs.w are followed too: 3300 jsr $3100(pc); 3304 jmp $3200.w
    let mut cpu = cpu_with(&[
        (0x3300, &[0x4eba, 0xfdfe, 0x4ef8, 0x3200]),
        (0x3100, &[0x4a40, 0x6602, 0x4e71, 0x4e75]),
        (0x3200, &[0x4a41, 0x6700, 0x0002, 0x4e75]),
    ]);
    assert_eq!(cpu.conditional_branches(0x3300), vec![0x3102, 0x3202]);
}

#[test]
fn reads_are_data_space_only() {
    // 4000 move.w $5000.l,d0; 4006 move.w ($400c,pc),d1; 400a rts; 400c dc.w $1234
    let mut cpu = cpu_with(&[(0x4000, &[0x3039, 0x0000, 0x5000, 0x323a, 0x0004, 0x4e75, 0x1234])]);
    cpu.set_trace(true);
    let r = cpu.call(0x4000, regs([0; 8], 0), 100).unwrap();
    assert_eq!(r.regs.d[1] & 0xffff, 0x1234);
    // The operand, and the return address rts pops; not the code, not the PC-relative word.
    assert_eq!(
        cpu.last_trace().reads,
        vec![0x5000, 0x5001, STACK - 4, STACK - 3, STACK - 2, STACK - 1]
    );
}

#[test]
fn tracing_changes_nothing_a_call_returns_and_is_off_by_default() {
    let code: &[u16] = &[
        0x4a80, 0x6704, 0x7401, 0x6006, 0x4a81, 0x6602, 0x7402, 0x4a83, 0x6b02, 0x7803,
        // move.l d4,$6000.l; rts
        0x23c4, 0x0000, 0x6000, 0x4e75,
    ];
    let mut cpu = cpu_with(&[(0x2000, code)]);
    let mut g = Gen::new(3);
    for _ in 0..20 {
        let mut d = [0u32; 8];
        for x in &mut d {
            *x = if g.one_in(3) { 0 } else { g.next() as u32 };
        }
        let plain = cpu.call(0x2000, regs(d, 0), 1000).unwrap();
        assert_eq!(cpu.last_trace(), CallTrace::default(), "off by default");
        cpu.set_trace(true);
        let traced = cpu.call(0x2000, regs(d, 0), 1000).unwrap();
        assert!(!cpu.last_trace().branches.is_empty());
        cpu.set_trace(false);
        assert_eq!(plain, traced);
        let again = cpu.call(0x2000, regs(d, 0), 1000).unwrap();
        assert_eq!(cpu.last_trace(), CallTrace::default(), "off again");
        assert_eq!(plain, again);
    }
}
