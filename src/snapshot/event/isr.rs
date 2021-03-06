use crate::time::Timestamp;
use crate::types::{IsrName, IsrPriority, ObjectHandle};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:'{name}':{priority}")]
pub struct IsrEvent {
    pub handle: ObjectHandle,
    pub name: IsrName,
    pub priority: IsrPriority,
    pub timestamp: Timestamp,
}

pub type IsrBeginEvent = IsrEvent;
pub type IsrResumeEvent = IsrEvent;
