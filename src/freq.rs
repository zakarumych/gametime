//! Contains types and functions to work with frequencies.

use core::{convert::TryInto, num::NonZeroU64};

use crate::{
    span::{NonZeroTimeSpan, TimeSpan, TimeSpanNumExt},
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
        (self.count * span.as_nanos()) / self.period
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

pub struct FrequencyTicker {
    freq: Frequency,
    until_next: NonZeroU64,
    now: TimeStamp,
}

impl FrequencyTicker {
    pub fn new(freq: Frequency, now: TimeStamp) -> Self {
        FrequencyTicker {
            freq,
            until_next: freq.period,
            now,
        }
    }

    pub fn now(&self) -> TimeStamp {
        self.now
    }

    pub fn ticks(&mut self, now: TimeStamp) -> FrequencyTickerIter {
        let span = (now - self.now).as_nanos() * self.freq.count;
        self.now = now;

        let iter = FrequencyTickerIter {
            now: self.now,
            span,
            freq: self.freq,
            until_next: self.until_next,
        };

        if iter.span < self.until_next.get() {
            self.until_next = unsafe {
                // # Safety
                // a < b hence b - a > 0
                NonZeroU64::new(self.until_next.get() - iter.span).unwrap_unchecked()
            };
        } else {
            self.until_next = iter.excess();
        }

        iter
    }

    #[inline(always)]
    pub fn tick_count(&mut self, now: TimeStamp) -> u64 {
        self.ticks(now).ticks()
    }

    /// Advances ticker and calls provided closure for each tick.
    #[inline(always)]
    pub fn tick_with(&mut self, now: TimeStamp, f: impl FnMut(TimeStamp)) {
        self.ticks(now).for_each(f)
    }

    pub fn step_ticks(&mut self, step: TimeSpan) -> FrequencyTickerIter {
        let span = step.as_nanos() * self.freq.count;
        self.now += step;

        let iter = FrequencyTickerIter {
            now: self.now,
            span,
            freq: self.freq,
            until_next: self.until_next,
        };

        if iter.span < self.until_next.get() {
            self.until_next = unsafe {
                // # Safety
                // a < b hence b - a > 0
                NonZeroU64::new(self.until_next.get() - iter.span).unwrap_unchecked()
            };
        } else {
            self.until_next = iter.excess();
        }
        iter
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
    span: u64,
    freq: Frequency,
    until_next: NonZeroU64,
}

impl FrequencyTickerIter {
    #[inline]
    pub fn ticks(&self) -> u64 {
        if self.span < self.until_next.get() {
            return 0;
        }

        let span = self.span - self.until_next.get();
        1 + span / self.freq.period
    }

    #[inline]
    fn excess(&self) -> NonZeroU64 {
        debug_assert!(self.span >= self.until_next.get());
        let span = self.span - self.until_next.get();
        unsafe {
            // # Safety
            // b = x % a < a
            // hence a - b > 0
            NonZeroU64::new_unchecked(self.freq.period.get() - span % self.freq.period)
        }
    }
}

impl Iterator for FrequencyTickerIter {
    type Item = TimeStamp;

    #[inline]
    fn next(&mut self) -> Option<TimeStamp> {
        if self.span < self.until_next.get() {
            self.span = 0;
            return None;
        }

        let next = self.now + ((self.until_next.get() - 1) / self.freq.count + 1).nanoseconds();

        self.span -= self.until_next.get();
        self.until_next = self.freq.period;
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

    for _ in 0..10 {
        now += TimeSpan::NANOSECOND;
        assert_eq!(ticker.tick_count(now), 0);

        now += TimeSpan::NANOSECOND;
        assert_eq!(ticker.tick_count(now), 0);

        now += TimeSpan::NANOSECOND;
        assert_eq!(ticker.tick_count(now), 0);

        now += TimeSpan::NANOSECOND;
        assert_eq!(ticker.tick_count(now), 1);

        now += TimeSpan::NANOSECOND;
        assert_eq!(ticker.tick_count(now), 0);

        now += TimeSpan::NANOSECOND;
        assert_eq!(ticker.tick_count(now), 0);

        now += TimeSpan::NANOSECOND;
        assert_eq!(ticker.tick_count(now), 1);

        now += TimeSpan::NANOSECOND;
        assert_eq!(ticker.tick_count(now), 0);

        now += TimeSpan::NANOSECOND;
        assert_eq!(ticker.tick_count(now), 0);

        now += TimeSpan::NANOSECOND;
        assert_eq!(ticker.tick_count(now), 1);
    }
}
