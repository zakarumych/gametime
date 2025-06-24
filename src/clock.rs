//! Contains definitions to work with arbitrary clocks
//! that handle time spans and time stamps
//! where actual passing time spans are provided externally.

use std::time::{Duration, Instant};

#[cfg(feature = "global_reference")]
use crate::ClockStep;
use crate::{stamp::TimeStamp, Frequency, FrequencyTicker};


#[cfg(feature = "std")]
/// Time measuring device.
/// Uses system monotonic clock counter
/// and yields `ClockStep`s for each step.
#[derive(Clone)] // Not Copy to avoid accidental copying.
pub struct Clock {
    /// Instant of the clock start.
    start: Instant,

    /// Time stamp corresponding to "now" measured from the start.
    now: TimeStamp,
}

impl Default for Clock {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Clock {
    /// Returns new `Clock` instance.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Clock {
            start: Instant::now(),
            now: TimeStamp::start(),
        }
    }

    /// Returns time stamp corresponding to "now" of the last step.
    #[must_use]
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
    #[must_use]
    pub fn stamp_instant(&self, stamp: TimeStamp) -> Instant {
        self.start + Duration::from_nanos(stamp.nanos_since_start())
    }

    /// Creates a new `FrequencyTicker` instance
    /// that will yield with given frequency starting from the current clock's time.
    #[must_use]
    pub fn ticker(&self, freq: Frequency) -> FrequencyTicker {
        FrequencyTicker::new(freq, self.now)
    }
}


/// Time measuring device.
/// Uses system monotonic clock counter
/// and yields `ClockStep`s for each step.
/// 
/// Unlike `Clock`, this clock uses global reference point instead of own start time.
#[cfg(feature = "global_reference")]
pub struct GlobalClock {
    /// Time stamp corresponding to "now" measured from global reference point.
    now: TimeStamp,
}

#[cfg(feature = "global_reference")]
impl GlobalClock {
    /// Returns new `GlobalClock` instance.
    /// 
    /// This clock uses global reference point and set's now to the current time.
    #[inline]
    pub fn new() -> Self {
        let now = TimeStamp::now();

        GlobalClock { now }
    }

    /// Returns time stamp corresponding to "now" of the last step.
    pub fn now(&self) -> TimeStamp {
        self.now
    }

    /// Advances the clock and returns `ClockStep` result
    /// with new time stamp and time span since previous step.
    pub fn step(&mut self) -> ClockStep {
        let now = TimeStamp::now();
        let step = now - self.now;

        ClockStep { now, step }
    }

    /// Returns `Instant` corresponding to given `TimeStamp`.
    pub fn stamp_instant(stamp: TimeStamp) -> Instant {
        let reference = crate::global_reference::reference();
        reference + Duration::from_nanos(stamp.nanos_since_start())
    }

    /// Creates a new `FrequencyTicker` instance
    /// that will yield with given frequency starting from the current clock's time.
    pub fn ticker(&self, freq: Frequency) -> FrequencyTicker {
        FrequencyTicker::new(freq, self.now)
    }
}
