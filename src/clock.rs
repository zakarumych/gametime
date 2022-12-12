//! Contains definitions to work with arbitrary clocks
//! that handle time spans and time stamps
//! where actual passing time spans are provided externally.

use std::time::Instant;

use crate::{span::TimeSpan, stamp::TimeStamp};

/// Time measuring device.
/// Uses system monotonic clock counter
/// and yields `ClockStep`s for each step.
pub struct Clock {
    start: Instant,
    now: TimeStamp,
}

/// Result of `Clock` step.
/// Contains time stamp corresponding to "now"
/// and time span since previous step.
pub struct ClockStep {
    pub now: TimeStamp,
    pub step: TimeSpan,
}

impl Clock {
    #[inline(always)]
    pub fn new() -> Self {
        Clock {
            start: Instant::now(),
            now: TimeStamp::start(),
        }
    }

    pub fn now(&self) -> TimeStamp {
        self.now
    }

    pub fn step(&mut self) -> ClockStep {
        let from_start = self.start.elapsed();
        let nanos = from_start.as_nanos();

        #[cold]
        #[inline(never)]
        fn impressive() -> ! {
            panic!("Process runs for more than 500 years. Impressive. Upgrade to version with u128 value type")
        }

        if nanos > (u64::MAX - 1) as u128 {
            impressive();
        }

        let nanos = nanos as u64 + 1;

        let step = TimeSpan::new(nanos);
        self.now += step;

        ClockStep {
            now: self.now,
            step,
        }
    }
}
