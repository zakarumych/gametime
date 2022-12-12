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
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Sub, SubAssign},
};

/// An interval in between time stamps.
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
                write!(f, "{}d", days)?;
            }

            if span >= Self::HOUR {
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

impl TimeSpan {
    pub const ZERO: Self = TimeSpan { nanos: 0 };
    pub const NANOSECOND: Self = TimeSpan { nanos: 1 };
    pub const MICROSECOND: Self = TimeSpan { nanos: 1_000 };
    pub const MILLISECOND: Self = TimeSpan { nanos: 1_000_000 };
    pub const SECOND: Self = TimeSpan {
        nanos: 1_000_000_000,
    };
    pub const MINUTE: Self = TimeSpan {
        nanos: 60_000_000_000,
    };
    pub const HOUR: Self = TimeSpan {
        nanos: 3_600_000_000_000,
    };
    pub const DAY: Self = TimeSpan {
        nanos: 86_400_000_000_000,
    };

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
    pub const NANOSECOND: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(1) },
    };
    pub const MICROSECOND: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(1_000) },
    };
    pub const MILLISECOND: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(1_000_000) },
    };
    pub const SECOND: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(1_000_000_000) },
    };
    pub const MINUTE: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(60_000_000_000) },
    };
    pub const HOUR: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(3_600_000_000_000) },
    };
    pub const DAY: Self = NonZeroTimeSpan {
        nanos: unsafe { NonZeroU64::new_unchecked(86_400_000_000_000) },
    };

    /// Constructs time span from number of nanoseconds.
    #[inline(always)]
    pub const fn new(nanos: NonZeroU64) -> NonZeroTimeSpan {
        NonZeroTimeSpan { nanos }
    }

    /// Returns number of nanoseconds in this time span.
    #[inline(always)]
    pub const fn as_nanos(&self) -> NonZeroU64 {
        self.nanos
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
        self.checked_mul(rhs.into())
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
        self.checked_div(rhs.into())
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
        self.checked_rem(rhs.into())
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
        self.checked_mul(rhs.into())
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
        self.checked_div(rhs.into())
            .expect("divide by zero error when dividing span by scalar")
    }
}

impl Rem<u64> for NonZeroTimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn rem(self, rhs: u64) -> TimeSpan {
        self.checked_rem(rhs.into())
            .expect("divide by zero error when dividing span by scalar")
    }
}

impl Mul<NonZeroU64> for TimeSpan {
    type Output = Self;

    #[inline(always)]
    fn mul(self, rhs: NonZeroU64) -> Self {
        self.checked_mul(rhs.get().into())
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
        self.div(rhs.into())
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
        self.rem(rhs.into())
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
        self.checked_mul_non_zero(rhs.into())
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
        self.div(rhs.into())
    }
}

impl Rem<NonZeroU64> for NonZeroTimeSpan {
    type Output = TimeSpan;

    #[inline(always)]
    fn rem(self, rhs: NonZeroU64) -> TimeSpan {
        self.rem(rhs.into())
    }
}

pub trait TimeSpanNumExt {
    fn nanoseconds(self) -> TimeSpan;
    fn microseconds(self) -> TimeSpan;
    fn milliseconds(self) -> TimeSpan;
    fn seconds(self) -> TimeSpan;
    fn minutes(self) -> TimeSpan;
    fn hours(self) -> TimeSpan;
    fn days(self) -> TimeSpan;
}

pub trait NonZeroTimeSpanNumExt {
    fn nanoseconds(self) -> NonZeroTimeSpan;
    fn microseconds(self) -> NonZeroTimeSpan;
    fn milliseconds(self) -> NonZeroTimeSpan;
    fn seconds(self) -> NonZeroTimeSpan;
    fn minutes(self) -> NonZeroTimeSpan;
    fn hours(self) -> NonZeroTimeSpan;
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
