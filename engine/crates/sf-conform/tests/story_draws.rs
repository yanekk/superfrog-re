//! The intro's draws as the CPU runs them (T53): `bobs::draw`'s program (`sim::Prog`) on
//! `story::sim`, started where the original's draw wrote DMACON `$8400`, on the original's own
//! copper list and on its memory as it was at that write, against every register write the
//! original's draw made: BLTSIZE above all. The vertical blank's handler a draw holds off is the
//! original's: the CPU leaves the program where the oracle took the interrupt and comes back after
//! the handler's acknowledge (`INTREQ = $20`) and its tail. A draw the handler lets go of (its
//! wait in `draw_frames` or `draw_next`) runs from that handler's acknowledge, through the
//! instructions to the draw ([`intro::handler_to_draw`], T54), so its DMACON `$8400` is checked
//! too.
//!
//! The audio channels' DMA slots come from the original's Paula writes, every frame from the sync
//! point on, through `sf_paula::AudioDma` (T96): from scene 1's effect (oracle frame 3 034, line 48)
//! channel 0's slot `$0D` holds the blits up.

use std::collections::VecDeque;
use std::ops::Range;

use sf_conform::{Case, sync};
use sf_core::front::story::sim::{self, Fixed, Sim};
use sf_core::front::story::{StoryMem, bobs, cpu::LINE, intro, mem::RAM_BYTES, vbl};
use sf_core::render::copper::{self, Fetch};
use sf_paula::AudioDma;

/// The oracle's frames from here on are followed: COP1LC is picked up from the CPU's writes (it
/// sets it in 3 005).
const FROM: u32 = 3_000;
/// Scene 1's draws (from oracle frame 3 027), its sound effects from 3 034 on.
const SCENE_1: Range<u32> = 3_022..3_727;
/// Scene 3's first draws by `draw_next` (h0+$84ca, from oracle frame 5 882), whose wait is on
/// `vbl_count` (T54).
const SCENE_3: Range<u32> = 5_880..5_900;
/// Scene 4's draws (the last in 6 308, the fade out after it), its first sound effect in 6 240,
/// line 17. In 6 436 `buffers_clear` starts as a draw does (DMACON `$8400`, the masks, BLTCON0
/// `$09f0`) but is no draw: it has no DMACON `$0400`.
const SCENE_4: Range<u32> = 6_192..6_436;

const BLTCON0: u16 = 0x040;
const BLTAFWM: u16 = 0x044;
const BLTSIZE: u16 = 0x058;
const DMACON: u16 = 0x096;
const INTREQ: u16 = 0x09c;
const COP1LCH: u16 = 0x080;
const COP1LCL: u16 = 0x082;

/// One oracle frame, its times counted from its own start.
struct Frame {
    number: u32,
    /// RAM at the CPU's first write of DMACON `$8400` in the frame, if it made one.
    draw_mem: Option<sf_oracle::AmigaMemory>,
    /// The CPU's writes: (cycle, register, value).
    writes: Vec<(u32, u16, u16)>,
    /// Level-3 interrupts taken: their cycle.
    irqs: Vec<u32>,
    copper: Vec<copper::Beam>,
    copper_writes: Vec<copper::Write>,
    /// The audio channels' DMA slots ([`AudioDma::frame`]).
    audio: Vec<(u32, u8)>,
    fetch: Fetch,
    /// DMACON as the frame found it.
    dmacon: u16,
    /// The PC the frame's handler returned to ([`sync::intro_resumes`]).
    resumes: Option<u32>,
}

/// A beam position of the step as a cycle from the frame's start (`next`: in the frame after).
fn cycle(vpos: u16, hpos: u16, next: bool) -> u32 {
    vpos as u32 * LINE + hpos as u32 + if next { sim::FRAME } else { 0 }
}

/// Step the oracle through the last of `ranges`, checking each draw that starts in one of them;
/// gives the oracle frames of the draws checked, each with whether it ran from the handler's
/// acknowledge.
fn check_draws(ranges: &[Range<u32>]) -> Vec<(u32, bool)> {
    let upto = ranges.iter().map(|r| r.end).max().unwrap() + 3;
    let case = Case::load(&sf_conform::cases_dir().join("story-intro.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    let synced = sync::reach(&case.sync, &case.script, upto + 20).unwrap();
    let mut amiga = synced.amiga;
    amiga.watch_write(Some((DMACON, 0x8400)));
    let mut window: VecDeque<Frame> = VecDeque::new();
    let mut cop1lc = 0u32;
    let mut fetch = Fetch::default();
    use sim::dmacon::*;
    let mut dma = DMAEN | BLTEN;
    let mut of = 0;
    let mut audio = AudioDma::new();
    let mut checked = Vec::new();
    while amiga.frame() < upto {
        let number = amiga.frame() + 1;
        // The copper runs the list from COP1LC on the frame's memory as it starts.
        let followed = number >= FROM;
        let mut bus = Vec::new();
        let copper_writes = if followed {
            let word = |a: u32| {
                let b = amiga.peek(a, 2);
                u16::from_be_bytes([b[0], b[1]])
            };
            copper::run_bus(&|a, _, _| word(a), cop1lc, fetch, &mut bus)
        } else {
            Vec::new()
        };
        let resumes = sync::intro_resumes(&amiga);
        let cap = sync::step_inserting(&mut amiga, case.script.input_at(of)).unwrap();
        of += 1;
        let slots = audio.frame(&cap.paula, amiga.frame_lines()).to_vec();
        let raw = amiga.cpu_writes();
        for w in &raw {
            match w.reg {
                COP1LCH => cop1lc = cop1lc & 0xffff | (w.value as u32) << 16,
                COP1LCL => cop1lc = cop1lc & 0xffff_0000 | w.value as u32,
                _ => {}
            }
        }
        if !followed {
            continue;
        }
        // Writes past the frame's end (the step runs on to an instruction's end) count on.
        let mut writes = Vec::new();
        let mut frame_of = 0;
        let mut last = 0;
        for w in &raw {
            let mut t = frame_of * sim::FRAME + w.vpos as u32 * LINE + w.hpos as u32;
            if t < last {
                frame_of += 1;
                t += sim::FRAME;
            }
            last = t;
            writes.push((t, w.reg, w.value));
        }
        let irqs = amiga
            .irqs()
            .iter()
            .filter(|i| i.level == 3)
            .map(|i| cycle(i.vpos, i.hpos, i.next))
            .collect();
        let near = ranges
            .iter()
            .any(|r| r.contains(&number) || r.contains(&(number - 1)));
        let frame = Frame {
            number,
            draw_mem: if near { amiga.watched_memory() } else { None },
            writes,
            irqs,
            copper: bus,
            copper_writes,
            audio: slots,
            fetch,
            dmacon: dma,
            resumes,
        };
        for w in &frame.copper_writes {
            fetch.write(w.reg, w.value);
        }
        for &(_, reg, value) in &frame.writes {
            if reg == DMACON {
                if value & 0x8000 != 0 {
                    dma |= value & 0x7ff;
                } else {
                    dma &= !value;
                }
            }
        }
        window.push_back(frame);
        if window.len() > 3 {
            window.pop_front();
        }
        if window.len() == 3
            && ranges.iter().any(|r| r.contains(&window[0].number))
            && let Some(from_ack) = check_draw(window.make_contiguous())
        {
            checked.push((window[0].number, from_ack));
        }
    }
    checked
}

/// The draw starting in `frames[0]`, if one does: run and compared; whether it was run from the
/// handler's acknowledge before it (T54).
fn check_draw(frames: &[Frame]) -> Option<bool> {
    let a = &frames[0];
    let n = a.number;
    // The draw's first writes: DMACON $8400, BLTAFWM/BLTALWM, then BLTCON0 $09f0 (long writes).
    let at = a.writes.windows(5).position(|w| {
        w[0].1 == DMACON
            && w[0].2 == 0x8400
            && w[1].1 == BLTAFWM
            && w[3].1 == BLTCON0
            && w[3].2 == 0x09f0
    })?;
    assert!(
        !a.writes[..at]
            .iter()
            .any(|w| w.1 == DMACON && w.2 == 0x8400),
        "oracle frame {n}: the draw's DMACON $8400 is the frame's first (its memory is taken there)"
    );
    let start = a.writes[at].0;
    // The original's writes from there on, through all three frames, to the draw's DMACON $0400.
    let theirs_all: Vec<(u32, u16, u16)> = frames
        .iter()
        .enumerate()
        .flat_map(|(k, f)| {
            f.writes
                .iter()
                .map(move |&(t, r, v)| (t + k as u32 * sim::FRAME, r, v))
        })
        .skip(at)
        .collect();
    let end = theirs_all
        .iter()
        .position(|w| w.1 == DMACON && w.2 == 0x0400)
        .unwrap_or_else(|| panic!("oracle frame {n}: the draw ends within three frames"));
    let theirs_all = &theirs_all[..=end];
    // Interrupts taken during the draw and their acknowledges.
    let irqs: Vec<u32> = frames
        .iter()
        .enumerate()
        .flat_map(|(k, f)| f.irqs.iter().map(move |&t| t + k as u32 * sim::FRAME))
        .filter(|&t| t > start && t < theirs_all[end].0)
        .collect();
    let mut handlers = Vec::new();
    for &e in &irqs {
        let ack = theirs_all
            .iter()
            .find(|w| w.0 > e && w.1 == INTREQ && w.2 == 0x0020)
            .unwrap_or_else(|| panic!("oracle frame {n}: the handler from {e} acknowledges"))
            .0;
        handlers.push((e, ack));
    }
    let theirs: Vec<(u32, u16, u16)> = theirs_all
        .iter()
        .copied()
        .filter(|w| !handlers.iter().any(|&(e, ack)| w.0 >= e && w.0 <= ack))
        .collect();

    // The engine's program from the original's memory at the draw's DMACON write, the buffer
    // toggle its first instructions flipped put back (the buffer it stores is the same again).
    let mem = a.draw_mem.as_ref().expect("RAM at the draw's start");
    let mut m = StoryMem {
        chip: mem.chip.clone(),
        slow: mem.slow.clone(),
    };
    m.chip.resize(RAM_BYTES, 0);
    m.slow.resize(RAM_BYTES, 0);
    m.set_w(bobs::TOGGLE, !m.w(bobs::TOGGLE));
    let prog = bobs::draw(&mut m);
    let from = prog.find_reg(DMACON, 0x8400).unwrap();
    let fixed = frames.iter().fold(Fixed::new(), |fx, f| {
        fx.frame(&f.copper, &f.copper_writes, f.fetch)
            .audio(&f.audio)
    });
    // A draw the handler let go of runs from its acknowledge: the handler's tail, the wait's last
    // turn and the calls to it, as the intro times it (T54); one the main program came to by
    // itself from its DMACON write.
    let lead = a.resumes.and_then(intro::handler_to_draw).and_then(|lead| {
        let ack = a.writes[..at]
            .iter()
            .rev()
            .find(|w| w.1 == INTREQ && w.2 == 0x0020)?
            .0;
        Some((ack, lead))
    });
    let (t0, from) = match &lead {
        Some((ack, _)) => (ack + 1, 0),
        None => (start - 1, from),
    };
    let mut s = Sim::new(t0, &fixed, a.dmacon);
    if let Some((_, lead)) = &lead {
        lead.run(&mut s, 0, &mut |_| {});
    }
    let mut pending: VecDeque<(u32, u32)> = handlers.iter().copied().collect();
    let mut entries = Vec::new();
    // Where the intro takes a blank's interrupt in a draw: at the first instruction end that sees
    // its request, from line 0, cycle `IRQ_ENTRY` on (`Intro::time_draw`).
    let mut blank = (t0 / sim::FRAME + 1) * sim::FRAME + intro::IRQ_ENTRY;
    let mut taken = Vec::new();
    prog.run(&mut s, from, &mut |s| {
        if s.irq_seen(blank) {
            taken.push(s.t);
            blank += sim::FRAME;
        }
        if let Some(&(e, ack)) = pending.front()
            && s.t >= e
        {
            entries.push((s.t, e));
            pending.pop_front();
            s.t = ack + 1;
            s.run(vbl::TAIL);
        }
    });
    if let Some(i) = s.regs.iter().zip(&theirs).position(|(a, b)| a != b) {
        let lo = i.saturating_sub(4);
        panic!(
            "oracle frame {n}: write {i} of {}: ours {:x?}, theirs {:x?}",
            theirs.len(),
            &s.regs[lo..(i + 3).min(s.regs.len())],
            &theirs[lo..(i + 3).min(theirs.len())]
        );
    }
    for (ours, theirs) in &entries {
        assert_eq!(
            ours, theirs,
            "oracle frame {n}: the interrupt's instruction end"
        );
    }
    assert_eq!(
        taken, irqs,
        "oracle frame {n}: the handlers the draw holds off start where the original's did"
    );
    let blits = |w: &[(u32, u16, u16)]| -> Vec<(u32, u16)> {
        w.iter()
            .filter(|w| w.1 == BLTSIZE)
            .map(|w| (w.0, w.2))
            .collect()
    };
    assert_eq!(
        blits(&s.regs),
        blits(&theirs),
        "oracle frame {n}: BLTSIZE writes"
    );
    assert_eq!(s.regs, theirs, "oracle frame {n}: every register write");
    Some(lead.is_some())
}

/// Every draw of scenes 1 and 4, their sound effects' audio DMA among them (T96), and scene 3's
/// first by `draw_next`: 350, 10 and 52 (about a minute). Those the handler lets go of (all but the
/// 47 the main program comes to by itself, among them the two after scene 4's masks and the first
/// of scene 3's) run from its acknowledge (T54).
#[test]
fn draws_write_their_blits_on_the_originals_cycles() {
    let checked = check_draws(&[SCENE_1, SCENE_3, SCENE_4]);
    let count = |r: &Range<u32>| checked.iter().filter(|f| r.contains(&f.0)).count();
    assert_eq!(
        (count(&SCENE_1), count(&SCENE_3), count(&SCENE_4)),
        (350, 10, 52),
        "draws checked: {checked:?}"
    );
    let from_ack = checked.iter().filter(|f| f.1).count();
    assert_eq!(
        from_ack, 365,
        "draws run from the handler's end: {checked:?}"
    );
}

/// Scene 1's first sound effects: channel 0 from oracle frame 3 034, line 48.
const EFFECT: Range<u32> = 3_030..3_130;

/// The slots the audio channels' DMA takes (`AudioDma` on the original's Paula writes, every frame
/// from the sync point on) against the original's DMA profile, across scene 1's first effects: on
/// every line the profile samples, the same channels in the same slots. The profile's row sampled
/// at line *L*'s end is line *L − 1*'s (FINDINGS 2026-10-13, T53); a line the CPU is held through
/// by a blit is not sampled.
#[test]
fn audio_dma_takes_the_originals_slots_across_scene_1s_effect() {
    let case = Case::load(&sf_conform::cases_dir().join("story-intro.toml"))
        .unwrap_or_else(|e| panic!("{e}"));
    let synced = sync::reach(&case.sync, &case.script, EFFECT.end + 20).unwrap();
    let mut amiga = synced.amiga;
    let mut audio = AudioDma::new();
    let (mut of, mut compared) = (0, 0);
    while amiga.frame() + 1 < EFFECT.end {
        let number = amiga.frame() + 1;
        if number == EFFECT.start {
            amiga.set_dma_profile();
        }
        let cap = sync::step_inserting(&mut amiga, case.script.input_at(of)).unwrap();
        of += 1;
        let ours = audio.frame(&cap.paula, amiga.frame_lines());
        if !EFFECT.contains(&number) {
            continue;
        }
        let samples = amiga.line_samples();
        let lines: Vec<u32> = samples
            .iter()
            .filter(|s| s.vpos > 0)
            .map(|s| s.vpos as u32 - 1)
            .collect();
        let theirs: Vec<(u32, u8)> = samples
            .iter()
            .filter(|s| s.vpos > 0)
            .flat_map(|s| {
                (0..4u8).filter_map(move |n| {
                    let h = 0x0d + 2 * n as usize;
                    (4..=7)
                        .contains(&s.owners[h])
                        .then(|| ((s.vpos as u32 - 1) * LINE + h as u32, s.owners[h] - 4))
                })
            })
            .collect();
        let ours: Vec<(u32, u8)> = ours
            .iter()
            .copied()
            .filter(|s| lines.contains(&(s.0 / LINE)))
            .collect();
        assert_eq!(ours, theirs, "oracle frame {number}: audio slots");
        compared += theirs.len();
    }
    // Channel 0's effect plays through these frames: about a fetch every 3.4 lines it runs.
    assert!(compared > 1_000, "audio slots compared: {compared}");
}
