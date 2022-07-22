use crate::time::Timestamp;
use crate::types::UserEventArgRecordCount;
use derive_more::{Binary, Deref, Display, Into, LowerHex, Octal, UpperHex};

pub use base::BaseEvent;
pub use isr::{IsrBeginEvent, IsrDefineEvent, IsrEvent, IsrResumeEvent};
pub use object_name::ObjectNameEvent;
pub use parser::EventParser;
pub use task::{
    TaskActivateEvent, TaskBeginEvent, TaskCreateEvent, TaskEvent, TaskPriorityEvent,
    TaskReadyEvent, TaskResumeEvent,
};
pub use trace_start::TraceStartEvent;
pub use ts_config::TsConfigEvent;
pub use user::UserEvent;

pub mod base;
pub mod isr;
pub mod object_name;
pub mod parser;
pub mod task;
pub mod trace_start;
pub mod ts_config;
pub mod user;

#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    Into,
    Display,
    Binary,
    Octal,
    LowerHex,
    UpperHex,
    Deref,
)]
#[display(fmt = "{_0}")]
pub struct EventCount(pub(crate) u16);

#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    Into,
    Display,
    Binary,
    Octal,
    LowerHex,
    UpperHex,
    Deref,
)]
#[display(fmt = "{_0}")]
pub struct EventParameterCount(pub(crate) u8);

impl EventParameterCount {
    pub const MAX: usize = 15;
}

impl From<EventParameterCount> for usize {
    fn from(c: EventParameterCount) -> Self {
        c.0.into()
    }
}

/// Event codes for streaming mode
/// Note that the upper 4 bits are the parameter count
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    Into,
    Display,
    Binary,
    Octal,
    LowerHex,
    UpperHex,
)]
#[display(fmt = "{_0:X}")]
pub struct EventCode(u16);

impl EventCode {
    pub fn event_id(&self) -> EventId {
        EventId(self.0 & 0x0F_FF)
    }

    pub fn event_type(&self) -> EventType {
        EventType::from(self.event_id())
    }

    /// Return the number of 32-bit parameters for the event
    pub fn parameter_count(&self) -> EventParameterCount {
        EventParameterCount(((self.0 >> 12) & 0x0F) as u8)
    }
}

/// Event IDs for streaming mode, derived from the lower 12 bits of the EventId
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    Into,
    Display,
    Binary,
    Octal,
    LowerHex,
    UpperHex,
)]
#[display(fmt = "{_0:X}")]
pub struct EventId(u16);

/// Event types for streaming mode
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum EventType {
    #[display(fmt = "NULL")]
    Null,

    #[display(fmt = "TRACE_START")]
    TraceStart,
    #[display(fmt = "TS_CONFIG")]
    TsConfig,
    #[display(fmt = "OBJECT_NAME")]
    ObjectName,
    #[display(fmt = "TASK_PRIORITY")]
    TaskPriority,
    #[display(fmt = "DEFINE_ISR")]
    DefineIsr,

    #[display(fmt = "TASK_CREATE")]
    TaskCreate,

    #[display(fmt = "TASK_READY")]
    TaskReady,
    #[display(fmt = "TS_ISR_BEGIN")]
    TaskSwitchIsrBegin,
    #[display(fmt = "TS_ISR_RESUME")]
    TaskSwitchIsrResume,
    #[display(fmt = "TS_TASK_BEGIN")]
    TaskSwitchTaskBegin,
    #[display(fmt = "TS_TASK_RESUME")]
    TaskSwitchTaskResume,
    #[display(fmt = "TASK_ACTIVATE")]
    TaskActivate,

    // User events
    // Note that user event code range is 0x90..=0x9F
    // Allow for 0-15 arguments (the arg count == word count, always 32 bits) is added to event code
    // num_args = EventCode - 0x90
    #[display(fmt = "USER_EVENT")]
    UserEvent(UserEventArgRecordCount),

    // Variant to handle unknown/unsupported event ID
    #[display(fmt = "UNKNOWN({_0})")]
    Unknown(EventId),
}

impl From<EventId> for EventType {
    fn from(id: EventId) -> Self {
        use EventType::*;
        match u16::from(id) {
            0x00 => Null,

            0x01 => TraceStart,
            0x02 => TsConfig,
            0x03 => ObjectName,
            0x04 => TaskPriority,
            0x07 => DefineIsr,

            0x10 => TaskCreate,

            0x30 => TaskReady,
            0x33 => TaskSwitchIsrBegin,
            0x34 => TaskSwitchIsrResume,
            0x35 => TaskSwitchTaskBegin,
            0x36 => TaskSwitchTaskResume,
            0x37 => TaskActivate,

            raw @ 0x90..=0x9F => UserEvent(UserEventArgRecordCount(raw as u8 - 0x90)),

            _ => Unknown(id),
        }
    }
}

impl From<EventType> for EventId {
    fn from(et: EventType) -> Self {
        use EventType::*;
        let id = match et {
            Null => 0x00,

            TraceStart => 0x01,
            TsConfig => 0x02,
            ObjectName => 0x03,
            TaskPriority => 0x04,
            DefineIsr => 0x07,

            TaskCreate => 0x10,

            TaskReady => 0x30,
            TaskSwitchIsrBegin => 0x33,
            TaskSwitchIsrResume => 0x34,
            TaskSwitchTaskBegin => 0x35,
            TaskSwitchTaskResume => 0x36,
            TaskActivate => 0x37,

            UserEvent(ac) => (0x90 + ac.0).into(),

            Unknown(raw) => raw.0,
        };
        EventId(id)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum Event {
    #[display(fmt = "TraceStart({_0})")]
    TraceStart(TraceStartEvent),
    #[display(fmt = "TsConfig({_0})")]
    TsConfig(TsConfigEvent),
    #[display(fmt = "ObjectName({_0})")]
    ObjectName(ObjectNameEvent),
    #[display(fmt = "TaskPriority({_0})")]
    TaskPriority(TaskPriorityEvent),
    #[display(fmt = "IsrDefine({_0})")]
    IsrDefine(IsrDefineEvent),

    #[display(fmt = "TaskCreate({_0})")]
    TaskCreate(TaskCreateEvent),

    #[display(fmt = "TaskReady({_0})")]
    TaskReady(TaskReadyEvent),
    #[display(fmt = "IsrBegin({_0})")]
    IsrBegin(IsrBeginEvent),
    #[display(fmt = "IsrResume({_0})")]
    IsrResume(IsrResumeEvent),
    #[display(fmt = "TaskBegin({_0})")]
    TaskBegin(TaskBeginEvent),
    #[display(fmt = "TaskResume({_0})")]
    TaskResume(TaskResumeEvent),
    #[display(fmt = "TaskActivate({_0})")]
    TaskActivate(TaskActivateEvent),

    #[display(fmt = "User({_0})")]
    User(UserEvent),

    #[display(fmt = "BaseEvent({_0})")]
    Unknown(BaseEvent),
}

impl Event {
    pub fn event_count(&self) -> EventCount {
        use Event::*;
        match self {
            TraceStart(e) => e.event_count,
            TsConfig(e) => e.event_count,
            ObjectName(e) => e.event_count,
            TaskPriority(e) => e.event_count,
            IsrDefine(e) => e.event_count,
            TaskCreate(e) => e.event_count,
            TaskReady(e) => e.event_count,
            IsrBegin(e) => e.event_count,
            IsrResume(e) => e.event_count,
            TaskBegin(e) => e.event_count,
            TaskResume(e) => e.event_count,
            TaskActivate(e) => e.event_count,
            User(e) => e.event_count,
            Unknown(e) => e.event_count,
        }
    }

    pub fn timestamp(&self) -> Timestamp {
        use Event::*;
        match self {
            TraceStart(e) => e.timestamp,
            TsConfig(e) => e.timestamp,
            ObjectName(e) => e.timestamp,
            TaskPriority(e) => e.timestamp,
            IsrDefine(e) => e.timestamp,
            TaskCreate(e) => e.timestamp,
            TaskReady(e) => e.timestamp,
            IsrBegin(e) => e.timestamp,
            IsrResume(e) => e.timestamp,
            TaskBegin(e) => e.timestamp,
            TaskResume(e) => e.timestamp,
            TaskActivate(e) => e.timestamp,
            User(e) => e.timestamp,
            Unknown(e) => e.timestamp,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn event_type_roundtrip() {
        for raw in 0..=0xFF {
            let eid = EventId(raw);
            let et = EventType::from(eid);
            assert_eq!(eid, EventId::from(et));
        }
    }
}
