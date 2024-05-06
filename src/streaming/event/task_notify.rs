use crate::streaming::event::EventCount;
use crate::time::{Ticks, Timestamp};
use crate::types::{ObjectHandle, TaskName};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}")]
pub struct TaskNotifyEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    /// Task-to-notify for TaskNotify and TaskNotifyFromIsr
    /// Current task for TaskNotifyWait and TaskNotifyWaitBlock
    pub handle: ObjectHandle,
    pub task_name: Option<TaskName>,
    pub ticks_to_wait: Option<Ticks>,
}

pub type TaskNotifyFromIsrEvent = TaskNotifyEvent;
pub type TaskNotifyWaitEvent = TaskNotifyEvent;
pub type TaskNotifyWaitBlockEvent = TaskNotifyEvent;
