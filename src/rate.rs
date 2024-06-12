//! Contains definitions to work with arbitrary clocks
//! that handle time spans and time stamps
//! where actual passing time spans are provided externally.

use core::num::NonZeroU64;

use crate::{gcd, span::TimeSpan, stamp::TimeStamp, ClockStep, Frequency, FrequencyTicker};

/// Time measuring device.
/// Uses system monotonic clock counter
/// and yields `ClockStep`s for each step.
///
/// Rate can be set to control the speed of the clock.
#[derive(Clone)] // Not Copy to avoid accidental copying.
pub struct ClockRate {
    now: TimeStamp,
    nom: u64,
    denom: NonZeroU64,
    until_next: u64,
}

impl Default for ClockRate {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}

impl ClockRate {
    /// Returns new `ClockRate` instance.
    #[inline(always)]
    pub fn new() -> Self {
        ClockRate {
            now: TimeStamp::start(),
            nom: 1,
            denom: NonZeroU64::new(1).unwrap(),
            until_next: 0,
        }
    }

    /// Resets the clock.
    /// Sets start to the given instant.
    /// And set clocks to start.
    #[inline(always)]
    pub fn reset(&mut self) {
        self.now = TimeStamp::start();
        self.until_next = 0;
    }

    /// Sets current clock time to given time stamp.
    pub fn set_now(&mut self, now: TimeStamp) {
        self.now = now;
    }

    /// Sets current clock time to given time stamp.
    pub fn with_now(mut self, now: TimeStamp) -> Self {
        self.set_now(now);
        self
    }

    /// Returns time stamp corresponding to "now" of the last step.
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
    pub fn with_rate(mut self, rate: f32) -> Self {
        self.set_rate(rate);
        self
    }

    /// Returns current rate.
    pub fn rate(&self) -> f64 {
        self.nom as f64 / self.denom.get() as f64
    }

    /// Set rate to specified ratio.
    pub fn set_rate_ratio(&mut self, nom: u64, denom: NonZeroU64) {
        self.nom = nom;
        self.denom = denom;
    }

    /// Set rate to specified ratio.
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
    pub fn step(&mut self, span: TimeSpan) -> ClockStep {
        let nanos = span.as_nanos();
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

        let clock_span = TimeSpan::new(clock_nanos);
        self.now += clock_span;

        ClockStep {
            now: self.now,
            step: clock_span,
        }
    }

    pub fn ticker(&self, freq: Frequency) -> FrequencyTicker {
        let gcd1 = gcd(self.nom, freq.period.get());
        let nom = self.nom / gcd1;
        let period = freq.period.get() / gcd1;

        let gcd2 = gcd(freq.count, self.denom.get());
        let count = freq.count / gcd2;
        let denom = self.denom.get() / gcd2;

        FrequencyTicker::new(
            Frequency {
                count: nom * count,
                period: NonZeroU64::new(denom * period).unwrap(),
            },
            self.now,
        )
    }
}

fn rate2ratio(rate: f32) -> (u64, NonZeroU64) {
    let (n, d) = ftor(rate);
    (n, NonZeroU64::new(d).unwrap())
}

fn ftor(value: f32) -> (u64, u64) {
    const EPSILON: f32 = 1e-6;
    const MAX_ITER: usize = 50;

    let v = value.max(0.0);

    let mut d = 1;
    let mut n = v;

    for _ in 0..MAX_ITER {
        let f = n.fract();
        if f < EPSILON {
            break;
        }

        if d > u32::MAX as u64 {
            break;
        }

        d = (d as f32 / f).ceil() as u64;
        n = v * d as f32;
    }

    let z = n.trunc() as u64;

    let g = gcd(z, d);
    return (z / g, d / g);
}


#[test]
fn test_large() {
    fn check_ftor(v: f32) {
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
