use crate::streaming::event::{EventCode, EventCount, EventParameterCount};
use crate::time::Timestamp;
use derive_more::Display;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(
    fmt = "[{timestamp}]:{}:{}:{event_count}",
    "self.code.event_id()",
    "self.code.parameter_count()"
)]
pub struct BaseEvent {
    pub code: EventCode,
    pub event_count: EventCount,
    pub timestamp: Timestamp,
    pub(crate) parameters: [u32; EventParameterCount::MAX],
}

impl BaseEvent {
    pub fn parameters(&self) -> &[u32] {
        // SAFETY: parameter_count is always <= EventParameterCount::MAX
        let num_params = usize::from(self.code.parameter_count());
        debug_assert!(num_params <= self.parameters.len());
        &self.parameters[..num_params]
    }
}
