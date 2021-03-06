use crate::snapshot::object_properties::TaskState;
use crate::time::Timestamp;
use crate::types::{ObjectHandle, TaskName, TaskPriority};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:'{name}':{state}:{priority}")]
pub struct TaskEvent {
    pub handle: ObjectHandle,
    pub name: TaskName,
    pub state: TaskState,
    pub priority: TaskPriority,
    pub timestamp: Timestamp,
}

pub type TaskBeginEvent = TaskEvent;
pub type TaskReadyEvent = TaskEvent;
pub type TaskResumeEvent = TaskEvent;
pub type TaskCreateEvent = TaskEvent;
