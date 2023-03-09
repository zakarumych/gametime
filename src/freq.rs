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
    fn period_excess(&self, span: Elements) -> Elements {
        Elements(self.period.get() - span.0 % self.period)
    }

    #[inline(always)]
    fn span(&self, span: Elements) -> Option<TimeSpan> {
        match (span.0, self.count) {
            (0, 0) => Some(TimeSpan::ZERO),
            (_, 0) => None,
            (span, count) => Some(TimeSpan::new((span + (count - 1)) / count)),
        }
    }

    #[inline(always)]
    pub fn ticker(&self, start: TimeStamp) -> FrequencyTicker {
        FrequencyTicker::new(*self, start)
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

impl ops::Sub for Elements {
    type Output = Self;

    #[inline(always)]
    fn sub(self, rhs: Self) -> Self::Output {
        Elements(self.0 - rhs.0)
    }
}

pub struct FrequencyTicker {
    freq: Frequency,

    /// Number of elements until next tick.
    until_next: Elements,
    now: TimeStamp,
}

impl FrequencyTicker {
    #[inline(always)]
    pub fn new(freq: Frequency, start: TimeStamp) -> Self {
        FrequencyTicker::with_delay(freq, start, 0)
    }

    #[inline(always)]
    pub fn with_delay(freq: Frequency, start: TimeStamp, periods: u64) -> Self {
        let periods = if freq.count == 0 {
            periods.max(1)
        } else {
            periods
        };
        FrequencyTicker {
            freq,
            until_next: freq.periods(periods),
            now: start,
        }
    }

    #[inline(always)]
    pub fn now(&self) -> TimeStamp {
        self.now
    }

    /// Returns iterator over ticks that happened since `self.now()` until `now`.
    /// Sets `self.now()` to `now` even if iterator is not consumed.
    #[inline(always)]
    pub fn ticks(&mut self, now: TimeStamp) -> FrequencyTickerIter {
        let span = self.freq.elements(now - self.now);

        let iter = FrequencyTickerIter {
            now: self.now,
            span,
            freq: self.freq,
            until_next: self.until_next,
        };
        self.until_next = iter.excess();
        self.now = now;
        iter
    }

    /// Returns iterator over ticks that happened since `self.now()` until `self.now() + span`.
    /// Sets `self.now()` to `self.now() + span` even if iterator is not consumed.
    #[inline(always)]
    pub fn step_ticks(&mut self, step: TimeSpan) -> FrequencyTickerIter {
        let now = self.now + step;
        self.ticks(now)
    }

    /// Returns number of ticks that happened since `self.now()` until `now`.
    /// Sets `self.now()` to `now`.
    #[inline(always)]
    pub fn tick_count(&mut self, now: TimeStamp) -> u64 {
        self.ticks(now).ticks()
    }

    /// Calls closure with ticks that happened since `self.now()` until `now`.
    /// Sets `self.now()` to `now`.
    #[inline(always)]
    pub fn tick_with(&mut self, now: TimeStamp, f: impl FnMut(TimeStamp)) {
        self.ticks(now).for_each(f)
    }

    /// Returns next timestamp when next tick will happen.
    #[inline(always)]
    pub fn next_tick(&mut self) -> Option<TimeStamp> {
        Some(self.now + self.freq.span(self.until_next)?)
    }

    #[inline(always)]
    pub fn step_tick_count(&mut self, step: TimeSpan) -> u64 {
        self.step_ticks(step).ticks()
    }

    /// Advances ticker and calls provided closure for each tick.
    #[inline(always)]
    pub fn step_tick_with(&mut self, step: TimeSpan, f: impl FnMut(TimeStamp)) {
        self.step_ticks(step).for_each(f)
    }
}

pub struct FrequencyTickerIter {
    now: TimeStamp,
    span: Elements,
    freq: Frequency,
    until_next: Elements,
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

    /// Returns elements excess that will left after iteration.
    #[inline]
    fn excess(&self) -> Elements {
        if self.span >= self.until_next {
            let span = self.span - self.until_next;
            self.freq.period_excess(span)
        } else {
            self.until_next - self.span
        }
    }
}

impl Iterator for FrequencyTickerIter {
    type Item = TimeStamp;

    #[inline]
    fn next(&mut self) -> Option<TimeStamp> {
        if self.span < self.until_next {
            return None;
        }

        // TimeSpan::ZERO is used because if `self.count` is zero then
        // `self.until_next` is zero too.
        // Otherwise `self.span` would be less than `self.until_next`
        // because it is produced by mutliplying with `self.count`
        let next = self.now + self.freq.span(self.until_next).unwrap_or(TimeSpan::ZERO);

        self.span = self.span - self.until_next;
        self.until_next = self.freq.period();
        Some(next)
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

impl_for_int!(u8 u16 u32 u64);

#[test]
fn test_freq_ticker() {
    use crate::span::NonZeroTimeSpanNumExt;

    let mut ticker = FrequencyTicker::new(
        Frequency::new(3, NonZeroU64::new(10).unwrap().nanoseconds()),
        TimeStamp::start(),
    );

    let mut now = TimeStamp::start();

    let ticks = [1, 0, 0, 0, 1, 0, 0, 1, 0, 0];

    for _ in 0..10 {
        for tick in ticks {
            assert_eq!(ticker.tick_count(now), tick);
            now += TimeSpan::NANOSECOND;
        }
    }
}

#[test]
fn test_freq_ticker_delay() {
    use crate::span::NonZeroTimeSpanNumExt;

    const DELAY: u64 = 123;

    let mut ticker = FrequencyTicker::with_delay(
        Frequency::new(3, NonZeroU64::new(10).unwrap().nanoseconds()),
        TimeStamp::start(),
        DELAY,
    );

    let mut now = TimeStamp::start();

    let ticks = [1, 0, 0, 0, 1, 0, 0, 1, 0, 0];
    let mut delay = DELAY;

    for _ in 0..100 {
        for mut tick in ticks {
            if delay >= tick {
                delay -= tick;
                tick = 0;
            }
            assert_eq!(ticker.tick_count(now), tick);
            now += TimeSpan::NANOSECOND;
        }
    }
}

#[test]
fn test_freq_ticker_next_tick() {
    use crate::span::NonZeroTimeSpanNumExt;

    const DELAY: u64 = 123;

    let mut ticker = FrequencyTicker::with_delay(
        Frequency::new(3, NonZeroU64::new(10).unwrap().nanoseconds()),
        TimeStamp::start(),
        DELAY,
    );

    let mut now = TimeStamp::start();

    let ticks = [1, 0, 0, 0, 1, 0, 0, 1, 0, 0];
    let mut delay = DELAY;
    let mut next_tick = ticker.next_tick();

    for _ in 0..100 {
        for mut tick in ticks {
            if delay >= tick {
                delay -= tick;
                tick = 0;
            }
            if tick > 0 {
                assert_eq!(next_tick.take(), Some(now));
            } else {
                if let Some(next_tick) = next_tick {
                    assert_eq!(Some(next_tick), ticker.next_tick());
                }
                next_tick = ticker.next_tick();
            }
            assert_eq!(ticker.tick_count(now), tick);
            now += TimeSpan::NANOSECOND;
        }
    }
}
