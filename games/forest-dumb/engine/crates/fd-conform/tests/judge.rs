//! The translation service's judge (plans/forest-dumb-level-1 T33) on the main program from the
//! owner's disks: the stub built in without `FD_PORT` fails (the negative check), and a hand
//! translation of `h0+$774` against `Cpu` and `Mem` passes (the positive one).

use fd_conform::generators::{self, h};
use fd_conform::judge::{self, Cpu, port};
use fd_conform::routine::{Generator, Rig, Setup};

/// `h0+$774` by hand: a raster wait (reads only), then one 80-row, one-word blit of tile `d5` from
/// h3+$13540 to `a3`, A to D, D's modulo $2c, its registers written in the original's order. The
/// blit itself is `Mem`'s, run by the BLTSIZE write.
fn tile_blit(cpu: &mut Cpu) {
    cpu.d[5] = (cpu.d[5] & 0xffff) * 0xa0;
    let a5 = h(3, 0x1_3540).wrapping_add(cpu.d[5]);
    let a3 = cpu.a[3];
    let m = &mut cpu.mem;
    m.wl(0xdff054, a3);
    m.wl(0xdff050, a5);
    m.ww(0xdff066, 0x2c);
    m.ww(0xdff064, 0);
    m.wl(0xdff044, 0xffff_ffff);
    m.ww(0xdff040, 0x09f0);
    m.ww(0xdff042, 0);
    m.ww(0xdff058, 0x1401);
}

/// The same with D's modulo wrong: every state differs.
fn tile_blit_wrong_modulo(cpu: &mut Cpu) {
    tile_blit(cpu);
    cpu.mem.hw_writes[4].1 = 0x2e;
}

#[test]
fn the_stub_fails_and_a_hand_port_passes() {
    let mut r = Rig::new().unwrap_or_else(|e| panic!("{e}"));
    let generator = generators::generator("h0+$774").unwrap();
    let entry = h(0, 0x774);
    // The test build has no FD_PORT, so `port::port` is the stub, which writes nothing.
    let f = judge::check(&mut r, "h0+$774", 200, entry, &generator, port::port).unwrap_err();
    assert_eq!(f.failures, 200, "{f}");
    assert!(f.first.contains("custom-chip writes: original made"), "{f}");

    let rep = judge::check(&mut r, "h0+$774", 2000, entry, &generator, tile_blit)
        .unwrap_or_else(|f| panic!("{f}"));
    assert_eq!((rep.passed, rep.hangs, rep.faults), (2000, 0, 0));

    let f = judge::check(
        &mut r,
        "h0+$774",
        50,
        entry,
        &generator,
        tile_blit_wrong_modulo,
    )
    .unwrap_err();
    assert_eq!(f.failures, 50, "{f}");
    assert!(f.first.contains("$dff066=$002c"), "{f}");
}

/// `move.w #con1,$dff042.l; move.w #$0041,$dff058.l; rts` at $2000 in empty memory.
fn blit_with_bltcon1(con1: u16) -> Rig {
    let code: [u16; 9] = [
        0x33fc, con1, 0x00df, 0xf042, 0x33fc, 0x0041, 0x00df, 0xf058, 0x4e75,
    ];
    let mut mem = vec![0u8; 0x10_0000];
    for (i, w) in code.iter().enumerate() {
        mem[0x2000 + i * 2..0x2002 + i * 2].copy_from_slice(&w.to_be_bytes());
    }
    let mut r = Rig::from_memory(mem);
    r.set_max_instr(1_000);
    r
}

#[test]
fn line_and_fill_blits_are_counted_as_unmodelled() {
    let random = Generator {
        base: None,
        case: Setup::random,
    };
    for (con1, unmodelled) in [
        (0x0000, 0),
        (0x0002, 0),
        (0x0001, 20),
        (0x0008, 20),
        (0x0010, 20),
    ] {
        let rep = blit_with_bltcon1(con1).survey("blit", 20, 0x2000, &random);
        assert_eq!(rep.passed, 20, "BLTCON1 ${con1:04x}");
        assert_eq!(rep.unmodelled_blits, unmodelled, "BLTCON1 ${con1:04x}");
    }
}
