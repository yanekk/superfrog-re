//! CIA-A as a program reads and writes it (translator DESIGN §2.8): the fire buttons on PRA, the
//! interrupt control register with its interrupt line, and the keyboard on the serial port with its
//! handshake. The timers and the time-of-day counter are not modelled: starting a timer or touching
//! its registers is a stop, never a guess.

use std::collections::VecDeque;

use amiga_input::{KeySet, key_bytes};

/// CIA registers by number (address bits 8 to 11).
pub const PRA: u16 = 0x0;
pub const PRB: u16 = 0x1;
pub const DDRA: u16 = 0x2;
pub const DDRB: u16 = 0x3;
pub const SDR: u16 = 0xC;
pub const ICR: u16 = 0xD;
pub const CRA: u16 = 0xE;
pub const CRB: u16 = 0xF;

/// The ICR's serial-port flag and mask bit.
pub const ICR_SP: u8 = 0x08;
/// CRA's SPMODE: 1 drives the serial line (output), which the keyboard reads as the handshake.
pub const CRA_SPMODE: u8 = 0x40;
/// CRA's and CRB's START bit.
const CR_START: u8 = 0x01;

/// CPU cycles from the end of a handshake, or from a key change on an idle keyboard, to the
/// keyboard's byte in SDR: 8 bits at about
/// 60 µs a bit (Hardware Reference Manual, keyboard chapter) at 7.09 MHz. From the manual, not
/// measured; the frame bar admits it moving inside the frame (DESIGN §2.7).
pub const KEY_BYTE_CYCLES: u64 = 3_405;

/// CIA-A: `$BFE001`, odd bytes.
#[derive(Clone, Debug)]
pub struct CiaA {
    /// Port 2's fire button held (PRA bit 7 reads 0).
    pub fire: bool,
    /// Port 1's button held, the left mouse button (PRA bit 6 reads 0).
    pub mouse_left: bool,
    pra: u8,
    ddra: u8,
    prb: u8,
    ddrb: u8,
    sdr: u8,
    icr_data: u8,
    icr_mask: u8,
    cra: u8,
    crb: u8,
    /// The time-of-day registers, unmodelled: the last byte written.
    tod: [u8; 4],
    /// The keys the keyboard last saw held.
    held: KeySet,
    /// Bytes the keyboard has yet to send, oldest first.
    queue: VecDeque<u8>,
    /// Whether the last byte put in SDR still waits for its handshake.
    awaiting_handshake: bool,
    /// The cycle count from which the keyboard may send its next byte.
    ready_at: u64,
    /// The interrupt line (/INT) is driven: some flag is set whose mask bit is set.
    line: bool,
    /// The line went from released to driven since the last [`CiaA::take_interrupt`].
    edge: bool,
    /// The first unsupported access: the register's number.
    fault: Option<u16>,
}

impl Default for CiaA {
    /// As a reset leaves the chip: every port an input, every interrupt masked, the timers
    /// stopped; no button held and no key down.
    fn default() -> CiaA {
        CiaA {
            fire: false,
            mouse_left: false,
            pra: 0,
            ddra: 0,
            prb: 0,
            ddrb: 0,
            sdr: 0,
            icr_data: 0,
            icr_mask: 0,
            cra: 0,
            crb: 0,
            tod: [0; 4],
            held: KeySet::NONE,
            queue: VecDeque::new(),
            awaiting_handshake: false,
            ready_at: 0,
            line: false,
            edge: false,
            fault: None,
        }
    }
}

impl CiaA {
    /// What the pins of port A read: fire buttons low when held; OVL, LED and the disk lines (all
    /// inactive high) high.
    fn pins_a(&self) -> u8 {
        let mut v = 0xFF;
        if self.fire {
            v &= !0x80;
        }
        if self.mouse_left {
            v &= !0x40;
        }
        v
    }

    /// The register `reg` read at cycle count `_at`. Reading ICR clears its flags and releases
    /// the line.
    pub fn read(&mut self, reg: u16, _at: u64) -> u8 {
        match reg {
            PRA => self.pra & self.ddra | self.pins_a() & !self.ddra,
            // The parallel port: nothing drives it, so its inputs read high.
            PRB => self.prb & self.ddrb | !self.ddrb,
            DDRA => self.ddra,
            DDRB => self.ddrb,
            SDR => self.sdr,
            ICR => {
                let v = self.icr_data | if self.line { 0x80 } else { 0 };
                self.icr_data = 0;
                self.line = false;
                v
            }
            // LOAD (bit 4) is a strobe and CRA's bit 7 unused: both read 0 (vAmiga `peek`).
            CRA => self.cra & !0x90,
            CRB => self.crb & !0x10,
            0x8..=0xB => self.tod[reg as usize - 8],
            _ => self.unsupported(reg),
        }
    }

    /// `v` written to `reg` at cycle count `at`.
    pub fn write(&mut self, reg: u16, v: u8, at: u64) {
        match reg {
            PRA => self.pra = v,
            PRB => self.prb = v,
            DDRA => self.ddra = v,
            DDRB => self.ddrb = v,
            SDR => self.sdr = v,
            ICR => {
                if v & 0x80 != 0 {
                    self.icr_mask |= v & 0x1F;
                } else {
                    self.icr_mask &= !v & 0x1F;
                }
                self.update_line();
            }
            CRA => {
                if v & CR_START != 0 {
                    self.unsupported(reg);
                }
                // The handshake: SPMODE set (the line driven low) and released. The keyboard then
                // sends its next byte, if it has one.
                if self.cra & CRA_SPMODE != 0 && v & CRA_SPMODE == 0 && self.awaiting_handshake {
                    self.awaiting_handshake = false;
                    self.ready_at = at + KEY_BYTE_CYCLES;
                }
                self.cra = v;
            }
            CRB => {
                if v & CR_START != 0 {
                    self.unsupported(reg);
                }
                self.crb = v;
            }
            0x8..=0xB => self.tod[reg as usize - 8] = v,
            _ => {
                self.unsupported(reg);
            }
        }
    }

    /// The keys the keyboard has already sent as held, no byte queued: a snapshot's, whose
    /// keyboard sent them before it was taken.
    pub fn set_held(&mut self, keys: KeySet) {
        self.held = keys;
    }

    /// The keys held this frame, at cycle count `at`: the keyboard queues a byte per change and,
    /// if it was idle, starts clocking the first out, in SDR [`KEY_BYTE_CYCLES`] later.
    pub fn set_keys(&mut self, keys: KeySet, at: u64) {
        // An idle keyboard takes a byte's 8 bits to send it, as after a handshake: vAmiga's
        // `wakeUp` starts `sendKeyCode` at the press, and the CIA loads SDR on the eighth CNT edge.
        // Landing it at once ran the original's level 2 handler 7 lines early, across the title's
        // `wait_line_1`, and the title lost a pass (title plan T07).
        let idle = self.queue.is_empty() && !self.awaiting_handshake;
        self.queue.extend(key_bytes(self.held, keys));
        if idle && !self.queue.is_empty() {
            self.ready_at = at + KEY_BYTE_CYCLES;
        }
        self.held = keys;
        self.tick(at);
    }

    /// The cycle count at which [`CiaA::tick`] would put the keyboard's next byte in SDR, if one
    /// waits and its handshake is done.
    pub fn next_at(&self) -> Option<u64> {
        (!self.awaiting_handshake && !self.queue.is_empty()).then_some(self.ready_at)
    }

    /// Time has reached `at`: the keyboard sends its next byte if the last was acknowledged and
    /// its time has come. The byte lands in SDR and sets the serial-port flag.
    pub fn tick(&mut self, at: u64) {
        if self.awaiting_handshake || at < self.ready_at {
            return;
        }
        if let Some(b) = self.queue.pop_front() {
            self.sdr = b;
            self.icr_data |= ICR_SP;
            self.awaiting_handshake = true;
            self.update_line();
        }
    }

    /// Drives the line if a flag is set whose mask bit is set. Only a read of ICR releases it:
    /// vAmiga's ICR write pulls the line and never lets it go, so masking a raised interrupt and
    /// unmasking it again (the program's `system_off`, `$7f` then `$88`) gives no second edge.
    fn update_line(&mut self) {
        if self.icr_data & self.icr_mask != 0 && !self.line {
            self.line = true;
            self.edge = true;
        }
    }

    fn unsupported(&mut self, reg: u16) -> u8 {
        self.fault.get_or_insert(reg);
        0
    }

    /// Whether the interrupt line was driven since the last call: Paula then sets INTREQ's PORTS
    /// bit (level 2), on the line's falling edge as vAmiga's `pullDownInterruptLine` does.
    pub fn take_interrupt(&mut self) -> bool {
        std::mem::take(&mut self.edge)
    }

    /// The interrupt line is driven now.
    pub fn line(&self) -> bool {
        self.line
    }

    /// The ICR's mask. A host starting from a snapshot sets it as the program left it; the
    /// oracle's snapshot holds no CIA state.
    pub fn icr_mask(&self) -> u8 {
        self.icr_mask
    }
    pub fn set_icr_mask(&mut self, mask: u8) {
        self.icr_mask = mask & 0x1F;
        self.update_line();
    }

    /// Bytes the keyboard has still to send.
    pub fn keys_queued(&self) -> usize {
        self.queue.len()
    }

    /// The first unsupported register touched (a timer), if any.
    pub fn fault(&self) -> Option<u16> {
        self.fault
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amiga_input::key_byte;

    #[test]
    fn the_fire_buttons_read_low_on_pra_inputs_only() {
        let mut c = CiaA::default();
        assert_eq!(c.read(PRA, 0), 0xFF);
        c.fire = true;
        assert_eq!(c.read(PRA, 0), 0x7F);
        c.mouse_left = true;
        assert_eq!(c.read(PRA, 0), 0x3F);
        // OVL and LED as outputs read the latch.
        c.write(DDRA, 0x03, 0);
        c.write(PRA, 0x02, 0);
        assert_eq!(c.read(PRA, 0), 0x3E);
        c.fire = false;
        assert_eq!(c.read(PRA, 0), 0xBE);
    }

    #[test]
    fn a_key_sets_sp_and_the_line_only_when_unmasked() {
        let mut c = CiaA::default();
        c.set_keys(KeySet::key(0x28), 100);
        c.tick(100 + KEY_BYTE_CYCLES);
        assert_eq!(c.read(SDR, 100), key_byte(0x28, false));
        assert!(!c.line() && !c.take_interrupt(), "masked");
        // Unmasking a set flag drives the line.
        c.write(ICR, 0x80 | ICR_SP, 110);
        assert!(c.line() && c.take_interrupt());
        assert!(!c.take_interrupt(), "one edge");
        assert_eq!(c.read(ICR, 120), 0x80 | ICR_SP);
        assert_eq!(c.read(ICR, 121), 0, "read clears");
        assert!(!c.line());
        c.write(ICR, ICR_SP, 130);
        assert_eq!(c.icr_mask(), 0);
    }

    #[test]
    fn masking_does_not_release_the_line_only_reading_icr_does() {
        // vAmiga `CIA::poke` ICR only ever pulls the line; `spypeek`/`peek` ICR releases it. The
        // program's `system_off` writes `$7f` then `$88` with a key possibly pending.
        let mut c = CiaA::default();
        c.set_icr_mask(ICR_SP);
        c.set_keys(KeySet::key(0x45), 0);
        c.tick(KEY_BYTE_CYCLES);
        assert!(c.take_interrupt());
        c.write(ICR, 0x7F, 10);
        assert!(c.line(), "a mask write leaves the line driven");
        c.write(ICR, 0x80 | ICR_SP, 20);
        assert!(!c.take_interrupt(), "no second edge");
        assert_eq!(c.read(ICR, 30), 0x80 | ICR_SP);
        assert!(!c.line());
    }

    #[test]
    fn the_next_byte_waits_for_the_handshake() {
        let mut c = CiaA::default();
        c.set_icr_mask(ICR_SP);
        // Two keys down: two bytes, the second queued.
        c.set_keys(KeySet::key(0x10).with(0x20), 0);
        c.tick(KEY_BYTE_CYCLES);
        assert_eq!(c.read(SDR, 0), key_byte(0x10, false));
        assert!(c.take_interrupt());
        assert_eq!(c.keys_queued(), 1);
        c.read(ICR, 10);
        c.tick(1_000_000);
        assert_eq!(c.read(SDR, 0), key_byte(0x10, false), "no handshake yet");
        // SPMODE set, then released at 2 000: the next byte KEY_BYTE_CYCLES later.
        c.write(CRA, CRA_SPMODE, 1_500);
        c.write(SDR, 0, 1_600);
        c.write(CRA, 0, 2_000);
        c.tick(2_000 + KEY_BYTE_CYCLES - 1);
        assert_eq!(c.read(SDR, 0), 0);
        assert!(!c.take_interrupt());
        c.tick(2_000 + KEY_BYTE_CYCLES);
        assert_eq!(c.read(SDR, 0), key_byte(0x20, false));
        assert!(c.take_interrupt());
        assert_eq!(c.keys_queued(), 0);
        // A handshake with nothing queued sends nothing; release then arrives after one.
        c.write(CRA, CRA_SPMODE, 9_000);
        c.write(CRA, 0, 9_100);
        c.tick(20_000);
        assert!(!c.take_interrupt());
        c.set_keys(KeySet::NONE, 30_000);
        assert_eq!(c.next_at(), Some(30_000 + KEY_BYTE_CYCLES));
        c.tick(30_000 + KEY_BYTE_CYCLES);
        assert_eq!(c.read(SDR, 0), key_byte(0x10, true));
        assert_eq!(c.keys_queued(), 1);
    }

    #[test]
    fn an_idle_keyboard_takes_a_bytes_time_to_send_its_first() {
        // vAmiga `Keyboard::wakeUp` starts sending at the press; SDR loads on the eighth CNT edge.
        let mut c = CiaA::default();
        c.set_icr_mask(ICR_SP);
        c.set_keys(KeySet::key(0x45), 5_000);
        assert_eq!(c.next_at(), Some(5_000 + KEY_BYTE_CYCLES));
        c.tick(5_000 + KEY_BYTE_CYCLES - 1);
        assert!(!c.take_interrupt() && c.read(SDR, 0) == 0, "still clocking");
        c.tick(5_000 + KEY_BYTE_CYCLES);
        assert!(c.take_interrupt());
        assert_eq!(c.read(SDR, 0), key_byte(0x45, false));
        // A key changing while a byte is on its way or awaits its handshake queues behind it.
        c.set_keys(KeySet::NONE, 6_000);
        assert_eq!(c.next_at(), None, "awaiting the handshake");
    }

    #[test]
    fn a_key_changing_while_a_byte_clocks_out_does_not_restart_it() {
        // vAmiga `wakeUp` starts sending only when no keyboard event is pending: a byte already
        // on its way keeps its time, the new one queues behind it.
        let mut c = CiaA::default();
        c.set_icr_mask(ICR_SP);
        c.set_keys(KeySet::key(0x45), 5_000);
        c.set_keys(KeySet::NONE, 6_000);
        assert_eq!(c.next_at(), Some(5_000 + KEY_BYTE_CYCLES));
        c.tick(5_000 + KEY_BYTE_CYCLES);
        assert_eq!(c.read(SDR, 0), key_byte(0x45, false));
        assert_eq!(c.keys_queued(), 1);
    }

    #[test]
    fn cra_and_crb_read_their_strobe_bits_as_zero() {
        // vAmiga `CIA::peek`: CRA without bits 4 and 7, CRB without bit 4 (LOAD is a strobe).
        let mut c = CiaA::default();
        c.write(CRA, 0xD0, 0);
        c.write(CRB, 0x10, 0);
        assert_eq!((c.read(CRA, 0), c.read(CRB, 0)), (0x40, 0x00));
        assert_eq!(c.fault(), None);
    }

    #[test]
    fn the_timers_are_a_fault() {
        let mut c = CiaA::default();
        c.write(CRA, CRA_SPMODE, 0);
        assert_eq!(c.fault(), None);
        c.read(0x4, 0);
        assert_eq!(c.fault(), Some(0x4));
        let mut c = CiaA::default();
        c.write(CRB, 0x01, 0);
        assert_eq!(c.fault(), Some(CRB));
        let mut c = CiaA::default();
        c.write(0x7, 0x01, 0);
        c.write(CRA, 0x01, 0);
        assert_eq!(c.fault(), Some(0x7), "the first is kept");
    }
}
