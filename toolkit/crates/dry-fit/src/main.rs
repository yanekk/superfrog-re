//! Runs the dry fit's frame and prints what each chip did.

fn main() {
    let f = dry_fit::run();
    let p = &f.picture;
    println!(
        "blit: {} words written, the last at CPU cycle {}",
        f.blit_writes.len(),
        f.blit_writes.last().copied().unwrap_or(0)
    );
    println!("copper: {} register writes", f.writes.len());
    for w in &f.writes {
        println!(
            "  line {:#05x} h {:#04x}: {:#05x} = {:#06x}",
            w.line, w.h, w.reg, w.value
        );
    }
    println!(
        "picture: {}x{} at ({}, {}), {} split(s)",
        p.width,
        p.height,
        p.left,
        p.top,
        p.splits.len()
    );
    for y in dry_fit::BLIT_Y..dry_fit::BLIT_Y + dry_fit::PATTERN_ROWS.len() as u32 {
        let row: String = (dry_fit::BLIT_X - 4..dry_fit::BLIT_X + 20)
            .map(|x| match p.rgb12(x as u16, y as u16) {
                dry_fit::ABOVE => 'R',
                dry_fit::BELOW => 'G',
                _ => '.',
            })
            .collect();
        println!("  {row}");
    }
    println!(
        "clock: line {} cycle {} after the frame",
        f.clock.line, f.clock.into
    );
    let loudest = f
        .audio
        .iter()
        .map(|s| s[0].unsigned_abs())
        .max()
        .unwrap_or(0);
    println!(
        "paula: {} host samples, loudest {loudest}; {} channel clocks",
        f.audio.len(),
        f.tap.len()
    );
}
