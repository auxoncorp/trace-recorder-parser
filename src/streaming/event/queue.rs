use crate::streaming::event::EventCount;
use crate::time::{Ticks, Timestamp};
use crate::types::{ObjectHandle, QueueName};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}:{queue_length}")]
pub struct QueueCreateEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: Option<QueueName>,
    pub queue_length: u32,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}:{messages_waiting}")]
pub struct QueueEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: Option<QueueName>,
    pub ticks_to_wait: Option<Ticks>,
    pub messages_waiting: u32,
}

pub type QueueSendEvent = QueueEvent;
pub type QueueSendBlockEvent = QueueEvent;
pub type QueueSendFromIsrEvent = QueueEvent;
pub type QueueSendFrontEvent = QueueEvent;
pub type QueueSendFrontBlockEvent = QueueEvent;
pub type QueueSendFrontFromIsrEvent = QueueEvent;
pub type QueueReceiveEvent = QueueEvent;
pub type QueueReceiveBlockEvent = QueueEvent;
pub type QueueReceiveFromIsrEvent = QueueEvent;
pub type QueuePeekEvent = QueueEvent;
pub type QueuePeekBlockEvent = QueueEvent;
