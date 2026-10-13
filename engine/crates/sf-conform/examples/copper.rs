//! Scratch (T36): the level copper list at a level's start.
use sf_conform::{SyncPoint, sync, InputScript};
use sf_core::layout::Ram;
fn main() {
    let id = "W1-1".parse().unwrap();
    let s = sync::reach(&SyncPoint::AtLevel(id), &InputScript::idle(), 1).unwrap();
    let m = s.memory;
    let base = sf_core::layout::h2(0x6bd0);
    let mut a = base;
    while a < base + 0x420 {
        let w1 = (m.byte(a) as u16) << 8 | m.byte(a + 1) as u16;
        let w2 = (m.byte(a + 2) as u16) << 8 | m.byte(a + 3) as u16;
        if w1 & 1 == 1 { println!("+{:03x} WAIT {:02x},{:02x} mask {:04x}", a - base, w1 >> 8, (w1 & 0xfe), w2); }
        else { println!("+{:03x} MOVE {:03x} = {:04x}", a - base, w1, w2); }
        a += 4;
    }
}
