//! The mixer on hand-made register writes and memory (T10). The reference comparison is
//! `sf-conform/tests/paula_vs_oracle.rs`.

use sf_paula::{CLOCKS_PER_FRAME, CLOCKS_PER_LINE, ChipMemory, Paula, PaulaWrite, RESAMPLER_DELAY};

/// Chip RAM as a byte vector.
struct Mem(Vec<u8>);

impl Mem {
    fn with(at: u32, bytes: &[u8]) -> Mem {
        let mut m = Mem(vec![0; 0x1000]);
        m.put(at, bytes);
        m
    }

    fn put(&mut self, at: u32, bytes: &[u8]) {
        self.0[at as usize..at as usize + bytes.len()].copy_from_slice(bytes);
    }
}

impl ChipMemory for Mem {
    fn chip_word(&self, addr: u32) -> u16 {
        let a = addr as usize;
        u16::from_be_bytes([self.0[a], self.0[a + 1]])
    }
}

fn w(reg: u16, value: u16) -> PaulaWrite {
    PaulaWrite {
        reg,
        value,
        hpos: 0,
        vpos: 0,
    }
}

fn at(reg: u16, value: u16, vpos: u16, hpos: u16) -> PaulaWrite {
    PaulaWrite {
        reg,
        value,
        hpos,
        vpos,
    }
}

/// The writes that start channel `n` playing `words` words from `addr` at `period` and `volume`.
fn play(n: u16, addr: u32, words: u16, period: u16, volume: u16) -> Vec<PaulaWrite> {
    let base = 0xA0 + 0x10 * n;
    vec![
        w(base, (addr >> 16) as u16),
        w(base + 2, addr as u16),
        w(base + 4, words),
        w(base + 6, period),
        w(base + 8, volume),
        w(0x96, 0x8200 | 1 << n),
    ]
}

/// Run `frames` frames, the first with `first` writes; return each channel's taps end to end and
/// the host output.
fn run(
    host: u32,
    first: &[PaulaWrite],
    mem: &dyn ChipMemory,
    frames: usize,
) -> ([Vec<i16>; 4], Vec<[i16; 2]>) {
    let mut p = Paula::new(host);
    let mut taps: [Vec<i16>; 4] = Default::default();
    let mut out = Vec::new();
    for f in 0..frames {
        p.frame(if f == 0 { first } else { &[] }, mem, &mut out);
        for (n, t) in p.channel_taps().iter().enumerate() {
            taps[n].extend_from_slice(t);
        }
    }
    (taps, out)
}

/// (clock, new value) wherever a tap sequence changes.
fn changes(t: &[i16]) -> Vec<(usize, i16)> {
    (1..t.len())
        .filter(|&i| t[i] != t[i - 1])
        .map(|i| (i, t[i]))
        .collect()
}

#[test]
fn a_channel_at_period_428_loops_its_waveform_at_the_right_frequency() {
    // Four distinct bytes, two words: one cycle is 4 x 428 = 1 712 Paula clocks, 2 071.8 Hz.
    let mem = Mem::with(0x100, &[10, 20, 30, 40]);
    let (taps, _) = run(48_000, &play(0, 0x100, 2, 428, 64), &mem, 2);
    let ch = changes(&taps[0]);
    assert!(ch.len() > 300, "{} changes", ch.len());
    // DMACON lands 2 clocks after its write; the requests reach Agnus at the next two line
    // starts and the second word arrives in line 2's slot 0x0D, when the first byte plays.
    assert_eq!(ch[0], (2 * CLOCKS_PER_LINE as usize + 0x0D, 640));
    for pair in ch.windows(2) {
        assert_eq!(pair[1].0 - pair[0].0, 428, "{pair:?}");
    }
    let values: Vec<i16> = ch.iter().take(8).map(|c| c.1).collect();
    assert_eq!(values, [640, 1280, 1920, 2560, 640, 1280, 1920, 2560]);
    // Every other channel stays silent.
    assert!(taps[1..].iter().all(|t| t.iter().all(|&v| v == 0)));
}

#[test]
fn the_pointer_and_length_reload_at_the_end_of_each_block() {
    // A 2-word block at 0x100; LC moved to 0x200 mid-play takes effect at the next block end.
    let mut mem = Mem::with(0x100, &[1, 2, 5, 6]);
    mem.put(0x200, &[3, 4, 7, 8]);
    let mut p = Paula::new(48_000);
    let mut out = Vec::new();
    let mut taps = Vec::new();
    p.frame(&play(0, 0x100, 2, 300, 1), &mem, &mut out);
    taps.extend_from_slice(p.channel_taps()[0]);
    p.frame(&[at(0xA2, 0x200, 100, 0)], &mem, &mut out);
    taps.extend_from_slice(p.channel_taps()[0]);
    let ch = changes(&taps);
    // The block loops 1 2 5 6 1 2 5 6 ... until the new LC, then 3 4 7 8 ...
    let v: Vec<i16> = ch.iter().map(|c| c.1).collect();
    let switch = v
        .iter()
        .position(|&x| x == 3)
        .expect("never played the new block");
    assert_eq!(switch % 4, 0, "the new block started mid-block: {v:?}");
    assert!(v[..switch].chunks(4).all(|c| c == [1, 2, 5, 6]), "{v:?}");
    assert!(
        v[switch..].chunks(4).all(|c| [3, 4, 7, 8].starts_with(c)),
        "{v:?}"
    );
    let switched_at = ch[switch].0;
    assert!(switched_at > CLOCKS_PER_FRAME as usize + 100 * CLOCKS_PER_LINE as usize);
}

#[test]
fn a_period_below_the_dma_minimum_starves_rather_than_speeds_up() {
    // Nothing clamps AUDxPER. DMA brings one word (two bytes) per line, and the DAC takes a new
    // value only when a new word arrived since its last one, so a period of 1 plays at most two
    // new bytes per line, not one per clock.
    let bytes: Vec<u8> = (1..=64).collect();
    let mem = Mem::with(0x100, &bytes);
    let (taps, _) = run(48_000, &play(1, 0x100, 32, 1, 64), &mem, 1);
    let ch = changes(&taps[1]);
    assert!(ch.len() > 100, "{} changes", ch.len());
    for line in 0..CLOCKS_PER_FRAME as usize / CLOCKS_PER_LINE as usize {
        let span = line * CLOCKS_PER_LINE as usize..(line + 1) * CLOCKS_PER_LINE as usize;
        let n = ch.iter().filter(|c| span.contains(&c.0)).count();
        assert!(n <= 2, "line {line}: {n} new values");
    }
}

#[test]
fn volume_64_is_full_and_0_is_silent() {
    let mem = Mem::with(0x100, &[0x80, 0x7F]);
    let (full, out) = run(48_000, &play(0, 0x100, 1, 400, 64), &mem, 2);
    assert!(full[0].contains(&(-128 * 64)) && full[0].contains(&(127 * 64)));
    // A full-scale square on one side reaches near full scale: one channel is half of it.
    assert!(
        out.iter().any(|s| s[0] <= -16_000),
        "{:?}",
        out.iter().map(|s| s[0]).min()
    );
    // Above 64 counts as 64 (the low 7 bits, at most 64).
    let (over, _) = run(48_000, &play(0, 0x100, 1, 400, 0x7F), &mem, 1);
    assert_eq!(over[0][..], full[0][..over[0].len()]);
    let (silent, out) = run(48_000, &play(0, 0x100, 1, 400, 0), &mem, 2);
    assert!(silent[0].iter().all(|&v| v == 0));
    assert!(out.iter().all(|&s| s == [0, 0]));
}

#[test]
fn channels_0_and_3_are_left_1_and_2_right() {
    let mem = Mem::with(0x100, &[0x60, 0xA0]);
    for (n, side) in [(0, 0), (1, 1), (2, 1), (3, 0)] {
        let (_, out) = run(48_000, &play(n, 0x100, 1, 300, 64), &mem, 2);
        let loud = |s: usize| out.iter().any(|x| x[s].abs() > 1_000);
        assert!(loud(side), "channel {n} silent on side {side}");
        assert!(
            out.iter().all(|x| x[1 - side] == 0),
            "channel {n} leaks to side {}",
            1 - side
        );
    }
}

#[test]
fn the_output_is_a_pure_function_of_the_writes_and_memory() {
    let bytes: Vec<u8> = (0..200u32).map(|i| (i * 37 % 256) as u8).collect();
    let mem = Mem::with(0x100, &bytes);
    let mut writes = play(0, 0x100, 100, 250, 50);
    writes.extend(play(2, 0x140, 30, 333, 40));
    writes.push(at(0xB8, 20, 150, 7));
    let a = run(44_100, &writes, &mem, 3);
    let b = run(44_100, &writes, &mem, 3);
    assert_eq!(a, b);
    assert!(a.1.iter().any(|s| s[0] != 0) && a.1.iter().any(|s| s[1] != 0));
}

#[test]
fn a_frame_makes_the_host_rate_share_of_samples_with_a_fixed_delay() {
    let mut p = Paula::new(48_000);
    let mut out = Vec::new();
    let mut clocks = 0u64;
    for _ in 0..50 {
        let before = out.len();
        p.frame(&[], &Mem(vec![0; 16]), &mut out);
        clocks += CLOCKS_PER_FRAME as u64;
        // 48 000 / 49.92: 961 or 962 a long frame, exactly floor(clocks * rate / clock) in total.
        assert!((961..=962).contains(&(out.len() - before)));
        assert_eq!(out.len() as u64, clocks * 48_000 / 3_546_895);
    }
    // A step settles at exactly its level, RESAMPLER_DELAY samples late.
    let mem = Mem::with(0x100, &[0x40; 4]);
    let (_, out) = run(48_000, &play(0, 0x100, 2, 200, 64), &mem, 3);
    let first = out.iter().position(|s| s[0] != 0).unwrap();
    let settled = &out[first + 2 * RESAMPLER_DELAY..];
    assert!(
        settled.iter().all(|s| s[0] == 0x40 * 64 * 2),
        "{:?}",
        &settled[..4]
    );
}

#[test]
fn a_short_frame_is_one_line_shorter() {
    let mut p = Paula::new(48_000);
    let mut out = Vec::new();
    p.set_frame_lines(312);
    p.frame(&[], &Mem(vec![0; 16]), &mut out);
    assert_eq!(p.channel_taps()[0].len(), 312 * CLOCKS_PER_LINE as usize);
    p.set_frame_lines(313);
    p.frame(&[], &Mem(vec![0; 16]), &mut out);
    assert_eq!(p.channel_taps()[0].len(), CLOCKS_PER_FRAME as usize);
}

#[test]
#[should_panic(expected = "a PAL frame of 200 lines")]
fn a_frame_is_long_or_short_and_nothing_else() {
    Paula::new(48_000).set_frame_lines(200);
}

#[test]
fn writes_listed_after_the_frame_wrapped_land_at_the_next_frames_start() {
    // The reference lists a frame from just after its start to just after the next one's: here
    // the last write, at (0, 5), belongs to the next frame and must not start the channel early.
    let mem = Mem::with(0x100, &[0x10, 0x20]);
    let mut writes = play(0, 0x100, 1, 300, 64);
    writes.pop();
    for x in &mut writes {
        x.vpos = 300;
    }
    writes.push(at(0x96, 0x8201, 0, 5));
    let (taps, _) = run(48_000, &writes, &mem, 2);
    let ch = changes(&taps[0]);
    // DMACON at clock 5 of frame 2, +2, then two line starts: the first byte at line 2's slot.
    let expected = CLOCKS_PER_FRAME as usize + 2 * CLOCKS_PER_LINE as usize + 0x0D;
    assert_eq!(ch[0], (expected, 0x10 * 64));
}

#[test]
fn with_dma_off_the_cpu_plays_a_word_per_interrupt_and_intreq_lands_a_clock_late() {
    // Interrupt mode: AUD0DAT from the CPU plays one word, raises the interrupt, and the channel
    // stops at the word's end while the interrupt is pending. A DAT write then does nothing until
    // INTREQ clears it, and an INTREQ write lands one clock after it is made (68000, vAmiga
    // `Paula::pokeINTREQ`), so a DAT in the same clock is still ignored.
    let line = CLOCKS_PER_LINE as usize;
    let writes = [
        at(0x9C, 0x0780, 10, 0),
        at(0xA8, 64, 10, 1),
        at(0xA6, 100, 10, 2),
        at(0xAA, 0x1020, 20, 0),
        at(0xAA, 0x5060, 30, 0),
        at(0x9C, 0x0080, 40, 0),
        at(0xAA, 0x7008, 40, 0),
        at(0xAA, 0x3040, 40, 1),
    ];
    let (taps, _) = run(48_000, &writes, &Mem(vec![0; 16]), 1);
    let (a, b) = (20 * line, 40 * line + 1);
    assert_eq!(
        changes(&taps[0]),
        [
            (a, 0x10 * 64),
            (a + 100, 0x20 * 64),
            (b, 0x30 * 64),
            (b + 100, 0x40 * 64)
        ]
    );
}

#[test]
fn dmacon_lands_two_clocks_after_its_write() {
    // DMACON lands 2 clocks late (vAmiga `Agnus::pokeDMACON`). Written at hpos 225 it lands at the
    // next line's clock 0, just after Agnus took that line's requests, so the channel's first
    // request waits a line longer than one written at hpos 224.
    let line = CLOCKS_PER_LINE as usize;
    let mem = Mem::with(0x100, &[10, 20, 30, 40]);
    for (hpos, first) in [(224, 2 * line + 0x0D), (225, 3 * line + 0x0D)] {
        let mut writes = play(0, 0x100, 2, 428, 64);
        writes.pop();
        writes.push(at(0x96, 0x8201, 0, hpos));
        let (taps, _) = run(48_000, &writes, &mem, 1);
        assert_eq!(changes(&taps[0])[0], (first, 640), "DMACON at hpos {hpos}");
    }
}
