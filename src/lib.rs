//!
//! gametime crate.
//!

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
mod clock;

mod freq;
mod span;
mod stamp;

pub mod prelude {
    pub use crate::{
        freq::{Frequency, FrequencyTicker, FrequencyTickerIter},
        span::{TimeSpan, TimeSpanNumExt},
        stamp::TimeStamp,
    };

    #[cfg(feature = "std")]
    pub use crate::clock::{Clock, ClockStep};

    #[cfg(feature = "global_reference")]
    pub use crate::stamp::global_reference;
}
