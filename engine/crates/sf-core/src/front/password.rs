//! Level codes (docs/13 "Passwords"): `password_table` (h0+$24dc) and the entry on the options
//! screen (`password_entry` h0+$22ba).

use super::{Cx, Yield};

/// The level-code entry: not yet ported.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct Entry {
    pc: u8,
}

impl Entry {
    pub(super) fn run(&mut self, cx: &mut Cx) -> Yield {
        let _ = self.pc;
        cx.unported("password_entry");
        Yield::Return
    }
}
