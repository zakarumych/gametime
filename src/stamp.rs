//! Contains `TimeStamp` type, that represents fixed points in time,
//! traits and functions to work with it.

use core::{
    num::NonZeroU64,
    ops::{Add, AddAssign, Sub},
    time::Duration,
};

use crate::span::TimeSpan;

/// A fixed point in time relative to the reference point in time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TimeStamp {
    /// Number of nanoseconds elapsed from reference point in time.
    nanos: NonZeroU64,
}

impl TimeStamp {
    /// Constructs time stamp from number of nanoseconds elapsed since reference point in time.
    #[inline(always)]
    pub const fn start() -> Self {
        TimeStamp {
            nanos: unsafe { NonZeroU64::new_unchecked(1) },
        }
    }

    /// Constructs time stamp from number of nanoseconds elapsed since reference point in time.
    #[inline(always)]
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
    #[inline(always)]
    pub unsafe fn new_unchecked(nanos: u64) -> Self {
        TimeStamp {
            nanos: unsafe { NonZeroU64::new_unchecked(nanos) },
        }
    }

    /// Returns time stamp corresponding to "now".
    #[cfg(feature = "global_reference")]
    #[inline(always)]
    pub fn now() -> Self {
        let (now, reference) = global_reference::now_and_reference();
        let duration = now.duration_since(reference);

        match TimeStamp::from_duration(duration) {
            Some(stamp) => stamp,
            None => impressive(),
        }
    }

    /// Constructs time stamp from duration.
    #[inline(always)]
    pub fn from_duration(duration: Duration) -> Option<Self> {
        let nanos = duration.as_nanos();
        if nanos > (u64::MAX - 1) as u128 {
            return None;
        }
        Some(TimeStamp {
            nanos: unsafe { NonZeroU64::new_unchecked(nanos as u64 + 1) },
        })
    }

    /// Constructs time stamp from duration observed by the process.
    ///
    /// It guarantees that it fits into `TimeStamp` as it takes more that 500 years
    /// to overflow `TimeStamp` with `u64` nanoseconds.
    #[inline(always)]
    pub fn from_observed_duration(duration: Duration) -> Self {
        let nanos = duration.as_nanos();
        if nanos > (u64::MAX - 1) as u128 {
            impressive();
        }
        TimeStamp {
            nanos: unsafe { NonZeroU64::new_unchecked(nanos as u64 + 1) },
        }
    }

    #[inline(always)]
    pub const fn checked_elapsed_since(self, earlier: TimeStamp) -> Option<TimeSpan> {
        match self.nanos.get().checked_sub(earlier.nanos.get()) {
            None => None,
            Some(nanos) => Some(TimeSpan::new(nanos)),
        }
    }

    #[inline(always)]
    pub fn elapsed_since(self, earlier: TimeStamp) -> TimeSpan {
        self.checked_elapsed_since(earlier)
            .expect("overflow when calculating time span elapsed since earlier")
    }

    #[inline(always)]
    pub fn elapsed_since_start(self) -> TimeSpan {
        TimeSpan::new(self.nanos.get() - 1)
    }

    #[inline(always)]
    pub fn nanos_since_start(self) -> u64 {
        self.nanos.get() - 1
    }

    #[inline(always)]
    pub fn add_span(self, span: TimeSpan) -> Option<TimeStamp> {
        let nanos = self.nanos.get().checked_add(span.as_nanos())?;

        Some(TimeStamp {
            // Safety: a > 0, b >= 0 hence a + b > 0
            nanos: unsafe { NonZeroU64::new_unchecked(nanos) },
        })
    }
}

impl Add<TimeSpan> for TimeStamp {
    type Output = TimeStamp;

    #[inline(always)]
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
    #[inline(always)]
    fn add_assign(&mut self, rhs: TimeSpan) {
        *self = *self + rhs;
    }
}

impl Sub<TimeStamp> for TimeStamp {
    type Output = TimeSpan;

    #[inline(always)]
    fn sub(self, rhs: TimeStamp) -> TimeSpan {
        self.elapsed_since(rhs)
    }
}

#[cold]
#[inline(always)]
fn impressive() -> ! {
    panic!(
        "Process runs for more than 500 years. Impressive. Upgrade to version with u128 value type"
    )
}

#[cfg(feature = "global_reference")]
pub mod global_reference {
    use core::mem::MaybeUninit;
    use std::{sync::Once, time::Instant};

    static GLOBAL_REFERENCE_INIT: Once = Once::new();
    static mut GLOBAL_REFERENCE: MaybeUninit<Instant> = MaybeUninit::uninit();

    fn get_or_init(value: Instant) -> Instant {
        GLOBAL_REFERENCE_INIT.call_once(|| unsafe {
            GLOBAL_REFERENCE.write(value);
        });
        unsafe { *GLOBAL_REFERENCE.assume_init_ref() }
    }

    #[inline(always)]
    pub fn get() -> Instant {
        get_or_init(Instant::now())
    }

    #[inline(always)]
    pub fn now_and_reference() -> (Instant, Instant) {
        let now = Instant::now();
        let reference = get_or_init(now);
        (now, reference)
    }
}
