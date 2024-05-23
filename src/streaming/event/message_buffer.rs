use crate::streaming::event::EventCount;
use crate::time::Timestamp;
use crate::types::{MessageBufferName, ObjectHandle};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}:{buffer_size}")]
pub struct MessageBufferCreateEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: Option<MessageBufferName>,
    pub buffer_size: u32,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}:{bytes_in_buffer}")]
pub struct MessageBufferEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: Option<MessageBufferName>,
    pub bytes_in_buffer: u32,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}")]
pub struct MessageBufferBlockEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: Option<MessageBufferName>,
}

pub type MessageBufferSendEvent = MessageBufferEvent;
pub type MessageBufferSendBlockEvent = MessageBufferBlockEvent;
pub type MessageBufferSendFromIsrEvent = MessageBufferEvent;
pub type MessageBufferReceiveEvent = MessageBufferEvent;
pub type MessageBufferReceiveBlockEvent = MessageBufferBlockEvent;
pub type MessageBufferReceiveFromIsrEvent = MessageBufferEvent;
pub type MessageBufferResetEvent = MessageBufferEvent;
