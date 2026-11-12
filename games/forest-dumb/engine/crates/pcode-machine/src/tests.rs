use std::sync::{Arc, Mutex};

use amiga_disk::hunk::{HUNK_CODE, HUNK_DATA, HUNK_END, HUNK_HEADER, HUNK_RELOC32};

use super::*;

const HUNK_BASES: [u32; 2] = [0xC0_1000, 0x2000];

/// A made-up hunk file: hunk 0 code, 4 longs; hunk 1 data, 2 longs of 4 declared. Hunk 0's long
/// at 4 points at hunk 1's byte 4, its long at 8 at its own byte $c.
fn program() -> Vec<u8> {
    let longs: Vec<u32> = vec![
        HUNK_HEADER,
        0, // no resident libraries
        2,
        0,
        1,
        4, // hunk 0: 4 longs
        4, // hunk 1: 4 longs, 2 in the file
        HUNK_CODE,
        4,
        0x4E75_4E71,
        0x0000_0004,
        0x0000_000C,
        0xDEAD_BEEF,
        HUNK_RELOC32,
        1,
        1, // one offset into hunk 1
        4,
        1,
        0, // one offset into hunk 0
        8,
        0,
        HUNK_END,
        HUNK_DATA,
        2,
        0x1122_3344,
        0x5566_7788,
        HUNK_END,
    ];
    longs.iter().flat_map(|l| l.to_be_bytes()).collect()
}

fn machine() -> Machine {
    Machine::new(Config::a500(HUNK_BASES.to_vec()), &program()).unwrap()
}

/// A device that logs every access and reads `0x1000 | reg`.
type Access = (char, u16, u16, u64);

#[derive(Clone, Default)]
struct Log(Arc<Mutex<Vec<Access>>>);

impl Device for Log {
    fn read(&mut self, reg: u16, at: u64) -> u16 {
        self.0.lock().unwrap().push(('r', reg, 0, at));
        0x1000 | reg
    }
    fn write(&mut self, reg: u16, v: u16, at: u64) {
        self.0.lock().unwrap().push(('w', reg, v, at));
    }
}

impl Log {
    fn take(&self) -> Vec<Access> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

fn img() -> Image<'static> {
    Image {
        bases: &[],
        sizes: &[],
        code: &[],
    }
}

/// The machine's time set to `n` cycles with no CPU work behind it (`set_cycles`: no bus
/// contention to fit), then a block start.
fn at_cycles(m: &mut Machine, n: u64) {
    m.set_cycles(n);
    let mut r = Regs::new();
    r.tick(n as u32);
    m.poll(&mut r, &img()).unwrap();
}

#[test]
fn loading_places_hunks_at_their_bases_and_relocates() {
    let mut m = machine();
    assert_eq!(m.hunk_sizes(), &[16, 16]);
    assert_eq!(m.rl(0xC0_1000), 0x4E75_4E71);
    assert_eq!(m.rl(0xC0_1004), 0x2004, "hunk 1's base added");
    assert_eq!(m.rl(0xC0_1008), 0xC0_100C, "hunk 0's own base added");
    assert_eq!(m.rl(0xC0_100C), 0xDEAD_BEEF, "not relocated");
    assert_eq!(m.rl(0x2000), 0x1122_3344);
    assert_eq!(m.rl(0x2004), 0x5566_7788);
    assert_eq!(m.rl(0x2008), 0, "the declared rest is cleared");
    assert_eq!(m.slow()[0x1000], 0x4E);
    assert_eq!(m.chip()[0x2003], 0x44);
    assert_eq!(m.fault(), None);
}

#[test]
fn a_program_that_does_not_fit_is_a_load_error() {
    let bad = |bases: Vec<u32>| Machine::new(Config::a500(bases), &program()).err().unwrap();
    assert_eq!(
        bad(vec![0x2000]),
        LoadError::HunkCount { file: 2, bases: 1 }
    );
    // Chip RAM's last 8 bytes cannot hold 16, and a mirror is not a place to load.
    assert_eq!(
        bad(vec![0xC0_1000, 0x7_FFF8]),
        LoadError::Placement {
            hunk: 1,
            base: 0x7_FFF8,
            size: 16
        }
    );
    assert!(matches!(
        bad(vec![0x8_0000, 0x2000]),
        LoadError::Placement { hunk: 0, .. }
    ));
    assert!(matches!(
        bad(vec![0xC7_FFF8, 0x2000]),
        LoadError::Placement { hunk: 0, .. }
    ));
    assert!(matches!(
        bad(vec![0xF8_0000, 0x2000]),
        LoadError::Placement { hunk: 0, .. }
    ));
    let e = Machine::new(Config::a500(HUNK_BASES.to_vec()), b"not a hunk file")
        .err()
        .unwrap();
    assert!(matches!(e, LoadError::Hunk(_)), "{e}");
    let mut cfg = Config::a500(HUNK_BASES.to_vec());
    cfg.chip = 0x6_0000;
    assert!(matches!(
        Machine::new(cfg, &program()).err().unwrap(),
        LoadError::Config(_)
    ));
}

#[test]
fn chip_ram_mirrors_and_slow_ram_ends_at_its_size() {
    let mut m = machine();
    m.ww(0x10, 0xABCD);
    assert_eq!(m.rw(0x8_0010), 0xABCD);
    assert_eq!(m.rw(0x18_0010), 0xABCD);
    m.wb(0x18_0011, 0x55);
    assert_eq!(m.chip()[0x11], 0x55);
    m.wl(0xC7_FFFC, 0x0102_0304);
    assert_eq!(m.slow()[0x7_FFFF], 4);
    assert_eq!(m.rl(0xC7_FFFC), 0x0102_0304);
    assert_eq!(m.chip().len(), 0x8_0000);
    assert_eq!(m.slow().len(), 0x8_0000);
    // Past slow RAM the window mirrors the custom registers ($C80096 is DMACON).
    m.ww(0xC8_0096, 0x8200);
    assert_eq!(m.custom()[0x96 / 2], 0x8200);
    assert_eq!(m.fault(), None);
}

#[test]
fn an_access_outside_memory_is_a_stop_naming_it_at_the_next_block() {
    for (a, write) in [
        // Kickstart's ROM past the OS's made-up structures, and a write to them (translator T26).
        (0xFC_0000u32, false),
        (0xF8_1000, true),
        (0x20_0000, true),
        (0xE8_0000, false),
        (0xDC_0000, true),
        (0xFFF0_0000, false),
    ] {
        let mut m = machine();
        let mut r = Regs::new();
        assert_eq!(m.poll(&mut r, &img()), Ok(()));
        if write {
            m.wl(a, 0x1234_5678);
        } else {
            assert_eq!(m.rw(a), 0);
        }
        let want = Stop::Outside(format!("${:06x}", a & 0xFF_FFFF));
        assert_eq!(m.fault(), Some(&want));
        assert_eq!(m.poll(&mut r, &img()), Err(want.clone()));
        // The first one is named; later ones do not replace it.
        m.rb(0xF0_0000);
        assert_eq!(m.poll(&mut r, &img()), Err(want));
    }
    // A long straddling chip RAM's window names the first byte outside it.
    let mut m = machine();
    m.rl(0x1F_FFFE);
    assert_eq!(m.fault(), Some(&Stop::Outside("$200000".into())));
    assert_eq!(m.fault().unwrap().to_string(), "outside-memory: $200000");
}

#[test]
fn vhposr_after_n_cycles_is_the_313_line_frames_beam() {
    let mut m = machine();
    let frame = 313 * 454;
    for (n, line, h) in [
        (0u64, 0u16, 0u16),
        (453, 0, 226),
        (454, 1, 0),
        (454 * 0x2C + 2 * 0x40 + 1, 0x2C, 0x40),
        (454 * 256 + 20, 256, 10),
        (frame - 2, 312, 226),
        (frame, 0, 0),
        (3 * frame + 454 * 100 + 2 * 0xE2, 100, 0xE2),
    ] {
        at_cycles(&mut m, n);
        assert_eq!(m.beam(n), (line, h), "{n}");
        let v = m.rw(0xDF_F004);
        let vh = m.rw(0xDF_F006);
        assert_eq!(vh, (line & 0xFF) << 8 | h, "VHPOSR after {n}");
        assert_eq!(v, 0x8000 | line >> 8, "VPOSR after {n}");
        // The byte reads: VHPOSR's high byte is V7-V0, its low byte the position.
        assert_eq!(m.rb(0xDF_F006), (line & 0xFF) as u8);
        assert_eq!(m.rb(0xDF_F007), h as u8);
    }
}

#[test]
fn a_snapshot_starts_the_beam_where_its_registers_say() {
    let cfg = Config::a500(vec![]);
    let mut mem = vec![0u8; 0x10_0000];
    mem[0x100] = 7;
    mem[0x8_0000 + 0x10] = 9;
    let mut custom = [0u16; 256];
    custom[2] = 0x8001; // VPOSR: line 256 + ...
    custom[3] = 0x0510; // ... 5, h $10
    custom[0x180 / 2] = 0x0FFF;
    let mut m = Machine::from_snapshot(cfg, &mem, &custom);
    assert_eq!(m.beam(0), (261, 0x10));
    assert_eq!(m.beam(454), (262, 0x10));
    assert_eq!(m.beam(454 * 52), (0, 0x10), "the frame wraps at 313");
    assert_eq!(m.rb(0x100), 7);
    assert_eq!(m.rb(0xC0_0010), 9);
    assert_eq!(
        m.rw(0xDF_F180),
        0x0FFF,
        "a readable register reads the snapshot's value"
    );
    assert_eq!(m.rw(0xDF_F006), 0x0510);
    m.sync_beam(1000, 0, 0);
    assert_eq!(m.beam(1000), (0, 0));
    assert_eq!(m.beam(1000 + 454 * 2 + 4), (2, 2));
}

#[test]
fn a_write_only_register_read_takes_the_buses_last_word() {
    let mut m = machine();
    let dmacon = Log::default();
    m.attach(0x096..0x098, Box::new(dmacon.clone()));
    at_cycles(&mut m, 40);
    // The last word the bus moved is the RAM word just read.
    assert_eq!(m.rw(0x2004), 0x5566);
    assert_eq!(m.rw(0xDF_F096), 0x5566, "read back as the bus held it");
    assert_eq!(
        dmacon.take(),
        vec![('w', 0x096, 0x5566, 40)],
        "and written into the register"
    );
    assert_eq!(m.custom()[0x96 / 2], 0x5566);
    // A write moves the bus too: the next faulty read writes it again.
    m.ww(0x10, 0x0F0F);
    assert_eq!(m.rw(0xDF_F0A8), 0x0F0F);
    assert_eq!(m.custom()[0xA8 / 2], 0x0F0F);
    // A byte read of a write-only register is the word read's half.
    m.ww(0x10, 0xAB12);
    assert_eq!(m.rb(0xDF_F097), 0x12);
    assert_eq!(dmacon.take(), vec![('w', 0x096, 0xAB12, 40)]);
}

#[test]
fn a_read_only_register_write_changes_nothing_but_the_bus() {
    let mut m = machine();
    let paula = Log::default();
    m.attach(0x010..0x020, Box::new(paula.clone()));
    at_cycles(&mut m, 7);
    m.ww(0xDF_F01C, 0xC020); // INTENAR: read-only
    m.ww(0xDF_F006, 0x1234); // VHPOSR
    assert_eq!(
        paula.take(),
        vec![],
        "no device sees a write to a read-only register"
    );
    assert_eq!(m.custom()[0x1C / 2], 0);
    assert_eq!(m.custom()[0x06 / 2], 0);
    assert_eq!(m.rw(0xDF_F006), 0x0003, "the beam is untouched");
    // A readable register reads its device, at the access's cycle count (ADKCONR; INTENAR and
    // INTREQR are the machine's own since T23).
    assert_eq!(m.rw(0xDF_F010), 0x1010);
    assert_eq!(paula.take(), vec![('r', 0x010, 0, 7)]);
    // The write still put its word on the bus.
    m.ww(0xDF_F01E, 0x7777);
    assert_eq!(
        m.rw(0xDF_F000),
        0x7777,
        "BLTDDAT, write-only, reads the bus"
    );
}

/// The routine judge (translator T27) reads the CPU's custom writes without ending a frame: every
/// one, read-only registers and a write-only read's write included, in order with its cycle.
#[test]
fn custom_writes_lists_the_frame_so_far_in_order() {
    let mut m = machine();
    at_cycles(&mut m, 5);
    m.ww(0xDF_F180, 0x0123);
    m.wb(0xDF_F01C, 0x40); // INTENAR: read-only, still written on the bus
    let _ = m.rw(0xDF_F0A8); // AUD0VOL: write-only, the read writes the bus's word
    assert_eq!(
        m.custom_writes(),
        &[(5, 0x180, 0x0123), (5, 0x01C, 0x4040), (5, 0x0A8, 0x4040)]
    );
}

#[test]
fn custom_writes_reach_their_device_and_bytes_fill_both_halves() {
    let mut m = machine();
    let aud = Log::default();
    m.attach(0x0A0..0x0E0, Box::new(aud.clone()));
    at_cycles(&mut m, 99);
    m.ww(0xDF_F0A8, 0x0040);
    m.wb(0xDF_F0B9, 0x20);
    m.wl(0xDF_F0A0, 0x0001_2340); // AUD0LCH then AUD0LCL
    m.ww(0xDF_F180, 0x0F00); // COLOR00: no device, kept
    assert_eq!(
        aud.take(),
        vec![
            ('w', 0x0A8, 0x0040, 99),
            ('w', 0x0B8, 0x2020, 99),
            ('w', 0x0A0, 0x0001, 99),
            ('w', 0x0A2, 0x2340, 99),
        ]
    );
    assert_eq!(m.custom()[0x180 / 2], 0x0F00);
    assert_eq!(m.rw(0xDF_F180), 0x0F00);
}

#[test]
#[should_panic(expected = "already has a device")]
fn a_register_takes_one_device() {
    let mut m = machine();
    m.attach(0x0A0..0x0B0, Box::new(Log::default()));
    m.attach(0x0AE..0x0C0, Box::new(Log::default()));
}

#[test]
fn the_cias_answer_on_their_byte_lanes() {
    let mut m = machine();
    let (a, b) = (Log::default(), Log::default());
    m.attach_cia(Cia::A, Box::new(a.clone()));
    m.attach_cia(Cia::B, Box::new(b.clone()));
    at_cycles(&mut m, 5);
    assert_eq!(m.rb(0xBF_E001), 0x00, "CIA-A PRA: the device's low byte");
    assert_eq!(m.rb(0xBF_EC01), 0x0C);
    assert_eq!(m.rb(0xBF_D100), 0x01, "CIA-B PRB");
    assert_eq!(a.take(), vec![('r', 0, 0, 5), ('r', 0xC, 0, 5)]);
    assert_eq!(b.take(), vec![('r', 1, 0, 5)]);
    m.wb(0xBF_E201, 0x03);
    m.wb(0xBF_D100, 0xF8);
    m.ww(0xBF_C300, 0xAA55); // both selected: A the low byte, B the high
    assert_eq!(a.take(), vec![('w', 2, 0x03, 5), ('w', 3, 0x55, 5)]);
    assert_eq!(b.take(), vec![('w', 1, 0xF8, 5), ('w', 3, 0xAA, 5)]);
    // A CIA with no device keeps what was written.
    let mut m = machine();
    m.wb(0xBF_E201, 0x03);
    assert_eq!(m.rb(0xBF_E201), 0x03);
    assert_eq!(m.rb(0xBF_D200), 0x00);
    assert_eq!(m.fault(), None);
}

#[test]
fn the_a500_bank_map_puts_custom_registers_at_d80000() {
    // vAmiga's A500 bank map (not A1000/A2000A) makes $D8-$DB custom; $DC is the absent clock.
    let mut m = machine();
    m.ww(0xD8_0096, 0x8200);
    assert_eq!(m.custom()[0x96 / 2], 0x8200);
    m.ww(0xDB_F180, 0x0123);
    assert_eq!(m.rw(0xDB_F180), 0x0123);
    assert_eq!(m.fault(), None);
    m.rb(0xDC_0001);
    assert_eq!(m.fault(), Some(&Stop::Outside("$dc0001".into())));
}

#[test]
fn a_cia_access_puts_its_byte_on_the_bus() {
    // vAmiga peek8/poke8/poke16 <CIA> set dataBus, so a later faulty read takes the CIA's byte.
    let mut m = machine();
    let a = Log::default();
    m.attach_cia(Cia::A, Box::new(a.clone()));
    m.ww(0x10, 0xFFFF);
    assert_eq!(m.rb(0xBF_EC01), 0x0C);
    assert_eq!(m.rw(0xDF_F096), 0x000C, "a CIA byte read");
    m.ww(0x10, 0xFFFF);
    m.wb(0xBF_E201, 0x03);
    assert_eq!(m.rw(0xDF_F096), 0x0003, "a CIA byte write");
    m.ww(0x10, 0xFFFF);
    m.ww(0xBF_E200, 0xAA55);
    assert_eq!(m.rw(0xDF_F096), 0xAA55, "a CIA word write");
    m.ww(0x10, 0xFFFF);
    assert_eq!(m.rw(0xBF_C000), 0x0000);
    assert_eq!(
        m.rw(0xDF_F096),
        0x0000,
        "a CIA word read, B with no device then A"
    );
}

// Translator T25: Paula's writes, CIA-A and the input.

const JOY1DAT_A: u32 = 0xDF_F00C;
const CIAA_PRA: u32 = 0xBF_E001;
const CIAA_SDR: u32 = 0xBF_EC01;
const CIAA_ICR: u32 = 0xBF_ED01;
const CIAA_CRA: u32 = 0xBF_EE01;

#[test]
fn joy1dat_and_the_fire_buttons_read_as_set() {
    let mut m = machine();
    assert_eq!(m.rb(CIAA_PRA) & 0xC0, 0xC0, "nothing held");
    m.set_input(&Input {
        joy1: 0x0103,
        fire: true,
        keys: vec![],
        mouse_left: false,
    });
    assert_eq!(m.rw(JOY1DAT_A), 0x0103);
    assert_eq!(m.rb(JOY1DAT_A + 1), 0x03);
    assert_eq!(m.rb(CIAA_PRA) & 0xC0, 0x40, "fire is bit 7, low");
    // JOY1DAT is read-only: a write does not change what it reads.
    m.ww(JOY1DAT_A, 0xFFFF);
    assert_eq!(m.rw(JOY1DAT_A), 0x0103);
    m.set_input(&Input {
        mouse_left: true,
        ..Input::default()
    });
    assert_eq!((m.rw(JOY1DAT_A), m.rb(CIAA_PRA) & 0xC0), (0, 0x80));
    // The hand engine's input converts to the same.
    use amiga_input::{FrameInput, Joy, KeySet};
    let i = Input::from(FrameInput {
        joy: Joy::RIGHT | Joy::UP | Joy::FIRE,
        keys: KeySet::key(0x45).with(0x19),
    });
    assert_eq!(
        i,
        Input {
            joy1: 0x0103,
            fire: true,
            keys: vec![0x19, 0x45],
            mouse_left: false
        }
    );
    assert_eq!(m.fault(), None);
}

#[test]
fn keys_already_held_at_a_snapshot_are_not_sent_again() {
    let held = Input {
        keys: vec![0x19],
        ..Input::default()
    };
    let mut m = machine();
    m.wb(CIAA_ICR, 0x88);
    at_cycles(&mut m, 1_000);
    m.keys_held(&held);
    m.set_input(&held);
    assert_eq!(m.take_cia_interrupts(), 0, "the held key is not pressed again");
    assert_eq!(m.cia_a().keys_queued(), 0);
    // Letting it go is a change: its up byte is sent.
    m.set_input(&Input::default());
    assert_eq!(m.take_cia_interrupts(), INTREQ_PORTS);
    // Without `keys_held` the same input is a press.
    let mut m = machine();
    m.wb(CIAA_ICR, 0x88);
    at_cycles(&mut m, 1_000);
    m.set_input(&held);
    assert_eq!(m.take_cia_interrupts(), INTREQ_PORTS);
}

#[test]
fn a_key_arrives_through_sdr_with_its_interrupt_and_waits_for_the_handshake() {
    use amiga_input::key_byte;
    let mut m = machine();
    // As the program's entry does: every CIA-A interrupt off, then the serial port's on.
    m.wb(CIAA_ICR, 0x7F);
    m.wb(CIAA_ICR, 0x88);
    at_cycles(&mut m, 1_000);
    m.set_input(&Input {
        keys: vec![0x28, 0x40],
        ..Input::default()
    });
    assert_eq!(m.take_cia_interrupts(), INTREQ_PORTS);
    assert_eq!(m.take_cia_interrupts(), 0);
    // The level 2 handler: ICR's serial flag, the byte, the handshake.
    assert_eq!(m.rb(CIAA_ICR), 0x88);
    assert_eq!(m.rb(CIAA_SDR), key_byte(0x28, false));
    let cra = m.rb(CIAA_CRA);
    m.wb(CIAA_CRA, cra | 0x40);
    m.wb(CIAA_SDR, 0);
    // Without the handshake's end the next byte never comes.
    at_cycles(&mut m, 500_000);
    assert_eq!((m.take_cia_interrupts(), m.rb(CIAA_SDR)), (0, 0));
    m.wb(CIAA_CRA, cra & !0x40);
    at_cycles(&mut m, 500_000 + cia::KEY_BYTE_CYCLES - 1);
    assert_eq!(m.take_cia_interrupts(), 0);
    at_cycles(&mut m, 500_000 + cia::KEY_BYTE_CYCLES);
    assert_eq!(m.take_cia_interrupts(), INTREQ_PORTS);
    assert_eq!(m.rb(CIAA_ICR), 0x88);
    assert_eq!(m.rb(CIAA_SDR), key_byte(0x40, false));
    assert_eq!(m.fault(), None);
}

#[test]
fn a_masked_key_raises_nothing_until_unmasked() {
    let mut m = machine();
    m.set_input(&Input {
        keys: vec![0x45],
        ..Input::default()
    });
    assert_eq!(m.take_cia_interrupts(), 0);
    m.wb(CIAA_ICR, 0x88);
    assert_eq!(m.take_cia_interrupts(), INTREQ_PORTS);
    // A host starting from a snapshot sets the mask itself.
    let mut m = machine();
    m.cia_a_mut().set_icr_mask(0x08);
    m.set_input(&Input {
        keys: vec![0x45],
        ..Input::default()
    });
    assert_eq!(m.take_cia_interrupts(), INTREQ_PORTS);
}

#[test]
fn paula_writes_are_kept_in_order_with_their_cycles() {
    let mut m = machine();
    at_cycles(&mut m, 454 * 0x2C + 2 * 0x40);
    m.ww(0xDF_F0A8, 0x0040); // AUD0VOL
    m.ww(0xDF_F180, 0x0FFF); // COLOR00: not Paula's
    m.wl(0xDF_F0A0, 0x0001_2340); // AUD0LCH, AUD0LCL
    at_cycles(&mut m, 454 * 0x2D);
    m.ww(0xDF_F096, 0x8201); // DMACON
    m.wb(0xDF_F09E, 0x80); // ADKCON, the byte on both halves
    m.ww(0xDF_F09A, 0xC018); // INTENA
    m.ww(0xDF_F09C, 0x0008); // INTREQ
    let w = |at: u64, reg: u16, value: u16, vpos: u16, hpos: u16| {
        (
            at,
            amiga_paula::PaulaWrite {
                reg,
                value,
                hpos,
                vpos,
            },
        )
    };
    let t0 = 454 * 0x2C + 2 * 0x40;
    let t1 = 454 * 0x2D;
    let want = [
        w(t0, 0xA8, 0x0040, 0x2C, 0x40),
        w(t0, 0xA0, 0x0001, 0x2C, 0x40),
        w(t0, 0xA2, 0x2340, 0x2C, 0x40),
        w(t1, 0x96, 0x8201, 0x2D, 0),
        w(t1, 0x9E, 0x8080, 0x2D, 0),
        w(t1, 0x9A, 0xC018, 0x2D, 0),
        w(t1, 0x9C, 0x0008, 0x2D, 0),
    ];
    assert_eq!(m.paula_writes(), &want[..]);
    // A read of a write-only audio register writes the bus's word into it, as on the oracle.
    m.rw(0xDF_F0B8);
    assert_eq!(m.paula_writes().last(), Some(&w(t1, 0xB8, 0x0008, 0x2D, 0)));
    assert_eq!(m.take_paula_writes().len(), 8);
    assert!(m.paula_writes().is_empty(), "the next frame starts empty");
}

#[test]
fn a_cia_a_timer_is_a_stop_at_the_next_block() {
    let mut m = machine();
    let mut r = Regs::new();
    m.wb(CIAA_CRA, 0x40);
    m.rb(0xBF_E101); // PRB
    assert_eq!(m.poll(&mut r, &img()), Ok(()));
    m.wb(0xBF_E401, 0x10); // TALO
    let want = Stop::Unsupported {
        op: "CIA-A timer".into(),
        at: "$bfe401".into(),
    };
    assert_eq!(m.poll(&mut r, &img()), Err(want));
    // CIA-B keeps its last byte written.
    let mut m = machine();
    m.wb(0xBF_D100, 0x87);
    assert_eq!(m.rb(0xBF_D100), 0x87);
    assert_eq!(m.fault(), None);
}
