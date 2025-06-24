use crate::{TimeSpan, TimeStamp};


/// Result of `Clock` step.
/// Contains time stamp corresponding to "now"
/// and time span since previous step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockStep {
    /// `TimeStamp` corresponding to "now".
    pub now: TimeStamp,

    /// Time span since previous step.
    pub step: TimeSpan,
}
