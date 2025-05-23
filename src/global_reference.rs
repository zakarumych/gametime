use std::{
    sync::OnceLock,
    time::{Duration, Instant},
};

static GLOBAL_REFERENCE: OnceLock<Instant> = OnceLock::new();

fn get_or_init(value: Instant) -> Instant {
    *GLOBAL_REFERENCE.get_or_init(|| value)
}

/// Returns the global reference point for time measurement and duration since
#[inline]
pub fn reference_and_elapsed() -> (Instant, Duration) {
    let now = Instant::now();
    let reference = get_or_init(now);
    (reference, now.duration_since(reference))
}

/// Returns the global reference point for time measurement.
#[inline]
pub fn reference() -> Instant {
    let (reference, _elapsed) = reference_and_elapsed();
    reference
}

/// Returns the duration since the global reference point for time measurement.
#[inline]
pub fn elapsed() -> Duration {
    let (_reference, elapsed) = reference_and_elapsed();
    elapsed
}
