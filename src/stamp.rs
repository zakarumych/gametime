//! Contains `TimeStamp` type, that represents fixed points in time,
//! traits and functions to work with it.

use core::{
    fmt,
    num::NonZeroU64,
    ops::{Add, AddAssign, Sub},
    time::Duration,
};

use crate::span::TimeSpan;

/// A fixed point in time relative to a reference point in time.
/// 
/// The reference point depends on how the time stamp is created:
/// - `Clock` return time stamp relative to the clock start.
/// - `TimeStamp::now` returns time stamp relative to the global reference point initialized by the first call to this function.
/// - Functions that create time stamp from another time stamp return time stamp relative to the reference point of the original time stamp.
/// - User decides what reference point is for time spans returned by other mechanisms.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TimeStamp {
    /// Number of nanoseconds elapsed from reference point in time.
    nanos: NonZeroU64,
}

impl fmt::Debug for TimeStamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let elapsed: TimeSpan = self.elapsed_since_start();
        fmt::Debug::fmt(&elapsed, f)?;
        f.write_str(" since start")
    }
}

impl fmt::Display for TimeStamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let elapsed: TimeSpan = self.elapsed_since_start();
        fmt::Display::fmt(&elapsed, f)?;
        f.write_str(" since start")
    }
}

impl TimeStamp {
    /// Constructs the smallest possible time stamp.
    #[inline]
    #[must_use]
    pub const fn start() -> Self {
        TimeStamp {
            nanos: unsafe { NonZeroU64::new_unchecked(1) },
        }
    }

    /// Constructs the largest possible time stamp.
    ///
    /// It is practically impossible to reach it without using artificially large time spans.
    #[inline]
    #[must_use]
    pub const fn never() -> Self {
        TimeStamp {
            nanos: unsafe { NonZeroU64::new_unchecked(u64::MAX) },
        }
    }

    /// Constructs time stamp from number of nanoseconds elapsed since reference point in time.
    #[inline]
    #[must_use]
    pub fn from_elapsed(nanos: u64) -> Option<Self> {
        let nanos = nanos.checked_add(1)?;
        Some(TimeStamp {
            nanos: unsafe { NonZeroU64::new_unchecked(nanos) },
        })
    }

    /// Constructs time stamp from number of nanoseconds elapsed since reference point in time.
    ///
    /// # Safety
    ///
    /// `nanos` must not be 0.
    #[inline]
    #[must_use]
    pub unsafe fn new_unchecked(nanos: u64) -> Self {
        TimeStamp {
            nanos: unsafe { NonZeroU64::new_unchecked(nanos) },
        }
    }

    /// Returns time stamp corresponding to "now".
    /// Uses global reference point in time initialized by first call to this function.
    /// 
    /// This function is only available if `global_reference` feature is enabled.
    #[cfg(feature = "global_reference")]
    #[inline]
    pub fn now() -> Self {
        match TimeStamp::from_duration(crate::global_reference::elapsed()) {
            Some(stamp) => stamp,
            None => impressive(),
        }
    }

    /// Constructs time stamp from duration since reference point in time.
    #[inline]
    #[must_use]
    pub fn from_duration(duration: Duration) -> Option<Self> {
        #![allow(clippy::cast_possible_truncation)] // Truncation is not possible due to check.

        let nanos = duration.as_nanos();
        if nanos > u128::from(u64::MAX - 1) {
            return None;
        }
        Some(TimeStamp {
            nanos: unsafe { NonZeroU64::new_unchecked(nanos as u64 + 1) },
        })
    }

    /// Constructs time stamp from duration observed by the process.
    ///
    /// Given that duration is measured by the process, it is impossible to overflow
    /// as it would mean that process runs for more than 500 years.
    ///
    /// # Panics
    ///
    /// Panics if overflow occurs.
    #[inline]
    #[must_use]
    pub fn from_observed_duration(duration: Duration) -> Self {
        match TimeStamp::from_duration(duration) {
            Some(stamp) => stamp,
            None => impressive(),
        }
    }

    /// Returns time span elapsed since `earlier` point in time.
    #[inline]
    #[must_use]
    pub const fn checked_elapsed_since(self, earlier: TimeStamp) -> Option<TimeSpan> {
        match self.nanos.get().checked_sub(earlier.nanos.get()) {
            None => None,
            Some(nanos) => Some(TimeSpan::new(nanos)),
        }
    }

    /// Returns time span elapsed since `earlier` point in time.
    /// 
    /// # Panics
    /// 
    /// Panics if `earlier` time stamp is greater than `self` time stamp.
    #[inline]
    #[must_use]
    pub fn elapsed_since(self, earlier: TimeStamp) -> TimeSpan {
        self.checked_elapsed_since(earlier)
            .expect("`earlier` time stamp is greater than `self` time stamp")
    }

    /// Returns time span elapsed since start point in time.
    #[inline]
    #[must_use]
    pub fn elapsed_since_start(self) -> TimeSpan {
        TimeSpan::new(self.nanos.get() - 1)
    }

    /// Returns time span elapsed since start point in time.
    #[inline]
    #[must_use]
    pub fn nanos_since_start(self) -> u64 {
        self.nanos.get() - 1
    }

    /// Returns checked time stamp after adding given time span.
    #[inline]
    #[must_use]
    pub fn checked_add(self, span: TimeSpan) -> Option<TimeStamp> {
        let nanos = self.nanos.get().checked_add(span.as_nanos())?;

        Some(TimeStamp {
            // Safety: a > 0, b >= 0 hence a + b > 0
            nanos: unsafe { NonZeroU64::new_unchecked(nanos) },
        })
    }
}

impl Add<TimeSpan> for TimeStamp {
    type Output = TimeStamp;

    #[inline]
    fn add(self, rhs: TimeSpan) -> Self {
        let nanos = self
            .nanos
            .get()
            .checked_add(rhs.as_nanos())
            .expect("overflow when adding time span to time stamp");

        TimeStamp {
            // Safety: a > 0, b >= 0 hence a + b > 0
            nanos: unsafe { NonZeroU64::new_unchecked(nanos) },
        }
    }
}

impl AddAssign<TimeSpan> for TimeStamp {
    #[inline]
    fn add_assign(&mut self, rhs: TimeSpan) {
        *self = *self + rhs;
    }
}

impl Sub<TimeStamp> for TimeStamp {
    type Output = TimeSpan;

    #[inline]
    fn sub(self, rhs: TimeStamp) -> TimeSpan {
        self.elapsed_since(rhs)
    }
}

#[cold]
#[inline]
fn impressive() -> ! {
    panic!(
        "Process runs for more than 500 years. Impressive. Upgrade to version with u128 value type"
    )
}
