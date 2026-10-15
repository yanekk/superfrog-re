//! Scratch (T129): `slot_draw_lamps` from its DMACON `$8400` on line 299 run on `story::sim`'s bus
//! (refresh, the audio slots `AudioDma` gives on the original's writes, the next frame's copper as
//! the original's DMA owners show it), next to the original's BLTSIZE writes.
//! `slot_lamps_sim <case.toml> <from> <to>` (original frames). The lamp program is the one
//! `sf_core::slot::draw` runs; `IRQ=<cycle>` moves where the interrupt is seen (default 1),
//! `LINEAUDIO=1` drops the writes' cycles, `FIXSTART=1` starts at 299.88 (what normal play has).
use sf_conform::{Case, sync};
use sf_core::front::story::cpu::{self, Bus};
use sf_core::front::story::sim::{Prog, Sim};
use sf_paula::AudioDma;

const LINE: u32 = 227;

struct B {
    lines: u32,
    planes: Vec<u32>,
    audio: Vec<u32>,
    copper: Vec<u32>,
}
impl Bus for B {
    fn taken(&self, t: u32) -> bool {
        cpu::refresh(t)
            || self.planes.binary_search(&t).is_ok()
            || self.audio.binary_search(&t).is_ok()
            || (t >= self.lines * LINE
                && self.copper.binary_search(&(t - self.lines * LINE)).is_ok())
    }
}

/// The lamps' program from DMACON `$8400`'s write: `on[k]` lamp k lit, sizes from the blits.
fn prog(on: &[bool], sizes: &[u16]) -> Prog {
    let mut p = Prog::new();
    p.ins("p"); // the DMACON write's last prefetch
    p.ins("ppr^pw"); // clr.w BLTCON1
    p.ins("ppppwwp"); // move.l #-1, BLTAFWM
    p.ins("ppp");
    p.ins("ppp");
    p.ins("ppp"); // move.l #$1a, d7
    for (k, &lit) in on.iter().enumerate() {
        p.ins("pp");
        p.ins("prp");
        if lit {
            p.ins("npp");
        } else {
            p.ins("nnp");
            p.ins("pp");
        }
        let con0: u16 = if lit { 0xd0c } else { 0xdfc };
        p.ins_regs("ppWp", &[(0x40, con0)]);
        p.ins("ppp");
        p.ins("ppp");
        p.ins("pn");
        p.ins("rp");
        p.ins("pnnnn");
        p.ins("pnn");
        p.ins("pnn");
        p.ins("prp");
        p.ins("ppnnnnnnnnnnnnnnnnnnn");
        p.ins("p");
        p.ins("pnn");
        p.ins("pnn");
        p.ins("pn");
        p.ins("pn");
        p.ins("prp");
        p.ins("pnnnnn");
        p.ins("prp");
        p.ins("pnnnnnnn");
        p.ins("p");
        p.ins("p");
        p.ins("pp");
        p.ins("p");
        for _ in 0..3 {
            p.ins("ppwp");
        }
        for _ in 0..3 {
            p.ins("ppw^wp");
        }
        let size = sizes.get(k).copied().unwrap_or(0x803);
        p.ins_regs("ppWp", &[(0x58, size)]);
        p.ins("pwwpp");
        p.wait_blit(false);
        p.ins("rrpp");
        p.ins("pppnn");
        p.ins("pppnn");
        p.ins("p");
        if k + 1 < on.len() {
            p.ins("npp");
        } else {
            p.ins("nnpp");
        }
    }
    p
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&a[0])).unwrap();
    let from: u32 = a[1].parse().unwrap();
    let to: u32 = a[2].parse().unwrap();
    let s = sync::reach(&case.sync, &case.script, to + 100).unwrap();
    let mut am = s.amiga;
    let mut audio = AudioDma::booted();
    let lamps = sf_core::layout::h0(0x6842);
    // (frame, its lines, DMACON write cycle, lamps on, audio slots of the frame, lamp blit sizes)
    let mut pending: Option<(u32, u32, u32, Vec<bool>, Vec<u32>, Vec<(u16, u16, u16)>)> = None;
    let mut planes_of: Vec<u32> = Vec::new();
    let mut next_start: Vec<sf_oracle::LineSample> = Vec::new();
    let mut rows_seen = std::collections::BTreeMap::<u16, sf_oracle::LineSample>::new();
    for f in 0..=to {
        am.set_dma_profile();
        if f + 1 < from {
            am.set_profile(false);
        }
        let mem_before: Vec<bool> = (0..26)
            .map(|k| am.peek(lamps + 8 * k, 2) != [0, 0])
            .collect();
        let cap = sync::step_inserting(&mut am, case.script.input_at(f)).unwrap();
        let lines = am.frame_lines();
        let feed: Vec<sf_paula::PaulaWrite> = if std::env::var("LINEAUDIO").is_ok() {
            cap.paula
                .iter()
                .map(|w| sf_paula::PaulaWrite { hpos: 0, ..*w })
                .collect()
        } else {
            cap.paula.clone()
        };
        let slots: Vec<u32> = audio.frame(&feed, lines).iter().map(|s| s.0).collect();
        let starts = am.blit_starts();
        if let Some((pf, plines, w, on, pslots, pstarts)) = pending.take() {
            let mut ls_next = next_start.clone();
            ls_next.extend(
                am.line_samples()
                    .iter()
                    .take_while(|s| s.vpos < 296)
                    .cloned(),
            );
            for r in ls_next.iter() {
                if r.vpos >= 1 && r.vpos <= 4 {
                    rows_seen.insert(r.vpos, *r);
                }
            }
            let copper: Vec<u32> = rows_seen
                .values()
                .filter(|s| (1..=4).contains(&s.vpos))
                .flat_map(|s| {
                    let l = s.vpos as u32 - 1;
                    s.owners
                        .iter()
                        .enumerate()
                        .filter(|(_, o)| **o == 24)
                        .map(move |(h, _)| l * LINE + h as u32)
                        .collect::<Vec<_>>()
                })
                .collect();
            let mut aud = pslots.clone();
            aud.extend(slots.iter().map(|t| t + plines * LINE));
            let bus = B {
                lines: plines,
                planes: planes_of.clone(),
                audio: aud,
                copper,
            };
            let mut sizes: Vec<u16> = pstarts.iter().map(|b| b.0).collect();
            sizes.extend(starts.iter().map(|b| b.0));
            let p = prog(&on, &sizes);
            let w = if std::env::var("FIXSTART").is_ok() {
                299 * LINE + 88
            } else {
                w
            };
            let mut sim = Sim::new(w + 1, &bus, 0x0240);
            sim.poke(w, 0x96, 0x8400);
            let irq = plines * LINE + std::env::var("IRQ").map_or(1, |v| v.parse().unwrap());
            let mut taken = None;
            let mut take = |s: &mut Sim| {
                if taken.is_none() && s.irq_seen(irq) {
                    taken = Some(s.t);
                }
            };
            p.run(&mut sim, 0, &mut take);
            let cut = taken.unwrap_or(u32::MAX);
            let ours: Vec<String> = sim
                .regs
                .iter()
                .filter(|r| r.1 == 0x58 && r.0 < cut)
                .map(|r| {
                    let t = r.0;
                    if t >= plines * LINE {
                        format!(
                            "{}.{}",
                            (t - plines * LINE) / LINE,
                            (t - plines * LINE) % LINE
                        )
                    } else {
                        format!("{}.{}", t / LINE, t % LINE)
                    }
                })
                .collect();
            let tail = pstarts
                .iter()
                .position(|b| b.1 >= 299)
                .unwrap_or(pstarts.len());
            let theirs: Vec<String> = pstarts[tail..]
                .iter()
                .filter(|b| b.1 >= 299 || b.1 < 12)
                .map(|b| format!("{}.{}", b.1, b.2))
                .collect();
            if std::env::var("EV").is_ok() {
                let fmt = |t: u32| {
                    if t >= plines * LINE {
                        format!(
                            "+{}.{}",
                            (t - plines * LINE) / LINE,
                            (t - plines * LINE) % LINE
                        )
                    } else {
                        format!("{}.{}", t / LINE, t % LINE)
                    }
                };
                let ev: Vec<String> = sim
                    .events
                    .iter()
                    .filter(|e| e.t < cut)
                    .map(|e| format!("{:?}@{}", e.kind, fmt(e.t)))
                    .collect();
                println!("   sim blit {}", ev.join(" "));
                let w: Vec<String> = sim
                    .writes
                    .iter()
                    .filter(|&&t| t + 600 > plines * LINE && t < cut + 50)
                    .map(|&t| fmt(t))
                    .collect();
                for s in ls_next.iter().filter(|s| s.vpos <= 2) {
                    let row: String = s
                        .owners
                        .iter()
                        .map(|&o| match o {
                            0 => '.',
                            1 => 'c',
                            2 => 'r',
                            4..=7 => (b'0' + o - 4) as char,
                            24 => 'P',
                            25 => 'B',
                            _ => '?',
                        })
                        .collect();
                    println!(
                        "   next row {} pc {:x} {}",
                        s.vpos,
                        s.pc.wrapping_sub(0xc00fa0),
                        &row[..80]
                    );
                }
                println!("   sim writes {}", w.join(" "));
            }
            let off = on
                .iter()
                .take(8)
                .map(|&o| if o { '1' } else { '0' })
                .collect::<String>();
            let ok = ours == theirs;
            println!(
                "{pf} lamps {off} {} irq@{:?}\n  sim   {}\n  orig  {}",
                if ok { "SAME" } else { "DIFF" },
                Some(cut).map(|t| t as i64 - (plines * LINE) as i64),
                ours.join(" "),
                theirs.join(" ")
            );
        }
        if f >= from && std::env::var("EV").is_ok() {
            let irq: Vec<String> = am
                .irqs()
                .iter()
                .map(|i| {
                    format!(
                        "L{}@{}{}.{}",
                        i.level,
                        if i.next { "+" } else { "" },
                        i.vpos,
                        i.hpos
                    )
                })
                .collect();
            let ev: Vec<String> = am
                .blit_events()
                .iter()
                .filter(|e| e.vpos >= 296 || e.vpos < 4)
                .map(|e| {
                    format!(
                        "{:?}@{}{}.{}",
                        e.kind,
                        if e.next { "+" } else { "" },
                        e.vpos,
                        e.hpos
                    )
                })
                .collect();
            println!(
                "   [{f}] irqs {}\n   [{f}] blit {}",
                irq.join(" "),
                ev.join(" ")
            );
            let cw: Vec<String> = am
                .cpu_writes()
                .iter()
                .filter(|w| w.vpos >= 310 || w.vpos < 3)
                .map(|w| format!("{:02x}@{}.{}", w.reg, w.vpos, w.hpos))
                .collect();
            println!("   [{f}] writes {}", cw.join(" "));
        }
        if f >= from {
            let w = am
                .cpu_writes()
                .iter()
                .find(|w| w.reg == 0x96 && w.value == 0x8400 && w.vpos >= 298)
                .map(|w| w.vpos as u32 * LINE + w.hpos as u32);
            let reels = starts.iter().any(|b| b.1 >= 298 && b.0 == 0x1002);
            if let (Some(w), false) = (w, reels) {
                let on: Vec<bool> = (0..26)
                    .map(|k| am.peek(lamps + 8 * k, 2) != [0, 0])
                    .collect();
                let _ = mem_before;
                planes_of = am
                    .line_samples()
                    .iter()
                    .filter(|s| (297..=301).contains(&s.vpos))
                    .flat_map(|s| {
                        let l = s.vpos as u32 - 1;
                        s.owners
                            .iter()
                            .enumerate()
                            .filter(|(_, o)| (8..=15).contains(*o))
                            .map(move |(h, _)| l * LINE + h as u32)
                            .collect::<Vec<_>>()
                    })
                    .collect();
                planes_of.sort();
                let ls = am.line_samples();
                let last_late = ls.iter().rposition(|s| s.vpos >= 296).unwrap_or(ls.len());
                next_start = ls[last_late..]
                    .iter()
                    .filter(|s| s.vpos < 296)
                    .cloned()
                    .collect();
                pending = Some((f, lines, w, on, slots, starts));
            }
        }
    }
}
