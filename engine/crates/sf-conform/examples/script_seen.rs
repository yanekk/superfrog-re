//! Plays a script on the original and prints each frame the frog's hearts drop and, for each
//! world-2 object routine watched, the first frame a live slot runs it, and the first a crumbling
//! ledge shows broken: what a completion meets even while its case is red (T122, T181).
//!
//! `cargo run --release --example script_seen -- <level> <script.txt> <frames>`
use sf_conform::{InputScript, SyncPoint, sync};
use sf_core::frog::{FROG, f};
use sf_core::layout::h0;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let id: sf_core::LevelId = a[0].parse().unwrap();
    let p = InputScript::load(std::path::Path::new(&a[1])).unwrap();
    let n: u32 = a[2].parse().unwrap();
    let mut s = sync::reach(&SyncPoint::AtLevel(id), &InputScript::idle(), n + 100).unwrap();
    let watch = [
        (0x2619c, "mace"),
        (0x262be, "mace link"),
        (0x26692, "spikes"),
        (0x257e4, "swooper"),
        (0x23b34, "killed fall"),
        (0x23bee, "stunned"),
        (0x23e6c, "reward arc"),
        (0x2740c, "crumbling ledge"),
        (0x25990, "flyer"),
        (0x256ea, "walker"),
        (0x25d30, "hopper"),
        (0x25fa8, "dart"),
        (0x26786, "bouncer"),
    ];
    let mut first = vec![None; watch.len()];
    let mut hearts = None;
    // A crumbling ledge showing its broken frame (h0+$2ebac): it broke under the frog (T181).
    let mut broken = None;
    for k in 0..n {
        sync::step_inserting(&mut s.amiga, p.input_at(k)).unwrap();
        let st = sf_core::State::transplant(&sync::core_memory(s.amiga.memory()));
        for (i, (u, _)) in watch.iter().enumerate() {
            if first[i].is_none()
                && (0..20).any(|j| {
                    let sl = h0(0x1ba6c + j * 0x7c);
                    st.l(sl + 0x40) != 0 && st.l(sl + 4) == h0(*u)
                })
            {
                first[i] = Some(k);
            }
        }
        if broken.is_none()
            && (0..20).any(|j| {
                let sl = h0(0x1ba6c + j * 0x7c);
                st.l(sl + 0x40) != 0
                    && st.l(sl + 4) == h0(0x2740c)
                    && st.l(st.l(st.l(sl + 0x50) + 0x16)) == h0(0x2ebac)
            })
        {
            broken = Some(k);
        }
        let h = st.w(h0(0x830));
        if hearts.is_some_and(|x| h < x) {
            println!("frame {k}: hearts {h}, lives {}", st.w(FROG + f::LIVES));
        }
        hearts = Some(h);
    }
    for (i, (_, w)) in watch.iter().enumerate() {
        println!("{w}: {:?}", first[i]);
    }
    println!("crumbling ledge broken: {broken:?}");
}
