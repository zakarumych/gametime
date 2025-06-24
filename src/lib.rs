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
//!   relative to global reference point that is initialized by first call to
//!   [`TimeStamp::now`].
//! - `serde` - enables `serde` support for [`TimeSpan`] and [`Frequency`].
//!

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]

#[cfg(feature = "std")]
mod clock;

mod freq;
mod rate;
mod span;
mod stamp;

/// Provides access to global reference point for time measurement.
#[cfg(feature = "global_reference")]
pub mod global_reference;

pub use crate::{
    freq::{Frequency, FrequencyNumExt, FrequencyTicker, FrequencyTickerIter},
    rate::ClockRate,
    span::{TimeSpan, TimeSpanNumExt},
    stamp::TimeStamp,
};

#[cfg(feature = "std")]
pub use crate::clock::{Clock, ClockStep};

#[cfg(all(feature = "global_reference", feature = "std"))]
pub use crate::clock::GlobalClock;

#[doc(hidden)]
pub trait I64ORF64 {}

impl I64ORF64 for i64 {}
impl I64ORF64 for f64 {}

/// Cast helper for [`timespan!`] macro.
#[doc(hidden)]
#[inline]
pub const fn __as<T>(a: T, _: &T) -> T
where
    T: I64ORF64,
{
    a
}

/// Converts human-readable expression into `TimeSpan`.
#[macro_export]
macro_rules! timespan {
    ($y:literal y) => { $crate::timespan!($y years) };

    ($y:literal years) => {{
        let years = $y * $crate::__as($crate::TimeSpan::YEAR.as_nanos() as _, &$y);
        $crate::TimeSpan::new(years as u64)
    }};

    ($d:literal d) => { $crate::timespan!($d days) };

    ($d:literal days) => {{
        let days = $d * $crate::__as($crate::TimeSpan::DAY.as_nanos() as _, &$d);
        $crate::TimeSpan::new(days as u64)
    }};

    ($h:literal : $m:literal : $s:literal) => {{
        let hours = $h * $crate::TimeSpan::HOUR.as_nanos();
        let minutes = $m * $crate::TimeSpan::MINUTE.as_nanos();
        let seconds = $s * $crate::__as($crate::TimeSpan::SECOND.as_nanos() as _, &$s);

        $crate::TimeSpan::new(hours + minutes + seconds as i64)
    }};

    ($h:literal h) => { $crate::timespan!($h hours) };

    ($h:literal hrs) => { $crate::timespan!($h hours) };

    ($h:literal hours) => {{
        let hours = $h * $crate::__as($crate::TimeSpan::HOUR.as_nanos() as _, &$h);
        $crate::TimeSpan::new(hours as i64)
    }};

    ($m:literal : $s:literal) => {{
        let minutes = $m * $crate::TimeSpan::MINUTE.as_nanos();
        let seconds = $s * $crate::__as($crate::TimeSpan::SECOND.as_nanos() as _, &$s);
        $crate::TimeSpan::new(minutes + seconds as i64)
    }};

    ($m:literal m) => { $crate::timespan!($m minutes) };

    ($m:literal mins) => { $crate::timespan!($m minutes) };

    ($m:literal minutes) => {{
        let minutes = $m * $crate::__as($crate::TimeSpan::MINUTE.as_nanos() as _, &$m);
        $crate::TimeSpan::new(minutes as i64)
    }};

    ($s:literal s) => { $crate::timespan!($s seconds) };

    ($s:literal secs) => { $crate::timespan!($s seconds) };

    ($s:literal seconds) => {{
        let seconds = $s * $crate::__as($crate::TimeSpan::SECOND.as_nanos() as _, &$s);
        $crate::TimeSpan::new(seconds as i64)
    }};

    ($(1)?year) => { $crate::TimeSpan::YEAR };
    ($(1)?weak) => { $crate::TimeSpan::WEEK };
    ($(1)?day) => { $crate::TimeSpan::DAY };
    ($(1)?hour) => { $crate::TimeSpan::HOUR };
    ($(1)?hr) => { $crate::TimeSpan::HOUR };
    ($(1)?minute) => { $crate::TimeSpan::MINUTE };
    ($(1)?min) => { $crate::TimeSpan::MINUTE };
    ($(1)?second) => { $crate::TimeSpan::SECOND };
    ($(1)?sec) => { $crate::TimeSpan::SECOND };
}

/// Converts human-readable expression into `TimeSpan`.
/// Shortcut for [`timespan!`].
#[macro_export]
macro_rules! ts {
    ($($tt:tt)*) => { $crate::timespan!($($tt)*) };
}

#[cfg(test)]
const TEST_SPANS: [TimeSpan; 7] = [
    timespan!(1 day),   // 1 day
    timespan!(2:3:1),   // 2 hours, 3 minutes, 1 second
    timespan!(3 hrs),   // 3 hours
    timespan!(2:3),     // 2 minutes, 3 seconds
    timespan!(3 mins),  // 3 minutes
    timespan!(42 secs), // 42 seconds
    timespan!(2 years), // 2 years
];

#[test]
fn test_timespan_macro() {
    assert_eq!(TEST_SPANS[0], TimeSpan::DAY);
    assert_eq!(TEST_SPANS[1], TimeSpan::hms(2, 3, 1));
    assert_eq!(TEST_SPANS[2], TimeSpan::HOUR * 3);
    assert_eq!(TEST_SPANS[3], TimeSpan::hms(0, 2, 3));
    assert_eq!(TEST_SPANS[4], TimeSpan::MINUTE * 3);
    assert_eq!(TEST_SPANS[5], TimeSpan::SECOND * 42);
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}
