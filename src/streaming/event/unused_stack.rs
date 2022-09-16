use crate::streaming::event::EventCount;
use crate::time::Timestamp;
use crate::types::{ObjectHandle, TaskName};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:'{task}':{low_mark}")]
pub struct UnusedStackEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub task: TaskName,
    pub low_mark: u32,
}
