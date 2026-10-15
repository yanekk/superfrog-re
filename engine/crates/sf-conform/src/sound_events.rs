//! Sound as events (DESIGN §2.1 check 3): the Paula writes reduced, per channel, to what a
//! listener hears: a sample started, a period or volume change, a channel stopped. The order of
//! writes within a frame and their beam positions are not compared.
//!
//! A frame's writes are applied in order for what latches when a channel starts (its sample's
//! address and length, read as the DMA starts); its period and volume, which Paula reads all the
//! time, are taken as the frame leaves them. So writes that sound the same in a frame reduce to the
//! same events whatever their order.

use std::collections::VecDeque;
use std::fmt;

use sf_paula::PaulaWrite;

/// One thing a listener hears on a channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundEvent {
    /// A sample started: its start address and length (words) as the DMA latched them, and the
    /// period and volume the frame left.
    Start {
        ch: u8,
        addr: u32,
        len: u16,
        period: u16,
        vol: u8,
    },
    Period {
        ch: u8,
        period: u16,
    },
    Volume {
        ch: u8,
        vol: u8,
    },
    Stop {
        ch: u8,
    },
}

impl SoundEvent {
    pub fn ch(&self) -> u8 {
        match *self {
            SoundEvent::Start { ch, .. }
            | SoundEvent::Period { ch, .. }
            | SoundEvent::Volume { ch, .. }
            | SoundEvent::Stop { ch } => ch,
        }
    }
}

impl fmt::Display for SoundEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            SoundEvent::Start {
                addr,
                len,
                period,
                vol,
                ..
            } => write!(
                f,
                "sample ${addr:06x} ({len} words) started, period {period}, volume {vol}"
            ),
            SoundEvent::Period { period, .. } => write!(f, "period {period}"),
            SoundEvent::Volume { vol, .. } => write!(f, "volume {vol}"),
            SoundEvent::Stop { .. } => write!(f, "stopped"),
        }
    }
}

/// The volume Paula plays for an `AUDxVOL` value: its low 7 bits, 64 and above as 64.
pub fn played_volume(value: u16) -> u8 {
    (value & 0x7f).min(64) as u8
}

const DMACON: u16 = 0x96;
/// DMACON's master enable (DMAEN).
const DMAEN: u16 = 1 << 9;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Channel {
    lc: u32,
    len: u16,
    /// `None` until written.
    per: Option<u16>,
    vol: Option<u8>,
    /// The channel's DMA bit: `None` until a write says, as at a sync point the original's
    /// hardware state is not in its memory.
    dma: Option<bool>,
    /// The period and volume last heard (reported).
    heard_per: Option<u16>,
    heard_vol: Option<u8>,
}

/// Paula's audio registers as the writes leave them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paula {
    /// DMACON's master enable: on until a write turns it off (the game turns DMA on as it takes
    /// the machine over, before any sync point).
    master: bool,
    ch: [Channel; 4],
}

impl Default for Paula {
    fn default() -> Paula {
        Paula {
            master: true,
            ch: [Channel::default(); 4],
        }
    }
}

impl Paula {
    /// Whether channel `c` plays: `None` while its DMA bit is unknown.
    fn playing(&self, c: usize) -> Option<bool> {
        if !self.master {
            return Some(false);
        }
        self.ch[c].dma
    }

    /// The period and volume channel `c` plays as `start`'s, else as changes from what was
    /// last heard: what a listener heard of them up to now.
    fn heard(&mut self, c: usize, start: Option<&mut SoundEvent>, out: &mut Vec<SoundEvent>) {
        let k = &mut self.ch[c];
        if let Some(SoundEvent::Start { period, vol, .. }) = start {
            *period = k.per.unwrap_or(0);
            *vol = k.vol.unwrap_or(0);
            (k.heard_per, k.heard_vol) = (k.per, k.vol);
            return;
        }
        if k.per.is_some() && k.per != k.heard_per {
            k.heard_per = k.per;
            out.push(SoundEvent::Period {
                ch: c as u8,
                period: k.per.unwrap_or(0),
            });
        }
        if k.vol.is_some() && k.vol != k.heard_vol {
            k.heard_vol = k.vol;
            out.push(SoundEvent::Volume {
                ch: c as u8,
                vol: k.vol.unwrap_or(0),
            });
        }
    }

    /// One frame's writes, in order: the events they make.
    pub fn frame(&mut self, writes: &[PaulaWrite]) -> Vec<SoundEvent> {
        let mut out = Vec::new();
        // Each channel's start this frame, by index in `out`, until its period and volume are known.
        let mut started: [Option<usize>; 4] = [None; 4];
        for w in writes {
            match w.reg {
                DMACON => {
                    let before: Vec<Option<bool>> = (0..4).map(|c| self.playing(c)).collect();
                    let set = w.value & 0x8000 != 0;
                    if w.value & DMAEN != 0 {
                        self.master = set;
                    }
                    for c in 0..4 {
                        if w.value & 1 << c != 0 {
                            self.ch[c].dma = Some(set);
                        }
                        let now = self.playing(c);
                        if now == Some(true) && before[c] != Some(true) {
                            let k = &self.ch[c];
                            started[c] = Some(out.len());
                            out.push(SoundEvent::Start {
                                ch: c as u8,
                                addr: k.lc,
                                len: k.len,
                                period: 0,
                                vol: 0,
                            });
                        } else if now == Some(false) && before[c] != Some(false) {
                            // What it played until now was heard.
                            match started[c].take() {
                                Some(i) => {
                                    let mut e = out[i];
                                    self.heard(c, Some(&mut e), &mut out);
                                    out[i] = e;
                                }
                                None => self.heard(c, None, &mut out),
                            }
                            out.push(SoundEvent::Stop { ch: c as u8 });
                        }
                    }
                }
                0xA0..=0xDF => {
                    let k = &mut self.ch[((w.reg - 0xA0) / 0x10) as usize];
                    match (w.reg - 0xA0) % 0x10 {
                        0x0 => k.lc = (w.value as u32) << 16 | (k.lc & 0xffff),
                        0x2 => k.lc = (k.lc & 0xffff_0000) | w.value as u32,
                        0x4 => k.len = w.value,
                        0x6 => k.per = Some(w.value),
                        0x8 => k.vol = Some(played_volume(w.value)),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        // A start takes the period and volume the frame leaves; a channel playing on reports a
        // change of either from what was last heard.
        for (c, start) in started.into_iter().enumerate() {
            if let Some(i) = start {
                let mut e = out[i];
                self.heard(c, Some(&mut e), &mut out);
                out[i] = e;
            } else if self.playing(c) != Some(false) {
                self.heard(c, None, &mut out);
            }
        }
        out
    }
}

/// The events one frame's `writes` make, from a Paula no write has touched yet.
pub fn sound_events(writes: &[PaulaWrite]) -> Vec<SoundEvent> {
    Paula::default().frame(writes)
}

/// The original's events against the engine's, frame by frame: each of the original's matched by
/// the same event on the same channel within `frames` frames, in the same order per channel, and
/// the engine making none the original does not.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SoundMatch {
    frames: u32,
    theirs: [VecDeque<(u32, SoundEvent)>; 4],
    ours: [VecDeque<(u32, SoundEvent)>; 4],
    /// Events matched a frame or more apart: tolerated, counted.
    pub shifts: u32,
    pub matched: u32,
    /// Period and volume changes left unmatched as their channel stopped within the tolerance.
    pub trailing: u32,
}

/// A period or volume change.
fn change(e: &SoundEvent) -> bool {
    matches!(e, SoundEvent::Period { .. } | SoundEvent::Volume { .. })
}

/// Whether the queue's first event is a change followed, after changes only, by its channel's
/// stop within `frames` frames of it.
fn fades_out(q: &VecDeque<(u32, SoundEvent)>, frames: u32) -> bool {
    let Some(&(at, first)) = q.front() else {
        return false;
    };
    change(&first)
        && q.iter()
            .find(|(_, e)| !change(e))
            .is_some_and(|&(stop, e)| matches!(e, SoundEvent::Stop { .. }) && stop - at <= frames)
}

impl SoundMatch {
    pub fn new(frames: u32) -> SoundMatch {
        SoundMatch {
            frames,
            ..SoundMatch::default()
        }
    }

    /// Frame `f`'s events on both sides: what differs, if anything. A channel that differs is
    /// cleared on both sides, so a scan goes on from there.
    pub fn frame(
        &mut self,
        f: u32,
        theirs: &[SoundEvent],
        ours: &[SoundEvent],
    ) -> Result<(), String> {
        for e in theirs {
            self.theirs[e.ch() as usize & 3].push_back((f, *e));
        }
        for e in ours {
            self.ours[e.ch() as usize & 3].push_back((f, *e));
        }
        let mut errors = Vec::new();
        for c in 0..4 {
            if let Err(e) = self.channel(c, f) {
                self.theirs[c].clear();
                self.ours[c].clear();
                errors.push(e);
            }
        }
        match errors.is_empty() {
            true => Ok(()),
            false => Err(errors.join("; ")),
        }
    }

    fn channel(&mut self, c: usize, f: u32) -> Result<(), String> {
        let frames = self.frames;
        loop {
            let (t, o) = (
                self.theirs[c].front().copied(),
                self.ours[c].front().copied(),
            );
            if let (Some((ft, te)), Some((fo, oe))) = (t, o)
                && te == oe
            {
                if ft.abs_diff(fo) > frames {
                    return Err(format!(
                        "channel {c}: {te}: at frame {ft} in the original, {fo} in the engine, more than {frames} frames apart"
                    ));
                }
                self.shifts += (ft != fo) as u32;
                self.matched += 1;
                self.theirs[c].pop_front();
                self.ours[c].pop_front();
                continue;
            }
            // A change heard only for the frames before its channel stops could go unheard with
            // the stop the tolerance's frames sooner: it need not be matched.
            if t.is_some() && o != t && fades_out(&self.theirs[c], frames) {
                self.theirs[c].pop_front();
                self.trailing += 1;
                continue;
            }
            if o.is_some() && o != t && fades_out(&self.ours[c], frames) {
                self.ours[c].pop_front();
                self.trailing += 1;
                continue;
            }
            break;
        }
        let (t, o) = (
            self.theirs[c].front().copied(),
            self.ours[c].front().copied(),
        );
        // A change may still be followed by its stop within the tolerance: wait for it.
        let waits = |e: Option<(u32, SoundEvent)>| {
            e.is_some_and(|(at, e)| change(&e) && f.saturating_sub(at) <= frames)
        };
        if let (Some((ft, te)), Some((fo, oe))) = (t, o)
            && !waits(t)
            && !waits(o)
        {
            return Err(format!(
                "channel {c}: the original's next event is {te} at frame {ft}, the engine's {oe} at frame {fo}"
            ));
        }
        for (side, other, e) in [
            ("the original", "the engine", t),
            ("the engine", "the original", o),
        ] {
            if let Some((at, e)) = e
                && f.saturating_sub(at) > frames
            {
                return Err(format!(
                    "channel {c}: {side}: {e} at frame {at}; {other}: nothing within {frames} frames"
                ));
            }
        }
        Ok(())
    }

    /// Events left unmatched when the case ends: their match could only come after it (any older
    /// one has failed already). Tolerated and counted.
    pub fn cut_off(&self) -> u32 {
        self.theirs
            .iter()
            .chain(&self.ours)
            .map(|q| q.len() as u32)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(reg: u16, value: u16) -> PaulaWrite {
        PaulaWrite {
            reg,
            value,
            hpos: 0,
            vpos: 0,
        }
    }

    #[test]
    fn played_volume_is_the_low_seven_bits_capped_at_64() {
        assert_eq!(played_volume(0xf4fd), 64);
        assert_eq!(played_volume(0x33fc), 64);
        assert_eq!(played_volume(0x0605), 5);
        assert_eq!(played_volume(64), 64);
        assert_eq!(played_volume(63), 63);
    }

    #[test]
    fn a_channel_of_unknown_state_starts_on_its_bit_and_stops_on_its_clear() {
        let mut p = Paula::default();
        assert_eq!(
            p.frame(&[w(0x96, 0x0002)]),
            vec![SoundEvent::Stop { ch: 1 }]
        );
        assert_eq!(p.frame(&[w(0x96, 0x0002)]), vec![], "already stopped");
        let e = p.frame(&[
            w(0xB0, 0x0001),
            w(0xB2, 0x2000),
            w(0xB4, 100),
            w(0x96, 0x8002),
            w(0xB6, 428),
            w(0xB8, 40),
        ]);
        assert_eq!(
            e,
            vec![SoundEvent::Start {
                ch: 1,
                addr: 0x12000,
                len: 100,
                period: 428,
                vol: 40
            }]
        );
        // The loop's pointer set after the start is not a new sample; a volume change is heard.
        assert_eq!(
            p.frame(&[w(0xB0, 0), w(0xB2, 0x10), w(0xB8, 30)]),
            vec![SoundEvent::Volume { ch: 1, vol: 30 }]
        );
        assert_eq!(p.frame(&[w(0xB8, 30)]), vec![], "the same volume again");
        // The master enable off stops every channel that plays.
        // (and the ones never written, as their state was not known).
        assert_eq!(
            p.frame(&[w(0x96, 0x0200)]),
            (0..4).map(|ch| SoundEvent::Stop { ch }).collect::<Vec<_>>()
        );
    }
}
