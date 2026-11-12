//! An engine that loads at once against the original, whose loads take frames: the original is run
//! on past each load the engine did at once and compared once its load is over. The frames it
//! spends loading are the disk's time, not the game's, so they are not compared; a load that never
//! ends within a game's cap is a divergence, not a wait.
//!
//! Two ways to know a load is over, both read off the original by the game:
//!
//! - [`OriginalLoad`] with a [`LoadEnd`]: the original's call chain (the return addresses on its
//!   main program's stack, innermost first) has reached an address, or has been inside a call and
//!   left it. The game names the address per load.
//! - [`ByReturns`]: the game counts returns from its file routines on both sides; the original runs
//!   on while it has returned from fewer loads than the engine, and a frame is compared only when
//!   neither side is inside a load.
//!
//! The game keeps what is its own: which routines load, where its chain is read, what it does
//! with the frames skipped (a timed loading screen, the sound they played).

use std::fmt;

/// How the original's load is known to be over, from its call chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoadEnd {
    /// The chain holds this return address among its [`REACHED_DEPTH`] innermost (a call the
    /// program makes after the load).
    Reached(u32),
    /// The chain has held this return address (the load's call) and holds it no more.
    Left(u32),
}

/// How deep in the chain [`LoadEnd::Reached`] looks: the call made after the load is near the top.
pub const REACHED_DEPTH: usize = 4;

/// The original's load ran past the cap: it never ends where the engine's did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadTooLong {
    /// Frames the original ran inside the load, the cap among them.
    pub frames: u32,
    pub cap: u32,
}

impl fmt::Display for LoadTooLong {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the original's load was not over within {} frames of the engine's",
            self.cap
        )
    }
}

impl std::error::Error for LoadTooLong {}

/// One load the engine did at once, while the original runs it.
#[derive(Debug, Clone)]
pub struct OriginalLoad {
    end: Option<LoadEnd>,
    seen: bool,
    frames: u32,
    cap: u32,
}

impl OriginalLoad {
    /// A load that ends at `end`, of at most `cap` frames on the original.
    pub fn new(end: LoadEnd, cap: u32) -> OriginalLoad {
        OriginalLoad {
            end: Some(end),
            seen: false,
            frames: 0,
            cap,
        }
    }

    /// A load whose end the game tells by its own rule ([`ByReturns`]): only the cap is kept here.
    pub fn counted(cap: u32) -> OriginalLoad {
        OriginalLoad {
            end: None,
            seen: false,
            frames: 0,
            cap,
        }
    }

    /// Whether the load is over, the original's chain as it is now. Call it before each frame the
    /// original runs inside the load: a [`LoadEnd::Left`] load must be seen inside its call first.
    pub fn over(&mut self, chain: &[u32]) -> bool {
        match self.end {
            Some(LoadEnd::Reached(a)) => chain.iter().take(REACHED_DEPTH).any(|&x| x == a),
            Some(LoadEnd::Left(a)) => {
                let inside = chain.contains(&a);
                self.seen |= inside;
                self.seen && !inside
            }
            None => false,
        }
    }

    /// The original is to run one more frame inside the load: an error once that would reach the
    /// cap, which the frame then does not run.
    pub fn frame(&mut self) -> Result<(), LoadTooLong> {
        self.frames += 1;
        if self.frames >= self.cap {
            return Err(LoadTooLong {
                frames: self.frames,
                cap: self.cap,
            });
        }
        Ok(())
    }

    /// Frames the original has run inside the load.
    pub fn frames(&self) -> u32 {
        self.frames
    }
}

/// Loads counted by their returns on both sides, for a game whose file routines are watched (an
/// engine that runs the original's own program, so its loads are the same calls).
///
/// After each engine frame, call [`ByReturns::engine`]; then, while [`ByReturns::behind`], run
/// the original a frame and call [`ByReturns::original`] (an error past the cap). The pair is
/// compared when [`ByReturns::compared`]: both sides have returned from the same loads, and
/// neither is inside one at its frame's end, since a load's frames on the engine are not the
/// original's (the engine read the file at once, the original is still reading it).
#[derive(Debug, Clone)]
pub struct ByReturns {
    engine: u32,
    original: u32,
    engine_loading: bool,
    original_loading: bool,
    cap: u32,
    load: Option<OriginalLoad>,
    skipped: u32,
}

impl ByReturns {
    /// Each load at most `cap` frames on the original.
    pub fn new(cap: u32) -> ByReturns {
        ByReturns {
            engine: 0,
            original: 0,
            engine_loading: false,
            original_loading: false,
            cap,
            load: None,
            skipped: 0,
        }
    }

    /// The engine's frame: `returned` loads returned in it; `loading`, one runs at its end.
    pub fn engine(&mut self, returned: u32, loading: bool) {
        self.engine += returned;
        self.engine_loading = loading;
    }

    /// The original's frame, the one paired with the engine's or one more it ran while behind
    /// ([`ByReturns::skip`] counted those): `returned` loads returned in it; `loading`, one runs at
    /// its end.
    pub fn original(&mut self, returned: u32, loading: bool) {
        self.original += returned;
        self.original_loading = loading;
        if !self.behind() {
            self.load = None;
        }
    }

    /// Whether the original has returned from fewer loads than the engine: it runs on, its frames
    /// not compared.
    pub fn behind(&self) -> bool {
        self.original < self.engine
    }

    /// Count one frame the original runs because it is behind: an error once it reaches the cap.
    pub fn skip(&mut self) -> Result<(), LoadTooLong> {
        self.skipped += 1;
        self.load
            .get_or_insert_with(|| OriginalLoad::counted(self.cap))
            .frame()
    }

    /// Whether this pair of frames is compared: the same loads returned on both sides, and
    /// neither inside one.
    pub fn compared(&self) -> bool {
        self.original == self.engine && !self.engine_loading && !self.original_loading
    }

    /// Loads returned on each side: the engine's, the original's.
    pub fn returned(&self) -> (u32, u32) {
        (self.engine, self.original)
    }

    /// Frames the original ran because it was behind, over the run.
    pub fn skipped(&self) -> u32 {
        self.skipped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reached_looks_near_the_top_of_the_chain() {
        let mut l = OriginalLoad::new(LoadEnd::Reached(0x40), 10);
        assert!(!l.over(&[1, 2, 3, 4, 0x40]), "fifth innermost is too deep");
        assert!(l.over(&[1, 0x40]));
    }

    #[test]
    fn left_must_be_seen_inside_first() {
        let mut l = OriginalLoad::new(LoadEnd::Left(0x40), 10);
        assert!(!l.over(&[1]), "not yet in the load");
        assert!(!l.over(&[1, 0x40]), "inside");
        assert!(l.over(&[1]), "out again");
    }

    #[test]
    fn the_cap_counts_frames_run_inside_the_load() {
        let mut l = OriginalLoad::counted(3);
        assert_eq!(l.frame(), Ok(()));
        assert_eq!(l.frame(), Ok(()));
        let e = l.frame().unwrap_err();
        assert_eq!(e, LoadTooLong { frames: 3, cap: 3 });
        assert_eq!(
            e.to_string(),
            "the original's load was not over within 3 frames of the engine's"
        );
        assert!(!l.over(&[]), "a counted load is never over by the chain");
    }

    /// One side of a scripted run: per frame, the loads returned in it and whether one runs at its
    /// end.
    type Side = Vec<(u32, bool)>;

    /// Drive [`ByReturns`] as a runner does over `engine` and `original` frames; the original runs
    /// one frame per engine frame, and more while behind. Returns the pairs compared, (engine
    /// frame, original frame), and the frames skipped.
    fn drive(
        engine: &Side,
        original: &Side,
        cap: u32,
    ) -> Result<(Vec<(usize, usize)>, u32), LoadTooLong> {
        let mut a = ByReturns::new(cap);
        let mut o = 0usize;
        let mut pairs = Vec::new();
        for (e, &(r, l)) in engine.iter().enumerate() {
            let (or, ol) = original[o];
            a.original(or, ol);
            a.engine(r, l);
            while a.behind() {
                a.skip()?;
                o += 1;
                let (or, ol) = original[o];
                a.original(or, ol);
            }
            if a.compared() {
                pairs.push((e, o));
            }
            o += 1;
        }
        Ok((pairs, a.skipped()))
    }

    /// The engine loads in frame 2 at once; the original's load runs from its frame 2 for 51
    /// frames: the 50 extra are skipped and the frame after each side's return is paired.
    #[test]
    fn an_original_load_fifty_frames_longer_aligns_at_the_return() {
        let mut engine: Side = vec![(0, false); 6];
        engine[2] = (1, false);
        let mut original: Side = vec![(0, false); 60];
        for f in &mut original[2..52] {
            *f = (0, true);
        }
        original[52] = (1, false);
        let (pairs, skipped) = drive(&engine, &original, 100).unwrap();
        assert_eq!(skipped, 50);
        assert_eq!(pairs, [(0, 0), (1, 1), (2, 52), (3, 53), (4, 54), (5, 55)]);
    }

    /// Frames with a load running at their end on either side are not compared: the engine's next
    /// load has started (read at once) while the original reads it.
    #[test]
    fn a_frame_inside_a_load_on_either_side_is_not_compared() {
        // Engine: load 1 returns in frame 1 and load 2 starts, running at frames 1 and 2' ends,
        // returning in 3.
        let engine: Side = vec![(0, false), (1, true), (0, true), (1, false), (0, false)];
        let original: Side = vec![
            (0, false),
            (0, true),
            (0, true),
            (1, true),
            (0, true),
            (1, false),
            (0, false),
        ];
        let (pairs, skipped) = drive(&engine, &original, 100).unwrap();
        assert_eq!(skipped, 2, "the original's frames 2 and 4");
        assert_eq!(pairs, [(0, 0), (3, 5), (4, 6)]);
    }

    /// The original enters its load a frame before the engine does: that frame has the same loads
    /// returned on both sides, but the original is inside one, so it is not compared.
    #[test]
    fn a_frame_the_original_alone_is_loading_is_not_compared() {
        let engine: Side = vec![(0, false), (0, false), (1, false), (0, false)];
        let original: Side = vec![(0, false), (0, true), (0, true), (1, false), (0, false)];
        let (pairs, skipped) = drive(&engine, &original, 100).unwrap();
        assert_eq!(skipped, 1);
        assert_eq!(pairs, [(0, 0), (2, 3), (3, 4)], "the original's frame 1 left out");
    }

    #[test]
    fn an_original_load_past_the_cap_is_an_error() {
        let engine: Side = vec![(0, false), (1, false), (0, false)];
        let original: Side = vec![(0, true); 30];
        assert_eq!(
            drive(&engine, &original, 10),
            Err(LoadTooLong {
                frames: 10,
                cap: 10
            })
        );
    }
}
