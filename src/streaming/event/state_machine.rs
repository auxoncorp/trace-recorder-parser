use crate::streaming::event::EventCount;
use crate::time::Timestamp;
use crate::types::{ObjectHandle, StateMachineName, StateMachineStateName};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}:{name}")]
pub struct StateMachineCreateEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: StateMachineName,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}:{name}:{state}")]
pub struct StateMachineStateEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: StateMachineName,
    pub state_handle: ObjectHandle,
    pub state: StateMachineStateName,
}

pub type StateMachineStateCreateEvent = StateMachineStateEvent;
pub type StateMachineStateChangeEvent = StateMachineStateEvent;
