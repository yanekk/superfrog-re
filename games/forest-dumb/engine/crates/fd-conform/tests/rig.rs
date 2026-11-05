//! The routine rig (plans/forest-dumb-level-1 T32). The comparison rules run on routines assembled
//! by hand into empty memory, with no disk; the spike's generators and the hand port of `h0+$774`
//! run on the main program from the owner's disks.

use std::collections::BTreeMap;

use amiga_oracle::Regs;
use fd_conform::generators::{self, Generator, Setup, h};
use fd_conform::routine::{Effects, Rig};

const CODE: u32 = 0x2000;

fn words(code: &[u16]) -> Vec<u8> {
    code.iter().flat_map(|w| w.to_be_bytes()).collect()
}

/// A rig over empty memory with `code` at [`CODE`].
fn rig(code: &[u16]) -> Rig {
    let mut mem = vec![0u8; 0x10_0000];
    let bytes = words(code);
    mem[CODE as usize..CODE as usize + bytes.len()].copy_from_slice(&bytes);
    let mut r = Rig::from_memory(mem);
    r.set_max_instr(1_000);
    r
}

/// The port of a routine that changes nothing: the setup's registers back.
fn unchanged(s: &Setup) -> Effects {
    Effects {
        regs: Regs {
            d: s.d,
            a: s.a,
            ..Regs::default()
        },
        ..Effects::default()
    }
}

const RANDOM: Generator = Generator {
    base: None,
    case: Setup::random,
};

#[test]
fn a_wrong_return_register_fails() {
    // moveq #5,d0; rts
    let mut r = rig(&[0x7005, 0x4e75]);
    let right = |s: &Setup| {
        let mut e = unchanged(s);
        e.regs.d[0] = 5;
        e
    };
    let rep = r
        .check("moveq", 50, CODE, &RANDOM, right)
        .expect("the right port passes");
    assert_eq!(
        (rep.passed, rep.paths, rep.hangs, rep.faults),
        (50, 1, 0, 0)
    );
    let wrong = |s: &Setup| {
        let mut e = unchanged(s);
        e.regs.d[0] = 6;
        e
    };
    let f = r.check("moveq", 50, CODE, &RANDOM, wrong).unwrap_err();
    assert_eq!(f.failures, 50);
    assert!(
        f.first.contains("d0: original $00000005, port $00000006"),
        "{f}"
    );
    // A register the port forgot to give back is a difference too.
    let f = r
        .check("moveq", 5, CODE, &RANDOM, |_| Effects::default())
        .unwrap_err();
    assert!(f.first.contains("a6: original"), "{f}");
}

/// `move.w #$0000,$dff040.l; move.w #$0041,$dff058.l; rts`: BLTCON0, then BLTSIZE (a blit with no
/// channel, which writes nothing).
const BLIT: [u16; 9] = [
    0x33fc, 0x0000, 0x00df, 0xf040, 0x33fc, 0x0041, 0x00df, 0xf058, 0x4e75,
];

#[test]
fn custom_chip_writes_are_compared_in_order() {
    let mut r = rig(&BLIT);
    let with = |hw: Vec<(u16, u16)>| {
        move |s: &Setup| Effects {
            hw_writes: hw.clone(),
            ..unchanged(s)
        }
    };
    r.check(
        "blit",
        20,
        CODE,
        &RANDOM,
        with(vec![(0x40, 0), (0x58, 0x41)]),
    )
    .expect("in order");
    let f = r
        .check(
            "blit",
            20,
            CODE,
            &RANDOM,
            with(vec![(0x58, 0x41), (0x40, 0)]),
        )
        .unwrap_err();
    assert!(
        f.first.contains("custom-chip writes: original made 2, port 2; the first 0 agree, then original [$dff040=$0000 $dff058=$0041] port [$dff058=$0041 $dff040=$0000]"),
        "{f}"
    );
    let f = r
        .check("blit", 20, CODE, &RANDOM, with(vec![(0x40, 0)]))
        .unwrap_err();
    assert!(
        f.first
            .contains("original made 2, port 1; the first 1 agree"),
        "{f}"
    );
    // Memory and registers alone would have passed: the hardware writes are what failed.
    let f = r.check("blit", 20, CODE, &RANDOM, unchanged).unwrap_err();
    assert!(f.first.contains("original made 2, port 0"), "{f}");
}

/// `move.w d1,$5000.l; move.l d2,-(a7); move.l (a7)+,d2; rts`: one word to memory, one long on the
/// stack.
const STORE: [u16; 6] = [0x33c1, 0x0000, 0x5000, 0x2f02, 0x241f, 0x4e75];

#[test]
fn memory_is_compared_byte_by_byte_off_the_stack() {
    let mut r = rig(&STORE);
    let store = |s: &Setup| {
        let [hi, lo] = (s.d[1] as u16).to_be_bytes();
        let mut e = unchanged(s);
        // Out of order, written twice, and the stack: none of it matters.
        e.writes = vec![(0x5001, 0), (0x5000, hi), (0x5001, lo), (0x7E_FFF0, 1)];
        e
    };
    let rep = r
        .check("store", 100, CODE, &RANDOM, store)
        .expect("the right port passes");
    assert_eq!(rep.passed, 100);
    let missed = |s: &Setup| {
        let mut e = unchanged(s);
        e.writes = vec![(0x5000, (s.d[1] >> 8) as u8)];
        e
    };
    let f = r.check("store", 100, CODE, &RANDOM, missed).unwrap_err();
    assert!(f.first.contains("byte $005001: original wrote $"), "{f}");
    assert!(f.first.contains("port left it $00"), "{f}");
    let extra = |s: &Setup| {
        let mut e = store(s);
        e.writes.push((0x6000, 0x55));
        // Writing a byte with the value it holds is no change.
        e.writes.push((0x6002, 0));
        e
    };
    let f = r.check("store", 10, CODE, &RANDOM, extra).unwrap_err();
    assert!(
        f.first
            .contains("byte $006000: original left it $00, port wrote $55"),
        "{f}"
    );
    assert!(!f.first.contains("$006002"), "{f}");
}

/// A setup's pokes are what both sides start from.
#[test]
fn pokes_and_hardware_reads_reach_the_original() {
    // move.w $5000.l,d0; move.b $dff00c.l,d1; rts
    let mut r = rig(&[0x3039, 0x0000, 0x5000, 0x1239, 0x00df, 0xf00c, 0x4e75]);
    fn case(g: &mut generators::Gen) -> Setup {
        let mut s = Setup::random(g);
        s.w(0x5000, g.next() as u16);
        s.hw_reads.push((0xdf_f00c, g.next() as u8));
        s
    }
    let generator = Generator { base: None, case };
    let mem = r.memory_for(&generator);
    let port = |s: &Setup| {
        let mut e = unchanged(s);
        let w = (s.byte(&mem, 0x5000) as u32) << 8 | s.byte(&mem, 0x5001) as u32;
        e.regs.d[0] = s.d[0] & 0xffff_0000 | w;
        e.regs.d[1] = s.d[1] & 0xffff_ff00 | s.hw_reads[0].1 as u32;
        e
    };
    assert_eq!(
        r.check("poke", 100, CODE, &generator, port).unwrap().passed,
        100
    );
}

/// `tst.l d0; beq.s *; rts`: hangs when d0 is zero.
const HANG: [u16; 3] = [0x4a80, 0x67fe, 0x4e75];

#[test]
fn a_hang_is_counted_not_passed() {
    let mut r = rig(&HANG);
    fn sometimes(g: &mut generators::Gen) -> Setup {
        let mut s = Setup::random(g);
        if g.one_in(4) {
            s.d[0] = 0;
        }
        s
    }
    fn always(g: &mut generators::Gen) -> Setup {
        let mut s = Setup::random(g);
        s.d[0] = 0;
        s
    }
    let rep = r
        .check(
            "hang",
            200,
            CODE,
            &Generator {
                base: None,
                case: sometimes,
            },
            unchanged,
        )
        .unwrap();
    assert!(rep.hangs > 20 && rep.hangs < 80, "{rep:?}");
    assert_eq!(rep.passed + rep.hangs, 200);
    // The same states, run alone, report the same count.
    let alone = r.survey(
        "hang",
        200,
        CODE,
        &Generator {
            base: None,
            case: sometimes,
        },
    );
    assert_eq!(alone, rep);
    let f = r
        .check(
            "hang",
            50,
            CODE,
            &Generator {
                base: None,
                case: always,
            },
            unchanged,
        )
        .unwrap_err();
    assert_eq!((f.report.passed, f.report.hangs), (0, 50));
    assert!(
        f.first.contains(
            "compared nothing: the original returned on none of 50 states (50 hang(s), 0 fault(s))"
        ),
        "{f}"
    );
    // The port is never run on a state the original did not return from.
    let mut ran = 0;
    let _ = r.check(
        "hang",
        50,
        CODE,
        &Generator {
            base: None,
            case: always,
        },
        |s| {
            ran += 1;
            unchanged(s)
        },
    );
    assert_eq!(ran, 0);
}

#[test]
fn a_fault_is_counted_and_a_panicking_port_fails() {
    // move.w (a0),d0; rts: an address error on an odd a0.
    let mut r = rig(&[0x3010, 0x4e75]);
    fn odd(g: &mut generators::Gen) -> Setup {
        let mut s = Setup::random(g);
        s.a[0] = if g.one_in(2) { 0x5001 } else { 0x5000 };
        s
    }
    let generator = Generator {
        base: None,
        case: odd,
    };
    let port = |s: &Setup| {
        let mut e = unchanged(s);
        e.regs.d[0] &= 0xffff_0000;
        e
    };
    let rep = r.check("fault", 100, CODE, &generator, port).unwrap();
    assert!(rep.faults > 25 && rep.faults < 75, "{rep:?}");
    assert_eq!(rep.passed + rep.faults, 100);
    let f = r
        .check("fault", 10, CODE, &generator, |_| panic!("no such opcode"))
        .unwrap_err();
    assert!(f.first.contains("the port panicked: no such opcode"), "{f}");
}

#[test]
fn paths_count_the_branches_the_states_reach() {
    // tst.l d0; beq.s +2; moveq #1,d1; rts
    let mut r = rig(&[0x4a80, 0x6702, 0x7201, 0x4e75]);
    fn half_zero(g: &mut generators::Gen) -> Setup {
        let mut s = Setup::random(g);
        if g.one_in(2) {
            s.d[0] = 0;
        }
        s
    }
    let port = |s: &Setup| {
        let mut e = unchanged(s);
        if s.d[0] != 0 {
            e.regs.d[1] = 1;
        }
        e
    };
    let both = r
        .check(
            "paths",
            100,
            CODE,
            &Generator {
                base: None,
                case: half_zero,
            },
            port,
        )
        .unwrap();
    assert_eq!(both.paths, 2);
    // A generator that never makes d0 zero never takes the branch, and reports fewer.
    let one = r.check("paths", 100, CODE, &RANDOM, port).unwrap();
    assert_eq!(one.paths, 1);
}

#[test]
fn every_generator_has_one_name() {
    let mut seen = BTreeMap::new();
    for (name, _) in generators::all() {
        *seen.entry(name).or_insert(0) += 1;
    }
    let twice: Vec<_> = seen.iter().filter(|(_, n)| **n > 1).collect();
    assert!(twice.is_empty(), "registered twice: {twice:?}");
    assert!(generators::generator("h0+$774").is_some());
    assert!(generators::generator("char_hit_check").is_some());
    assert!(generators::generator("no_such_routine").is_none());
}

// ---------------------------------------------------------------------------------------------
// On the main program, from the owner's disks
// ---------------------------------------------------------------------------------------------

/// The hand translation of `h0+$774` the spike's positive check used, rewritten against [`Effects`]:
/// a raster wait (VHPOSR and DMACONR reads, nothing written), then one 80-row, one-word blit of tile
/// `d5` from h3+$13540 to `a3`, A to D, D's modulo $2c.
fn tile_blit_port(mem: &[u8], s: &Setup) -> Effects {
    let mut d = s.d;
    d[5] = (d[5] & 0xffff) * 0xa0; // mulu.w #$a0,d5
    let a5 = h(3, 0x1_3540).wrapping_add(d[5]);
    let a3 = s.a[3];
    let hw_writes = vec![
        (0x54, (a3 >> 16) as u16),
        (0x56, a3 as u16),
        (0x50, (a5 >> 16) as u16),
        (0x52, a5 as u16),
        (0x66, 0x2c),
        (0x64, 0),
        (0x44, 0xffff),
        (0x46, 0xffff),
        (0x40, 0x09f0),
        (0x42, 0),
        (0x58, 0x1401),
    ];
    // The blit, as the routine oracle's blitter runs it: pointers masked to chip RAM at the start,
    // each word read after the words already written (the tile and the screen may overlap).
    let mut out: BTreeMap<u32, u8> = BTreeMap::new();
    let rd =
        |out: &BTreeMap<u32, u8>, a: u32| out.get(&a).copied().unwrap_or_else(|| s.byte(mem, a));
    let (mut apt, mut dpt) = (a5 & 0x7_fffe, a3 & 0x7_fffe);
    for _ in 0..0x50 {
        let at = apt & 0xff_fffe;
        let (hi, lo) = (rd(&out, at), rd(&out, at + 1));
        let to = dpt & 0xff_fffe;
        out.insert(to, hi);
        out.insert(to + 1, lo);
        apt = apt.wrapping_add(2);
        dpt = dpt.wrapping_add(2 + 0x2c);
    }
    Effects {
        regs: Regs {
            d,
            a: s.a,
            ..Regs::default()
        },
        writes: out.into_iter().collect(),
        hw_writes,
    }
}

#[test]
fn a_hand_port_of_the_tile_blit_passes() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let generator = generators::generator("h0+$774").unwrap();
    let mem = r.memory_for(&generator);
    let rep = r
        .check("h0+$774", 2000, h(0, 0x774), &generator, |s| {
            tile_blit_port(&mem, s)
        })
        .unwrap_or_else(|f| panic!("{f}"));
    assert_eq!((rep.passed, rep.hangs, rep.faults), (2000, 0, 0));
    assert!(rep.paths >= 2, "{rep:?}");
    // The same port with the wrong modulo is caught.
    let f = r
        .check("h0+$774", 50, h(0, 0x774), &generator, |s| {
            let mut e = tile_blit_port(&mem, s);
            e.hw_writes[4].1 = 0x2e;
            e
        })
        .unwrap_err();
    assert!(f.first.contains("$dff066=$002c"), "{f}");
}

/// Each of the spike's routines and its entry.
const SPIKE: [(&str, u32); 9] = [
    ("h0+$24ce", h(0, 0x24ce)),
    ("h0+$2dd6", h(0, 0x2dd6)),
    ("h0+$2fae", h(0, 0x2fae)),
    ("h0+$774", h(0, 0x774)),
    ("redraw_edge_a", h(0, 0x844)),
    ("redraw_edge_b", h(0, 0xb20)),
    ("h0+$3adc", h(0, 0x3adc)),
    ("h0+$3b2a", h(0, 0x3b2a)),
    ("h4+$18870", h(4, 0x1_8870)),
];

#[test]
fn the_spike_generators_run_the_original() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let names: Vec<&str> = generators::spike::ALL.iter().map(|(n, _)| *n).collect();
    assert_eq!(
        names,
        SPIKE.map(|(n, _)| n),
        "a spike generator without an entry here"
    );
    for (name, entry) in SPIKE {
        let generator = generators::generator(name).unwrap();
        let rep = r.survey(name, 200, entry, &generator);
        assert_eq!(rep.hangs, 0, "{name}: {rep:?}");
        assert!(
            rep.passed > 150,
            "{name} returned on too few states: {rep:?}"
        );
    }
}
