//! This module contains `TimeSpan` type that represents durations.
//! The name `Duration` is not used to avoid confusion with std type.
//!
//!
//! Contains traits and functions to work with `TimeSpan`s.
//!

use core::{
    fmt::{self, Debug, Display},
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Range, Rem, RemAssign, Sub, SubAssign},
    str::FromStr,
    time::Duration,
};

const MAX_TIME_SPAN_STRING: usize = 48;

/// An interval in between time stamps.
/// This type is used to represent durations with nanosecond precision.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TimeSpan {
    nanos: i64,
}

impl TimeSpan {
    fn fmt(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self == Self::ZERO {
            f.write_str("0")
        } else {
            if self.is_negative() {
                f.write_str("-")?;
            }
            let mut span = self.abs();

            let days = span / Self::DAY;
            span %= Self::DAY;

            let hours = span / Self::HOUR;
            span %= Self::HOUR;

            let minutes = span / Self::MINUTE;
            span %= Self::MINUTE;

            let seconds = span / Self::SECOND;
            span %= Self::SECOND;

            let millis = span / Self::MILLISECOND;
            span %= Self::MILLISECOND;

            let micros = span / Self::MICROSECOND;
            span %= Self::MICROSECOND;

            let nanos = span / Self::NANOSECOND;

            if days > 0 || hours > 0 || minutes > 0 {
                if days > 0 {
                    write!(f, "{days}d")?;
                }

                if days > 0 {
                    write!(f, "{hours:02}:")?;
                } else if hours > 0 {
                    write!(f, "{hours}:")?;
                }

                if days > 0 || hours > 0 {
                    write!(f, "{minutes:02}")?;
                } else if minutes > 0 {
                    write!(f, "{minutes}")?;
                }

                if nanos > 0 {
                    write!(f, ":{seconds:02}.{millis:03}{micros:03}{nanos:03}")
                } else if micros > 0 {
                    write!(f, ":{seconds:02}.{millis:03}{micros:03}")
                } else if millis > 0 {
                    write!(f, ":{seconds:02}.{millis:03}")
                } else if seconds > 0 || days == 0 {
                    write!(f, ":{seconds:02}")
                } else {
                    Ok(())
                }
            } else if seconds > 0 {
                if nanos > 0 {
                    write!(f, "{seconds}.{millis:03}{micros:03}{nanos:03}s")
                } else if micros > 0 {
                    write!(f, "{seconds}.{millis:03}{micros:03}s")
                } else if millis > 0 {
                    write!(f, "{seconds}.{millis:03}s")
                } else {
                    write!(f, "{seconds}s")
                }
            } else if millis > 0 {
                if nanos > 0 {
                    write!(f, "{millis}.{micros:03}{nanos:03}ms")
                } else if micros > 0 {
                    write!(f, "{millis}.{micros:03}ms")
                } else {
                    write!(f, "{millis}ms")
                }
            } else if micros > 0 {
                if nanos > 0 {
                    write!(f, "{micros}.{nanos:03}us")
                } else {
                    write!(f, "{micros}us")
                }
            } else {
                write!(f, "{nanos}ns")
            }
        }
    }

    fn fmt_full(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_negative() {
            f.write_str("-")?;
        }
        let mut span = self.abs();
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
            "{days:01}d{hours:02}:{minutes:02}:{seconds:02}.{nanos:09}"
        )
    }

    fn fmt_nanos(self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}ns", self.nanos)
    }

    /// Longest possible time span in displayed format is
    /// `-106751d23:59:59.999999999`
    const MAX_DISPLAY_LENGTH: usize = 26;

    /// Buffer enough to hold any time span in displayed format.
    pub const DISPLAY_BUFFER: [u8; Self::MAX_DISPLAY_LENGTH] = [0; Self::MAX_DISPLAY_LENGTH];

    /// Formats this time span into a buffer.
    /// Returns a string slice that contains the formatted time span.
    ///
    /// `&mut Self::DISPLAY_BUFFER` can be used as second argument.
    pub fn display_to_buffer(self, buf: &mut [u8; Self::MAX_DISPLAY_LENGTH]) -> &mut str {
        #![allow(clippy::missing_panics_doc)] // False positive. Panics is not possible here.

        use std::io::Write;
        let mut write = &mut buf[..];

        match write!(&mut write, "{self}") {
            Ok(()) => {
                let unused = write.len();
                let used = buf.len() - unused;
                str::from_utf8_mut(&mut buf[..used]).expect("Valid UTF-8 written to buffer")
            }
            Err(_) => unreachable!("Buffer is large enough to hold any time span"),
        }
    }
}

impl Debug for TimeSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            self.fmt_nanos(f)
        } else {
            Self::fmt(*self, f)
        }
    }
}

impl Display for TimeSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            self.fmt_full(f)
        } else {
            Self::fmt(*self, f)
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
    HoursOutOfBound { hours: i64 },
    MinutesOutOfBound { minutes: i64 },
    SecondsOutOfBound { seconds: i64 },
}

impl fmt::Display for TimeSpanParseErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonASCII => f.write_str("Time spans encoded in strings are always ASCII"),
            Self::StringTooLarge { len } => {
                write!(
                    f,
                    "Valid time span string may never exceed {MAX_TIME_SPAN_STRING} bytes. String is {len}"
                )
            }
            Self::IntParseError { .. } => f.write_str("Failed to parse integer"),
            Self::UnexpectedDelimiter { delim, pos } => {
                write!(f, "Unexpected delimiter '{delim}' at {pos}")
            }
            Self::UnexpectedEndOfString => f.write_str("Unexpected end of string"),
            Self::UnexpectedSuffix => {
                f.write_str("Unexpected suffix. Only `s`, `ms` and `us` suffixes are supported")
            }
            Self::HoursOutOfBound { hours } => {
                write!(f, "Hours must be in range 0-23 when days are specified. Value at hours position is '{hours}'")
            }
            Self::MinutesOutOfBound { minutes } => {
                write!(f, "Minutes must be in range 0-59 when hours are specified. Value at minutes position is '{minutes}'")
            }
            Self::SecondsOutOfBound { seconds } => {
                write!(
                    f,
                    "Seconds must be in range 0-59 when minutes are specified. Value at seconds position is '{seconds}'"
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
        let seconds: i64 = self
            .seconds
            .map_or(Ok(0), |r| s[r].trim().parse())
            .map_err(|source| TimeSpanParseErr::IntParseError { source })?;

        if self.minutes.is_some() && seconds > 59 {
            return Err(TimeSpanParseErr::SecondsOutOfBound { seconds });
        }

        let minutes: i64 = self
            .minutes
            .map_or(Ok(0), |r| s[r].trim().parse())
            .map_err(|source| TimeSpanParseErr::IntParseError { source })?;

        if self.hours.is_some() && minutes > 59 {
            return Err(TimeSpanParseErr::MinutesOutOfBound { minutes });
        }

        let hours: i64 = self
            .hours
            .map_or(Ok(0), |r| s[r].trim().parse())
            .map_err(|source| TimeSpanParseErr::IntParseError { source })?;

        if self.days.is_some() && hours > 23 {
            return Err(TimeSpanParseErr::HoursOutOfBound { hours });
        }

        let days: i64 = self
            .days
            .map_or(Ok(0), |r| s[r].trim().parse())
            .map_err(|source| TimeSpanParseErr::IntParseError { source })?;

        let fract: i64 = self
            .fract
            .map_or(Ok(0), |r| s[r].trim().parse())
            .map_err(|source| TimeSpanParseErr::IntParseError { source })?;

        let micros = match self.denom {
            denom @ 0..6 => fract * 10i64.pow(6 - denom),
            6 => fract,
            denom @ 7.. => fract / 10i64.pow(denom - 6),
        };

        Ok(days * TimeSpan::DAY
            + hours * TimeSpan::HOUR
            + minutes * TimeSpan::MINUTE
            + seconds * TimeSpan::SECOND
            + micros * TimeSpan::MICROSECOND)
    }
}

impl FromStr for TimeSpan {
    type Err = TimeSpanParseErr;

    #[allow(clippy::too_many_lines)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        #![allow(clippy::cast_possible_truncation)]

        if !s.is_ascii() {
            return Err(TimeSpanParseErr::NonASCII);
        }

        if s.len() > MAX_TIME_SPAN_STRING {
            return Err(TimeSpanParseErr::StringTooLarge { len: s.len() });
        }

        let mut separators =
            s.match_indices(|c: char| !c.is_ascii_digit() && !c.is_ascii_whitespace());

        match separators.next() {
            Some((dh, "d" | "D" | "t" | "T")) => match separators.next() {
                Some((hm, ":")) => match separators.next() {
                    None => Ranges {
                        days: Some(0..dh),
                        hours: Some(dh + 1..hm),
                        minutes: Some(hm + 1..s.len()),
                        seconds: None,
                        fract: None,
                        denom: 0,
                    },
                    Some((ms, ":")) => match separators.next() {
                        None => Ranges {
                            days: Some(0..dh),
                            hours: Some(dh + 1..hm),
                            minutes: Some(hm + 1..ms),
                            seconds: Some(ms + 1..s.len()),
                            fract: None,
                            denom: 0,
                        },
                        Some((sf, ".")) => {
                            if let Some((pos, delim)) = separators.next() {
                                return Err(TimeSpanParseErr::UnexpectedDelimiter {
                                    delim: delim.chars().next().unwrap(),
                                    pos,
                                });
                            }
                            Ranges {
                                days: Some(0..dh),
                                hours: Some(dh + 1..hm),
                                minutes: Some(hm + 1..ms),
                                seconds: Some(ms + 1..sf),
                                fract: Some(sf + 1..s.len().min(sf + 21)),
                                denom: (s.len() - sf - 1).min(20) as u32,
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
            Some((hms, ":")) => match separators.next() {
                Some((ms, ":")) => match separators.next() {
                    Some((sf, ".")) => {
                        if let Some((pos, delim)) = separators.next() {
                            return Err(TimeSpanParseErr::UnexpectedDelimiter {
                                delim: delim.chars().next().unwrap(),
                                pos,
                            });
                        }
                        Ranges {
                            days: None,
                            hours: Some(0..hms),
                            minutes: Some(hms + 1..ms),
                            seconds: Some(ms + 1..sf),
                            fract: Some(sf + 1..s.len().min(sf + 21)),
                            denom: (s.len() - sf - 1).min(20) as u32,
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
                    if let Some((pos, delim)) = separators.next() {
                        return Err(TimeSpanParseErr::UnexpectedDelimiter {
                            delim: delim.chars().next().unwrap(),
                            pos,
                        });
                    }
                    Ranges {
                        days: None,
                        hours: None,
                        minutes: Some(0..hms),
                        seconds: Some(hms + 1..sf),
                        fract: Some(sf + 1..s.len()),
                        denom: (s.len() - sf - 1).min(20) as u32,
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
                if let Some((pos, delim)) = separators.next() {
                    return Err(TimeSpanParseErr::UnexpectedDelimiter {
                        delim: delim.chars().next().unwrap(),
                        pos,
                    });
                }
                Ranges {
                    days: None,
                    hours: None,
                    minutes: None,
                    seconds: Some(0..sf),
                    fract: Some(sf + 1..s.len()),
                    denom: (s.len() - sf - 1).min(20) as u32,
                }
            }

            Some((suffix, "s")) => {
                if s[suffix..].trim() != "s" {
                    return Err(TimeSpanParseErr::UnexpectedSuffix);
                }

                let seconds: i64 = s[..suffix]
                    .trim()
                    .parse()
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;
                return Ok(seconds * Self::SECOND);
            }

            Some((suffix, "m")) => {
                if s[suffix..].trim() != "ms" {
                    return Err(TimeSpanParseErr::UnexpectedSuffix);
                }

                let millis: i64 = s[..suffix]
                    .trim()
                    .parse()
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;
                return Ok(millis * Self::MILLISECOND);
            }

            Some((suffix, "u")) => {
                if s[suffix..].trim() != "us" {
                    return Err(TimeSpanParseErr::UnexpectedSuffix);
                }

                let micros: i64 = s[..suffix]
                    .trim()
                    .parse()
                    .map_err(|source| TimeSpanParseErr::IntParseError { source })?;
                return Ok(micros * Self::MICROSECOND);
            }

            None => {
                let seconds: i64 = s
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
            let mut buf = Self::DISPLAY_BUFFER;
            let s = self.display_to_buffer(&mut buf);

            serializer.serialize_str(s)
        } else {
            serializer.serialize_i64(self.nanos)
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

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v > i64::MAX as u64 {
                    return Err(E::custom("Time span is too large to fit into i64"));
                }

                Ok(TimeSpan { nanos: v as i64 })
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(TimeSpan { nanos: v })
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

impl TimeSpan {
    /// Converts [`Duration`] into [`TimeSpan`].
    ///
    /// # Panics
    ///
    /// Panics if the duration is out of bounds for `TimeSpan`.
    /// Which is longer than `i64::MAX` nanoseconds.
    #[inline]
    #[must_use]
    pub fn from_duration(duration: Duration) -> Self {
        let nanos = duration.as_nanos();
        TimeSpan {
            nanos: nanos
                .try_into()
                .expect("Duration is out of bounds for TimeSpan"),
        }
    }
}

impl TimeSpan {
    /// Converts [`TimeSpan`] into [`Duration`].
    ///
    /// # Panics
    ///
    /// Panics if the time span is negative.
    #[inline]
    #[must_use]
    pub fn into_duration(self) -> Duration {
        assert!(
            !self.is_negative(),
            "Cannot convert negative TimeSpan into Duration"
        );

        Duration::new(
            #[allow(clippy::cast_sign_loss)]
            {
                self.as_seconds() as u64
            },
            #[allow(clippy::cast_sign_loss)]
            {
                (self.as_nanos() % 1_000_000_000) as u32
            },
        )
    }
}

impl TimeSpan {
    /// Zero time span.
    ///
    /// Represents duration between equal time points.
    pub const ZERO: Self = TimeSpan { nanos: 0 };

    /// Minimal possible time span.
    pub const MIN: Self = TimeSpan {
        // Use negative i64::MAX to represent minimal possible time span.
        // Which is equal to i64::MIN + 1.
        // This is to avoid overflow when negating or taking absolute value.
        nanos: -i64::MAX,
    };

    /// Maximal possible time span.
    pub const MAX: Self = TimeSpan {
        // Use i64::MAX to represent maximal possible time span.
        nanos: i64::MAX,
    };

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
    pub const YEAR: Self = Self::GREGORIAN_YEAR;

    /// Constructs time span from number of nanoseconds.
    #[inline]
    #[must_use]
    pub const fn new(nanos: i64) -> TimeSpan {
        TimeSpan { nanos }
    }

    /// Returns number of nanoseconds in this time span.
    #[inline]
    #[must_use]
    pub const fn as_nanos(self) -> i64 {
        self.nanos
    }

    /// Returns number of microseconds this value represents.
    #[inline]
    #[must_use]
    pub const fn as_micros(self) -> i64 {
        self.nanos / Self::MICROSECOND.nanos
    }

    /// Returns number of whole milliseconds this value represents.
    #[inline]
    #[must_use]
    pub const fn as_millis(self) -> i64 {
        self.nanos / Self::MILLISECOND.nanos
    }

    /// Returns number of whole seconds this value represents.
    #[inline]
    #[must_use]
    pub const fn as_seconds(self) -> i64 {
        self.nanos / Self::SECOND.nanos
    }

    /// Returns number of whole minutes this value represents.
    #[inline]
    #[must_use]
    pub const fn as_minutes(self) -> i64 {
        self.nanos / Self::MINUTE.nanos
    }

    /// Returns number of whole hours this value represents.
    #[inline]
    #[must_use]
    pub const fn as_hours(self) -> i64 {
        self.nanos / Self::HOUR.nanos
    }

    /// Returns number of whole days this value represents.
    #[inline]
    #[must_use]
    pub const fn as_days(self) -> i64 {
        self.nanos / Self::DAY.nanos
    }

    /// Returns number of whole weeks this value represents.
    #[inline]
    #[must_use]
    pub const fn as_weeks(self) -> i64 {
        self.nanos / Self::WEEK.nanos
    }

    /// Returns number of seconds as floating point value.
    /// This function should be used for small-ish spans when high precision is not required.
    #[inline]
    #[must_use]
    pub fn as_secs_f32(self) -> f32 {
        #![allow(clippy::cast_precision_loss)]

        self.nanos as f32 / Self::SECOND.nanos as f32
    }

    /// Returns number of seconds as high precision floating point value.
    #[inline]
    #[must_use]
    pub fn as_secs_f64(self) -> f64 {
        #![allow(clippy::cast_precision_loss)]

        self.nanos as f64 / Self::SECOND.nanos as f64
    }

    /// Returns absolute value of this time span.
    #[inline]
    #[must_use]
    pub fn abs(self) -> TimeSpan {
        TimeSpan {
            nanos: self.nanos.abs(),
        }
    }

    /// Returns true if this time span is negative.
    #[inline]
    #[must_use]
    pub fn is_negative(self) -> bool {
        self.nanos < 0
    }

    /// Returns sum of two time spans unless it overflows.
    #[inline]
    #[must_use]
    pub const fn checked_add(self, span: TimeSpan) -> Option<TimeSpan> {
        match self.nanos.checked_add(span.nanos) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    /// Returns checked difference of two time spans unless it overflows.
    #[inline]
    #[must_use]
    pub const fn checked_sub(self, span: TimeSpan) -> Option<TimeSpan> {
        match self.nanos.checked_sub(span.nanos) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    /// Returns difference of two time spans unless it overflows.
    #[inline]
    #[must_use]
    pub const fn checked_mul(self, value: i64) -> Option<TimeSpan> {
        match self.nanos.checked_mul(value) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    /// Returns quotient of time span and a scalar value unless it overflows or denominator is zero.
    #[inline]
    #[must_use]
    pub const fn checked_div(self, value: i64) -> Option<TimeSpan> {
        match self.nanos.checked_div(value) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    /// Returns quotient of time span and a scalar value unless it overflows or denominator is zero.
    #[inline]
    #[must_use]
    pub const fn checked_div_span(self, span: TimeSpan) -> Option<i64> {
        match self.nanos.checked_div(span.nanos) {
            None => None,
            Some(value) => Some(value),
        }
    }

    /// Returns quotient of time span divided by a non-zero time span.
    #[inline]
    #[must_use]
    pub const fn div_span(self, span: TimeSpan) -> i64 {
        self.nanos / span.nanos
    }

    /// Returns remainder of time span divided by a scalar value unless it overflows or denominator is zero.
    #[inline]
    #[must_use]
    pub const fn checked_rem(self, value: i64) -> Option<TimeSpan> {
        match self.nanos.checked_rem(value) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    /// Returns remainder of time span divided by a scalar value.
    #[inline]
    #[must_use]
    pub const fn rem(self, value: i64) -> TimeSpan {
        let nanos = self.nanos % value;
        TimeSpan { nanos }
    }

    /// Returns remainder of time span divided by a non-zero time span unless it overflows or denominator is zero.
    #[inline]
    #[must_use]
    pub const fn checked_rem_span(self, span: TimeSpan) -> Option<TimeSpan> {
        match self.nanos.checked_rem(span.nanos) {
            None => None,
            Some(nanos) => Some(TimeSpan { nanos }),
        }
    }

    /// Returns remainder of time span divided by a non-zero time span.
    #[inline]
    #[must_use]
    pub const fn rem_span(self, span: TimeSpan) -> TimeSpan {
        let nanos = self.nanos % span.nanos;
        TimeSpan { nanos }
    }

    /// Returns time span from hours, minutes and seconds.
    #[inline]
    #[must_use]
    pub const fn hms(hours: i64, minutes: i64, seconds: i64) -> TimeSpan {
        TimeSpan {
            nanos: hours * Self::HOUR.nanos
                + minutes * Self::MINUTE.nanos
                + seconds * Self::SECOND.nanos,
        }
    }

    /// Returns time span from days, hours, minutes and seconds.
    #[inline]
    #[must_use]
    pub const fn dhms(days: i64, hours: i64, minutes: i64, seconds: i64) -> TimeSpan {
        TimeSpan {
            nanos: days * Self::DAY.nanos
                + hours * Self::HOUR.nanos
                + minutes * Self::MINUTE.nanos
                + seconds * Self::SECOND.nanos,
        }
    }

    /// Returns time span from years, days, hours, minutes and seconds.
    ///
    /// This function uses gregorian year length of 365.2425 days.
    #[inline]
    #[must_use]
    pub const fn ydhms(years: i64, days: i64, hours: i64, minutes: i64, seconds: i64) -> TimeSpan {
        TimeSpan {
            nanos: years * Self::GREGORIAN_YEAR.nanos
                + days * Self::DAY.nanos
                + hours * Self::HOUR.nanos
                + minutes * Self::MINUTE.nanos
                + seconds * Self::SECOND.nanos,
        }
    }
}

impl Add<TimeSpan> for TimeSpan {
    type Output = Self;

    #[inline]
    fn add(self, rhs: TimeSpan) -> Self {
        self.checked_add(rhs).expect("overflow when adding spans")
    }
}

impl AddAssign<TimeSpan> for TimeSpan {
    fn add_assign(&mut self, rhs: TimeSpan) {
        *self = *self + rhs;
    }
}

impl Sub<TimeSpan> for TimeSpan {
    type Output = TimeSpan;

    #[inline]
    fn sub(self, rhs: TimeSpan) -> Self {
        self.checked_sub(rhs)
            .expect("overflow when subtracting spans")
    }
}

impl SubAssign<TimeSpan> for TimeSpan {
    fn sub_assign(&mut self, rhs: TimeSpan) {
        *self = *self - rhs;
    }
}

impl Div<TimeSpan> for TimeSpan {
    type Output = i64;

    #[inline]
    fn div(self, rhs: TimeSpan) -> i64 {
        self.checked_div_span(rhs)
            .expect("divide by zero error when dividing span by span")
    }
}

impl Rem<TimeSpan> for TimeSpan {
    type Output = TimeSpan;

    #[inline]
    fn rem(self, rhs: TimeSpan) -> TimeSpan {
        self.checked_rem_span(rhs)
            .expect("divide by zero error when dividing span by span")
    }
}

impl RemAssign<TimeSpan> for TimeSpan {
    #[inline]
    fn rem_assign(&mut self, rhs: TimeSpan) {
        *self = *self % rhs;
    }
}

impl Mul<i64> for TimeSpan {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: i64) -> Self {
        self.checked_mul(rhs)
            .expect("overflow when multiplying span by scalar")
    }
}

impl Mul<TimeSpan> for i64 {
    type Output = TimeSpan;

    #[inline]
    fn mul(self, rhs: TimeSpan) -> TimeSpan {
        rhs * self
    }
}

impl MulAssign<i64> for TimeSpan {
    #[inline]
    fn mul_assign(&mut self, rhs: i64) {
        *self = *self * rhs;
    }
}

impl Div<i64> for TimeSpan {
    type Output = TimeSpan;

    #[inline]
    fn div(self, rhs: i64) -> Self {
        self.checked_div(rhs)
            .expect("divide by zero error when dividing span by scalar")
    }
}

impl DivAssign<i64> for TimeSpan {
    #[inline]
    fn div_assign(&mut self, rhs: i64) {
        *self = *self / rhs;
    }
}

impl Rem<i64> for TimeSpan {
    type Output = TimeSpan;

    #[inline]
    fn rem(self, rhs: i64) -> Self {
        self.checked_rem(rhs)
            .expect("divide by zero error when dividing span by scalar")
    }
}

impl RemAssign<i64> for TimeSpan {
    #[inline]
    fn rem_assign(&mut self, rhs: i64) {
        *self = *self % rhs;
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

macro_rules! impl_for_int {
    ($($int:ty)*) => {
        $(
            impl_for_int!(@ $int);
        )*
    };

    (@ $int:ty) => {
        impl TimeSpanNumExt for $int {
            #[inline]
            fn nanoseconds(self) -> TimeSpan {
                TimeSpan::NANOSECOND * i64::from(self)
            }
            #[inline]
            fn microseconds(self) -> TimeSpan {
                TimeSpan::MICROSECOND * i64::from(self)
            }
            #[inline]
            fn milliseconds(self) -> TimeSpan {
                TimeSpan::MILLISECOND * i64::from(self)
            }
            #[inline]
            fn seconds(self) -> TimeSpan {
                TimeSpan::SECOND * i64::from(self)
            }
            #[inline]
            fn minutes(self) -> TimeSpan {
                TimeSpan::MINUTE * i64::from(self)
            }
            #[inline]
            fn hours(self) -> TimeSpan {
                TimeSpan::HOUR * i64::from(self)
            }
            #[inline]
            fn days(self) -> TimeSpan {
                TimeSpan::DAY * i64::from(self)
            }
        }
    };
}

impl_for_int!(i64);

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
