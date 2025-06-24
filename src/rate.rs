//! Contains definitions to work with arbitrary clocks
//! that handle time spans and time stamps
//! where actual passing time spans are provided externally.

use core::num::NonZeroU64;

use crate::{gcd, span::TimeSpan, stamp::TimeStamp, step::ClockStep, Frequency, FrequencyTicker};

/// Produces clock steps with given rate.
/// Uses external time span to advance the clock,
/// usually from some `Clock`.
pub struct ClockRate {
    now: TimeStamp,
    nom: u64,
    denom: NonZeroU64,
    until_next: u64,
}

impl Default for ClockRate {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl ClockRate {
    /// Returns new `ClockRate` instance.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        #![allow(clippy::missing_panics_doc)] // False positive. Panics is in const context.

        ClockRate {
            now: TimeStamp::start(),
            nom: 1,
            denom: const { NonZeroU64::new(1).unwrap() },
            until_next: 0,
        }
    }

    /// Resets the clock.
    /// Sets start to the given instant.
    /// And set clocks to start.
    #[inline]
    pub fn reset(&mut self) {
        self.now = TimeStamp::start();
        self.until_next = 0;
    }

    /// Sets current clock time to given time stamp.
    pub fn set_now(&mut self, now: TimeStamp) {
        self.now = now;
    }

    /// Sets current clock time to given time stamp.
    #[must_use]
    pub fn with_now(mut self, now: TimeStamp) -> Self {
        self.set_now(now);
        self
    }

    /// Returns time stamp corresponding to "now" of the last step.
    #[must_use]
    pub fn now(&self) -> TimeStamp {
        self.now
    }

    /// Set rate to specified float value.
    pub fn set_rate(&mut self, rate: f32) {
        let (nom, denom) = rate2ratio(rate);
        self.nom = nom;
        self.denom = denom;
    }

    /// Set rate to specified float value.
    #[must_use]
    pub fn with_rate(mut self, rate: f32) -> Self {
        self.set_rate(rate);
        self
    }

    /// Returns current rate as float value.
    /// May be approximated.
    #[must_use]
    pub fn rate(&self) -> f64 {
        #![allow(clippy::cast_precision_loss)] // Precision loss is acceptable here.
        self.nom as f64 / self.denom.get() as f64
    }

    /// Set rate to specified ratio.
    pub fn set_rate_ratio(&mut self, nom: u64, denom: NonZeroU64) {
        self.nom = nom;
        self.denom = denom;
    }

    /// Set rate to specified ratio.
    #[must_use]
    pub fn with_rate_ratio(mut self, nom: u64, denom: NonZeroU64) -> Self {
        self.set_rate_ratio(nom, denom);
        self
    }

    /// Returns current rate ratio.
    pub fn rate_ratio(&mut self) -> (u64, NonZeroU64) {
        (self.nom, self.denom)
    }

    /// Set rate to 0.
    pub fn pause(&mut self) {
        self.nom = 0;
    }

    /// Advances the clock by given time span and returns `ClockStep` result.
    /// with new time stamp and time span since previous step.
    ///
    /// # Panics
    ///
    /// Panics if the given time span is negative.
    pub fn step(&mut self, span: TimeSpan) -> ClockStep {
        #![allow(clippy::cast_sign_loss)] // Sign loss is not possible due to check.

        assert!(!span.is_negative(), "Negative time span is not allowed");

        let nanos = span.as_nanos() as u64;
        let nom_nanos = nanos * self.nom;

        if self.until_next > nom_nanos {
            // Same game nanosecond.
            self.until_next -= nom_nanos;
            return ClockStep {
                now: self.now,
                step: TimeSpan::ZERO,
            };
        }

        let clock_nanos = (nom_nanos - self.until_next) / self.denom;
        let nom_nanos_left = (nom_nanos - self.until_next) % self.denom;
        self.until_next = self.denom.get() - nom_nanos_left;

        debug_assert!(clock_nanos <= u64::MAX >> 1, "Clock nanoseconds overflow");

        #[allow(clippy::cast_possible_wrap)]
        let clock_span = TimeSpan::new(clock_nanos as i64);

        self.now += clock_span;

        ClockStep {
            now: self.now,
            step: clock_span,
        }
    }

    /// Creates a new `ClockRate` instance with frequency multiplied by this clock rate.
    #[must_use]
    pub fn ticker(&self, freq: Frequency) -> FrequencyTicker {
        let gcd1 = gcd(self.nom, freq.cycle.get());
        let gcd2 = gcd(freq.count, self.denom.get());

        let nom = self.nom / gcd1;
        let denom = self.denom.get() / gcd2;

        let count = freq.count / gcd2;
        let period = freq.cycle.get() / gcd1;

        let count = nom * count;

        match NonZeroU64::new(denom * period) {
            None => unreachable!(),
            Some(cycle) => FrequencyTicker::new(Frequency { count, cycle }, self.now),
        }
    }
}

fn rate2ratio(rate: f32) -> (u64, NonZeroU64) {
    let (n, d) = ftor(rate);
    (n, NonZeroU64::new(d).unwrap())
}

fn ftor(value: f32) -> (u64, u64) {
    #![allow(clippy::cast_sign_loss)] // False positive. Sign loss never occurs here, as values are positive.
    #![allow(clippy::cast_precision_loss)] // Precision loss is acceptable here.
    #![allow(clippy::cast_possible_truncation)] // Truncation is acceptable here.

    const EPSILON: f32 = 1e-6;
    const MAX_ITER: usize = 50;

    let value = value.max(0.0);

    if value == f32::INFINITY {
        return (1, 0);
    }

    if value > u64::MAX as f32 {
        // This is closest approximation.
        return (u64::MAX, 1);
    }

    let mut denom = 1;
    let mut nom = value;

    for _ in 0..MAX_ITER {
        let f = nom.fract();
        if f < EPSILON {
            break;
        }

        if denom > u64::from(u32::MAX) {
            break;
        }

        denom = (denom as f32 / f).ceil() as u64;

        let next = value * denom as f32;

        if next > u64::MAX as f32 {
            break;
        }

        nom = next;
    }

    let nom = nom.trunc() as u64;

    let g = gcd(nom, denom);
    (nom / g, denom / g)
}

#[test]
fn test_large() {
    fn check_ftor(v: f32) {
        #![allow(clippy::cast_precision_loss)]

        let (n, d) = ftor(v);
        let e = (v - (n as f32 / d as f32)).abs();
        assert!(e < 1e-6);
    }
    check_ftor(1.0);
    check_ftor(1.0 / 3.0);
    check_ftor(1.0 / 7.0);
    check_ftor(1.0 / 13.0);
    check_ftor(1.001);
    check_ftor(1234.1234);
}
