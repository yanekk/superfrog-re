//! T130's probe: when the original's fade writes each colour of the level's copper list, on one
//! frame of a case (objects frozen as the case freezes them). `fade_times <case.toml> <frame>
//! <first line> <last line>` runs the frame again and again from a snapshot with the wait before
//! the level's colour moves (from `h2+$6ea4`, `level_start`'s fade in's list) set to each slot of each line, and
//! reads each colour back off the picture. Prints, per colour, every value the copper found and the
//! first wait position (`t` = line × 227 + slot) it was found from; a colour `k` is read about
//! `4 k` slots after the wait.
use sf_conform::{Case, sync};
use sf_core::layout::h2;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let case = Case::load(std::path::Path::new(&args[0])).expect("case");
    let frame: u32 = args[1].parse().unwrap();
    let (first, last): (u16, u16) = (args[2].parse().unwrap(), args[3].parse().unwrap());
    let frozen = case.freeze();
    let synced = sync::reach_frozen(&case.sync, &case.script, frame + 10, frozen).expect("sync");
    let mut amiga = synced.amiga;
    let display = sf_conform::display_data();
    for f in 0..frame {
        frozen.apply(&mut amiga, &display);
        amiga.step(case.script.input_at(f)).expect("step");
    }
    let input = case.script.input_at(frame);
    let snap = amiga.snapshot();
    let w = |a: &sf_oracle::Amiga, at: u32| {
        let b = a.peek(at, 2);
        u16::from_be_bytes([b[0], b[1]])
    };
    let mut at = h2(0x6ea4);
    if std::env::var("DUMP").is_ok() {
        for a in (h2(0x6bd0)..at + 0x100).step_by(4) {
            println!(
                "h2+{:x}: {:04x} {:04x}",
                a - h2(0),
                w(&amiga, a),
                w(&amiga, a + 2)
            );
        }
    }
    // The copper's wait before the colour moves.
    let mut wait = at - 4;
    while w(&amiga, wait) & 1 == 0 {
        wait -= 4;
    }
    let mut moves = Vec::new();
    while (0x180..0x1c0).contains(&w(&amiga, at)) {
        moves.push(at + 2);
        at += 4;
    }
    println!(
        "wait at h2+{:x} ({:04x} {:04x}), {} colour moves",
        wait - h2(0),
        w(&amiga, wait),
        w(&amiga, wait + 2),
        moves.len()
    );
    // Where each colour shows: the colour words set to their own index, the wait at the top.
    let step = |a: &mut sf_oracle::Amiga| {
        frozen.apply(a, &display);
        a.step(input).expect("step")
    };
    amiga.restore(&snap);
    for (k, &m) in moves.iter().enumerate() {
        amiga.poke(m, &(k as u16).to_be_bytes());
    }
    amiga.poke(wait, &[0x00, 0x01, 0xff, 0xfe]);
    let cap = step(&mut amiga);
    let pic = &cap.picture;
    let mut where_: Vec<Option<(u16, u16)>> = vec![None; moves.len()];
    for y in 0..pic.height {
        for x in 0..pic.width {
            let c = pic.pixel(x, y) as usize;
            if c < moves.len() && where_[c].is_none() {
                where_[c] = Some((x, y));
            }
        }
    }
    let mut run = |line: u16, h: u16| -> Vec<Option<u16>> {
        amiga.restore(&snap);
        amiga.poke(wait, &[line as u8, (h as u8) | 1, 0xff, 0xfe]);
        let cap = step(&mut amiga);
        where_
            .iter()
            .map(|p| p.map(|(x, y)| cap.picture.pixel(x, y)))
            .collect()
    };
    let mut seen: Vec<Vec<(u32, u16)>> = vec![Vec::new(); moves.len()];
    let mut note = |t: u32, cs: &[Option<u16>]| {
        for (k, c) in cs.iter().enumerate() {
            if let Some(c) = *c
                && seen[k].last().is_none_or(|l| l.1 != c)
            {
                seen[k].push((t, c));
            }
        }
    };
    let mut prev = run(first, 0);
    note(first as u32 * 227, &prev);
    for line in first + 1..=last {
        let now = run(line, 0);
        if now != prev {
            // Something changed during the line before: every slot of it.
            for h in (2..0xe4).step_by(2) {
                let cs = run(line - 1, h);
                note((line as u32 - 1) * 227 + h as u32, &cs);
            }
        }
        note(line as u32 * 227, &now);
        prev = now;
    }
    for (k, s) in seen.iter().enumerate() {
        let parts: Vec<String> = s
            .iter()
            .map(|(t, c)| format!("{c:03x}@{}:{}(t{t})", t / 227, t % 227))
            .collect();
        println!("colour {k:2}: {}", parts.join(" "));
    }
}
