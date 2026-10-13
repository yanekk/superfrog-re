//! When the original's CPU writes a copper list's colour words (T46, the fades' colour race):
//! `fade_probe <case.toml> <oracle frame> <list hex>` runs that oracle frame again and again from
//! a snapshot, each time with the list's wait before its colour moves set to another beam
//! position, and reads each colour back off the picture. It prints, per colour, every value the
//! copper found and from which wait position on (`line:h`, DMA cycles; `t` = line × 227 + h).
//! The colour moves follow the wait two DMA cycles a word, so colour `k`'s word is read about
//! `4 k` cycles after the wait position.
use sf_conform::{Case, sync};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let case = Case::load(std::path::Path::new(&args[1])).unwrap();
    let frame: u32 = args[2].parse().unwrap();
    let list = u32::from_str_radix(args[3].trim_start_matches("0x"), 16).unwrap();
    let last_line: u16 = args.get(4).map_or(64, |a| a.parse().unwrap());
    let synced = sync::reach(&case.sync, &case.script, frame + 20).unwrap();
    let mut amiga = synced.amiga;
    for f in 0..frame {
        sync::step_inserting(&mut amiga, case.script.input_at(f)).unwrap();
    }
    let input = case.script.input_at(frame);
    let snap = amiga.snapshot();
    let w = |a: &sf_oracle::Amiga, at: u32| {
        let b = a.peek(at, 2);
        u16::from_be_bytes([b[0], b[1]])
    };
    // The wait before the first colour move, and the colour moves after it.
    let mut at = list;
    let mut wait = None;
    while w(&amiga, at) != 0xffff {
        let (a, b) = (w(&amiga, at), w(&amiga, at + 2));
        if a & 1 == 1 {
            wait = Some(at);
        } else if (0x180..0x1c0).contains(&a) && wait.is_some() {
            break;
        }
        let _ = b;
        at += 4;
    }
    let wait = wait.expect("a wait before the colour moves");
    let mut moves = Vec::new();
    while (0x180..0x1c0).contains(&w(&amiga, at)) {
        moves.push(at + 2);
        at += 4;
    }
    let natural: Vec<u16> = moves.iter().map(|&m| w(&amiga, m)).collect();
    println!(
        "wait at {wait:06x} ({:04x} {:04x}), {} colour moves, before the frame {:03x?}",
        w(&amiga, wait),
        w(&amiga, wait + 2),
        moves.len(),
        natural
    );
    // Which palette index each pixel shows: the colour words set to their own index, read at the
    // frame's top.
    amiga.restore(&snap);
    for (k, &m) in moves.iter().enumerate() {
        amiga.poke(m, &(k as u16).to_be_bytes());
    }
    amiga.poke(wait, &[0x00, 0x01, 0xff, 0xfe]);
    let cap = sync::step_inserting(&mut amiga, input).unwrap();
    let pic = &cap.picture;
    let mut index = Vec::new();
    let mut first: Vec<Option<(u16, u16)>> = vec![None; moves.len()];
    for y in 0..pic.height {
        for x in 0..pic.width {
            let c = pic.pixel(x, y);
            let k = (c as usize) < moves.len();
            index.push(k.then_some(c as u8));
            if k && first[c as usize].is_none() {
                first[c as usize] = Some((x, y));
            }
        }
    }
    // The colours seen with the wait at `(line, h)`.
    let mut run = |line: u16, h: u16| -> Vec<Option<u16>> {
        amiga.restore(&snap);
        amiga.poke(wait, &[line as u8, (h as u8) | 1, 0xff, 0xfe]);
        let cap = sync::step_inserting(&mut amiga, input).unwrap();
        first
            .iter()
            .map(|p| p.map(|(x, y)| cap.picture.pixel(x, y)))
            .collect()
    };
    let mut seen: Vec<Vec<(u32, u16)>> = vec![Vec::new(); moves.len()];
    let mut note = |t: u32, cs: &[Option<u16>], seen: &mut Vec<Vec<(u32, u16)>>| {
        for (k, c) in cs.iter().enumerate() {
            if let Some(c) = *c
                && seen[k].last().is_none_or(|l| l.1 != c)
            {
                seen[k].push((t, c));
            }
        }
    };
    let mut prev = run(0, 0);
    note(0, &prev, &mut seen);
    for line in 1..=last_line {
        let now = run(line, 0);
        if now != prev {
            // Something changed during the line before: every position in it.
            for h in (2..0xe4).step_by(2) {
                let cs = run(line - 1, h);
                note((line as u32 - 1) * 227 + h as u32, &cs, &mut seen);
            }
        }
        note(line as u32 * 227, &now, &mut seen);
        prev = now;
    }
    let _ = index;
    for (k, s) in seen.iter().enumerate() {
        let parts: Vec<String> = s
            .iter()
            .map(|(t, c)| format!("{:03x}@{}:{:02x}(t{})", c, t / 227, t % 227, t))
            .collect();
        println!("colour {k:2}: {}", parts.join(" "));
    }
}
