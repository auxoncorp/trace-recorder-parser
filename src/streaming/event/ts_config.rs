use crate::streaming::event::EventCount;
use crate::time::{Frequency, Timestamp};
use crate::types::TimerCounter;
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(
    fmt = "[{timestamp}]:{frequency}:{tick_rate_hz}:{hwtc_type}:{isr_chaining_threshold}:{htc_period:?}"
)]
pub struct TsConfigEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub frequency: Frequency,
    pub tick_rate_hz: u32,
    pub hwtc_type: TimerCounter,
    pub isr_chaining_threshold: u32,
    pub htc_period: Option<u32>,
}
