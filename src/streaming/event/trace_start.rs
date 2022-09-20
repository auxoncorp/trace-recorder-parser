use crate::streaming::event::EventCount;
use crate::time::Timestamp;
use crate::types::{ObjectHandle, TaskName};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{current_task}")]
pub struct TraceStartEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub current_task_handle: ObjectHandle,
    pub current_task: TaskName,
}
