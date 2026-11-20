//! Level work the original's main program does across frames between two levels (the restart's
//! `level_init_wN`, T37): the engine does it at once, then replays its time and its writes on the
//! front end's beam.
//!
//! The ported routines say what they cost as they run ([`super::LevelCtx::spend`], the blits by
//! their words); with the context's trace kept, that gives each Paula write's place in CPU cycles
//! from the work's start ([`of`]). A [`Timeline`] spends those cycles on the lines the
//! main program has (all of a line's: the screen is blank meanwhile) and makes each write where it
//! falls.

use crate::front::{BEAM_CYCLES_PER_LINE, Cx, Yield};
use crate::game::Job;
use crate::level::beam::Tick;
use amiga_clock::{TimelineBeam, TimelineYield};

/// Work done at once, replayed on the beam: the cycles before each write and the write, then the
/// cycles after the last. Its memory is compared once the original is out of it (`job`). The
/// stepping is `amiga-clock`'s; building it from a trace, and the front end's beam, are here.
pub type Timeline = amiga_clock::Timeline<Job>;

/// The timeline of a context's `trace` (with `costs`) and its Paula `writes` (register, value).
pub fn of(trace: &[Tick], costs: &[u32], writes: &[(u16, u16)], job: Job) -> Timeline {
    let mut steps = Vec::new();
    let mut cycles = 0u32;
    let mut last = 0u32;
    for t in trace {
        match *t {
            Tick::Cpu(k, n) => cycles += n * costs[k as usize],
            Tick::Blit(slots) => cycles += 2 * slots,
            Tick::Wait(lines) => cycles += lines as u32 * BEAM_CYCLES_PER_LINE,
            Tick::Write(k) => {
                if let Some(&(reg, value)) = writes.get(k as usize) {
                    steps.push((cycles - last, reg, value));
                    last = cycles;
                }
            }
        }
    }
    Timeline::new(steps, cycles - last, job)
}

/// Runs `t` on the front end's beam.
pub(crate) fn run(t: &mut Timeline, cx: &mut Cx) -> Yield {
    match t.run(cx, crate::front::FRAME_LINES) {
        TimelineYield::Poll => Yield::Poll,
        TimelineYield::Return => Yield::Return,
    }
}

impl TimelineBeam<Job> for Cx<'_> {
    fn line(&self) -> u16 {
        self.line
    }

    fn set_line(&mut self, line: u16) {
        self.line = line;
    }

    fn write(&mut self, reg: u16, value: u16) {
        self.wr(reg, value);
    }

    fn job(&mut self, job: Job) {
        self.jobs.push(job);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::front::FRAME_LINES;
    use crate::level::beam::{Clock, Cost};

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    /// Two writes 1000 and 500 CPU cycles in, 300 after the last; an index past the writes is
    /// skipped.
    fn sample() -> Timeline {
        let mut costs = vec![0; Cost::COUNT];
        costs[Cost::VblEntry as usize] = 500;
        let trace = [
            Tick::Cpu(Cost::VblEntry, 2),
            Tick::Write(0),
            Tick::Cpu(Cost::VblEntry, 1),
            Tick::Write(1),
            Tick::Write(5),
            Tick::Blit(150),
        ];
        let job = Job {
            inside: vec![0x1234_5678],
            writes: vec![(0x100, 4)],
        };
        of(&trace, &costs, &[(0xa0, 1), (0x96, 0x8001)], job)
    }

    /// A beam that records where each write was made.
    #[derive(Debug, Default, PartialEq)]
    struct Beam {
        line: u16,
        writes: Vec<(u16, u16, u16)>,
        jobs: Vec<Job>,
    }

    impl Beam {
        fn at(line: u16) -> Beam {
            Beam {
                line,
                ..Beam::default()
            }
        }
    }

    impl TimelineBeam<Job> for Beam {
        fn line(&self) -> u16 {
            self.line
        }
        fn set_line(&mut self, line: u16) {
            self.line = line;
        }
        fn write(&mut self, reg: u16, value: u16) {
            self.writes.push((self.line, reg, value));
        }
        fn job(&mut self, job: Job) {
            self.jobs.push(job);
        }
    }

    /// `Timeline` as it was before it moved to `amiga-clock` (one-clock T02), its `run` on the
    /// front end's `Cx` put on [`Beam`]: the reference the moved stepping is held to.
    struct Old {
        steps: Vec<(u32, u16, u16)>,
        tail: u32,
        at: usize,
        left: u32,
        spare: u32,
        job: Option<Job>,
    }

    impl Old {
        fn of(trace: &[Tick], costs: &[u32], writes: &[(u16, u16)], job: Job) -> Old {
            let mut steps = Vec::new();
            let mut cycles = 0u32;
            let mut last = 0u32;
            for t in trace {
                match *t {
                    Tick::Cpu(k, n) => cycles += n * costs[k as usize],
                    Tick::Blit(slots) => cycles += 2 * slots,
                    Tick::Wait(lines) => cycles += lines as u32 * 454,
                    Tick::Write(k) => {
                        if let Some(&(reg, value)) = writes.get(k as usize) {
                            steps.push((cycles - last, reg, value));
                            last = cycles;
                        }
                    }
                }
            }
            let left = steps.first().map_or(cycles, |s| s.0);
            Old {
                steps,
                tail: cycles - last,
                at: 0,
                left,
                spare: 0,
                job: Some(job),
            }
        }

        fn run(&mut self, cx: &mut Beam) -> TimelineYield {
            if let Some(j) = self.job.take() {
                cx.jobs.push(j);
            }
            loop {
                while self.left > 0 {
                    if cx.line >= 313 - 1 {
                        cx.line = 313 - 1;
                        self.spare = 0;
                        return TimelineYield::Poll;
                    }
                    let free = 454 - self.spare;
                    if self.left < free {
                        self.spare += self.left;
                        self.left = 0;
                    } else {
                        self.left -= free;
                        self.spare = 0;
                        cx.line += 1;
                    }
                }
                let Some(&(_, reg, value)) = self.steps.get(self.at) else {
                    return TimelineYield::Return;
                };
                cx.writes.push((cx.line, reg, value));
                self.at += 1;
                self.left = self.steps.get(self.at).map_or(self.tail, |s| s.0);
                if self.left == 0 && self.at >= self.steps.len() {
                    return TimelineYield::Return;
                }
            }
        }
    }

    /// Traces shaped like a restart's (`level_init`'s CPU work, blits, waits and the sound
    /// driver's writes), from a seed.
    fn traces() -> Vec<(Vec<Tick>, Vec<(u16, u16)>)> {
        let mut out = Vec::new();
        let mut x = 0x2545_f491u32;
        let mut rnd = |n: u32| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x % n
        };
        for len in [0usize, 1, 2, 5, 20, 200, 2000] {
            let mut trace = Vec::new();
            let mut writes = Vec::new();
            for _ in 0..len {
                trace.push(match rnd(6) {
                    0 | 1 => Tick::Cpu(Cost::Init, 1 + rnd(4)),
                    2 => Tick::Blit(rnd(3000)),
                    3 => Tick::Wait(rnd(3) as u16),
                    _ => {
                        writes.push((0xa0 + rnd(0x40) as u16, rnd(0x10000) as u16));
                        // Now and then an index past the writes, which is skipped.
                        Tick::Write(writes.len() as u32 - 1 + (rnd(9) == 0) as u32 * 50)
                    }
                });
            }
            // Writes back to back, and one at the very end.
            if len > 2 {
                writes.push((0x96, 0x8001));
                trace.push(Tick::Write(writes.len() as u32 - 1));
                trace.push(Tick::Write(writes.len() as u32 - 1));
            }
            out.push((trace, writes));
        }
        out
    }

    #[test]
    fn the_moved_timeline_steps_as_it_did() {
        let costs = super::super::beam::RESTART_COSTS;
        let job = || Job {
            inside: vec![7],
            writes: vec![(0x100, 2)],
        };
        let mut runs = 0;
        for (trace, writes) in traces() {
            let old = Old::of(&trace, &costs, &writes, job());
            let cycles = of(&trace, &costs, &writes, job()).cycles();
            assert_eq!(cycles, old.steps.iter().map(|s| s.0).sum::<u32>() + old.tail);
            for start in [0u16, 1, 100, 300, 311, 312, 400] {
                let mut new = of(&trace, &costs, &writes, job());
                let mut old = Old::of(&trace, &costs, &writes, job());
                let (mut nb, mut ob) = (Beam::at(start), Beam::at(start));
                // Frame after frame, the beam back at line 0 after each vertical blank.
                for frame in 0..1000 {
                    let (yn, yo) = (new.run(&mut nb, FRAME_LINES), old.run(&mut ob));
                    assert_eq!((yn, &nb), (yo, &ob), "start {start} frame {frame}");
                    runs += 1;
                    if yn == TimelineYield::Return {
                        break;
                    }
                    nb.line = 0;
                    ob.line = 0;
                }
            }
        }
        assert!(runs > 100, "{runs}");
    }

    /// Save states hold a `Clock` (in `Pending`) and a `Timeline` (in a front-end routine): their
    /// bytes, captured before the clock moved to `amiga-clock` (one-clock T02), must not change.
    #[test]
    fn a_clock_and_a_timeline_keep_their_save_state_bytes() {
        let clock = Clock::at(0x123, 77);
        assert_eq!(hex(&bincode::serialize(&clock).unwrap()), "23014d000000");
        let fresh = sample();
        assert_eq!(
            hex(&bincode::serialize(&fresh).unwrap()),
            "0200000000000000e8030000a0000100f4010000960001802c0100000000000000000000e8030000000000000101000000000000007856341201000000000000000001000004000000"
        );
        // Mid-run (yielded at the frame's end, its job handed over: 546 cycles of the first
        // step left) and finished (both writes made, 438 cycles of the last line used), as run;
        // the bytes were captured from the states built field by field before the move.
        let mut polled = sample();
        let mut beam = Beam::at(FRAME_LINES - 2);
        assert_eq!(polled.run(&mut beam, FRAME_LINES), TimelineYield::Poll);
        assert_eq!(
            hex(&bincode::serialize(&polled).unwrap()),
            "0200000000000000e8030000a0000100f4010000960001802c0100000000000000000000220200000000000000"
        );
        let mut done = sample();
        assert_eq!(done.run(&mut Beam::at(10), FRAME_LINES), TimelineYield::Return);
        assert_eq!(
            hex(&bincode::serialize(&done).unwrap()),
            "0200000000000000e8030000a0000100f4010000960001802c010000020000000000000000000000b601000000"
        );
    }
}
