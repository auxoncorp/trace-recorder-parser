use crate::snapshot::Timestamp;
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]")]
pub struct LowPowerEvent {
    pub timestamp: Timestamp,
}

pub type LowPowerBeginEvent = LowPowerEvent;
pub type LowPowerEndEvent = LowPowerEvent;
