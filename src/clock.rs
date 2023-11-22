//! Contains definitions to work with arbitrary clocks
//! that handle time spans and time stamps
//! where actual passing time spans are provided externally.

use std::time::{Duration, Instant};

use crate::{span::TimeSpan, stamp::TimeStamp, Frequency, FrequencyTicker};

/// Result of `Clock` step.
/// Contains time stamp corresponding to "now"
/// and time span since previous step.
#[derive(Clone, Copy, Debug)]
pub struct ClockStep {
    /// TimeStamp corresponding to "now".
    pub now: TimeStamp,
    pub step: TimeSpan,
}

/// Time measuring device.
/// Uses system monotonic clock counter
/// and yields `ClockStep`s for each step.
#[derive(Clone)] // Not Copy to avoid accidental copying.
pub struct Clock {
    start: Instant,
    now: TimeStamp,
}

impl Default for Clock {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
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
        let now = TimeStamp::from_observed_duration(from_start);
        let step = now - self.now;
        self.now = now;

        ClockStep {
            now: self.now,
            step,
        }
    }

    /// Returns `Instant` corresponding to given `TimeStamp`.
    pub fn stamp_instant(&self, stamp: TimeStamp) -> Instant {
        self.start + Duration::from_nanos(stamp.nanos_since_start())
    }

    pub fn ticker(&self, freq: Frequency) -> FrequencyTicker {
        FrequencyTicker::new(freq, self.now)
    }
}
