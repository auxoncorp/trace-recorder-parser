use crate::streaming::event::EventCount;
use crate::time::Timestamp;
use crate::types::{ObjectHandle, SymbolString};
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:{handle}:'{name}'")]
pub struct ObjectNameEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub handle: ObjectHandle,
    pub name: SymbolString,
}
