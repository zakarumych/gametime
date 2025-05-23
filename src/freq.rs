//! Contains types and functions to work with frequencies.

use core::{convert::TryInto, iter::FusedIterator, num::NonZeroU64, ops};

use crate::{
    gcd,
    span::{NonZeroTimeSpan, TimeSpan},
    stamp::TimeStamp,
    ClockStep,
};

#[cfg(feature = "serde")]
use serde::ser::SerializeTupleStruct;

/// Represents frequency as a rational number.
#[derive(Clone, Copy)]
pub struct Frequency {
    /// Number of periods in one cycle.
    pub count: u64,

    /// Number of nanoseconds in one cycle.
    pub cycle: NonZeroU64,
}

impl Frequency {
    /// Creates new frequency from number of periods in one cycle and cycle time span.
    /// Returns `None` if cycle is zero.
    #[inline]
    #[must_use]
    pub fn try_new(count: u64, cycle: TimeSpan) -> Option<Self> {
        cycle
            .try_into()
            .ok()
            .map(|cycle| Frequency::new(count, cycle))
    }

    /// Creates new frequency from number of periods in one cycle and cycle time span.
    /// Uses non-zero time span for cycle.
    #[must_use]
    pub fn new(count: u64, cycle: NonZeroTimeSpan) -> Self {
        let gcd = gcd(count, cycle.as_nanos().get());
        let count = count / gcd;
        let cycle_nanos = cycle.as_nanos().get() / gcd;

        match NonZeroU64::new(cycle_nanos) {
            None => unreachable!(),
            Some(cycle) => Frequency { count, cycle },
        }
    }

    /// Creates frequency from number of Hertz.
    #[inline]
    #[must_use]
    pub fn from_hz(value: u64) -> Self {
        Frequency::new(value, NonZeroTimeSpan::SECOND)
    }

    /// Creates frequency from number of `KiloHertz`.
    #[inline]
    #[must_use]
    pub fn from_khz(value: u64) -> Self {
        Frequency::new(value, NonZeroTimeSpan::MILLISECOND)
    }

    /// Creates frequency from number of `MegaHertz`.
    #[inline]
    #[must_use]
    pub fn from_mhz(value: u64) -> Self {
        Frequency::new(value, NonZeroTimeSpan::MICROSECOND)
    }

    /// Creates frequency from number of `GigaHertz`.
    #[inline]
    #[must_use]
    pub fn from_ghz(value: u64) -> Self {
        Frequency::new(value, NonZeroTimeSpan::NANOSECOND)
    }

    /// Element is a nanosecond divided by count of periods in one cycle.
    /// Return number of elements in the given time span.
    #[inline]
    fn elements(&self, span: TimeSpan) -> Elements {
        Elements(span.as_nanos() * self.count)
    }

    /// Returns the number of periods in the given time span.
    #[inline]
    #[must_use]
    pub fn periods_in_span(&self, span: TimeSpan) -> u64 {
        self.periods_in_elements(self.elements(span)).0
    }

    /// Returns the elements of a single period.
    #[inline]
    fn period_elements(&self) -> Elements {
        Elements(self.cycle.get())
    }

    /// Returns the elements of given number of periods.
    #[inline]
    fn periods_elements(&self, count: u64) -> Elements {
        Elements(self.cycle.get() * count)
    }

    /// Returns the number of periods fit in specified elements count.
    /// And remaining elements.
    #[inline]
    fn periods_in_elements(&self, span: Elements) -> (u64, Elements) {
        let periods = span.0 / self.cycle.get();
        let remaining = Elements(span.0 % self.cycle.get());
        (periods, remaining)
    }

    /// Returns the number of elements until next tick.
    #[inline]
    fn until_next(&self, span: Elements) -> Elements {
        Elements(self.cycle.get() - span.0 % self.cycle)
    }

    /// Span that contains the given number of frequency elements.
    #[inline]
    fn span_fitting_elements(&self, span: Elements) -> Option<TimeSpan> {
        match (span.0, self.count) {
            (0, 0) => Some(TimeSpan::ZERO),
            (_, 0) => None, // Element is infinite
            (span, count) => Some(TimeSpan::new(span.div_ceil(count))),
        }
    }

    // /// Span contained within the frequency elements.
    // #[inline]
    // fn span_fits_in_elements(&self, span: Elements) -> Option<TimeSpan> {
    //     match (span.0, self.count) {
    //         (0, 0) => Some(TimeSpan::ZERO),
    //         (_, 0) => None, // Element is infinite
    //         (span, count) => Some(TimeSpan::new(span / count)),
    //     }
    // }

    /// Returns new ticker with this frequency and given start time stamp.
    #[inline]
    #[must_use]
    pub fn ticker(&self, start: TimeStamp) -> FrequencyTicker {
        FrequencyTicker::new(*self, start)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Frequency {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            serializer.serialize_str(&format!("{}/{} Hz", self.count, self.cycle))
        } else {
            let mut serializer = serializer.serialize_tuple_struct("Frequency", 2)?;
            serializer.serialize_field(&self.count)?;
            serializer.serialize_field(&self.cycle)?;
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

                    let cycle = const { NonZeroU64::new(1).unwrap() };
                    return Ok(Frequency { count, cycle });
                }

                Some((count, s)) => {
                    let count = count.trim();
                    let count = count.parse().map_err(serde::de::Error::custom)?;
                    let cycle = s
                        .strip_suffix("Hz")
                        .ok_or_else(|| serde::de::Error::custom("Wrong frequency format"))?;
                    let cycle = cycle.trim();
                    let cycle = cycle.parse().map_err(serde::de::Error::custom)?;

                    return Ok(Frequency { count, cycle });
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
                    let cycle = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::custom("Frequency is empty"))?;
                    Ok(Frequency { count, cycle })
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

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Elements(self.0 + rhs.0)
    }
}

impl ops::AddAssign for Elements {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl ops::Sub for Elements {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Elements(self.0 - rhs.0)
    }
}

impl ops::SubAssign for Elements {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl ops::Rem for Elements {
    type Output = Self;

    #[inline]
    fn rem(self, rhs: Self) -> Self {
        Elements(self.0 % rhs.0)
    }
}

impl ops::RemAssign for Elements {
    #[inline]
    fn rem_assign(&mut self, rhs: Self) {
        self.0 %= rhs.0;
    }
}

/// Ticker with the given frequency.
///
/// Creates tick iterators on each step that emits `ClockStep`s with exactly given frequency.
pub struct FrequencyTicker {
    freq: Frequency,

    /// Number of elements until next tick.
    until_next: Elements,

    /// Last tick stamp.
    now: TimeStamp,
}

impl FrequencyTicker {
    /// Creates new ticker with given frequency and start timestamp.
    #[inline]
    #[must_use]
    pub fn new(freq: Frequency, now: TimeStamp) -> Self {
        FrequencyTicker::with_delay(freq, 0, now)
    }

    /// Creates new ticker with given frequency and delay in number of tick periods.
    #[inline]
    #[must_use]
    pub fn with_delay(freq: Frequency, periods: u64, now: TimeStamp) -> Self {
        FrequencyTicker {
            freq,
            until_next: freq.periods_elements(1 + periods),
            now,
        }
    }

    /// Returns next timestamp when next tick will happen.
    #[inline]
    #[must_use]
    pub fn next_tick(&self) -> Option<TimeStamp> {
        Some(self.now + self.freq.span_fitting_elements(self.until_next)?)
    }

    /// Advances ticker forward for `span` and returns iterator over ticks
    /// since last advancement.
    #[inline]
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
    #[inline]
    pub fn tick_count(&mut self, step: TimeSpan) -> u64 {
        self.ticks(step).ticks()
    }

    /// Advances ticker forward for `step` and calls provided closure with `ClockStep`s.
    #[inline]
    pub fn with_ticks(&mut self, step: TimeSpan, f: impl FnMut(ClockStep)) {
        self.ticks(step).for_each(f);
    }

    /// Returns frequency of the ticker.
    #[inline]
    #[must_use]
    pub fn frequency(&self) -> Frequency {
        self.freq
    }

    /// Sets new frequency of the ticker.
    ///
    /// If `clip_period` is true, then next tick will happen at least in the frequency period.
    /// If `clip_period` is false, then next tick will happen at the same time as before setting the frequency.
    #[inline]
    pub fn set_frequency(&mut self, freq: Frequency, clip_period: bool) {
        self.freq = freq;
        if clip_period {
            let period = freq.period_elements();
            if self.until_next > period {
                self.until_next = period;
            }
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
    #[must_use]
    pub fn ticks(&self) -> u64 {
        if self.span < self.until_next {
            return 0;
        }

        let span = self.span - self.until_next;
        1 + self.freq.periods_in_elements(span).0
    }
}

impl Iterator for FrequencyTickerIter {
    type Item = ClockStep;

    #[inline]
    fn next(&mut self) -> Option<ClockStep> {
        if self.accumulated > 0 {
            self.accumulated -= 1;
            return Some(ClockStep {
                now: self.now,
                step: TimeSpan::ZERO,
            });
        }

        if self.span < self.until_next {
            return None;
        }

        // This may not be None.
        // If freq.count is 0, then span is 0.
        // And this may only be called if until_next is 0.
        // But for better code-gen it uses unwrap_or to not generate panic code.
        let next = self
            .freq
            .span_fitting_elements(self.until_next)
            .unwrap_or(TimeSpan::ZERO);

        // Advance a whole number of nanoseconds in elements.
        let advance = self.freq.elements(next);

        debug_assert!(
            advance <= self.span,
            "Span cannot be greater than total span left in iterator"
        );
        debug_assert!(
            advance >= self.until_next,
            "Span cannot be less then span until next tick"
        );

        let (periods, remaining) = self.freq.periods_in_elements(advance - self.until_next);

        self.accumulated = periods;

        self.until_next = self.freq.period_elements() - remaining;

        self.span -= advance;
        self.now += next;

        Some(ClockStep {
            now: self.now,
            step: next,
        })
    }
}

impl FusedIterator for FrequencyTickerIter {}

/// This trait adds methods to integers to convert values into `Frequency`s.
pub trait FrequencyNumExt {
    /// Convert integer value into `Frequency` with that amount of Herz.
    fn hz(self) -> Frequency;

    /// Convert integer value into `Frequency` with that amount of `KiloHerz`.
    fn khz(self) -> Frequency;

    /// Convert integer value into `Frequency` with that amount of `MegaHerz`.
    fn mhz(self) -> Frequency;

    /// Convert integer value into `Frequency` with that amount of `GigaHerz`.
    fn ghz(self) -> Frequency;
}

impl FrequencyNumExt for u64 {
    #[inline]
    fn hz(self) -> Frequency {
        Frequency::from_hz(self)
    }

    #[inline]
    fn khz(self) -> Frequency {
        Frequency::from_khz(self)
    }

    #[inline]
    fn mhz(self) -> Frequency {
        Frequency::from_mhz(self)
    }

    #[inline]
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

    assert_eq!(ticker.tick_count(TimeSpan::NANOSECOND * 10), 3);

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

    const DELAY: u64 = 12;

    let mut ticker = FrequencyTicker::with_delay(
        Frequency::new(3, NonZeroU64::new(10).unwrap().nanoseconds()),
        DELAY,
        TimeStamp::start(),
    );

    assert_eq!(0, ticker.tick_count(TimeSpan::NANOSECOND * 40));

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
            ClockStep {
                now: TimeStamp::start() + TimeSpan::new(333_333_334),
                step: TimeSpan::new(333_333_334),
            },
            ClockStep {
                now: TimeStamp::start() + TimeSpan::new(666_666_667),
                step: TimeSpan::new(333_333_333),
            },
            ClockStep {
                now: TimeStamp::start() + TimeSpan::new(1_000_000_000),
                step: TimeSpan::new(333_333_333),
            },
        ]
    );
}
