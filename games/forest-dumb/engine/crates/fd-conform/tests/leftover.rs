//! A write a read of a write-only register made, compared by register only (translator T47, owner
//! 2026-11-12): `frames::LEFTOVER`, `leftover_in_order` (Paula's writes), `leftover_counted` (the
//! other custom writes). The lists are frame 5001's of power-on-level-1, `mt_end` leaving the title,
//! measured on the oracle; the engine's words are the translated `clr`'s prefetch.

use fd_conform::frames::{LEFTOVER, custom_difference, leftover_counted, leftover_in_order};

const L: u16 = LEFTOVER;

/// `mt_end`'s Paula writes on the original.
const ORIGINAL: [(u16, u16); 13] = [
    (0x0A6, 0x01AC),
    (0x0B6, 0),
    (0x0C6, 0),
    (0x0D6, 0),
    (0x0A8, 0x47E0),
    (0x0A8, 0),
    (0x0B8, 0x0005),
    (0x0B8, 0),
    (0x0C8, 0x5255),
    (0x0C8, 0),
    (0x0D8, 0x5245),
    (0x0D8, 0),
    (0x096, 0x000F),
];

/// The same on the translated program, each volume's first write marked.
fn engine() -> Vec<(u16, u16)> {
    let mut e = ORIGINAL.to_vec();
    for (i, w) in [(4, 0x4279), (6, 0x4279), (8, 0x4279), (10, 0x33FC)] {
        e[i] = (e[i].0 | L, w);
    }
    e
}

#[test]
fn a_marked_paula_write_takes_the_originals_value_at_its_place() {
    assert_eq!(leftover_in_order(&engine(), &ORIGINAL), ORIGINAL.to_vec());
}

#[test]
fn a_marked_paula_write_to_another_register_or_past_the_end_still_differs() {
    // Out of step by one: the original's write at that place is to another register.
    let mut theirs = ORIGINAL.to_vec();
    theirs.insert(0, (0x09C, 0x4010));
    let settled = leftover_in_order(&engine(), &theirs);
    assert_eq!(settled[4], (0x0A8, 0x4279), "unmarked, its own value");
    assert_ne!(settled, theirs);
    let short = &ORIGINAL[..4];
    assert_eq!(leftover_in_order(&engine(), short)[4], (0x0A8, 0x4279));
}

#[test]
fn an_unmarked_write_is_compared_by_value() {
    let mut ours = ORIGINAL.to_vec();
    ours[4].1 = 0x4279;
    assert_eq!(leftover_in_order(&ours, &ORIGINAL), ours);
}

#[test]
fn a_marked_custom_write_matches_an_original_write_to_its_register_that_ours_lack() {
    // COPJMP1 strobed by `clr.w $dff088` on both sides: $5055 then 0 on the original.
    let theirs = [(0x088, 0x5055), (0x088, 0), (0x080, 1)];
    let ours = [(0x088 | L, 0x4EB9), (0x088, 0), (0x080, 1)];
    let settled = leftover_counted(&ours, &theirs);
    assert_eq!(settled, vec![(0x088, 0x5055), (0x088, 0), (0x080, 1)]);
    assert_eq!(custom_difference(&settled, &theirs), None);
}

#[test]
fn a_marked_custom_write_with_nothing_to_match_is_counted_as_written() {
    // The original made no such write: ours stays, unmarked, and differs.
    let theirs = [(0x088, 0)];
    let ours = [(0x088 | L, 0x4EB9), (0x088, 0)];
    let settled = leftover_counted(&ours, &theirs);
    assert_eq!(settled, vec![(0x088, 0x4EB9), (0x088, 0)]);
    let d = custom_difference(&settled, &theirs).expect("differs");
    assert_eq!(
        (d.reg, d.value, d.original, d.engine),
        (0x088, 0x4EB9, 0, 1)
    );
    // An original write ours already made unmarked is not taken twice.
    let theirs = [(0x088, 0), (0x088, 0)];
    let ours = [(0x088 | L, 0x4EB9), (0x088, 0), (0x088, 0)];
    let settled = leftover_counted(&ours, &theirs);
    assert_eq!(settled[0], (0x088, 0x4EB9));
}
