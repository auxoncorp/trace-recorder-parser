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
#[display(fmt = "[{timestamp}]:{handle}:{state}")]
pub struct StateMachineStateCreateEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub state: StateMachineStateName,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}:{name}:{state}")]
pub struct StateMachineStateChangeEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: StateMachineName,
    pub state_handle: ObjectHandle,
    pub state: StateMachineStateName,
}
