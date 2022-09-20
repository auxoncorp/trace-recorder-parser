use crate::streaming::event::EventCount;
use crate::time::Timestamp;
use crate::types::Heap;
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:0x{address:X}:{size}")]
pub struct MemoryEvent {
    pub event_count: EventCount,
    pub timestamp: Timestamp,

    pub address: u32,
    pub size: u32,
    pub heap: Heap,
}

pub type MemoryAllocEvent = MemoryEvent;
pub type MemoryFreeEvent = MemoryEvent;
