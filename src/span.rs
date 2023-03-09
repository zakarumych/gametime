//! This module contains `TimeSpan` type that represents durations.
//! The name `Duration` is not used to avoid confusion with std type.
//!
//!
//! Contains traits and functions to work with `TimeSpan`s.
//!

use core::{
    convert::TryFrom,
    fmt::{self, Debug, Display},
    num::{NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, TryFromIntError},
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Range, Rem, RemAssign, Sub, SubAssign},
    str::FromStr,
    time::Duration,
};

/// An interval in between time stamps.
/// This type is used to represent durations with nanosecond precision.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TimeSpan {
    nanos: u64,
}

impl TimeSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *self == Self::ZERO {
            f.write_str("0")
        } else {
            let mut span = *self;
            if span >= Self::DAY {
                let days = span / Self::DAY;
                span %= Self::DAY;

                let hours = span / Self::HOUR;
                span %= Self::HOUR;

                let minutes = span / Self::MINUTE;
                span %= Self::MINUTE;

                let seconds = span / Self::SECOND;
                span %= Self::SECOND;

                let millis = span / Self::MILLISECOND;

                if millis > 0 {
                    write!(
                        f,
                        "{}d{:02}:{:02}:{:02}.{:03}",
                        days, hours, minutes, seconds, millis
                    )
                } else if seconds > 0 {
                    write!(f, "{}d{:02}:{:02}:{:02}", days, hours, minutes, seconds)
                } else {
                    write!(f, "{}d{:02}:{:02}", days, hours, minutes)
                }
            } else if span >= Self::HOUR {
                let hours = span / Self::HOUR;
                span %= Self::HOUR;

                let minutes = span / Self::MINUTE;
                span %= Self::MINUTE;

                let seconds = span / Self::SECOND;
                span %= Self::SECOND;

                let millis = span / Self::MILLISECOND;
                if millis > 0 {
                    write!(f, "{}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
                } else {
                    write!(f, "{}:{:02}:{:02}", hours, minutes, seconds)
                }
            } else if span >= Self::MINUTE {
                let minutes = span / Self::MINUTE;
                span %= Self::MINUTE;

                let seconds = span / Self::SECOND;
                span %= Self::SECOND;

                let millis = span / Self::MILLISECOND;
                if millis > 0 {
                    write!(f, "{}:{:02}.{:03}", minutes, seconds, millis)
                } else {
                    write!(f, "{}:{:02}", minutes, seconds)
                }
            } else if span >= Self::SECOND {
                let seconds = span / Self::SECOND;
                span %= Self::SECOND;

                let millis = span / Self::MILLISECOND;
                if millis > 0 {
                    write!(f, "{}.{:03}s", seconds, millis)
                } else {
                    write!(f, "{}s", seconds)
                }
            } else if span >= Self::MILLISECOND {
                let millis = span / Self::MILLISECOND;
                span %= Self::MILLISECOND;

                let micros = span / Self::MICROSECOND;
                if micros > 0 {
                    write!(f, "{}.{:03}ms", millis, micros)
                } else {
                    write!(f, "{}ms", millis)
                }
            } else if span >= Self::MICROSECOND {
                let micros = span / Self::MICROSECOND;
                span %= Self::MICROSECOND;

                let nanos = span / Self::NANOSECOND;
                if nanos > 0 {
                    write!(f, "{}.{:03}us", micros, nanos)
                } else {
                    write!(f, "{}us", micros)
                }
            } else {
                let nanos = span / Self::NANOSECOND;
                write!(f, "{}ns", nanos)
            }
        }
    }

    fn fmt_full(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut span = *self;
        let days = span / Self::DAY;
        span %= Self::DAY;
        let hours = span / Self::HOUR;
        span %= Self::HOUR;
        let minutes = span / Self::MINUTE;
        span %= Self::MINUTE;
        let seconds = span / Self::SECOND;
        span %= Self::SECOND;
        let nanos = span / Self::NANOSECOND;

        write!(
            f,
            "{:01}d{:02}:{:02}:{:02}.{:09}",
            days, hours, minutes, seconds, nanos
        )
    }

    fn fmt_nanos(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ns", self.nanos)
    }
}

impl Debug for TimeSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            self.fmt_nanos(f)
        } else {
            self.fmt(f)
        }
    }
}

impl Display for TimeSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            self.fmt_full(f)
        } else {
            self.fmt(f)
        }
    }
}

#[derive(Debug)]
pub enum TimeSpanParseErr {
    NonASCII,
    StringTooLarge { len: usize },
    IntParseError { source: core::num::ParseIntError },
    UnexpectedDelimiter { delim: char, pos: usize },
    UnexpectedEndOfString,
    UnexpectedSuffix,
    HoursOutOfBound { hours: u64 },
    MinutesOutOfBound { minutes: u64 },
    SecondsOutOfBound { seconds: u64 },
}

impl fmt::Display for TimeSpanParseErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonASCII => f.write_str("Time spans encoded in strings are always ASCII"),
            Self::StringTooLarge { len } => {
                write!(
                    f,
                    "Valid time span string may never exceed {} bytes. String is {}",
                    MAX_TIME_SPAN_STRING, len
                )
            }
            Self::IntParseError { .. } => f.write_str("Failed to parse integer"),
            Self::UnexpectedDelimiter { delim, pos } => {
                write!(f, "Unexpected delimiter '{}' at {}", delim, pos)
            }
            Self::UnexpectedEndOfString => f.write_str("Unexpected end of string"),
            Self::UnexpectedSuffix => {
                f.write_str("Unexpected suffix. Only `s`, `ms` and `us` suffixes are supported")
            }
            Self::HoursOutOfBound { hours } => {
                write!(f, "Hours must be in range 0-23 when days are specified. Value at hours position is '{}'", hours)
            }
            Self::MinutesOutOfBound { minutes } => {
                write!(f, "Minutes must be in range 0-59 when hours are specified. Value at minutes position is '{}'", minutes)
            }
            Self::SecondsOutOfBound { seconds } => {
                write!(
                    f,
                    "Seconds must be in range 0-59 when minutes are specified. Value at seconds position is '{}'", seconds
                )
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TimeSpanParseErr {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IntParseError { source } => Some(source),
            _ => None,
        }
    }
}

const MAX_TIME_SPAN_STRING: usize = 48;

impl FromStr for TimeSpan {
    type Err = TimeSpanParseErr;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.is_ascii() {
            return Err(TimeSpanParseErr::NonASCII);
        }

        if s.len() > MAX_TIME_SPAN_STRING {
            return Err(TimeSpanParseErr::StringTooLarge { len: s.len() });
        }

        let mut seps = s.match_indices(|c: char| !c.is_ascii_digit() && !c.is_ascii_whitespace());

        struct Ranges {
            days: Option<Range<usize>>,
            hours: Option<Range<usize>>,
            minutes: Option<Range<usize>>,
            seconds: Option<Range<usize>>,
            fract: Option<Range<usize>>,
            denom: u32,
        }

        impl Ranges {
            fn parse(self, s: &str) -> Result<TimeSpan, TimeSpanParseErr> {
                let seconds: u64 = self
                    .seconds
                    .map(|r| s[r].trim().parse())
                    .unwrap_or(Ok(0))
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;

                if self.minutes.is_some() && seconds > 59 {
                    return Err(TimeSpanParseErr::SecondsOutOfBound { seconds });
                }

                let minutes: u64 = self
                    .minutes
                    .map(|r| s[r].trim().parse())
                    .unwrap_or(Ok(0))
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;

                if self.hours.is_some() && minutes > 59 {
                    return Err(TimeSpanParseErr::MinutesOutOfBound { minutes });
                }

                let hours: u64 = self
                    .hours
                    .map(|r| s[r].trim().parse())
                    .unwrap_or(Ok(0))
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;

                if self.days.is_some() && hours > 23 {
                    return Err(TimeSpanParseErr::HoursOutOfBound { hours });
                }

                let days: u64 = self
                    .days
                    .map(|r| s[r].trim().parse())
                    .unwrap_or(Ok(0))
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;

                let fract: u64 = self
                    .fract
                    .map(|r| s[r].trim().parse())
                    .unwrap_or(Ok(0))
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;
                let micros = if self.denom > 6 {
                    fract / 10u64.pow(self.denom - 6)
                } else {
                    fract * 10u64.pow(6 - self.denom)
                };

                Ok(days * TimeSpan::DAY
                    + hours * TimeSpan::HOUR
                    + minutes * TimeSpan::MINUTE
                    + seconds * TimeSpan::SECOND
                    + micros * TimeSpan::MICROSECOND)
            }
        }

        match seps.next() {
            Some((dh, "d" | "D" | "t" | "T")) => match seps.next() {
                Some((hm, ":")) => match seps.next() {
                    None => Ranges {
                        days: Some(0..dh),
                        hours: Some(dh + 1..hm),
                        minutes: Some(hm + 1..s.len()),
                        seconds: None,
                        fract: None,
                        denom: 0,
                    },
                    Some((ms, ":")) => match seps.next() {
                        None => Ranges {
                            days: Some(0..dh),
                            hours: Some(dh + 1..hm),
                            minutes: Some(hm + 1..ms),
                            seconds: Some(ms + 1..s.len()),
                            fract: None,
                            denom: 0,
                        },
                        Some((sf, ".")) => {
                            if let Some((pos, delim)) = seps.next() {
                                return Err(TimeSpanParseErr::UnexpectedDelimiter {
                                    delim: delim.chars().next().unwrap(),
                                    pos,
                                });
                            } else {
                                Ranges {
                                    days: Some(0..dh),
                                    hours: Some(dh + 1..hm),
                                    minutes: Some(hm + 1..ms),
                                    seconds: Some(ms + 1..sf),
                                    fract: Some(sf + 1..s.len().min(sf + 21)),
                                    denom: (s.len() - sf - 1) as u32,
                                }
                            }
                        }

                        Some((pos, delim)) => {
                            return Err(TimeSpanParseErr::UnexpectedDelimiter {
                                delim: delim.chars().next().unwrap(),
                                pos,
                            });
                        }
                    },
                    Some((pos, delim)) => {
                        return Err(TimeSpanParseErr::UnexpectedDelimiter {
                            delim: delim.chars().next().unwrap(),
                            pos,
                        });
                    }
                },
                Some((pos, delim)) => {
                    return Err(TimeSpanParseErr::UnexpectedDelimiter {
                        delim: delim.chars().next().unwrap(),
                        pos,
                    });
                }
                None => {
                    return Err(TimeSpanParseErr::UnexpectedEndOfString);
                }
            },
            Some((hms, ":")) => match seps.next() {
                Some((ms, ":")) => match seps.next() {
                    Some((sf, ".")) => {
                        if let Some((pos, delim)) = seps.next() {
                            return Err(TimeSpanParseErr::UnexpectedDelimiter {
                                delim: delim.chars().next().unwrap(),
                                pos,
                            });
                        } else {
                            Ranges {
                                days: None,
                                hours: Some(0..hms),
                                minutes: Some(hms + 1..ms),
                                seconds: Some(ms + 1..sf),
                                fract: Some(sf + 1..s.len().min(sf + 21)),
                                denom: (s.len() - sf - 1) as u32,
                            }
                        }
                    }
                    None => Ranges {
                        days: None,
                        hours: Some(0..hms),
                        minutes: Some(hms + 1..ms),
                        seconds: Some(ms + 1..s.len()),
                        fract: None,
                        denom: 0,
                    },
                    Some((pos, delim)) => {
                        return Err(TimeSpanParseErr::UnexpectedDelimiter {
                            delim: delim.chars().next().unwrap(),
                            pos,
                        });
                    }
                },
                Some((sf, ".")) => {
                    if let Some((pos, delim)) = seps.next() {
                        return Err(TimeSpanParseErr::UnexpectedDelimiter {
                            delim: delim.chars().next().unwrap(),
                            pos,
                        });
                    } else {
                        Ranges {
                            days: None,
                            hours: None,
                            minutes: Some(0..hms),
                            seconds: Some(hms + 1..sf),
                            fract: Some(sf + 1..s.len()),
                            denom: (s.len() - sf - 1) as u32,
                        }
                    }
                }
                None => Ranges {
                    days: None,
                    hours: None,
                    minutes: Some(0..hms),
                    seconds: Some(hms + 1..s.len()),
                    fract: None,
                    denom: 0,
                },
                Some((pos, delim)) => {
                    return Err(TimeSpanParseErr::UnexpectedDelimiter {
                        delim: delim.chars().next().unwrap(),
                        pos,
                    });
                }
            },

            Some((sf, ".")) => {
                if let Some((pos, delim)) = seps.next() {
                    return Err(TimeSpanParseErr::UnexpectedDelimiter {
                        delim: delim.chars().next().unwrap(),
                        pos,
                    });
                } else {
                    Ranges {
                        days: None,
                        hours: None,
                        minutes: None,
                        seconds: Some(0..sf),
                        fract: Some(sf + 1..s.len()),
                        denom: (s.len() - sf - 1) as u32,
                    }
                }
            }

            Some((suffix, "s")) => {
                if s[suffix..].trim() != "s" {
                    return Err(TimeSpanParseErr::UnexpectedSuffix);
                }

                let seconds: u64 = s[..suffix]
                    .trim()
                    .parse()
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;
                return Ok(seconds * Self::SECOND);
            }

            Some((suffix, "m")) => {
                if s[suffix..].trim() != "ms" {
                    return Err(TimeSpanParseErr::UnexpectedSuffix);
                }

                let millis: u64 = s[..suffix]
                    .trim()
                    .parse()
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;
                return Ok(millis * Self::MILLISECOND);
            }

            Some((suffix, "u")) => {
                if s[suffix..].trim() != "us" {
                    return Err(TimeSpanParseErr::UnexpectedSuffix);
                }

                let micros: u64 = s[..suffix]
                    .trim()
                    .parse()
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;
                return Ok(micros * Self::MICROSECOND);
            }

            None => {
                let seconds: u64 = s
                    .trim()
                    .parse()
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;
                return Ok(seconds * Self::SECOND);
            }

            Some((pos, delim)) => {
                return Err(TimeSpanParseErr::UnexpectedDelimiter {
                    delim: delim.chars().next().unwrap(),
                    pos,
                });
            }
        }
        .parse(s)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for TimeSpan {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize in pretty format for human readable serializer
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_string())
        } else {
            serializer.serialize_u64(self.nanos)
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for TimeSpan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = TimeSpan;

            fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
                fmt.write_str("String with encoded time span or integer representing nanoseconds")
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
                Ok(TimeSpan { nanos: v })
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v < 0 {
                    Err(E::custom("TimeSpan cannot be negative"))
                } else {
                    Ok(TimeSpan { nanos: v as u64 })
                }
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.parse().map_err(|err| E::custom(err))
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(Visitor)
        } else {
            deserializer.deserialize_u64(Visitor)
        }
    }
}

impl From<Duration> for TimeSpan {
    #[inline]
    fn from(duration: Duration) -> Self {
        let nanos = duration.as_nanos();
        debug_assert!(u64::MAX as u128 > nanos);
        TimeSpan {
            nanos: nanos as u64,
        }
    }
}

impl From<TimeSpan> for Duration {
    #[inline]
    fn from(span: TimeSpan) -> Self {
        Duration::new(span.as_seconds(), (span.as_nanos() % 1000000000) as u32)
    }
}

impl TimeSpan {
    /// Zero time span.
    ///
    /// Represents duration between equal time points.
    pub const ZERO: Self = TimeSpan { nanos: 0 };

    /// One nanosecond span.
    /// Minimal possible time span supported by this type.
    pub const NANOSECOND: Self = TimeSpan { nanos: 1 };

    /// One microsecond span.
    pub const MICROSECOND: Self = TimeSpan { nanos: 1_000 };

    /// One millisecond span.
    pub const MILLISECOND: Self = TimeSpan { nanos: 1_000_000 };

    /// One second span.
    pub const SECOND: Self = TimeSpan {
        nanos: 1_000_000_000,
    };

    /// One minute span.
    pub const MINUTE: Self = TimeSpan {
        nanos: 60_000_000_000,
    };

    /// One hour span.
    pub const HOUR: Self = TimeSpan {
        nanos: 3_600_000_000_000,
    };

    /// One day span.
    pub const DAY: Self = TimeSpan {
        nanos: 86_400_000_000_000,
    };

    /// One week.
    /// Defined as 7 days.
    pub const WEEK: Self = TimeSpan {
        nanos: 604_800_000_000_000,
    };

    /// One Julian year.
    /// Average year length in Julian calendar.
    /// Defined as 365.25 days.
    pub const JULIAN_YEAR: Self = TimeSpan {
        nanos: 31_557_600_000_000_000,
    };

    /// One Gregorian year.
    /// Average year length in Gregorian calendar.
    /// Defined as 365.2425 days.
    pub const GREGORIAN_YEAR: Self = TimeSpan {
        nanos: 31_556_952_000_000,
    };

    /// One solar year (tropical year).
    /// Defined as 365.24219 days.
    pub const SOLAR_YEAR: Self = TimeSpan {
        nanos: 31_556_925_216_000_000,
    };

    /// One year.
    /// Closest value to the average length of a year on Earth.
    pub const YEAR: Self = Self::SOLAR_YEAR;

    /// Constructs time span from number of nanoseconds.
    #[inline(always)]
    pub const fn new(nanos: u64) -> TimeSpan {
        TimeSpan { nanos }
    }

    /// Returns number of nanoseconds in this time span.
    #[inline(always)]
    pub const fn as_nanos(self) -> u64 {
        self.nanos
    }

    /// Returns number of microseconds this value represents.
    #[inline]
    pub const fn as_micros(&self) -> u64 {
        self.nanos / Self::MICROSECOND.nanos
    }

    /// Returns number of whole milliseconds this value represents.
    #[inline]
    pub const fn as_millis(&self) -> u64 {
        self.nanos / Self::MILLISECOND.nanos
    }

    /// Returns number of whole seconds this value represents.
    #[inline]
    pub const fn as_seconds(&self) -> u64 {
        self.nanos / Self::SECOND.nanos
    }

    /// Returns number of whole minutes this value represents.
    #[inline]
    pub const fn as_minutes(&self) -> u64 {
        self.nanos / Self::MINUTE.nanos
    }

    /// Returns number of whole hours this value represents.
    #[inline]
    pub const fn as_hours(&self) -> u64 {
        self.nanos / Self::HOUR.nanos
    }

    /// Returns number of whole days this value represents.
    #[inline]
    pub const fn as_days(&self) -> u64 {
        self.nanos / Self::DAY.nanos
    }

    /// Returns number of whole weeks this value represents.
    #[inline]
    pub const fn as_weeks(&self) -> u64 {
        self.nanos / Self::WEEK.nanos
    }

    /// Returns number of seconds as floating point value.
    /// This function should be used for small-ish spans when high precision is not required.
    #[inline]
    pub fn as_secs_f32(&self) -> f32 {
        self.nanos as f32 / Self::SECOND.nanos as f32
    }

    /// Returns number of seconds as high precision floating point value.
    #[inline]
    pub fn as_secs_f64(&self) -> f64 {
        self.nanos as f64 / Self::SECOND.nanos as f64
    }

    #[inline(always)]
    pub const fn checked_add(self, span: TimeSpan) -> Option<TimeSpan> {
        match self.nanos.checked_add(span.nanos) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    #[inline(always)]
    pub const fn checked_sub(self, span: TimeSpan) -> Option<TimeSpan> {
        match self.nanos.checked_sub(span.nanos) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    #[inline(always)]
    pub const fn checked_mul(self, value: u64) -> Option<TimeSpan> {
        match self.nanos.checked_mul(value) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    #[inline(always)]
    pub const fn checked_div(self, value: u64) -> Option<TimeSpan> {
        match self.nanos.checked_div(value) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    #[inline(always)]
    pub const fn div(self, value: NonZeroU64) -> TimeSpan {
        let nanos = self.nanos / value.get();
        TimeSpan { nanos }
    }

    #[inline(always)]
    pub const fn checked_div_span(self, span: TimeSpan) -> Option<u64> {
        match self.nanos.checked_div(span.nanos) {
            None => None,
            Some(value) => Some(value),
        }
    }

    #[inline(always)]
    pub const fn div_span(self, span: NonZeroTimeSpan) -> u64 {
        self.nanos / span.nanos.get()
    }

    #[inline(always)]
    pub const fn checked_rem(self, value: u64) -> Option<TimeSpan> {
        match self.nanos.checked_rem(value) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    #[inline(always)]
    pub const fn rem(self, value: NonZeroU64) -> TimeSpan {
        let nanos = self.nanos % value.get();
        TimeSpan { nanos }
    }

    #[inline(always)]
    pub const fn checked_rem_span(self, span: TimeSpan) -> Option<TimeSpan> {
        match self.nanos.checked_rem(span.nanos) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    #[inline(always)]
    pub const fn rem_span(self, span: NonZeroTimeSpan) -> TimeSpan {
        let nanos = self.nanos % span.nanos.get();
        TimeSpan { nanos }
    }
}

/// An interval in between different time stamps.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct NonZeroTimeSpan {
    nanos: NonZeroU64,
}

impl From<NonZeroTimeSpan> for TimeSpan {
    #[inline(always)]
    fn from(span: NonZeroTimeSpan) -> Self {
        TimeSpan {
            nanos: span.nanos.get(),
        }
    }
}

impl TryFrom<TimeSpan> for NonZeroTimeSpan {
    type Error = TryFromIntError;

    #[inline(always)]
    fn try_from(span: TimeSpan) -> Result<Self, TryFromIntError> {
        match NonZeroU64::try_from(span.nanos) {
            Err(err) => Err(err),
            Ok(nanos) => Ok(NonZeroTimeSpan { nanos }),
        }
    }
}

impl NonZeroTimeSpan {
    /// One nanosecond span.
    /// Minimal possible time span supported by this type.
    pub const NANOSECOND: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(1) },
    };

    /// One microsecond span.
    pub const MICROSECOND: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(1_000) },
    };

    /// One millisecond span.
    pub const MILLISECOND: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(1_000_000) },
    };

    /// One second span.
    pub const SECOND: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(1_000_000_000) },
    };

    /// One minute span.
    pub const MINUTE: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(60_000_000_000) },
    };

    /// One hour span.
    pub const HOUR: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(3_600_000_000_000) },
    };

    /// One day span.
    pub const DAY: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(86_400_000_000_000) },
    };

    /// One week.
    /// Defined as 7 days.
    pub const WEEK: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(604_800_000_000_000) },
    };

    /// One Julian year.
    /// Average year length in Julian calendar.
    /// Defined as 365.25 days.
    pub const JULIAN_YEAR: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(31_557_600_000_000_000) },
    };

    /// One Gregorian year.
    /// Average year length in Gregorian calendar.
    /// Defined as 365.2425 days.
    pub const GREGORIAN_YEAR: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(31_556_952_000_000) },
    };

    /// One solar year (tropical year).
    /// Defined as 365.24219 days.
    pub const SOLAR_YEAR: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(31_556_925_216_000_000) },
    };

    /// One year.
    /// Closest value to the average length of a year on Earth.
    pub const YEAR: Self = Self::SOLAR_YEAR;

    /// Constructs time span from number of nanoseconds.
    #[inline(always)]
    pub const fn new(nanos: NonZeroU64) -> NonZeroTimeSpan {
        NonZeroTimeSpan { nanos }
    }
    /// Returns number of nanoseconds in this time span.
    #[inline(always)]
    pub const fn as_nanos(self) -> NonZeroU64 {
        self.nanos
    }

    /// Returns number of microseconds this value represents.
    #[inline]
    pub const fn as_micros(&self) -> u64 {
        self.nanos.get() / Self::MICROSECOND.nanos.get()
    }

    /// Returns number of whole milliseconds this value represents.
    #[inline]
    pub const fn as_millis(&self) -> u64 {
        self.nanos.get() / Self::MILLISECOND.nanos.get()
    }

    /// Returns number of whole seconds this value represents.
    #[inline]
    pub const fn as_seconds(&self) -> u64 {
        self.nanos.get() / Self::SECOND.nanos.get()
    }

    /// Returns number of whole minutes this value represents.
    #[inline]
    pub const fn as_minutes(&self) -> u64 {
        self.nanos.get() / Self::MINUTE.nanos.get()
    }

    /// Returns number of whole hours this value represents.
    #[inline]
    pub const fn as_hours(&self) -> u64 {
        self.nanos.get() / Self::HOUR.nanos.get()
    }

    /// Returns number of whole days this value represents.
    #[inline]
    pub const fn as_days(&self) -> u64 {
        self.nanos.get() / Self::DAY.nanos.get()
    }

    /// Returns number of whole weeks this value represents.
    #[inline]
    pub const fn as_weeks(&self) -> u64 {
        self.nanos.get() / Self::WEEK.nanos.get()
    }

    /// Returns number of seconds as floating point value.
    /// This function should be used for small-ish spans when high precision is not required.
    #[inline]
    pub fn as_secs_f32(&self) -> f32 {
        self.nanos.get() as f32 / Self::SECOND.nanos.get() as f32
    }

    /// Returns number of seconds as high precision floating point value.
    #[inline]
    pub fn as_secs_f64(&self) -> f64 {
        self.nanos.get() as f64 / Self::SECOND.nanos.get() as f64
    }

    #[inline(always)]
    pub const fn checked_add(self, span: TimeSpan) -> Option<NonZeroTimeSpan> {
        match self.nanos.get().checked_add(span.nanos) {
            None => None,
            Some(nanos) => Some(NonZeroTimeSpan {
                nanos: unsafe {
                    // # Safety
                    // Sum of non-zero and non-negative is non-zero
                    NonZeroU64::new_unchecked(nanos)
                },
            }),
        }
    }

    #[inline(always)]
    pub const fn checked_sub(self, span: TimeSpan) -> Option<TimeSpan> {
        match self.nanos.get().checked_sub(span.nanos) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    #[inline(always)]
    pub const fn checked_mul(self, value: u64) -> Option<TimeSpan> {
        match self.nanos.get().checked_mul(value) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    #[inline(always)]
    pub const fn checked_mul_non_zero(self, value: NonZeroU64) -> Option<NonZeroTimeSpan> {
        match self.nanos.get().checked_mul(value.get()) {
            None => None,
            Some(nanos) => Some(NonZeroTimeSpan {
                nanos: unsafe {
                    // # Safety
                    // a > 0, b > 0 hence a * b > 0
                    NonZeroU64::new_unchecked(nanos)
                },
            }),
        }
    }

    #[inline(always)]
    pub const fn checked_div(self, value: u64) -> Option<TimeSpan> {
        match self.nanos.get().checked_div(value) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    #[inline(always)]
    pub const fn div(self, value: NonZeroU64) -> TimeSpan {
        let nanos = self.nanos.get() / value.get();
        TimeSpan { nanos }
    }

    #[inline(always)]
    pub const fn checked_div_span(self, span: TimeSpan) -> Option<u64> {
        match self.nanos.get().checked_div(span.nanos) {
            None => None,
            Some(value) => Some(value),
        }
    }

    #[inline(always)]
    pub const fn div_span(self, span: NonZeroTimeSpan) -> u64 {
        self.nanos.get() / span.nanos.get()
    }

    #[inline(always)]
    pub const fn checked_rem(self, value: u64) -> Option<TimeSpan> {
        match self.nanos.get().checked_rem(value) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    #[inline(always)]
    pub const fn rem(self, value: NonZeroU64) -> TimeSpan {
        let nanos = self.nanos.get() % value.get();
        TimeSpan { nanos }
    }
    #[inline(always)]
    pub const fn checked_rem_span(self, span: TimeSpan) -> Option<TimeSpan> {
        match self.nanos.get().checked_rem(span.nanos) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    #[inline(always)]
    pub const fn rem_span(self, span: NonZeroTimeSpan) -> TimeSpan {
        let nanos = self.nanos.get() % span.nanos.get();
        TimeSpan { nanos }
    }
}

impl Add<TimeSpan> for TimeSpan {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: TimeSpan) -> Self {
        self.checked_add(rhs).expect("overflow when adding spans")
    }
}

impl Add<TimeSpan> for NonZeroTimeSpan {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: TimeSpan) -> Self {
        self.checked_add(rhs).expect("overflow when adding spans")
    }
}

impl Add<NonZeroTimeSpan> for TimeSpan {
    type Output = NonZeroTimeSpan;

    #[inline(always)]
    fn add(self, rhs: NonZeroTimeSpan) -> NonZeroTimeSpan {
        rhs.checked_add(self).expect("overflow when adding spans")
    }
}

impl Add<NonZeroTimeSpan> for NonZeroTimeSpan {
    type Output = NonZeroTimeSpan;

    #[inline(always)]
    fn add(self, rhs: NonZeroTimeSpan) -> Self {
        self.checked_add(rhs.into())
            .expect("overflow when adding spans")
    }
}

impl AddAssign<TimeSpan> for TimeSpan {
    fn add_assign(&mut self, rhs: TimeSpan) {
        *self = *self + rhs;
    }
}

impl AddAssign<NonZeroTimeSpan> for TimeSpan {
    fn add_assign(&mut self, rhs: NonZeroTimeSpan) {
        *self = (*self + rhs).into();
    }
}

impl AddAssign<TimeSpan> for NonZeroTimeSpan {
    fn add_assign(&mut self, rhs: TimeSpan) {
        *self = *self + rhs;
    }
}

impl AddAssign<NonZeroTimeSpan> for NonZeroTimeSpan {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sub<TimeSpan> for TimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn sub(self, rhs: TimeSpan) -> Self {
        self.checked_sub(rhs)
            .expect("overflow when subtracting spans")
    }
}

impl Sub<TimeSpan> for NonZeroTimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn sub(self, rhs: TimeSpan) -> TimeSpan {
        self.checked_sub(rhs)
            .expect("overflow when subtracting spans")
    }
}

impl Sub<NonZeroTimeSpan> for TimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn sub(self, rhs: NonZeroTimeSpan) -> TimeSpan {
        rhs.checked_sub(self)
            .expect("overflow when subtracting spans")
    }
}

impl Sub<NonZeroTimeSpan> for NonZeroTimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn sub(self, rhs: NonZeroTimeSpan) -> TimeSpan {
        self.checked_sub(rhs.into())
            .expect("overflow when subtracting spans")
    }
}

impl SubAssign<TimeSpan> for TimeSpan {
    fn sub_assign(&mut self, rhs: TimeSpan) {
        *self = *self - rhs;
    }
}

impl SubAssign<NonZeroTimeSpan> for TimeSpan {
    fn sub_assign(&mut self, rhs: NonZeroTimeSpan) {
        *self = *self - rhs;
    }
}

impl Div<TimeSpan> for TimeSpan {
    type Output = u64;

    #[inline(always)]
    fn div(self, rhs: TimeSpan) -> u64 {
        self.checked_div_span(rhs)
            .expect("divide by zero error when dividing span by span")
    }
}

impl Div<NonZeroTimeSpan> for TimeSpan {
    type Output = u64;

    #[inline(always)]
    fn div(self, rhs: NonZeroTimeSpan) -> u64 {
        self.div_span(rhs)
    }
}

impl Div<TimeSpan> for NonZeroTimeSpan {
    type Output = u64;

    #[inline(always)]
    fn div(self, rhs: TimeSpan) -> u64 {
        self.checked_div_span(rhs)
            .expect("divide by zero error when dividing span by span")
    }
}

impl Div<NonZeroTimeSpan> for NonZeroTimeSpan {
    type Output = u64;

    #[inline(always)]
    fn div(self, rhs: NonZeroTimeSpan) -> u64 {
        self.div_span(rhs)
    }
}

impl Rem<TimeSpan> for TimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn rem(self, rhs: TimeSpan) -> TimeSpan {
        self.checked_rem_span(rhs)
            .expect("divide by zero error when dividing span by span")
    }
}

impl RemAssign<TimeSpan> for TimeSpan {
    #[inline(always)]
    fn rem_assign(&mut self, rhs: TimeSpan) {
        *self = *self % rhs;
    }
}

impl Rem<NonZeroTimeSpan> for TimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn rem(self, rhs: NonZeroTimeSpan) -> TimeSpan {
        self.rem_span(rhs)
    }
}

impl RemAssign<NonZeroTimeSpan> for TimeSpan {
    #[inline(always)]
    fn rem_assign(&mut self, rhs: NonZeroTimeSpan) {
        *self = *self % rhs;
    }
}

impl Rem<TimeSpan> for NonZeroTimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn rem(self, rhs: TimeSpan) -> TimeSpan {
        self.checked_rem_span(rhs)
            .expect("divide by zero error when dividing span by span")
    }
}

impl Rem<NonZeroTimeSpan> for NonZeroTimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn rem(self, rhs: NonZeroTimeSpan) -> TimeSpan {
        self.rem_span(rhs)
    }
}

impl Mul<u64> for TimeSpan {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: u64) -> Self {
        self.checked_mul(rhs)
            .expect("overflow when multiplying span by scalar")
    }
}

impl Mul<TimeSpan> for u64 {
    type Output = TimeSpan;

    #[inline(always)]
    fn mul(self, rhs: TimeSpan) -> TimeSpan {
        rhs * self
    }
}

impl MulAssign<u64> for TimeSpan {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: u64) {
        *self = *self * rhs;
    }
}

impl Div<u64> for TimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn div(self, rhs: u64) -> Self {
        self.checked_div(rhs)
            .expect("divide by zero error when dividing span by scalar")
    }
}

impl DivAssign<u64> for TimeSpan {
    #[inline(always)]
    fn div_assign(&mut self, rhs: u64) {
        *self = *self / rhs;
    }
}

impl Rem<u64> for TimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn rem(self, rhs: u64) -> Self {
        self.checked_rem(rhs)
            .expect("divide by zero error when dividing span by scalar")
    }
}

impl RemAssign<u64> for TimeSpan {
    #[inline(always)]
    fn rem_assign(&mut self, rhs: u64) {
        *self = *self % rhs;
    }
}

impl Mul<u64> for NonZeroTimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn mul(self, rhs: u64) -> TimeSpan {
        self.checked_mul(rhs)
            .expect("overflow when multiplying span by scalar")
    }
}

impl Mul<NonZeroTimeSpan> for u64 {
    type Output = TimeSpan;

    #[inline(always)]
    fn mul(self, rhs: NonZeroTimeSpan) -> TimeSpan {
        rhs * self
    }
}

impl Div<u64> for NonZeroTimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn div(self, rhs: u64) -> TimeSpan {
        self.checked_div(rhs)
            .expect("divide by zero error when dividing span by scalar")
    }
}

impl Rem<u64> for NonZeroTimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn rem(self, rhs: u64) -> TimeSpan {
        self.checked_rem(rhs)
            .expect("divide by zero error when dividing span by scalar")
    }
}

impl Mul<NonZeroU64> for TimeSpan {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: NonZeroU64) -> Self {
        self.checked_mul(rhs.get())
            .expect("overflow when multiplying span by scalar")
    }
}

impl Mul<TimeSpan> for NonZeroU64 {
    type Output = TimeSpan;

    #[inline(always)]
    fn mul(self, rhs: TimeSpan) -> TimeSpan {
        rhs * self
    }
}

impl MulAssign<NonZeroU64> for TimeSpan {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: NonZeroU64) {
        *self = *self * rhs;
    }
}

impl Div<NonZeroU64> for TimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn div(self, rhs: NonZeroU64) -> Self {
        self.div(rhs)
    }
}

impl DivAssign<NonZeroU64> for TimeSpan {
    #[inline(always)]
    fn div_assign(&mut self, rhs: NonZeroU64) {
        *self = *self / rhs;
    }
}

impl Rem<NonZeroU64> for TimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn rem(self, rhs: NonZeroU64) -> Self {
        self.rem(rhs)
    }
}

impl RemAssign<NonZeroU64> for TimeSpan {
    #[inline(always)]
    fn rem_assign(&mut self, rhs: NonZeroU64) {
        *self = *self % rhs;
    }
}

impl Mul<NonZeroU64> for NonZeroTimeSpan {
    type Output = NonZeroTimeSpan;

    #[inline(always)]
    fn mul(self, rhs: NonZeroU64) -> NonZeroTimeSpan {
        self.checked_mul_non_zero(rhs)
            .expect("overflow when multiplying span by scalar")
    }
}

impl Mul<NonZeroTimeSpan> for NonZeroU64 {
    type Output = NonZeroTimeSpan;

    #[inline(always)]
    fn mul(self, rhs: NonZeroTimeSpan) -> NonZeroTimeSpan {
        rhs * self
    }
}

impl Div<NonZeroU64> for NonZeroTimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn div(self, rhs: NonZeroU64) -> TimeSpan {
        self.div(rhs)
    }
}

impl Rem<NonZeroU64> for NonZeroTimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn rem(self, rhs: NonZeroU64) -> TimeSpan {
        self.rem(rhs)
    }
}

/// This trait adds methods to integers to convert values into `TimeSpan`s.
pub trait TimeSpanNumExt {
    /// Convert integer value into `TimeSpan` with that amount of nanoseconds.
    fn nanoseconds(self) -> TimeSpan;

    /// Convert integer value into `TimeSpan` with that amount of microseconds.
    fn microseconds(self) -> TimeSpan;

    /// Convert integer value into `TimeSpan` with that amount of milliseconds.
    fn milliseconds(self) -> TimeSpan;

    /// Convert integer value into `TimeSpan` with that amount of seconds.
    fn seconds(self) -> TimeSpan;

    /// Convert integer value into `TimeSpan` with that amount of minutes.
    fn minutes(self) -> TimeSpan;

    /// Convert integer value into `TimeSpan` with that amount of hours.
    fn hours(self) -> TimeSpan;

    /// Convert integer value into `TimeSpan` with that amount of days.
    fn days(self) -> TimeSpan;
}

/// This trait adds methods to non-zero integers to convert values into `NonZeroTimeSpan`s.
pub trait NonZeroTimeSpanNumExt {
    /// Convert integer value into `NonZeroTimeSpan` with that amount of nanoseconds.
    fn nanoseconds(self) -> NonZeroTimeSpan;

    /// Convert integer value into `NonZeroTimeSpan` with that amount of microseconds.
    fn microseconds(self) -> NonZeroTimeSpan;

    /// Convert integer value into `NonZeroTimeSpan` with that amount of milliseconds.
    fn milliseconds(self) -> NonZeroTimeSpan;

    /// Convert integer value into `NonZeroTimeSpan` with that amount of seconds.
    fn seconds(self) -> NonZeroTimeSpan;

    /// Convert integer value into `NonZeroTimeSpan` with that amount of minutes.
    fn minutes(self) -> NonZeroTimeSpan;

    /// Convert integer value into `NonZeroTimeSpan` with that amount of hours.
    fn hours(self) -> NonZeroTimeSpan;

    /// Convert integer value into `NonZeroTimeSpan` with that amount of days.
    fn days(self) -> NonZeroTimeSpan;
}

macro_rules! impl_for_int {
    ($($int:ty)*) => {
        $(
            impl_for_int!(@ $int);
        )*
    };

    (@ $int:ty) => {
        impl TimeSpanNumExt for $int {
            #[inline(always)]
            fn nanoseconds(self) -> TimeSpan {
                TimeSpan::NANOSECOND * u64::from(self)
            }
            #[inline(always)]
            fn microseconds(self) -> TimeSpan {
                TimeSpan::MICROSECOND * u64::from(self)
            }
            #[inline(always)]
            fn milliseconds(self) -> TimeSpan {
                TimeSpan::MILLISECOND * u64::from(self)
            }
            #[inline(always)]
            fn seconds(self) -> TimeSpan {
                TimeSpan::SECOND * u64::from(self)
            }
            #[inline(always)]
            fn minutes(self) -> TimeSpan {
                TimeSpan::MINUTE * u64::from(self)
            }
            #[inline(always)]
            fn hours(self) -> TimeSpan {
                TimeSpan::HOUR * u64::from(self)
            }
            #[inline(always)]
            fn days(self) -> TimeSpan {
                TimeSpan::DAY * u64::from(self)
            }
        }
    };
}

impl_for_int!(u8 u16 u32 u64);

macro_rules! impl_for_nonzero_int {
($($int:ty)*) => {
        $(
            impl_for_nonzero_int!(@ $int);
        )*
    };

    (@ $int:ty) => {

        impl NonZeroTimeSpanNumExt for $int {
            #[inline(always)]
            fn nanoseconds(self) -> NonZeroTimeSpan {
                NonZeroTimeSpan::NANOSECOND * NonZeroU64::from(self)
            }
            #[inline(always)]
            fn microseconds(self) -> NonZeroTimeSpan {
                NonZeroTimeSpan::MICROSECOND * NonZeroU64::from(self)
            }
            #[inline(always)]
            fn milliseconds(self) -> NonZeroTimeSpan {
                NonZeroTimeSpan::MILLISECOND * NonZeroU64::from(self)
            }
            #[inline(always)]
            fn seconds(self) -> NonZeroTimeSpan {
                NonZeroTimeSpan::SECOND * NonZeroU64::from(self)
            }
            #[inline(always)]
            fn minutes(self) -> NonZeroTimeSpan {
                NonZeroTimeSpan::MINUTE * NonZeroU64::from(self)
            }
            #[inline(always)]
            fn hours(self) -> NonZeroTimeSpan {
                NonZeroTimeSpan::HOUR * NonZeroU64::from(self)
            }
            #[inline(always)]
            fn days(self) -> NonZeroTimeSpan {
                NonZeroTimeSpan::DAY * NonZeroU64::from(self)
            }
        }
    };
}

impl_for_nonzero_int!(NonZeroU8 NonZeroU16 NonZeroU32 NonZeroU64);

#[test]
fn test_span_print() {
    assert_eq!("1d00:00", TimeSpan::DAY.to_string());
    assert_eq!("1:00:00", TimeSpan::HOUR.to_string());
    assert_eq!("1:00", TimeSpan::MINUTE.to_string());
    assert_eq!("1s", TimeSpan::SECOND.to_string());

    assert_eq!(
        "1:02:11",
        (TimeSpan::HOUR + 2 * TimeSpan::MINUTE + 11 * TimeSpan::SECOND).to_string()
    );

    assert_eq!(
        "2:11.011",
        (2 * TimeSpan::MINUTE + 11 * TimeSpan::SECOND + 11 * TimeSpan::MILLISECOND).to_string()
    );

    assert_eq!(
        "2:11.011",
        (2 * TimeSpan::MINUTE + 11 * TimeSpan::SECOND + 11 * TimeSpan::MILLISECOND).to_string()
    );
}

#[test]
fn test_span_parse() {
    assert_eq!("1d00:00".parse::<TimeSpan>().unwrap(), TimeSpan::DAY);
    assert_eq!("1:00:00".parse::<TimeSpan>().unwrap(), TimeSpan::HOUR);
    assert_eq!("1:00".parse::<TimeSpan>().unwrap(), TimeSpan::MINUTE);
    assert_eq!("1s".parse::<TimeSpan>().unwrap(), TimeSpan::SECOND);

    assert_eq!(
        "1:02:11".parse::<TimeSpan>().unwrap(),
        TimeSpan::HOUR + 2 * TimeSpan::MINUTE + 11 * TimeSpan::SECOND
    );

    assert_eq!(
        "2:11.011".parse::<TimeSpan>().unwrap(),
        2 * TimeSpan::MINUTE + 11 * TimeSpan::SECOND + 11 * TimeSpan::MILLISECOND
    );

    assert_eq!(
        "2:11.011".parse::<TimeSpan>().unwrap(),
        2 * TimeSpan::MINUTE + 11 * TimeSpan::SECOND + 11 * TimeSpan::MILLISECOND
    );
}
