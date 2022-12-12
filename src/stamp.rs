//! Contains `TimeStamp` type, that represents fixed points in time,
//! traits and functions to work with it.

use core::{
    num::NonZeroU64,
    ops::{Add, AddAssign, Sub},
};

use crate::span::TimeSpan;

/// A fixed point in time relative to the reference point in time.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    pub const fn new(nanos: NonZeroU64) -> Self {
        TimeStamp { nanos }
    }

    /// Returns time stamp corresponding to "now".
    #[cfg(feature = "global_reference")]
    #[inline(always)]
    pub fn now() -> Self {
        let (now, reference) = global_reference::now_and_reference();
        let duration = now.duration_since(reference);
        let nanos = duration.as_nanos();

        #[cold]
        #[inline(never)]
        fn impressive() -> ! {
            panic!("Process runs for more than 500 years. Impressive. Upgrade to version with u128 value type")
        }

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
}

impl Add<TimeSpan> for TimeStamp {
    type Output = TimeStamp;

    #[inline(always)]
    fn add(self, rhs: TimeSpan) -> Self {
        TimeStamp {
            nanos: unsafe {
                // # Safety
                // a > 0, b >= 0 hence a + b > 0
                NonZeroU64::new_unchecked(self.nanos.get() + rhs.as_nanos())
            },
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
