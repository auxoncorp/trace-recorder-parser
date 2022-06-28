use crate::snapshot::{Dts16, Timestamp};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]")]
pub struct LowPowerEvent {
    pub dts: Dts16,
    pub timestamp: Timestamp,
}

pub type LowPowerBeginEvent = LowPowerEvent;
pub type LowPowerEndEvent = LowPowerEvent;
