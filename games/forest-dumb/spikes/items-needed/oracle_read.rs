//! Throwaway (map-page T00): each world's `items_left` on the original as `level_init` leaves it.
//! world-2-start's way in for every world, `world_index` poked as `load_world` is entered (N-1;
//! world 1 unpoked); reads `items_left` at the end of the step in which `load_world` returned
//! (h0+$232) and of the step `main_loop` first ran (h0+$252).
use amiga_oracle::{Amiga, OracleConfig, kickstart_path};
use fd_conform::{Case, HUNK_BASES, Inputs, disks};
use fd_core::layout_var;

fn main() {
    let h = |hunk: usize, off: u32| HUNK_BASES[hunk] + off;
    let text = std::fs::read_to_string(fd_conform::conform_dir().join("cases/world-2-start.toml")).unwrap();
    let case = Case::parse("w2", &text).unwrap();
    let inputs = Inputs::load(&case).unwrap();
    let wi = layout_var("world_index").unwrap();
    let il = layout_var("items_left").unwrap();
    let rd = |am: &Amiga| am.peek(h(il.hunk as usize, il.offset), 2);
    for world in 1..=6u16 {
        let cfg = OracleConfig::new(kickstart_path().unwrap(), disks::load().unwrap(), 7000);
        let mut am = Amiga::power_on(&cfg).unwrap();
        am.watch_pcs(&[h(0, 0x232), h(0, 0x252)]);
        let (mut after_load, mut at_loop) = (None, None);
        for frame in 0..7000 {
            if let Some(s) = case.swap.filter(|s| s.frame == frame) {
                am.insert_disk(0, s.disk as usize - 1);
            }
            // armed at the idle title: the boot runs h0+$578's address before the program is there
            if world > 1 && frame == case.sync_frame {
                am.poke_at(h(0, 0x578), h(wi.hunk as usize, wi.offset), &((world - 1) as u32).to_be_bytes()[4 - wi.width.bytes()..]);
            }
            am.step(inputs.at(frame)).unwrap();
            let hits: Vec<(u32, u32)> = am.pc_hits().to_vec();
            if frame > 4000 && hits.iter().any(|&(p, n)| p == h(0, 0x232) && n > 0) {
                after_load = Some((frame, rd(&am)));
            }
            if after_load.is_some() && hits.iter().any(|&(p, n)| p == h(0, 0x252) && n > 0) {
                at_loop = Some((frame, rd(&am)));
                break;
            }
        }
        let wv = am.peek(h(wi.hunk as usize, wi.offset), wi.width.bytes());
        println!("W{world}-1 world_index={wv:02x?} after_load={after_load:02x?} main_loop={at_loop:02x?}");
    }
}
