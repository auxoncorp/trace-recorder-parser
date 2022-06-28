use crate::snapshot::object_properties::{IsrPriority, ObjectHandle};
use crate::snapshot::Timestamp;
use derive_more::{Deref, Display, Into};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Into, Deref, Display)]
#[display(fmt = "{_0}")]
pub struct IsrName(pub(crate) String);

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
