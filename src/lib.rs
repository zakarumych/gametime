//!
//! This crate provides `TimeSpan` and `TimeStamp` types for working with time
//! in ergonomic way in game engines.
//!
//! [`TimeSpan`] is a type that represents a time span in nanoseconds.
//! Similar to [`std::time::Duration`], but smaller and thus faster.
//!
//! [`TimeStamp`] is a type that represents a point in time in nanoseconds,
//! relative to reference point.
//!
//! [`Clock`] is a type that represents a clock to measure time and steps.
//!
//! [`Frequency`] and [`FrequencyTicker`] allow exact frequency ticker using
//! rational values.
//!
//! # Features
//!
//! - `std` - enables `std` support, including `Clock` and `ClockStep` types.
//! - `global_reference` - enables [`TimeStamp::now`] function to get time stamp
//! relative to global reference point that is initialized by first call to
//! [`TimeStamp::now`].
//! - `serde` - enables `serde` support for [`TimeSpan`] and [`Frequency`].
//!

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
mod clock;

mod freq;
mod span;
mod stamp;

pub use crate::{
    freq::{Frequency, FrequencyTicker, FrequencyTickerIter},
    span::{TimeSpan, TimeSpanNumExt},
    stamp::TimeStamp,
};

#[cfg(feature = "std")]
pub use crate::clock::{Clock, ClockStep};

#[cfg(feature = "global_reference")]
pub use crate::stamp::global_reference;

#[doc(hidden)]
pub trait U64ORF64 {}

impl U64ORF64 for u64 {}
impl U64ORF64 for f64 {}

/// Cast helper for [`timespan!`] macro.
#[doc(hidden)]
#[inline(always)]
pub const fn __as<T>(a: T, _: &T) -> T
where
    T: U64ORF64,
{
    a
}

#[macro_export]
macro_rules! timespan {
    ($d:literal d) => {{
        let days = $d * $crate::__as($crate::TimeSpan::DAY.as_nanos() as _, &$d);
        $crate::TimeSpan::new(days as u64)
    }};

    ($d:literal days) => {{
        let days = $d * $crate::__as($crate::TimeSpan::DAY.as_nanos() as _, &$d);
        $crate::TimeSpan::new(days as u64)
    }};

    ($h:literal : $m:literal : $s:literal) => {{
        let hours = $h * $crate::TimeSpan::HOUR.as_nanos();
        let minutes = $m * $crate::TimeSpan::MINUTE.as_nanos();
        let seconds = $s * $crate::__as($crate::TimeSpan::SECOND.as_nanos() as _, &$s);

        $crate::TimeSpan::new(hours + minutes + seconds as u64)
    }};

    ($h:literal h) => {{
        let hours = $h * $crate::__as($crate::TimeSpan::HOUR.as_nanos() as _, &$h);
        $crate::TimeSpan::new(hours as u64)
    }};

    ($h:literal hrs) => {{
        let hours = $h * $crate::__as($crate::TimeSpan::HOUR.as_nanos() as _, &$h);
        $crate::TimeSpan::new(hours as u64)
    }};

    ($h:literal hours) => {{
        let hours = $h * $crate::__as($crate::TimeSpan::HOUR.as_nanos() as _, &$s);
        $crate::TimeSpan::new(hours as u64)
    }};

    ($m:literal : $s:literal) => {{
        let minutes = $m * $crate::TimeSpan::MINUTE.as_nanos();
        let seconds = $s * $crate::__as($crate::TimeSpan::SECOND.as_nanos() as _, &$s);
        $crate::TimeSpan::new(minutes + seconds as u64)
    }};

    ($m:literal m) => {{
        let minutes = $m * $crate::__as($crate::TimeSpan::MINUTE.as_nanos() as _, &$m);
        $crate::TimeSpan::new(minutes as u64)
    }};

    ($m:literal mins) => {{
        let minutes = $m * $crate::__as($crate::TimeSpan::MINUTE.as_nanos() as _, &$m);
        $crate::TimeSpan::new(minutes as u64)
    }};

    ($m:literal minutes) => {{
        let minutes = $m * $crate::__as($crate::TimeSpan::MINUTE.as_nanos() as _, &$m);
        $crate::TimeSpan::new(minutes as u64)
    }};

    ($s:literal s) => {{
        let seconds = $s * $crate::__as($crate::TimeSpan::SECOND.as_nanos() as _, &$s);
        $crate::TimeSpan::new(seconds as u64)
    }};

    ($s:literal secs) => {{
        let seconds = $s * $crate::__as($crate::TimeSpan::SECOND.as_nanos() as _, &$s);
        $crate::TimeSpan::new(seconds as u64)
    }};

    ($s:literal seconds) => {{
        let seconds = $s * $crate::__as($crate::TimeSpan::SECOND.as_nanos() as _, &$s);
        $crate::TimeSpan::new(seconds as u64)
    }};
}

#[cfg(test)]
const TEST_SPANS: [TimeSpan; 6] = [
    timespan!(1 d),     // 1 day
    timespan!(2:3:1),   // 2 hours, 3 minutes, 1 second
    timespan!(3 hrs),   // 3 hours
    timespan!(2:3),     // 2 minutes, 3 seconds
    timespan!(3 mins),  // 3 minutes
    timespan!(42 secs), // 42 seconds
];

#[test]
fn test_timespan_macro() {
    assert_eq!(TEST_SPANS[0], TimeSpan::DAY);
    assert_eq!(
        TEST_SPANS[1],
        TimeSpan::HOUR * 2 + TimeSpan::MINUTE * 3 + TimeSpan::SECOND
    );
    assert_eq!(TEST_SPANS[2], TimeSpan::HOUR * 3);
    assert_eq!(TEST_SPANS[3], TimeSpan::MINUTE * 2 + TimeSpan::SECOND * 3);
    assert_eq!(TEST_SPANS[4], TimeSpan::MINUTE * 3);
    assert_eq!(TEST_SPANS[5], TimeSpan::SECOND * 42);
}
