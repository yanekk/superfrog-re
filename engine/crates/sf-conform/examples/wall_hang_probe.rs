//! T105's probe: the original on a W2-1 script, each frame from `<from>`: the frog's x, y, its
//! wall flags (`+$0e` right, `+$10` left), facing (`+$12`) and the main program's PC, to see
//! its lives, whether it is spinning in `frog_tiles`'s wall test (h0+$e3fe..$e412) with both flags set.
//! `wall_hang_probe <script file> <frames> <from> [none|enemies] [level]` (T168: a level, e.g.
//! `W2-4`, synced at its start as `plan_exit` does, instead of `items-upper-w2-1`'s sync point).
use sf_conform::freeze::Freeze;
use sf_conform::{Case, InputScript, sync};
use sf_core::frog::{FROG, f};
use sf_core::layout::h0;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let script = InputScript::load(std::path::Path::new(&a[0])).unwrap();
    let frames: u32 = a[1].parse().unwrap();
    let from: u32 = a[2].parse().unwrap();
    let frozen = match a.get(3).map(String::as_str) {
        Some("none") => Freeze::None,
        _ => Freeze::Enemies,
    };
    let sp = match a.get(4) {
        Some(l) => sf_conform::SyncPoint::AtLevel(l.parse().unwrap()),
        None => {
            Case::load(&sf_conform::cases_dir().join("items-upper-w2-1.toml"))
                .unwrap()
                .sync
        }
    };
    let synced = sync::reach_frozen(&sp, &script, frames + 100, frozen).unwrap();
    let mut amiga = synced.amiga;
    let display = sf_conform::display_data();
    for fr in 0..frames {
        frozen.apply(&mut amiga, &display);
        sync::step_inserting(&mut amiga, script.input_at(fr)).unwrap();
        if fr < from {
            continue;
        }
        let w = |o: u32| {
            let b = amiga.peek(FROG + o, 2);
            u16::from_be_bytes([b[0], b[1]])
        };
        let cpu = amiga.cpu();
        let m = sync::core_memory(amiga.memory());
        let chain: Vec<String> = sync::main_chain(&cpu, &m)
            .iter()
            .map(|x| format!("{:x}", x.wrapping_sub(h0(0))))
            .collect();
        println!(
            "{fr}: clock {:x} x {} y {} wr {} wl {} face {} vx {} lives {} pc {:x} chain {}",
            u32::from_be_bytes(amiga.peek(h0(0x876), 4).try_into().unwrap()),
            w(f::X) as i16,
            w(f::Y) as i16,
            w(f::WALL_RIGHT),
            w(f::WALL_LEFT),
            w(f::FACING),
            w(f::VX) as i16,
            w(f::LIVES),
            cpu.pc.wrapping_sub(h0(0)),
            chain.join(" ")
        );
    }
}
