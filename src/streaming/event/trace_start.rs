use crate::streaming::event::EventCount;
use crate::time::Timestamp;
use crate::types::TaskName;
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{current_task}:{os_ticks}:{session_counter}")]
pub struct TraceStartEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub os_ticks: u32,
    pub current_task: TaskName,
    pub session_counter: u32,
}
