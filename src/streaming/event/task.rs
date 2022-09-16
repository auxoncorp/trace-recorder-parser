use crate::streaming::event::EventCount;
use crate::time::Timestamp;
use crate::types::{ObjectHandle, TaskName, TaskPriority};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:'{name}':{priority}")]
pub struct TaskEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: TaskName,
    pub priority: TaskPriority,
}

pub type TaskCreateEvent = TaskEvent;
pub type TaskReadyEvent = TaskEvent;
pub type TaskPriorityEvent = TaskEvent;
pub type TaskPriorityInheritEvent = TaskEvent;
pub type TaskPriorityDisinheritEvent = TaskEvent;
pub type TaskBeginEvent = TaskEvent;
pub type TaskResumeEvent = TaskEvent;
pub type TaskActivateEvent = TaskEvent;
