//! Contains definitions to work with arbitrary clocks
//! that handle time spans and time stamps
//! where actual passing time spans are provided externally.

use core::{num::NonZeroU64, time::Duration};
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
    /// TimeStamp corresponding to "now".
    pub now: TimeStamp,
    pub step: TimeSpan,
}

impl Clock {
    /// Returns new `Clock` instance.
    #[inline(always)]
    pub fn new() -> Self {
        Clock {
            start: Instant::now(),
            now: TimeStamp::start(),
        }
    }

    /// Returns time stamp corresponding to "now" of the last step.
    pub fn now(&self) -> TimeStamp {
        self.now
    }

    /// Advances the clock and returns `ClockStep` result
    /// with new time stamp and time span since previous step.
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

        // Safety:
        // `nanos` is guaranteed to be less than `u64::MAX`
        // Thus value is guaranteed to be in range 1..=u64::MAX.
        let nanos = unsafe { NonZeroU64::new(nanos as u64 + 1).unwrap_unchecked() };
        let now = TimeStamp::new(nanos);
        let step = now - self.now;
        self.now = now;

        ClockStep {
            now: self.now,
            step,
        }
    }

    /// Returns `Instant` corresponding to given `TimeStamp`.
    pub fn stamp_instant(&self, stamp: TimeStamp) -> Instant {
        self.start + Duration::from_nanos(stamp.elapsed_since(TimeStamp::start()).as_nanos() - 1)
    }
}
