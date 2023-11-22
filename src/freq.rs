//! Contains types and functions to work with frequencies.

use core::{convert::TryInto, num::NonZeroU64, ops};

use crate::{
    span::{NonZeroTimeSpan, TimeSpan},
    stamp::TimeStamp,
};

#[cfg(feature = "serde")]
use serde::ser::SerializeTupleStruct;

/// Represents frequency.
/// Able to accurately represent any rational frequency.
#[derive(Clone, Copy)]
pub struct Frequency {
    count: u64,
    period: NonZeroU64,
}

impl Frequency {
    #[inline(always)]
    pub fn try_new(count: u64, period: TimeSpan) -> Option<Self> {
        period
            .try_into()
            .ok()
            .map(|period| Frequency::new(count, period))
    }

    pub fn new(count: u64, period: NonZeroTimeSpan) -> Self {
        let period = period.as_nanos();
        let shift = count.trailing_zeros().min(period.trailing_zeros());

        Frequency {
            count: count >> shift,
            period: unsafe {
                // # Safety
                // Shifts only trailing zeros.
                // denominator is not 0.
                NonZeroU64::new(period.get() >> shift).unwrap_unchecked()
            },
        }
    }

    #[inline(always)]
    pub fn from_hz(value: u64) -> Self {
        Frequency::new(value, NonZeroTimeSpan::SECOND)
    }

    #[inline(always)]
    pub fn from_khz(value: u64) -> Self {
        Frequency::new(value, NonZeroTimeSpan::MILLISECOND)
    }

    #[inline(always)]
    pub fn from_mhz(value: u64) -> Self {
        Frequency::new(value, NonZeroTimeSpan::MICROSECOND)
    }

    #[inline(always)]
    pub fn from_ghz(value: u64) -> Self {
        Frequency::new(value, NonZeroTimeSpan::NANOSECOND)
    }

    #[inline(always)]
    pub fn periods_in(&self, span: TimeSpan) -> u64 {
        self.periods_in_elements(self.elements(span))
    }

    #[inline(always)]
    fn elements(&self, span: TimeSpan) -> Elements {
        Elements(span.as_nanos() * self.count)
    }

    #[inline(always)]
    fn periods_in_elements(&self, span: Elements) -> u64 {
        span.0 / self.period
    }

    #[inline(always)]
    fn period(&self) -> Elements {
        Elements(self.period.get())
    }

    #[inline(always)]
    fn periods(&self, count: u64) -> Elements {
        Elements(self.period.get() * count)
    }

    #[inline(always)]
    fn until_next(&self, span: Elements) -> Elements {
        Elements(self.period.get() - span.0 % self.period)
    }

    /// Span of time in frequency elements rounded up.
    /// Avoid accumulating rounding errors.
    #[inline(always)]
    fn span(&self, span: Elements) -> Option<TimeSpan> {
        match (span.0, self.count) {
            (0, 0) => Some(TimeSpan::ZERO),
            (_, 0) => None,
            (span, count) => Some(TimeSpan::new((span + (count - 1)) / count)),
        }
    }

    #[inline(always)]
    pub fn ticker(&self, now: TimeStamp) -> FrequencyTicker {
        FrequencyTicker::new(*self, now)
    }

    #[inline(always)]
    pub fn ticker_with_delay(&self, now: TimeStamp, periods: u64) -> FrequencyTicker {
        FrequencyTicker::with_delay(*self, periods, now)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Frequency {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&format!("{}/{} Hz", self.count, self.period))
        } else {
            let mut serializer = serializer.serialize_tuple_struct("Frequency", 2)?;
            serializer.serialize_field(&self.count)?;
            serializer.serialize_field(&self.period)?;
            serializer.end()
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Frequency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let s = String::deserialize(deserializer)?;

            match s.split_once("/") {
                None => {
                    let count = s
                        .strip_suffix("Hz")
                        .ok_or_else(|| serde::de::Error::custom("Wrong frequency format"))?;
                    let count = count.trim();
                    let count = count.parse().map_err(serde::de::Error::custom)?;

                    let period = NonZeroU64::new(1).unwrap();
                    return Ok(Frequency { count, period });
                }

                Some((count, s)) => {
                    let count = count.trim();
                    let count = count.parse().map_err(serde::de::Error::custom)?;
                    let period = s
                        .strip_suffix("Hz")
                        .ok_or_else(|| serde::de::Error::custom("Wrong frequency format"))?;
                    let period = period.trim();
                    let period = period.parse().map_err(serde::de::Error::custom)?;

                    return Ok(Frequency { count, period });
                }
            }
        } else {
            struct FrequencyVisitor;

            impl<'de> serde::de::Visitor<'de> for FrequencyVisitor {
                type Value = Frequency;

                fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                    formatter.write_str("a tuple of 2 elements")
                }

                fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
                where
                    A: serde::de::SeqAccess<'de>,
                {
                    let count = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::custom("Frequency is empty"))?;
                    let period = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::custom("Frequency is empty"))?;
                    Ok(Frequency { count, period })
                }
            }

            deserializer.deserialize_tuple_struct("Frequency", 2, FrequencyVisitor)
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
struct Elements(u64);

impl ops::Add for Elements {
    type Output = Self;

    #[inline(always)]
    fn add(self, rhs: Self) -> Self::Output {
        Elements(self.0 + rhs.0)
    }
}

impl ops::AddAssign for Elements {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl ops::Sub for Elements {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Elements(self.0 - rhs.0)
    }
}

impl ops::SubAssign for Elements {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

pub struct FrequencyTicker {
    freq: Frequency,

    /// Number of elements until next tick.
    until_next: Elements,

    /// Last tick stamp.
    now: TimeStamp,
}

impl FrequencyTicker {
    /// Creates new ticker with given frequency and start timestamp.
    #[inline(always)]
    pub fn new(freq: Frequency, now: TimeStamp) -> Self {
        FrequencyTicker::with_delay(freq, 1, now)
    }

    /// Creates new ticker with given frequency and delay in number of tick periods.
    #[inline(always)]
    pub fn with_delay(freq: Frequency, periods: u64, now: TimeStamp) -> Self {
        let periods = if freq.count == 0 {
            periods.max(1)
        } else {
            periods
        };
        FrequencyTicker {
            freq,
            until_next: freq.periods(periods),
            now,
        }
    }

    /// Returns next timestamp when next tick will happen.
    #[inline(always)]
    pub fn next_tick(&self) -> Option<TimeStamp> {
        Some(self.now + self.freq.span(self.until_next)?)
    }

    /// Advances ticker forward for `span` and returns iterator over ticks
    /// since last advancement.
    #[inline(always)]
    pub fn ticks(&mut self, step: TimeSpan) -> FrequencyTickerIter {
        let span = self.freq.elements(step);

        let iter = FrequencyTickerIter {
            span,
            freq: self.freq,
            until_next: self.until_next,
            accumulated: 0,
            now: self.now,
        };

        if span >= self.until_next {
            self.until_next = self.freq.until_next(span - self.until_next);
        } else {
            self.until_next -= span;
        }

        self.now += step;

        iter
    }

    /// Advances ticker forward to `now` and returns number of ticks
    /// since last advancement.
    #[inline(always)]
    pub fn tick_count(&mut self, step: TimeSpan) -> u64 {
        self.ticks(step).ticks()
    }

    /// Advances ticker forward for `step` and calls provided closure with ticks
    /// since last advancement.
    #[inline(always)]
    pub fn with_ticks(&mut self, step: TimeSpan, f: impl FnMut(TimeStamp)) {
        self.ticks(step).for_each(f)
    }

    /// Returns current frequency of the ticker.
    #[inline(always)]
    pub fn frequency(&self) -> Frequency {
        self.freq
    }

    /// Sets new frequency of the ticker.
    #[inline(always)]
    pub fn set_frequency(&mut self, freq: Frequency) {
        self.freq = freq;
        let period = freq.period();
        if self.until_next > period {
            self.until_next = period;
        }
    }
}

/// Iterator over ticks from `FrequencyTicker`.
pub struct FrequencyTickerIter {
    span: Elements,
    freq: Frequency,
    until_next: Elements,
    accumulated: u64,
    now: TimeStamp,
}

impl FrequencyTickerIter {
    /// Returns number of ticks this iterator will produce.
    #[inline]
    pub fn ticks(&self) -> u64 {
        if self.span < self.until_next {
            return 0;
        }

        let span = self.span - self.until_next;
        1 + self.freq.periods_in_elements(span)
    }
}

impl Iterator for FrequencyTickerIter {
    type Item = TimeStamp;

    #[inline]
    fn next(&mut self) -> Option<TimeStamp> {
        if self.accumulated > 0 {
            self.accumulated -= 1;
            return Some(self.now);
        }

        if self.span < self.until_next {
            return None;
        }

        // TimeSpan::ZERO is used because if `self.count` is zero then
        // `self.until_next` is zero too.
        // Otherwise `self.span` would be less than `self.until_next`
        // because it is produced by mutliplying with `self.count`
        let next = self.freq.span(self.until_next).unwrap_or(TimeSpan::ZERO);

        // Tick span elements
        let next_elements = self.freq.elements(next);

        debug_assert!(
            next_elements <= self.span,
            "Span cannot be greater than total span left in iterator"
        );
        debug_assert!(
            next_elements >= self.until_next,
            "Span cannot be less then span until next tick"
        );

        self.until_next += self.freq.period();
        if self.until_next < next_elements {
            self.accumulated = self
                .freq
                .periods_in_elements(next_elements - self.until_next);

            self.until_next = self.freq.until_next(next_elements - self.until_next);
        } else {
            self.until_next -= next_elements;
        }

        self.span -= next_elements;
        self.now += next;
        Some(self.now)
    }
}

/// This trait adds methods to integers to convert values into `Frequency`s.
pub trait FrequencyNumExt {
    /// Convert integer value into `Frequency` with that amount of Herz.
    fn hz(self) -> Frequency;

    /// Convert integer value into `Frequency` with that amount of KiloHerz.
    fn khz(self) -> Frequency;

    /// Convert integer value into `Frequency` with that amount of MegaHerz.
    fn mhz(self) -> Frequency;

    /// Convert integer value into `Frequency` with that amount of GigaHerz.
    fn ghz(self) -> Frequency;
}

macro_rules! impl_for_int {
    ($($int:ty)*) => {
        $(
            impl_for_int!(@ $int);
        )*
    };

    (@ $int:ty) => {
        impl FrequencyNumExt for $int {
            #[inline(always)]
            fn hz(self) -> Frequency {
                Frequency::from_hz(u64::from(self))
            }

            #[inline(always)]
            fn khz(self) -> Frequency {
                Frequency::from_khz(u64::from(self))
            }

            #[inline(always)]
            fn mhz(self) -> Frequency {
                Frequency::from_mhz(u64::from(self))
            }

            #[inline(always)]
            fn ghz(self) -> Frequency {
                Frequency::from_ghz(u64::from(self))
            }
        }
    };
}

// impl_for_int!(u8 u16 u32 u64);

impl FrequencyNumExt for u64 {
    #[inline(always)]
    fn hz(self) -> Frequency {
        Frequency::from_hz(self)
    }

    #[inline(always)]
    fn khz(self) -> Frequency {
        Frequency::from_khz(self)
    }

    #[inline(always)]
    fn mhz(self) -> Frequency {
        Frequency::from_mhz(self)
    }

    #[inline(always)]
    fn ghz(self) -> Frequency {
        Frequency::from_ghz(self)
    }
}

#[test]
fn test_freq_ticker() {
    use crate::span::NonZeroTimeSpanNumExt;

    let mut ticker = FrequencyTicker::new(
        Frequency::new(3, NonZeroU64::new(10).unwrap().nanoseconds()),
        TimeStamp::start(),
    );

    ticker.tick_count(TimeSpan::NANOSECOND * 10);
    let ticks = [0, 0, 0, 1, 0, 0, 1, 0, 0, 1];

    for _ in 0..10 {
        for tick in ticks {
            assert_eq!(ticker.tick_count(TimeSpan::NANOSECOND), tick);
        }
    }
}

#[test]
fn test_freq_ticker_delay() {
    use crate::span::NonZeroTimeSpanNumExt;

    const DELAY: u64 = 11;

    let mut ticker = FrequencyTicker::with_delay(
        Frequency::new(3, NonZeroU64::new(10).unwrap().nanoseconds()),
        DELAY,
        TimeStamp::start(),
    );

    let mut delay = DELAY;
    while delay > 0 {
        ticker.tick_count(TimeSpan::NANOSECOND * 10);
        delay -= 3;
    }

    let ticks = [0, 0, 0, 1, 0, 0, 1, 0, 0, 1];

    for _ in 0..10 {
        for tick in ticks {
            assert_eq!(ticker.tick_count(TimeSpan::NANOSECOND), tick);
        }
    }
}

#[test]
fn test_freq_ticker_next_tick() {
    use crate::span::NonZeroTimeSpanNumExt;

    let mut ticker = FrequencyTicker::new(
        Frequency::new(3, NonZeroU64::new(10).unwrap().nanoseconds()),
        TimeStamp::start(),
    );

    let ticks = [0, 0, 0, 1, 0, 0, 1, 0, 0, 1];

    let mut next_tick = ticker.next_tick().unwrap();

    for _ in 0..100 {
        for tick in ticks {
            assert_eq!(ticker.tick_count(TimeSpan::NANOSECOND), tick);

            if tick > 0 {
                assert_eq!(next_tick, ticker.now);
                next_tick = ticker.next_tick().unwrap();
            } else {
                assert_eq!(next_tick, ticker.next_tick().unwrap());
            }
        }
    }
}

#[test]
fn test_hz() {
    let mut freq = Frequency::from_hz(3).ticker(TimeStamp::start());

    let ticks = freq.ticks(TimeSpan::SECOND).collect::<Vec<_>>();
    assert_eq!(
        ticks,
        vec![
            TimeStamp::start() + TimeSpan::new(333_333_334),
            TimeStamp::start() + TimeSpan::new(666_666_667),
            TimeStamp::start() + TimeSpan::new(1_000_000_000),
        ]
    );
}
