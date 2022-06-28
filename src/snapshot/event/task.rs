use crate::snapshot::object_properties::{ObjectHandle, TaskPriority, TaskState};
use crate::snapshot::{Dts16, Timestamp};
use derive_more::{Deref, Display, Into};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Into, Deref, Display)]
#[display(fmt = "{_0}")]
pub struct TaskName(pub(crate) String);

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:'{name}':{state}:{priority}")]
pub struct TaskEvent {
    pub handle: ObjectHandle,
    pub name: TaskName,
    pub state: TaskState,
    pub priority: TaskPriority,
    pub dts: Dts16,
    pub timestamp: Timestamp,
}

pub type TaskBeginEvent = TaskEvent;
pub type TaskReadyEvent = TaskEvent;
pub type TaskResumeEvent = TaskEvent;
