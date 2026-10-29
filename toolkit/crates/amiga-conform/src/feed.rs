//! How loud the original was: the swing of each frame of its own mix (vAmiga's), which a game
//! records one line a frame of play under a header of its own, so an engine's sound can be held to
//! the original's.

/// How far a frame of the original's mix swings: half the range its samples span on the wider
/// side, in 16-bit units (0 to 32767). A level held still is no sound however far from zero: a
/// stopped channel leaves its last sample on the DAC, which vAmiga's mix keeps (stops have been
/// seen sitting at 367 and 786 for frames).
pub fn swing(mix: &[[f32; 2]]) -> u16 {
    let side = |c: usize| {
        let (lo, hi) = mix
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), s| (lo.min(s[c]), hi.max(s[c])));
        if mix.is_empty() { 0.0 } else { (hi - lo) / 2.0 }
    };
    (side(0).max(side(1)).min(1.0) * 32767.0).round() as u16
}

/// An audio file's swings, frame of play 1 first. Its first line must be `header`, the game's.
pub fn parse_audio(text: &str, header: &str) -> Result<Vec<u16>, String> {
    let mut lines = text.lines();
    if lines.next() != Some(header) {
        return Err(format!("not an audio-peak file (no `{header}` line)"));
    }
    lines
        .enumerate()
        .map(|(n, l)| l.parse().map_err(|_| format!("line {}: {l:?}", n + 2)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_swings_round_trip_and_a_held_level_is_none() {
        assert_eq!(swing(&[[0.0, -0.5], [0.25, 0.0]]), 8192);
        assert_eq!(swing(&[[0.0, -0.5], [0.0, 0.5]]), 16384);
        // A stopped channel's last sample, held: no sound.
        assert_eq!(swing(&[[0.02, 0.0]; 960]), 0);
        assert_eq!(swing(&[[-1.5, 0.0], [1.5, 0.0]]), 32767);
        assert_eq!(swing(&[]), 0);
        const HEADER: &str = "a game's audio swing 1";
        let text = format!("{HEADER}\n0\n12\n32767\n");
        assert_eq!(parse_audio(&text, HEADER).unwrap(), vec![0, 12, 32767]);
        assert!(parse_audio("0\n", HEADER).is_err());
        assert!(parse_audio(&format!("{HEADER}\n-1\n"), HEADER).is_err());
    }
}
