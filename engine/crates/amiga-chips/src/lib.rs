//! Pure: the Amiga's custom chips as a game engine uses them, naming no game. The blitter's
//! memory effect ([`Blit`]), the copper run for one frame ([`copper`]), the A500's RAM
//! ([`AmigaMemory`]) and which custom registers are Paula's ([`is_paula`]).

pub mod blit;
pub mod copper;
pub mod memory;

pub use blit::{Blit, BlitMem, FIXED_TO, bltsize_words, logic, word_times};
pub use memory::{AmigaMemory, Ram, SLOW_BASE};

/// Whether a custom register is Paula's (DMACON, INTENA, INTREQ, ADKCON, the audio channels):
/// the writes the sound side takes from the custom-register stream.
pub fn is_paula(reg: u16) -> bool {
    matches!(reg, 0x96 | 0x9a | 0x9c | 0x9e) || (0xa0..0xe0).contains(&reg)
}

