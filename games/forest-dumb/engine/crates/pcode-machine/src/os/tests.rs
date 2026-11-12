//! The OS's answers on made-up programs and a made-up OFS disk, built block by block here so no
//! game's disk enters the tree. The made-up program is the translator's fixture `os_call`
//! (`pcode-rt/tests/fixtures/os_call.json`): OldOpenLibrary("dos.library"), Open, Read, Close.

use pcode_rt::{Bus, Image, PC, Regs, Stop};

use super::*;
use crate::Config;

#[path = "../../../pcode-rt/tests/fixtures/os_call.rs"]
#[rustfmt::skip]
mod os_call;

const BASES: [u32; 2] = [0x1000, 0x3000];
const STACK: u32 = 0x8000;
const RETURN_TO: u32 = 0x7_0000;

// --- a made-up OFS disk: root-directory files of up to 72 blocks ---

const ADF: usize = 901_120;
const BLOCK: usize = 512;
const ROOT: usize = 880;

fn put(b: &mut [u8], long: usize, v: u32) {
    b[long * 4..long * 4 + 4].copy_from_slice(&v.to_be_bytes());
}

fn seal(b: &mut [u8], at: usize) {
    put(b, at, 0);
    let sum = b.chunks(4).fold(0u32, |s, l| {
        s.wrapping_add(u32::from_be_bytes(l.try_into().unwrap()))
    });
    put(b, at, sum.wrapping_neg());
}

fn name_at_tail(b: &mut [u8], name: &[u8]) {
    b[432] = name.len() as u8;
    b[433..433 + name.len()].copy_from_slice(name);
}

fn hash(name: &[u8]) -> usize {
    let h = name.iter().fold(name.len() as u32, |h, c| {
        (h * 13 + c.to_ascii_uppercase() as u32) & 0x7ff
    });
    h as usize % 72
}

/// An OFS volume `volume` holding `files` in its root.
fn disk(volume: &[u8], files: &[(&[u8], &[u8])]) -> Vec<u8> {
    let mut img = vec![0u8; ADF];
    img[..4].copy_from_slice(b"DOS\0");
    put(&mut img[..BLOCK], 2, ROOT as u32);
    let sum = img[..2 * BLOCK].chunks(4).fold(0u32, |s, l| {
        let (n, c) = s.overflowing_add(u32::from_be_bytes(l.try_into().unwrap()));
        n + c as u32
    });
    put(&mut img[..BLOCK], 1, !sum);
    let blk = |n: usize| n * BLOCK..(n + 1) * BLOCK;
    let r = blk(ROOT);
    put(&mut img[r.clone()], 0, 2);
    put(&mut img[r.clone()], 3, 72);
    put(&mut img[r.clone()], 78, u32::MAX);
    name_at_tail(&mut img[r.clone()], volume);
    put(&mut img[r.clone()], 127, 1);
    let mut next = ROOT + 1;
    for (name, data) in files {
        let header = next;
        let chunks: Vec<&[u8]> = data.chunks(488).collect();
        let blocks: Vec<usize> = (0..chunks.len()).map(|i| header + 1 + i).collect();
        next = header + 1 + chunks.len();
        for (i, (&n, chunk)) in blocks.iter().zip(&chunks).enumerate() {
            let b = &mut img[blk(n)];
            put(b, 0, 8);
            put(b, 1, header as u32);
            put(b, 2, i as u32 + 1);
            put(b, 3, chunk.len() as u32);
            put(b, 4, blocks.get(i + 1).copied().unwrap_or(0) as u32);
            b[24..24 + chunk.len()].copy_from_slice(chunk);
            seal(b, 5);
        }
        let slot = 6 + hash(name);
        let head = u32::from_be_bytes(
            img[ROOT * BLOCK + slot * 4..ROOT * BLOCK + slot * 4 + 4]
                .try_into()
                .unwrap(),
        );
        let h = &mut img[header * BLOCK..(header + 1) * BLOCK];
        put(h, 0, 2);
        put(h, 1, header as u32);
        put(h, 2, blocks.len() as u32);
        put(h, 4, blocks.first().copied().unwrap_or(0) as u32);
        for (k, &p) in blocks.iter().enumerate() {
            put(h, 77 - k, p as u32);
        }
        put(h, 81, data.len() as u32);
        put(h, 127, (-3i32) as u32);
        name_at_tail(h, name);
        put(h, 124, head);
        put(h, 125, ROOT as u32);
        seal(h, 5);
        put(&mut img[r.clone()], slot, header as u32);
    }
    seal(&mut img[r], 5);
    img
}

/// Bytes that differ across a block's edge.
fn pattern(len: usize) -> Vec<u8> {
    (0..len).map(|i| (i * 7 % 251) as u8).collect()
}

// --- the machine and calls into it ---

fn bare() -> Machine {
    Machine::from_snapshot(Config::a500(BASES.to_vec()), &vec![0; 0x10_0000], &[0; 256])
}

/// The made-up program's machine: hunk 1's strings, `name` as the file to open, its jsr opcode
/// words where the program has them, and a disk holding `hello` (20 bytes) and `big`.
fn program_machine(name: &str) -> Machine {
    let mut m = bare();
    let h1 = BASES[1] as usize;
    m.chip_mut()[h1..h1 + 11].copy_from_slice(b"dos.library");
    m.chip_mut()[h1 + 0x10..h1 + 0x10 + name.len()].copy_from_slice(name.as_bytes());
    for (at, disp) in [(0xa, -408i16), (0x1c, -30), (0x30, -42), (0x38, -36)] {
        let a = BASES[0] as usize + at;
        m.chip_mut()[a..a + 2].copy_from_slice(&[0x4E, 0xAD + (at == 0xa) as u8]);
        m.chip_mut()[a + 2..a + 4].copy_from_slice(&disp.to_be_bytes());
    }
    m.insert(Disks {
        df0: disk(
            b"MADEUP",
            &[(b"hello", &pattern(20)), (b"big", &pattern(1500))],
        ),
    });
    m
}

fn img<'a>(sizes: &'a [u32], code: &'a [(u32, u32, u32)]) -> Image<'a> {
    Image {
        bases: &BASES,
        sizes,
        code,
    }
}

fn run_program(m: &mut Machine) -> (Regs, Result<(), Stop>) {
    let mut r = Regs::new();
    r.set_a(7, STACK - 4);
    m.wl(STACK - 4, RETURN_TO);
    let f = os_call::entry("h0+$0").unwrap();
    let res = f(&mut r, m, &img(os_call::HUNK_SIZES, os_call::CODE));
    (r, res)
}

/// A `jsr d16(a6)` to `a` from h0+$40, as the program would make it: the opcode in memory and the
/// return address pushed.
fn call(m: &mut Machine, r: &mut Regs, a: u32) -> Result<(), Stop> {
    let site = BASES[0] + 0x40;
    m.chip_mut()[site as usize..site as usize + 2].copy_from_slice(&[0x4E, 0xAE]);
    r.set_a(7, STACK - 4);
    m.wl(STACK - 4, site + 4);
    m.call_outside(a, r, &img(&[0x100, 0x100], &[]))
}

fn lvo(lib: Library, n: i32) -> u32 {
    lib.base().wrapping_add(n as u32)
}

#[test]
fn the_made_up_program_opens_dos_and_reads_a_file_through_the_vectors() {
    let mut m = program_machine("df0:hello");
    let (r, res) = run_program(&mut m);
    res.unwrap();
    assert_eq!(r.get(PC, 4) as u32, RETURN_TO);
    assert_eq!(r.a(7), STACK);
    assert_eq!(r.a(5), Library::Dos.base(), "OldOpenLibrary's base");
    assert_ne!(r.d(4), 0, "Open's handle");
    assert_eq!(
        r.d(5),
        20,
        "Read's count: the whole file, under the $60 asked"
    );
    let b = BASES[1] as usize + 0x20;
    assert_eq!(&m.chip()[b..b + 20], &pattern(20)[..]);
    assert_eq!(
        &m.chip()[b + 20..b + 0x60],
        &[0u8; 0x4c][..],
        "nothing past the file"
    );
    assert_eq!(m.fault(), None);
}

#[test]
fn a_missing_file_opens_as_0_and_reading_handle_0_stops_at_its_site() {
    let mut m = program_machine("df0:nothere");
    let (r, res) = run_program(&mut m);
    assert_eq!(r.d(4), 0, "Open returned 0, as AmigaDOS does");
    assert_eq!(m.opened(), &[("df0:nothere".to_string(), false)]);
    assert_eq!(
        res.unwrap_err().to_string(),
        "os: Read of handle $00000000, not open, at h0+$30"
    );
}

#[test]
fn an_unknown_lvo_stops_naming_the_library_the_lvo_and_the_site() {
    let mut m = bare();
    let mut r = Regs::new();
    let e = call(&mut m, &mut r, lvo(Library::Dos, -48)).unwrap_err();
    assert_eq!(e, Stop::Os("unknown dos.library LVO -48 at h0+$40".into()));
    assert_eq!(e.to_string(), "os: unknown dos.library LVO -48 at h0+$40");
    let e = call(&mut m, &mut r, lvo(Library::Exec, -198)).unwrap_err();
    assert_eq!(e.to_string(), "os: unknown exec.library LVO -198 at h0+$40");
    let e = call(&mut m, &mut r, lvo(Library::Graphics, -30)).unwrap_err();
    assert_eq!(
        e.to_string(),
        "os: unknown graphics.library LVO -30 at h0+$40"
    );
    // Not on a vector's boundary, an OS interrupt handler, the OS's space, and elsewhere.
    let e = call(&mut m, &mut r, lvo(Library::Dos, -31)).unwrap_err();
    assert_eq!(
        e.to_string(),
        format!(
            "os: unknown OS address ${:06x} at h0+$40",
            lvo(Library::Dos, -31)
        )
    );
    let e = call(&mut m, &mut r, irq_handler(3)).unwrap_err();
    assert_eq!(
        e.to_string(),
        "os: the OS's level 3 interrupt handler, not modelled, entered at h0+$40"
    );
    let e = call(&mut m, &mut r, 0x20_0000).unwrap_err();
    assert_eq!(e, Stop::Untranslated("$200000".into()));
    assert_eq!(r.a(7), STACK - 4, "a stop pops nothing");
}

#[test]
fn every_answered_vector_returns_as_rts_would() {
    let mut m = bare();
    let mut r = Regs::new();
    m.chip_mut()[0x5000..0x500b].copy_from_slice(b"dos.library");
    r.set_a(1, 0x5000);
    call(&mut m, &mut r, lvo(Library::Exec, exec::OLD_OPEN_LIBRARY)).unwrap();
    assert_eq!(r.d(0), Library::Dos.base());
    assert_eq!(r.a(7), STACK);
    assert_eq!(r.get(PC, 4) as u32, BASES[0] + 0x44);
    for (lib, n, _) in ANSWERED {
        assert_eq!(lvo(*lib, *n) % 2, 0);
    }
}

#[test]
fn exec_opens_the_three_libraries_and_stops_on_any_other() {
    let mut m = bare();
    let mut r = Regs::new();
    for lib in Library::ALL {
        m.chip_mut()[0x5000..0x5020].fill(0);
        m.chip_mut()[0x5000..0x5000 + lib.name().len()].copy_from_slice(lib.name().as_bytes());
        r.set_a(1, 0x5000);
        call(&mut m, &mut r, lvo(Library::Exec, exec::OLD_OPEN_LIBRARY)).unwrap();
        assert_eq!(r.d(0), lib.base(), "{}", lib.name());
        // OpenLibrary with a version 1.3 has, and one it lacks.
        r.set_d(0, 34);
        call(&mut m, &mut r, lvo(Library::Exec, exec::OPEN_LIBRARY)).unwrap();
        assert_eq!(r.d(0), lib.base());
        r.set_d(0, 35);
        call(&mut m, &mut r, lvo(Library::Exec, exec::OPEN_LIBRARY)).unwrap();
        assert_eq!(r.d(0), 0, "version 35 is not in Kickstart 1.3");
        r.set_a(1, lib.base());
        call(&mut m, &mut r, lvo(Library::Exec, exec::CLOSE_LIBRARY)).unwrap();
    }
    r.set_a(1, name(&mut m, "intuition.library"));
    let e = call(&mut m, &mut r, lvo(Library::Exec, exec::OLD_OPEN_LIBRARY)).unwrap_err();
    assert_eq!(
        e.to_string(),
        "os: OpenLibrary of \"intuition.library\", a library the machine does not answer, at h0+$40"
    );
    r.set_a(1, 0x1234);
    let e = call(&mut m, &mut r, lvo(Library::Exec, exec::CLOSE_LIBRARY)).unwrap_err();
    assert_eq!(
        e.to_string(),
        "os: CloseLibrary of $00001234, no library's base, at h0+$40"
    );
}

#[test]
fn the_os_memory_a_program_reads_is_there() {
    let cfg = Config::a500(vec![0x1000]);
    // Power-on: ExecBase at $4, the autovectors on the OS's handlers.
    let mut m = bare();
    assert_eq!(
        m.rl(4),
        Library::Exec.base(),
        "a snapshot's $4 is the machine's ExecBase"
    );
    assert_eq!(m.rl(0x6c), 0, "a snapshot's vectors are its own");
    let hunk: Vec<u8> = [0x3F3u32, 0, 1, 0, 0, 1, 0x3E9, 1, 0x4E75_4E71, 0x3F2]
        .iter()
        .flat_map(|l| l.to_be_bytes())
        .collect();
    let mut p = Machine::new(cfg, &hunk).unwrap();
    assert_eq!(p.rl(4), Library::Exec.base());
    for n in 1..=7 {
        assert_eq!(p.rl(0x60 + 4 * n), irq_handler(n));
    }
    // GfxBase's copinit and LOFlist, readable pointers into readable memory.
    let g = Library::Graphics.base();
    for off in [0x26, 0x32] {
        let cop = m.rl(g + off);
        assert_eq!(m.rl(cop), 0xFFFF_FFFE, "a copper list that ends");
    }
    // A library node's name.
    let name = m.rl(Library::Dos.base() + 10);
    assert_eq!(m.c_string(name, "", "").unwrap(), b"dos.library");
    assert_eq!(m.rw(Library::Dos.base() + 20), 34);
    assert_eq!(m.fault(), None);
}

#[test]
fn graphics_opens_topaz_8_and_no_other_font() {
    let mut m = bare();
    let mut r = Regs::new();
    m.chip_mut()[0x5000..0x500a].copy_from_slice(b"topaz.font");
    // TextAttr: ta_Name, ta_YSize.
    m.wl(0x5100, 0x5000);
    m.ww(0x5104, 8);
    r.set_a(0, 0x5100);
    call(&mut m, &mut r, lvo(Library::Graphics, gfx::OPEN_FONT)).unwrap();
    let font = r.d(0);
    assert_ne!(font, 0);
    // tf_CharData (+$22, a long) and tf_Modulo (+$26, a word): what a program copies.
    let data = m.rl(font + 0x22);
    assert_ne!(data, 0);
    m.rb(data);
    assert_eq!(m.rw(font + 0x26), 192);
    assert_eq!(m.fault(), None);
    r.set_a(1, font);
    call(&mut m, &mut r, lvo(Library::Graphics, gfx::CLOSE_FONT)).unwrap();
    r.set_d(0, 0x1234);
    call(&mut m, &mut r, lvo(Library::Graphics, gfx::FREE_SPRITE)).unwrap();
    assert_eq!(r.d(0), 0x1234, "FreeSprite returns nothing");
    m.ww(0x5104, 9);
    let e = call(&mut m, &mut r, lvo(Library::Graphics, gfx::OPEN_FONT)).unwrap_err();
    assert_eq!(
        e.to_string(),
        "os: OpenFont of \"topaz.font\" size 9, not topaz 8, at h0+$40"
    );
}

/// d1, d2, d3 then a dos call.
fn dos(m: &mut Machine, r: &mut Regs, n: i32, d1: u32, d2: u32, d3: u32) -> Result<(), Stop> {
    r.set_d(1, d1);
    r.set_d(2, d2);
    r.set_d(3, d3);
    call(m, r, lvo(Library::Dos, n))
}

fn name(m: &mut Machine, s: &str) -> u32 {
    m.chip_mut()[0x5000..0x5040].fill(0);
    m.chip_mut()[0x5000..0x5000 + s.len()].copy_from_slice(s.as_bytes());
    0x5000
}

#[test]
fn reads_continue_where_the_last_ended_and_end_with_0() {
    let mut m = program_machine("");
    let mut r = Regs::new();
    let n = name(&mut m, "MADEUP:Big");
    dos(&mut m, &mut r, dos::OPEN, n, dos::MODE_OLDFILE, 0).unwrap();
    let h = r.d(0);
    assert_ne!(h, 0, "the volume's name and any case open it");
    dos(&mut m, &mut r, dos::READ, h, 0x6000, 1000).unwrap();
    assert_eq!(r.d(0), 1000);
    dos(&mut m, &mut r, dos::READ, h, 0x6000 + 1000, 1000).unwrap();
    assert_eq!(r.d(0), 500);
    dos(&mut m, &mut r, dos::READ, h, 0x6000 + 1500, 1000).unwrap();
    assert_eq!(r.d(0), 0, "at the end");
    assert_eq!(&m.chip()[0x6000..0x6000 + 1500], &pattern(1500)[..]);
    dos(&mut m, &mut r, dos::CLOSE, h, 0, 0).unwrap();
    let e = dos(&mut m, &mut r, dos::READ, h, 0x6000, 1).unwrap_err();
    assert_eq!(
        e.to_string(),
        format!("os: Read of handle ${h:08x}, not open, at h0+$40")
    );
    let e = dos(&mut m, &mut r, dos::CLOSE, h, 0, 0).unwrap_err();
    assert_eq!(
        e.to_string(),
        format!("os: Close of handle ${h:08x}, not open, at h0+$40")
    );
    // A name without a device opens too, and its handle reuses the freed slot.
    let n = name(&mut m, "hello");
    dos(&mut m, &mut r, dos::OPEN, n, dos::MODE_OLDFILE, 0).unwrap();
    assert_eq!(r.d(0), h);
}

#[test]
fn a_read_over_translated_code_or_outside_ram_stops() {
    let mut m = program_machine("");
    let mut r = Regs::new();
    let n = name(&mut m, "df0:hello");
    dos(&mut m, &mut r, dos::OPEN, n, dos::MODE_OLDFILE, 0).unwrap();
    let h = r.d(0);
    r.set_d(1, h);
    r.set_d(2, BASES[0] + 0x8);
    r.set_d(3, 20);
    let site = BASES[0] + 0x40;
    r.set_a(7, STACK - 4);
    m.wl(STACK - 4, site + 4);
    let code = [(0u32, 0x10u32, 0x20u32)];
    let e = m
        .call_outside(
            lvo(Library::Dos, dos::READ),
            &mut r,
            &img(&[0x100, 0x100], &code),
        )
        .unwrap_err();
    assert_eq!(e, Stop::WriteIntoCode("h0+$10".into()));
    let e = dos(&mut m, &mut r, dos::READ, h, 0xE0_0000, 20).unwrap_err();
    assert_eq!(
        e.to_string(),
        "os: Read into $e00000, outside RAM, at h0+$40"
    );
    let e = dos(&mut m, &mut r, dos::READ, h, 0x6000, 0x8000_0000).unwrap_err();
    assert_eq!(
        e.to_string(),
        "os: Read of $80000000 bytes, a negative length, at h0+$40"
    );
}

#[test]
fn open_stops_on_writing_another_device_too_many_files_or_no_disk() {
    let mut m = program_machine("");
    let mut r = Regs::new();
    let n = name(&mut m, "df0:hello");
    let e = dos(&mut m, &mut r, dos::OPEN, n, 0x3EE, 0).unwrap_err();
    assert_eq!(
        e.to_string(),
        "os: Open of \"df0:hello\" in mode $3ee, not MODE_OLDFILE, at h0+$40"
    );
    let n = name(&mut m, "ram:hello");
    let e = dos(&mut m, &mut r, dos::OPEN, n, dos::MODE_OLDFILE, 0).unwrap_err();
    assert_eq!(
        e.to_string(),
        "os: Open of \"ram:hello\": device \"ram\" is not df0, at h0+$40"
    );
    let n = name(&mut m, "hello");
    for _ in 0..dos::MAX_OPEN {
        dos(&mut m, &mut r, dos::OPEN, n, dos::MODE_OLDFILE, 0).unwrap();
    }
    let e = dos(&mut m, &mut r, dos::OPEN, n, dos::MODE_OLDFILE, 0).unwrap_err();
    assert_eq!(
        e.to_string(),
        "os: Open of \"hello\": 16 files already open, at h0+$40"
    );
    let mut m = bare();
    let n = name(&mut m, "hello");
    let e = dos(&mut m, &mut r, dos::OPEN, n, dos::MODE_OLDFILE, 0).unwrap_err();
    assert!(
        e.to_string()
            .starts_with("os: Open of \"hello\": df0 holds no OFS disk"),
        "{e}"
    );
}

/// An interrupt taken while its autovector still holds the OS's handler (as `new` leaves it, before
/// a program installs its own) stops naming that handler, as a call to it does (translator T26,
/// T23's interrupts).
#[test]
fn an_interrupt_on_the_os_handler_stops_naming_it() {
    let mut m = bare();
    m.wl(0x6C, irq_handler(3));
    m.ww(0xDF_F09A, 0x8000 | crate::irq::INTEN | 1 << 5);
    m.raise(5, 0);
    let mut r = Regs::new();
    r.set(pcode_rt::SR, 2, 0x2000);
    r.set_a(7, STACK);
    let e = m.poll(&mut r, &img(&[0x100, 0x100], &[])).unwrap_err();
    assert_eq!(
        e,
        Stop::Os("the OS's level 3 interrupt handler, not modelled, entered by an interrupt".into())
    );
}
