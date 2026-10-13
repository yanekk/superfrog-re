//! The intro's sound (docs/13 "Sound"). Its driver is the game's (docs/11) at other addresses,
//! its variables in the same order around its code, so the engine runs the game's port
//! ([`crate::audio::AudioDriver`]) on the intro's variables: taken from the intro's memory for each
//! use and written back after. What the intro does differently is here: starting the music
//! (`music_start` h0+$390c2, its module and samples given), the stream's buffer (h3+$621b0), the
//! timeline of sound events the vertical blank plays (`timeline_tick` h0+$b18c), and the tables
//! of effects and samples in hunk 2.

use sf_assets::sound::SoundData;
use sf_paula::PaulaWrite;

use super::mem::StoryMem;
use super::{data, h0, h2, h3};
use crate::audio::{AudioDriver, IntroVars};

/// `timeline_ptr` and `timeline_wait`.
const TIMELINE: u32 = h0(0xb1f4);
const TIMELINE_WAIT: u32 = h0(0xb1f8);
/// The driver's variables.
const VARS: IntroVars = IntroVars {
    sfx: h0(0x38efc),
    music_on: h0(0x39296),
    p41: h0(0x3a352),
    shadow: h0(0x3a556),
    stream: h0(0x3a726),
    samples: h2(0xdcda),
    effects: h2(0xdca2),
};
const MUSIC_ON: u32 = h0(0x39296);
const FADE: u32 = h0(0x3a556);
const STREAM_PTR: u32 = h0(0x3a726);
const STREAM_LEFT: u32 = h0(0x3a72a);
const STREAM_VOLUME: u32 = h0(0x3a72c);
/// The replayer's "free" byte for channel 3 (`$ff`: music's).
const FREE_3: u32 = h0(0x3a428);
/// Channel 3's effect record.
const SFX_3: u32 = h0(0x38f8a);
/// Where the streamed sample's next 160 bytes go for Paula.
const STREAM_BUFFER: u32 = h3(0x621b0);
/// The offset of the intro's copy of the replayer from the game's (`p41_play` h0+$392b2 against
/// the game's h0+$1343c): the tables it reads are as far from its code.
const REPLAYER_SHIFT: u32 = 0x392b2 - 0x1343c;

/// The sound data the driver reads besides chip RAM: the replayer's tables, from the intro's own
/// hunk 0 placed where the game's are.
pub fn sound_data(intro_h0: &[u8]) -> SoundData {
    let h0 = intro_h0
        .get(REPLAYER_SHIFT as usize..)
        .unwrap_or(&[])
        .to_vec();
    SoundData::from_parts(h0, Vec::new(), Vec::new(), Vec::new())
}

fn wr(out: &mut Vec<PaulaWrite>, reg: u16, value: u16) {
    out.push(PaulaWrite {
        reg,
        value,
        hpos: 0,
        vpos: 0,
    });
}

/// Run `f` on the driver as the intro's memory holds it, then write its state back.
fn session<R>(
    m: &mut StoryMem,
    sd: &SoundData,
    f: impl FnOnce(&mut AudioDriver, &SoundData) -> R,
) -> R {
    let chip = (data::INTRO_H2, m.read(data::INTRO_H2, CHIP_BYTES));
    let mut d = AudioDriver::intro_from(&|a| m.b(a), &VARS, chip);
    let r = f(&mut d, sd);
    let (vars, written) = d.intro_vars(&VARS);
    for (at, bytes) in vars {
        m.write(at, &bytes);
    }
    for (a, v) in written {
        m.set_b(a, v);
    }
    r
}

/// Hunk 2's size: modules, samples and effects.
const CHIP_BYTES: u32 = 0x1_8bd4;

/// `timeline_start` (h0+$b17c).
pub fn timeline_start(m: &mut StoryMem, list: u32) {
    m.set_l(TIMELINE, list);
    m.set_w(TIMELINE_WAIT, m.w(list + 2));
}

/// `music_start` (h0+$390c2).
pub fn music_start(
    m: &mut StoryMem,
    sd: &SoundData,
    module: u32,
    samples: u32,
    out: &mut Vec<PaulaWrite>,
) {
    session(m, sd, |d, sd| d.music_start_intro(sd, module, samples, out));
}

/// `music_stop` (h0+$391fc).
pub fn music_stop(m: &mut StoryMem, out: &mut Vec<PaulaWrite>) {
    m.set_w(MUSIC_ON, 0);
    m.set_w(FADE, 0);
    wr(out, 0x96, 0x000f);
}

/// The fade `music_fade_stop` (h0+$391d6) starts before it waits.
pub fn music_fade(m: &mut StoryMem) {
    m.set_w(FADE, 0x2001);
}

pub fn music_on(m: &StoryMem) -> bool {
    m.w(MUSIC_ON) != 0
}

/// `sfx_play` (h0+$38a16): effect `id` on `channel`.
pub fn sfx_play(
    m: &mut StoryMem,
    sd: &SoundData,
    id: u16,
    channel: u16,
    out: &mut Vec<PaulaWrite>,
) {
    session(m, sd, |d, sd| d.play_sfx(sd, id as u8, channel as u8, out));
}

/// `stream_tick` (h0+$3a65a): the streamed sample's next 160 bytes to channel 3.
pub fn stream_tick(m: &mut StoryMem, out: &mut Vec<PaulaWrite>) {
    if m.w(STREAM_LEFT) == 0xffff {
        return;
    }
    wr(out, 0xd6, 1);
    wr(out, 0x96, 8);
    let from = m.l(STREAM_PTR);
    let bytes = m.read(from, 0xc0);
    m.write(STREAM_BUFFER, &bytes);
    m.set_l(STREAM_PTR, from.wrapping_add(0xa0));
    let left = m.w(STREAM_LEFT).wrapping_sub(0x50);
    m.set_w(STREAM_LEFT, left);
    if (left as i16) < 0 {
        wr(out, 0x96, 8);
        wr(out, 0xd4, 2);
        // `clr.w $dff0d8`: the prefetched word ($33fc), then 0.
        wr(out, 0xd8, 0x33fc);
        wr(out, 0xd8, 0);
        m.set_w(STREAM_LEFT, 0xffff);
        m.set_b(FREE_3, 0xff);
        return;
    }
    wr(out, 0xd0, (STREAM_BUFFER >> 16) as u16);
    wr(out, 0xd2, STREAM_BUFFER as u16);
    wr(out, 0xd8, m.w(STREAM_VOLUME));
    wr(out, 0xd4, 0x50);
    wr(out, 0x96, 0x8008);
    wr(out, 0xd6, 0x1bc);
}

/// `stream_start` (h0+$3a72e): the sample its descriptor at `at` names streams on channel 3.
fn stream_start(m: &mut StoryMem, at: u32, out: &mut Vec<PaulaWrite>) {
    wr(out, 0x9a, 0x4000);
    m.clear(SFX_3, 0x34);
    m.set_b(FREE_3, 1);
    m.set_l(STREAM_PTR, m.l(at));
    m.set_w(STREAM_LEFT, (m.l(at + 4) as u16) >> 1);
    m.set_w(STREAM_VOLUME, m.w(at + 0xa));
    wr(out, 0x9a, 0xc000);
}

/// The vertical blank's sound after the fades: the music's tick and its DMA, the effects, the
/// timeline (h0+$3929a, $39b3a, $38bfa, $b18c; the handler waits eight lines after the music).
pub fn tick(m: &mut StoryMem, sd: &SoundData, out: &mut Vec<PaulaWrite>) {
    session(m, sd, |d, sd| {
        d.music_tick(sd, out);
        d.music_dma_tick(sd, out);
        d.sfx_tick(sd, out);
    });
    // `timeline_tick` (h0+$b18c).
    let wait = m.w(TIMELINE_WAIT).wrapping_sub(1);
    m.set_w(TIMELINE_WAIT, wait);
    let at = m.l(TIMELINE);
    if (wait as i16) >= 0 || at == 0 {
        return;
    }
    if m.l(at) == 0xffff_ffff {
        m.set_w(TIMELINE_WAIT, 0x7d00);
        return;
    }
    m.set_l(TIMELINE, at + 12);
    let next = m.w(at + 0xe);
    m.set_w(TIMELINE_WAIT, if (next as i16) < 0 { 0x7d00 } else { next });
    let d0 = m.l(at + 4);
    if d0 <= 0xff {
        let d1 = m.l(at + 8);
        sfx_play(m, sd, d0 as u16, d1 as u16, out);
    } else {
        stream_start(m, d0, out);
    }
}
