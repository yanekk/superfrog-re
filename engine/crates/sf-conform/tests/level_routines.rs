//! The level's own vertical-blank and pass routines (`sf_core::level`, T14), apart from the
//! frog's, against the original's on the routine oracle (`sf_conform::routine`).

mod common;

use common::*;
use sf_conform::routine::{Gen, Setup};
use sf_core::layout::h0;
use sf_core::level::scroll;

#[test]
fn scroll_latch_matches_the_original() {
    let mut rig = rig();
    let mut g = Gen::new(31);
    for case in 0..CASES {
        let mut s = Setup::default();
        s.poke_l(h0(0x16664), 0x10 + g.below(0x1200) as u32);
        s.poke_l(h0(0x16660), 0x20 + g.below(0x480) as u32);
        s.poke_w(h0(0x1665e), g.pick(&[1, 2]));
        if g.one_in(3) {
            s.poke_w(scroll::LEVEL_COPPER + 0x35c, g.pick(&[0xffdf, 0xffd7, 0x30df]));
            s.poke_w(scroll::LEVEL_COPPER + 0x35e, 0xfffe);
        }
        rig.check("scroll_latch", case, 0x1700a, &s, scroll::scroll_latch);
    }
}
