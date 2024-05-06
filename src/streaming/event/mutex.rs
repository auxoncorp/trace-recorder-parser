use crate::streaming::event::EventCount;
use crate::time::{Ticks, Timestamp};
use crate::types::{MutexName, ObjectHandle};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}")]
pub struct MutexCreateEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: Option<MutexName>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}")]
pub struct MutexEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: Option<MutexName>,
    pub ticks_to_wait: Option<Ticks>,
}

pub type MutexGiveEvent = MutexEvent;
pub type MutexGiveBlockEvent = MutexEvent;
pub type MutexGiveRecursiveEvent = MutexEvent;
pub type MutexTakeEvent = MutexEvent;
pub type MutexTakeBlockEvent = MutexEvent;
pub type MutexTakeRecursiveEvent = MutexEvent;
pub type MutexTakeRecursiveBlockEvent = MutexEvent;
