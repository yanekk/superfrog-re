//! What the player does in one frame, as the original sees it (DESIGN §2.6): the joystick and the
//! keys held, and the text form scripts and recordings write them in. Both the engine and the
//! oracle take this type, so a script drives either one. The types live in the toolkit's
//! `amiga-input` (toolkit-extract DESIGN §2.2); this path keeps every caller and chapter working.

pub use amiga_input::*;
