//! Contains `TimeStamp` type, that represents fixed points in time,
//! traits and functions to work with it.

use core::{
    fmt,
    num::NonZeroU64,
    ops::{Add, AddAssign, Sub, SubAssign},
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
            nanos: unsafe { NonZeroU64::new_unchecked(i64::MAX as u64) },
        }
    }

    /// Constructs time stamp from number of nanoseconds elapsed since reference point in time.
    #[inline]
    #[must_use]
    pub fn from_elapsed(nanos: u64) -> Option<Self> {
        if nanos > (i64::MAX - 1) as u64 {
            return None; // Overflow
        }

        Some(TimeStamp {
            nanos: unsafe { NonZeroU64::new_unchecked(nanos + 1) },
        })
    }

    /// Constructs time stamp from number of nanoseconds elapsed since reference point in time.
    ///
    /// # Safety
    ///
    /// `nanos` must not be 0.
    #[inline]
    #[must_use]
    pub unsafe fn from_elapsed_unchecked(nanos: u64) -> Self {
        debug_assert!(nanos < (i64::MAX - 1) as u64, "Nanos overflow");

        TimeStamp {
            nanos: unsafe { NonZeroU64::new_unchecked(nanos + 1) },
        }
    }

    const fn nanos(self) -> i64 {
        #![allow(clippy::cast_possible_wrap)] // Wrap is not possible due constraints.

        debug_assert!(
            self.nanos.get() <= i64::MAX as u64,
            "TimeStamp must fit in i64"
        );

        // Safety: `nanos` is always greater than 0, so it cannot overflow.
        // The value is guaranteed to be less than `i64::MAX - 1` due to checks in constructors.
        (self.nanos.get() - 1) as i64
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
        if nanos > (i64::MAX - 1) as u128 {
            return None;
        }
        Some(unsafe { TimeStamp::from_elapsed_unchecked(nanos as u64) })
    }

    /// Constructs time stamp from duration observed by the process.
    ///
    /// Given that duration is measured by the process, it is practically impossible to overflow
    /// as it would mean that process runs for more than 200 years.
    ///
    /// # Panics
    ///
    /// Panics if overflow occurs.
    #[inline]
    #[must_use]
    pub fn from_observed_duration(duration: Duration) -> Self {
        #![allow(clippy::cast_possible_truncation)] // Truncation is not possible due to check.

        let nanos = duration.as_nanos();
        if nanos > (i64::MAX - 1) as u128 {
            impressive();
        }

        unsafe { TimeStamp::from_elapsed_unchecked(nanos as u64) }
    }

    /// Returns time span elapsed since specified point in time unless it overflows.
    #[inline]
    #[must_use]
    pub const fn checked_elapsed_since(self, since: TimeStamp) -> Option<TimeSpan> {
        let lhs = self.nanos();
        let rhs = since.nanos();

        match lhs.checked_sub(rhs) {
            None => None,
            Some(diff) => Some(TimeSpan::new(diff)),
        }
    }

    /// Returns time span elapsed since `earlier` point in time.
    ///
    /// # Panics
    ///
    /// Panics if difference overflows.
    #[inline]
    #[must_use]
    pub fn elapsed_since(self, earlier: TimeStamp) -> TimeSpan {
        self.checked_elapsed_since(earlier)
            .expect("`earlier` time stamp is greater than `self` time stamp")
    }

    /// Returns time span elapsed since start point in time.
    ///
    /// # Panics
    ///
    /// Panics if time span is out of bounds.
    #[inline]
    #[must_use]
    pub fn elapsed_since_start(self) -> TimeSpan {
        TimeSpan::new(self.nanos())
    }

    /// Returns time span elapsed since start point in time.
    #[inline]
    #[must_use]
    pub fn nanos_since_start(self) -> u64 {
        self.nanos.get() - 1
    }

    /// Returns new time stamp by adding the specified time span to the current time stamp.
    #[inline]
    #[must_use]
    pub fn add_span(self, span: TimeSpan) -> Option<TimeStamp> {
        #![allow(clippy::cast_sign_loss)] // Sign loss is not possible due to check.

        let nanos = self.nanos().checked_add(span.as_nanos())?;

        if nanos < 0 {
            return None; // TimeStamp cannot be negative
        }

        Some(unsafe { TimeStamp::from_elapsed_unchecked(nanos as u64) })
    }

    /// Returns new time stamp by subtracting the specified time span from the current time stamp.
    #[inline]
    #[must_use]
    pub fn sub_span(self, span: TimeSpan) -> Option<TimeStamp> {
        #![allow(clippy::cast_sign_loss)] // Sign loss is not possible due to check.

        let nanos = self.nanos().checked_sub(span.as_nanos())?;

        if nanos < 0 {
            return None; // TimeStamp cannot be zero or negative
        }

        Some(unsafe { TimeStamp::from_elapsed_unchecked(nanos as u64) })
    }
}

impl Add<TimeSpan> for TimeStamp {
    type Output = TimeStamp;

    #[inline]
    fn add(self, rhs: TimeSpan) -> Self {
        self.add_span(rhs)
            .expect("overflow when adding time span to time stamp")
    }
}

impl AddAssign<TimeSpan> for TimeStamp {
    #[inline]
    fn add_assign(&mut self, rhs: TimeSpan) {
        *self = *self + rhs;
    }
}

impl Sub<TimeSpan> for TimeStamp {
    type Output = TimeStamp;

    #[inline]
    fn sub(self, rhs: TimeSpan) -> Self {
        self.sub_span(rhs)
            .expect("overflow when adding time span to time stamp")
    }
}

impl SubAssign<TimeSpan> for TimeStamp {
    #[inline]
    fn sub_assign(&mut self, rhs: TimeSpan) {
        *self = *self - rhs;
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
        "Process runs for more than 200 years. Impressive. Upgrade to version with 128 bit integers"
    )
}
