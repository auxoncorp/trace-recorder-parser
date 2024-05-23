use crate::streaming::event::EventCount;
use crate::time::Timestamp;
use crate::types::{EventGroupName, ObjectHandle};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}:0x{event_bits}")]
pub struct EventGroupCreateEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: Option<EventGroupName>,
    pub event_bits: u32,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}:0x{bits}")]
pub struct EventGroupEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: Option<EventGroupName>,
    /// Either bitsToWaitFor or bitsToClear
    pub bits: u32,
}

pub type EventGroupSyncEvent = EventGroupEvent;
pub type EventGroupWaitBitsEvent = EventGroupEvent;
pub type EventGroupClearBitsEvent = EventGroupEvent;
pub type EventGroupClearBitsFromIsrEvent = EventGroupEvent;
pub type EventGroupSetBitsEvent = EventGroupEvent;
pub type EventGroupSetBitsFromIsrEvent = EventGroupEvent;
pub type EventGroupSyncBlockEvent = EventGroupEvent;
pub type EventGroupWaitBitsBlockEvent = EventGroupEvent;
