//! The front end's pure parts (T20). Against the original: `sf-conform`'s `tests/front.rs` and the
//! `front-*` cases.

use sf_core::front::FrontData;
use sf_core::front::password::{self, Password, Progress};
use sf_core::layout::H0;

/// A hunk 0 with a level-code table at h0+$24dc: `codes` of six digits, each with a routine at
/// h0+$2700 + 22k setting `start_world` and `start_level` as the game's do.
fn data_with(codes: &[([u8; 6], u8, u8)]) -> FrontData {
    let mut h0 = vec![0u8; 0x3000];
    for (k, (digits, world, level)) in codes.iter().enumerate() {
        let e = 0x24dc + 10 * k;
        h0[e..e + 6].copy_from_slice(digits);
        let handler = 0x2700 + 22 * k as u32;
        h0[e + 6..e + 10].copy_from_slice(&(H0 + handler).to_be_bytes());
        let h = handler as usize;
        h0[h..h + 2].copy_from_slice(&[0x23, 0xfc]);
        h0[h + 2..h + 6].copy_from_slice(&(*world as u32).to_be_bytes());
        h0[h + 6..h + 10].copy_from_slice(&(H0 + 0xa).to_be_bytes());
        h0[h + 10..h + 12].copy_from_slice(&[0x23, 0xfc]);
        h0[h + 12..h + 16].copy_from_slice(&(*level as u32).to_be_bytes());
    }
    FrontData {
        h0,
        ..FrontData::default()
    }
}

#[test]
fn a_code_starts_where_its_routine_says_and_the_first_match_wins() {
    let d = data_with(&[(*b"111111", 1, 1), (*b"222222", 3, 4), (*b"222222", 6, 2)]);
    let t = password::table(&d);
    assert_eq!(t.len(), 3);
    assert_eq!(
        password::decode_password(&t, &Password(*b"222222")),
        Some(Progress { world: 3, level: 4 })
    );
    assert_eq!(password::decode_password(&t, &Password(*b"123456")), None);
    assert_eq!(
        password::password_for(&t, &Progress { world: 6, level: 2 }),
        Some(Password(*b"222222"))
    );
    assert_eq!(
        password::password_for(&t, &Progress { world: 5, level: 1 }),
        None
    );
}

#[test]
fn an_entry_whose_routine_is_not_the_two_moves_is_left_out() {
    let mut d = data_with(&[(*b"111111", 1, 1), (*b"333333", 2, 2)]);
    d.h0[0x2700 + 22] = 0x4e; // the second routine is not `move.l #imm,abs.l`
    let t = password::table(&d);
    assert_eq!(t.len(), 1);
    assert_eq!(password::decode_password(&t, &Password(*b"333333")), None);
}
