//! What a bug recording's comparison with the original says first, in the same words for every
//! game, because a game's bug-fix procedure (`docs/BUG-FIX.md`) reads its verdict by them: `same`,
//! `differs at frame F (engine step S): ...`, or `same up to frame F (engine step S), not judged
//! after it`. How a game retimes a recording onto the original and runs both sides stays the
//! game's.

/// Where the original's run left the recording's for good: nothing after it was judged on the
/// owner's game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Left {
    /// The original's frame.
    pub frame: u32,
    /// The recording's step it was fed.
    pub step: u32,
    pub what: String,
}

/// The verdict's line: `first` is the first difference as (the original's frame, the recording's
/// step, what differs), `left` where the original left the recording. A difference wins over a
/// leaving: it was judged on the original's path up to it.
pub fn verdict_line(first: Option<(u32, u32, &str)>, left: Option<&Left>) -> String {
    match (first, left) {
        (None, None) => "same".to_string(),
        (None, Some(l)) => format!(
            "same up to frame {} (engine step {}), not judged after it",
            l.frame, l.step
        ),
        (Some((f, s, what)), _) => format!("differs at frame {f} (engine step {s}): {what}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_verdicts() {
        let left = Left {
            frame: 900,
            step: 40,
            what: "x".into(),
        };
        assert_eq!(verdict_line(None, None), "same");
        assert_eq!(
            verdict_line(None, Some(&left)),
            "same up to frame 900 (engine step 40), not judged after it"
        );
        assert_eq!(
            verdict_line(Some((7, 3, "char_x")), Some(&left)),
            "differs at frame 7 (engine step 3): char_x"
        );
    }
}
