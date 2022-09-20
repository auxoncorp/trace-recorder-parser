use crate::streaming::event::EventCount;
use crate::time::{Ticks, Timestamp};
use crate::types::{ObjectHandle, SemaphoreName};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}")]
pub struct SemaphoreCreateEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: Option<SemaphoreName>,
    pub count: Option<u32>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}:{count}")]
pub struct SemaphoreEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: Option<SemaphoreName>,
    pub ticks_to_wait: Option<Ticks>,
    pub count: u32,
}

pub type SemaphoreGiveEvent = SemaphoreEvent;
pub type SemaphoreGiveBlockEvent = SemaphoreEvent;
pub type SemaphoreGiveFromIsrEvent = SemaphoreEvent;
pub type SemaphoreTakeEvent = SemaphoreEvent;
pub type SemaphoreTakeBlockEvent = SemaphoreEvent;
pub type SemaphoreTakeFromIsrEvent = SemaphoreEvent;
pub type SemaphorePeekEvent = SemaphoreEvent;
pub type SemaphorePeekBlockEvent = SemaphoreEvent;
