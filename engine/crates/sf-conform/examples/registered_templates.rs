//! What the engine's dispatch registers (T204's template sweep, `tools/template_sweep.py`): one
//! line of the template indices with a kind, one of the `world:marker` spawners.
//!
//! `cargo run -q --example registered_templates`

use sf_core::items::template;
use sf_core::level::objects::TEMPLATES;
use sf_core::level::standard_dispatch;

fn main() {
    let d = standard_dispatch();
    let kinds: Vec<String> = (0..TEMPLATES as u16)
        .filter(|&t| d.kind(t).is_some() || d.routine(init_of(t)).is_some())
        .map(|t| t.to_string())
        .collect();
    println!("templates {}", kinds.join(" "));
    let mut markers: Vec<(u8, u8)> = d.markers().collect();
    markers.sort();
    let markers: Vec<String> = markers.iter().map(|(w, m)| format!("{w}:{m}")).collect();
    println!("markers {}", markers.join(" "));
}

/// The template's init routine as the original's executable has it (relocated).
fn init_of(t: u16) -> u32 {
    let display = sf_conform::display_data();
    let at = (template(t as u32) - sf_core::layout::h0(0)) as usize;
    let h0 = &display.rom().h0;
    u32::from_be_bytes(h0[at..at + 4].try_into().unwrap())
}
