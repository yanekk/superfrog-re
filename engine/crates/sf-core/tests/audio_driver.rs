//! The sound driver on hand-built data: the entry points' Paula writes and state in the cases the
//! disks' own tunes and effects may never reach. The comparison with the original's routines, on
//! the real data, is `sf-conform/tests/audio_driver.rs`.

use sf_assets::sound::StreamSample;
use sf_core::audio::{
    AudioDriver, BANK_ADDR, BankId, File, H0, PaulaWrite, STREAM_BUFFER, SoundData, TuneId,
};

/// A stand-in hunk 0: zero, but for the streamer idle (`$148b4` = `$ffff`) as in the game.
fn h0() -> Vec<u8> {
    let mut h = vec![0; 0x50000];
    h[0x148b4..0x148b6].copy_from_slice(&[0xff, 0xff]);
    h
}

fn rv(w: &[PaulaWrite]) -> Vec<(u16, u16)> {
    w.iter().map(PaulaWrite::reg_value).collect()
}

fn be32(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}

/// A bank: effect sizes, two samples of 2 and 3 words, the end mark, then effect 0: priority 5,
/// one step of sample 1 for 2 ticks, period 400, volume 30, then stop.
fn bank() -> Vec<u8> {
    let mut b = vec![0u8; 0x400];
    let effect_len = 4 + 18;
    b[..4].copy_from_slice(&be32(effect_len));
    b.extend(be32(2));
    b.extend([1, 2, 3, 4]);
    b.extend(be32(3));
    b.extend([5, 6, 7, 8, 9, 10]);
    b.extend(be32(0xffff_ffff));
    b.extend([0, 0, 0, 5]);
    // flags 0 (stop after), sample 1, duration 2, period 400..400 in 1, volume 30..30 in 1, offset 0
    for w in [0x0001u16, 2, 400, 400, 1, 30, 30, 1, 0] {
        b.extend(w.to_be_bytes());
    }
    b
}

fn data_with(tune: Vec<u8>) -> SoundData {
    let stream = (0..8)
        .map(|k| StreamSample {
            addr: H0 + 0x40000 + k * 0x1000,
            bytes: vec![0; 0x150],
            len: 0x150,
            volume: 40,
        })
        .collect();
    SoundData::from_parts(
        h0(),
        vec![(TuneId::Title, tune)],
        vec![(BankId::Worlds12, bank())],
        stream,
    )
}

#[test]
fn a_file_that_is_not_a_module_does_not_start() {
    let d = data_with(b"NOPE, no module".to_vec());
    let mut a = AudioDriver::new(&d);
    a.set_module_addr(BANK_ADDR);
    let mut out = Vec::new();
    a.play_music(&d, TuneId::Title, &mut out);
    assert!(!a.music_on());
    // Interrupts off and on again, nothing else.
    assert_eq!(rv(&out), vec![(0x9a, 0x4000), (0x9a, 0xc000)]);
    out.clear();
    a.tick(&d, &mut out);
    assert!(out.is_empty(), "{out:?}");
}

#[test]
fn bank_init_points_at_samples_and_effects_and_puts_the_module_after_them() {
    let d = data_with(b"P41A".to_vec());
    let mut a = AudioDriver::new(&d);
    a.file_loaded(&d, BANK_ADDR, File::Bank(BankId::Worlds12));
    a.sfx_bank_init(&d);
    let tables = a.vars(&d)[5].1.clone();
    let long = |i: usize| u32::from_be_bytes(tables[i * 4..i * 4 + 4].try_into().unwrap());
    assert_eq!(long(0), BANK_ADDR + 0x400);
    assert_eq!(long(1), BANK_ADDR + 0x400 + 8);
    // Entries past the last sample keep what they held (zero here).
    assert_eq!(long(2), 0);
    let effects = BANK_ADDR + 0x400 + 8 + 10 + 4;
    assert_eq!(long(256), effects);
    assert_eq!(long(257), effects + 22);
    assert_eq!(a.module_addr(), effects + 22);
}

#[test]
fn an_effect_plays_its_step_then_hands_the_channel_back() {
    let d = data_with(b"P41A".to_vec());
    let mut a = AudioDriver::new(&d);
    a.file_loaded(&d, BANK_ADDR, File::Bank(BankId::Worlds12));
    a.sfx_bank_init(&d);
    let mut out = Vec::new();
    a.play_sfx(&d, 0, 2, &mut out);
    let sample = BANK_ADDR + 0x400 + 8 + 4;
    assert_eq!(
        rv(&out),
        vec![
            (0x96, 4),
            (0xc6, 1),
            (0x9a, 0x4000),
            (0x9a, 0xc000),
            (0xc6, 400),
            (0xc8, 30),
            (0xc4, 3),
            (0xc0, (sample >> 16) as u16),
            (0xc2, sample as u16),
            (0x96, 0x8004),
        ]
    );
    // The ramps write once and are done (they do not move); the step lasts two ticks, then the
    // effect stops.
    let mut ticks = Vec::new();
    for _ in 0..4 {
        let mut out = Vec::new();
        a.sfx_tick(&d, &mut out);
        ticks.push(rv(&out));
    }
    assert_eq!(ticks[0], vec![(0xc6, 400), (0xc8, 30)]);
    assert!(ticks[1].is_empty());
    assert_eq!(ticks[2], vec![(0x96, 4), (0xc6, 1), (0xc8, 0)]);
    assert!(ticks[3].is_empty());
}

#[test]
fn a_streamed_sample_plays_160_bytes_a_frame_then_stops() {
    let d = data_with(b"P41A".to_vec());
    let mut a = AudioDriver::new(&d);
    let mut out = Vec::new();
    assert_eq!(a.stream_source(), None);
    a.play_sample(&d, 3, &mut out);
    let src = d.stream[3].addr;
    assert_eq!(a.stream_source(), Some(src));
    // 0x150 bytes = 0xa8 words: frames at 0x58 and 0x08 words left, then the end.
    let mut frames = Vec::new();
    for _ in 0..4 {
        let mut out = Vec::new();
        a.stream_tick(&d, &mut out);
        frames.push(rv(&out));
    }
    let play = vec![
        (0xd6, 1),
        (0x96, 8),
        (0xd0, (STREAM_BUFFER >> 16) as u16),
        (0xd2, STREAM_BUFFER as u16),
        (0xd8, 40),
        (0xd4, 0x50),
        (0x96, 0x8008),
        (0xd6, 0x1bc),
    ];
    assert_eq!(frames[0], play);
    assert_eq!(frames[1], play);
    assert_eq!(
        frames[2],
        // `clr.w $dff0d8` writes the prefetched word first, as the real 68000 does.
        vec![
            (0xd6, 1),
            (0x96, 8),
            (0x96, 8),
            (0xd4, 2),
            (0xd8, 0x33fc),
            (0xd8, 0)
        ]
    );
    assert!(frames[3].is_empty());
    assert_eq!(a.stream_source(), None);
}

/// What a re-sync takes to be in chip RAM (T164): a world's bank and module in a level, but the
/// title's module when the module is still at the bank's place, as in the attract demo, where the
/// title's music plays on over W1-1.
#[test]
fn files_for_keeps_the_titles_module_in_the_attract_demo() {
    use sf_core::audio::files_for;
    assert_eq!(files_for(1, BANK_ADDR), [(BANK_ADDR, File::Tune(TuneId::Title))]);
    assert_eq!(
        files_for(1, BANK_ADDR + 0x8000),
        [
            (BANK_ADDR, File::Bank(BankId::Worlds12)),
            (BANK_ADDR + 0x8000, File::Tune(TuneId::World(1))),
        ]
    );
    assert_eq!(files_for(0, BANK_ADDR), [(BANK_ADDR, File::Tune(TuneId::Title))]);
}
