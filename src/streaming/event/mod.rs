use crate::time::Timestamp;
use crate::types::UserEventArgRecordCount;
use derive_more::{Binary, Deref, Display, From, Into, LowerHex, Octal, UpperHex};

pub use base::BaseEvent;
pub use isr::{IsrBeginEvent, IsrDefineEvent, IsrEvent, IsrResumeEvent};
pub use memory::{MemoryAllocEvent, MemoryEvent, MemoryFreeEvent};
pub use mutex::{
    MutexCreateEvent, MutexEvent, MutexGiveBlockEvent, MutexGiveEvent, MutexGiveRecursiveEvent,
    MutexTakeBlockEvent, MutexTakeEvent, MutexTakeRecursiveBlockEvent, MutexTakeRecursiveEvent,
};
pub use object_name::ObjectNameEvent;
pub use parser::EventParser;
pub use queue::{
    QueueCreateEvent, QueueEvent, QueuePeekBlockEvent, QueuePeekEvent, QueueReceiveBlockEvent,
    QueueReceiveEvent, QueueReceiveFromIsrEvent, QueueSendBlockEvent, QueueSendEvent,
    QueueSendFromIsrEvent, QueueSendFrontBlockEvent, QueueSendFrontEvent,
    QueueSendFrontFromIsrEvent,
};
pub use semaphore::{
    SemaphoreCreateEvent, SemaphoreEvent, SemaphoreGiveBlockEvent, SemaphoreGiveEvent,
    SemaphoreGiveFromIsrEvent, SemaphorePeekBlockEvent, SemaphorePeekEvent,
    SemaphoreTakeBlockEvent, SemaphoreTakeEvent, SemaphoreTakeFromIsrEvent,
};
pub use task::{
    TaskActivateEvent, TaskBeginEvent, TaskCreateEvent, TaskEvent, TaskPriorityDisinheritEvent,
    TaskPriorityEvent, TaskPriorityInheritEvent, TaskReadyEvent, TaskResumeEvent,
};
pub use trace_start::TraceStartEvent;
pub use ts_config::TsConfigEvent;
pub use unused_stack::UnusedStackEvent;
pub use user::UserEvent;

pub mod base;
pub mod isr;
pub mod memory;
pub mod mutex;
pub mod object_name;
pub mod parser;
pub mod queue;
pub mod semaphore;
pub mod task;
pub mod trace_start;
pub mod ts_config;
pub mod unused_stack;
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
    From,
    Into,
    Display,
    Binary,
    Octal,
    LowerHex,
    UpperHex,
)]
#[display(fmt = "{_0:X}")]
pub struct EventId(pub u16);

/// Event types for streaming mode
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum EventType {
    #[display(fmt = "NULL")]
    Null,

    #[display(fmt = "TRACE_START")]
    TraceStart,
    #[display(fmt = "TIMESTAMP_CONFIG")]
    TsConfig,
    #[display(fmt = "OBJECT_NAME")]
    ObjectName,
    #[display(fmt = "TASK_PRIORITY")]
    TaskPriority,
    #[display(fmt = "TASK_PRIORITY_INHERIT")]
    TaskPriorityInherit,
    #[display(fmt = "TASK_PRIORITY_DISINHERIT")]
    TaskPriorityDisinherit,
    #[display(fmt = "DEFINE_ISR")]
    DefineIsr,

    #[display(fmt = "TASK_CREATE")]
    TaskCreate,
    #[display(fmt = "TASK_CREATE_FAILED")]
    TaskCreateFailed,
    #[display(fmt = "TASK_READY")]
    TaskReady,
    #[display(fmt = "TASK_SWITCH_ISR_BEGIN")]
    TaskSwitchIsrBegin,
    #[display(fmt = "TASK_SWITCH_ISR_RESUME")]
    TaskSwitchIsrResume,
    #[display(fmt = "TASK_SWITCH_TASK_BEGIN")]
    TaskSwitchTaskBegin,
    #[display(fmt = "TASK_SWITCH_TASK_RESUME")]
    TaskSwitchTaskResume,
    #[display(fmt = "TASK_ACTIVATE")]
    TaskActivate,
    #[display(fmt = "TASK_DELAY_UNTIL")]
    TaskDelayUntil,
    #[display(fmt = "TASK_DELAY")]
    TaskDelay,
    #[display(fmt = "TASK_SUSPEND")]
    TaskSuspend,
    #[display(fmt = "TASK_RESUME")]
    TaskResume,
    #[display(fmt = "TASK_RESUME_FROM_ISR")]
    TaskResumeFromIsr,

    #[display(fmt = "MEMORY_ALLOC")]
    MemoryAlloc,
    #[display(fmt = "MEMORY_FREE")]
    MemoryFree,

    #[display(fmt = "QUEUE_CREATE")]
    QueueCreate,
    #[display(fmt = "QUEUE_CREATE_FAILED")]
    QueueCreateFailed,
    #[display(fmt = "QUEUE_SEND")]
    QueueSend,
    #[display(fmt = "QUEUE_SEND_FAILED")]
    QueueSendFailed,
    #[display(fmt = "QUEUE_SEND_BLOCK")]
    QueueSendBlock,
    #[display(fmt = "QUEUE_SEND_FROM_ISR")]
    QueueSendFromIsr,
    #[display(fmt = "QUEUE_SEND_FROM_ISR_FAILED")]
    QueueSendFromIsrFailed,
    #[display(fmt = "QUEUE_RECEIVE")]
    QueueReceive,
    #[display(fmt = "QUEUE_RECEIVE_FAILED")]
    QueueReceiveFailed,
    #[display(fmt = "QUEUE_RECEIVE_BLOCK")]
    QueueReceiveBlock,
    #[display(fmt = "QUEUE_RECEIVE_FROM_ISR")]
    QueueReceiveFromIsr,
    #[display(fmt = "QUEUE_RECEIVE_FROM_ISR_FAILED")]
    QueueReceiveFromIsrFailed,
    #[display(fmt = "QUEUE_PEEK")]
    QueuePeek,
    #[display(fmt = "QUEUE_PEEK_FAILED")]
    QueuePeekFailed,
    #[display(fmt = "QUEUE_PEEK_BLOCK")]
    QueuePeekBlock,
    #[display(fmt = "QUEUE_SEND_FRONT")]
    QueueSendFront,
    #[display(fmt = "QUEUE_SEND_FRONT_BLOCK")]
    QueueSendFrontBlock,
    #[display(fmt = "QUEUE_SEND_FRONT_FROM_ISR")]
    QueueSendFrontFromIsr,

    #[display(fmt = "MUTEX_CREATE")]
    MutexCreate,
    #[display(fmt = "MUTEX_CREATE_FAILED")]
    MutexCreateFailed,
    #[display(fmt = "MUTEX_GIVE")]
    MutexGive,
    #[display(fmt = "MUTEX_GIVE_FAILED")]
    MutexGiveFailed,
    #[display(fmt = "MUTEX_GIVE_BLOCK")]
    MutexGiveBlock,
    #[display(fmt = "MUTEX_GIVE_RECURSIVE")]
    MutexGiveRecursive,
    #[display(fmt = "MUTEX_TAKE")]
    MutexTake,
    #[display(fmt = "MUTEX_TAKE_FAILED")]
    MutexTakeFailed,
    #[display(fmt = "MUTEX_TAKE_BLOCK")]
    MutexTakeBlock,
    #[display(fmt = "MUTEX_TAKE_RECURSIVE")]
    MutexTakeRecursive,
    #[display(fmt = "MUTEX_TAKE_RECURSIVE_BLOCK")]
    MutexTakeRecursiveBlock,

    #[display(fmt = "SEMAPHORE_BINARY_CREATE")]
    SemaphoreBinaryCreate,
    #[display(fmt = "SEMAPHORE_BINARY_CREATE_FAILED")]
    SemaphoreBinaryCreateFailed,
    #[display(fmt = "SEMAPHORE_COUNTING_CREATE")]
    SemaphoreCountingCreate,
    #[display(fmt = "SEMAPHORE_COUNTING_CREATE_FAILED")]
    SemaphoreCountingCreateFailed,
    #[display(fmt = "SEMAPHORE_GIVE")]
    SemaphoreGive,
    #[display(fmt = "SEMAPHORE_GIVE_FAILED")]
    SemaphoreGiveFailed,
    #[display(fmt = "SEMAPHORE_GIVE_BLOCK")]
    SemaphoreGiveBlock,
    #[display(fmt = "SEMAPHORE_GIVE_FROM_ISR")]
    SemaphoreGiveFromIsr,
    #[display(fmt = "SEMAPHORE_GIVE_FROM_ISR_FAILED")]
    SemaphoreGiveFromIsrFailed,
    #[display(fmt = "SEMAPHORE_TAKE")]
    SemaphoreTake,
    #[display(fmt = "SEMAPHORE_TAKE_FAILED")]
    SemaphoreTakeFailed,
    #[display(fmt = "SEMAPHORE_TAKE_BLOCK")]
    SemaphoreTakeBlock,
    #[display(fmt = "SEMAPHORE_TAKE_FROM_ISR")]
    SemaphoreTakeFromIsr,
    #[display(fmt = "SEMAPHORE_TAKE_FROM_ISR_FAILED")]
    SemaphoreTakeFromIsrFailed,
    #[display(fmt = "SEMAPHORE_PEEK")]
    SemaphorePeek,
    #[display(fmt = "SEMAPHORE_PEEK_FAILED")]
    SemaphorePeekFailed,
    #[display(fmt = "SEMAPHORE_PEEK_BLOCK")]
    SemaphorePeekBlock,

    // User events
    // Note that user event code range is 0x90..=0x9F
    // Allow for 0-15 arguments (the arg count == word count, always 32 bits) is added to event code
    // num_args = EventCode - 0x90
    #[display(fmt = "USER_EVENT")]
    UserEvent(UserEventArgRecordCount),

    #[display(fmt = "UNUSED_STACK")]
    UnusedStack,

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
            0x05 => TaskPriorityInherit,
            0x06 => TaskPriorityDisinherit,
            0x07 => DefineIsr,

            0x10 => TaskCreate,
            0x40 => TaskCreateFailed,
            0x30 => TaskReady,
            0x33 => TaskSwitchIsrBegin,
            0x34 => TaskSwitchIsrResume,
            0x35 => TaskSwitchTaskBegin,
            0x36 => TaskSwitchTaskResume,
            0x37 => TaskActivate,
            0x79 => TaskDelayUntil,
            0x7A => TaskDelay,
            0x7B => TaskSuspend,
            0x7C => TaskResume,
            0x7D => TaskResumeFromIsr,

            0x38 => MemoryAlloc,
            0x39 => MemoryFree,

            0x11 => QueueCreate,
            0x41 => QueueCreateFailed,
            0x50 => QueueSend,
            0x53 => QueueSendFailed,
            0x56 => QueueSendBlock,
            0x59 => QueueSendFromIsr,
            0x5C => QueueSendFromIsrFailed,
            0x60 => QueueReceive,
            0x63 => QueueReceiveFailed,
            0x66 => QueueReceiveBlock,
            0x69 => QueueReceiveFromIsr,
            0x6C => QueueReceiveFromIsrFailed,
            0x70 => QueuePeek,
            0x73 => QueuePeekFailed,
            0x76 => QueuePeekBlock,
            0xC0 => QueueSendFront,
            0xC2 => QueueSendFrontBlock,
            0xC3 => QueueSendFrontFromIsr,

            0x13 => MutexCreate,
            0x43 => MutexCreateFailed,
            0x52 => MutexGive,
            0x55 => MutexGiveFailed,
            0x58 => MutexGiveBlock,
            0xC5 => MutexGiveRecursive,
            0x62 => MutexTake,
            0x65 => MutexTakeFailed,
            0x68 => MutexTakeBlock,
            0xC7 => MutexTakeRecursive,
            0xF6 => MutexTakeRecursiveBlock,

            0x12 => SemaphoreBinaryCreate,
            0x42 => SemaphoreBinaryCreateFailed,
            0x16 => SemaphoreCountingCreate,
            0x46 => SemaphoreCountingCreateFailed,
            0x51 => SemaphoreGive,
            0x54 => SemaphoreGiveFailed,
            0x57 => SemaphoreGiveBlock,
            0x5A => SemaphoreGiveFromIsr,
            0x5D => SemaphoreGiveFromIsrFailed,
            0x61 => SemaphoreTake,
            0x64 => SemaphoreTakeFailed,
            0x67 => SemaphoreTakeBlock,
            0x6A => SemaphoreTakeFromIsr,
            0x6D => SemaphoreTakeFromIsrFailed,
            0x71 => SemaphorePeek,
            0x74 => SemaphorePeekFailed,
            0x77 => SemaphorePeekBlock,

            raw @ 0x90..=0x9F => UserEvent(UserEventArgRecordCount(raw as u8 - 0x90)),

            0xEB => UnusedStack,

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
            TaskPriorityInherit => 0x05,
            TaskPriorityDisinherit => 0x06,
            DefineIsr => 0x07,

            TaskCreate => 0x10,
            TaskCreateFailed => 0x40,
            TaskReady => 0x30,
            TaskSwitchIsrBegin => 0x33,
            TaskSwitchIsrResume => 0x34,
            TaskSwitchTaskBegin => 0x35,
            TaskSwitchTaskResume => 0x36,
            TaskActivate => 0x37,
            TaskDelayUntil => 0x79,
            TaskDelay => 0x7A,
            TaskSuspend => 0x7B,
            TaskResume => 0x7C,
            TaskResumeFromIsr => 0x7D,

            MemoryAlloc => 0x38,
            MemoryFree => 0x39,

            QueueCreate => 0x11,
            QueueCreateFailed => 0x41,
            QueueSend => 0x50,
            QueueSendFailed => 0x53,
            QueueSendBlock => 0x56,
            QueueSendFromIsr => 0x59,
            QueueSendFromIsrFailed => 0x5C,
            QueueReceive => 0x60,
            QueueReceiveFailed => 0x63,
            QueueReceiveBlock => 0x66,
            QueueReceiveFromIsr => 0x69,
            QueueReceiveFromIsrFailed => 0x6C,
            QueuePeek => 0x70,
            QueuePeekFailed => 0x73,
            QueuePeekBlock => 0x76,
            QueueSendFront => 0xC0,
            QueueSendFrontBlock => 0xC2,
            QueueSendFrontFromIsr => 0xC3,

            MutexCreate => 0x13,
            MutexCreateFailed => 0x43,
            MutexGive => 0x52,
            MutexGiveFailed => 0x55,
            MutexGiveBlock => 0x58,
            MutexGiveRecursive => 0xC5,
            MutexTake => 0x62,
            MutexTakeFailed => 0x65,
            MutexTakeBlock => 0x68,
            MutexTakeRecursive => 0xC7,
            MutexTakeRecursiveBlock => 0xF6,

            SemaphoreBinaryCreate => 0x12,
            SemaphoreBinaryCreateFailed => 0x42,
            SemaphoreCountingCreate => 0x16,
            SemaphoreCountingCreateFailed => 0x46,
            SemaphoreGive => 0x51,
            SemaphoreGiveFailed => 0x54,
            SemaphoreGiveBlock => 0x57,
            SemaphoreGiveFromIsr => 0x5A,
            SemaphoreGiveFromIsrFailed => 0x5D,
            SemaphoreTake => 0x61,
            SemaphoreTakeFailed => 0x64,
            SemaphoreTakeBlock => 0x67,
            SemaphoreTakeFromIsr => 0x6A,
            SemaphoreTakeFromIsrFailed => 0x6D,
            SemaphorePeek => 0x71,
            SemaphorePeekFailed => 0x74,
            SemaphorePeekBlock => 0x77,

            UserEvent(ac) => (0x90 + ac.0).into(),

            UnusedStack => 0xEB,

            Unknown(raw) => raw.0,
        };
        EventId(id)
    }
}

impl EventType {
    /// Return the number of expected parameters for the event type, otherwise
    /// return None for event types with variable parameters.
    pub(crate) fn expected_parameter_count(&self) -> Option<usize> {
        use EventType::*;
        Some(match self {
            Null => 0,
            TraceStart => 1,

            TaskPriority | TaskPriorityInherit | TaskPriorityDisinherit => 2,

            TsConfig | ObjectName | DefineIsr | TaskActivate | UserEvent(_) | Unknown(_) => {
                return None
            }

            TaskCreate
            | QueueCreate
            | MutexCreate
            | SemaphoreCountingCreate
            | SemaphoreBinaryCreate => 2,

            TaskReady | TaskSwitchIsrBegin | TaskSwitchIsrResume | TaskSwitchTaskBegin
            | TaskSwitchTaskResume => 1,

            MemoryAlloc | MemoryFree => 2,

            QueueSend
            | QueueSendBlock
            | QueueSendFromIsr
            | QueueReceiveFromIsr
            | QueueSendFront
            | QueueSendFrontBlock
            | QueueSendFrontFromIsr => 2,

            QueueReceive | QueueReceiveBlock | QueuePeek | QueuePeekBlock => 3,

            MutexGive | MutexGiveBlock | MutexGiveRecursive => 1,
            MutexTake | MutexTakeBlock | MutexTakeRecursive | MutexTakeRecursiveBlock => 2,

            SemaphoreGive | SemaphoreGiveBlock | SemaphoreGiveFromIsr | SemaphoreTakeFromIsr => 2,

            SemaphoreTake | SemaphoreTakeBlock | SemaphorePeek | SemaphorePeekBlock => 3,

            UnusedStack => 2,

            _ /* Event types not handled */ => return None,
        })
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
    #[display(fmt = "TaskPriorityInherit({_0})")]
    TaskPriorityInherit(TaskPriorityInheritEvent),
    #[display(fmt = "TaskPriorityDisinherit({_0})")]
    TaskPriorityDisinherit(TaskPriorityDisinheritEvent),
    #[display(fmt = "IsrDefine({_0})")]
    IsrDefine(IsrDefineEvent),

    #[display(fmt = "TaskCreate({_0})")]
    TaskCreate(TaskCreateEvent),
    #[display(fmt = "QueueCreate({_0})")]
    QueueCreate(QueueCreateEvent),
    #[display(fmt = "MutexCreate({_0})")]
    MutexCreate(MutexCreateEvent),
    #[display(fmt = "SemaphoreBinaryCreate({_0})")]
    SemaphoreBinaryCreate(SemaphoreCreateEvent),
    #[display(fmt = "SemaphoreCountingCreate({_0})")]
    SemaphoreCountingCreate(SemaphoreCreateEvent),

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

    #[display(fmt = "MemoryAlloc({_0})")]
    MemoryAlloc(MemoryAllocEvent),
    #[display(fmt = "MemoryFree({_0})")]
    MemoryFree(MemoryFreeEvent),

    #[display(fmt = "QueueSend({_0})")]
    QueueSend(QueueSendEvent),
    #[display(fmt = "QueueSendBlock({_0})")]
    QueueSendBlock(QueueSendBlockEvent),
    #[display(fmt = "QueueSendFromIsr({_0})")]
    QueueSendFromIsr(QueueSendFromIsrEvent),
    #[display(fmt = "QueueReceive({_0})")]
    QueueReceive(QueueReceiveEvent),
    #[display(fmt = "QueueReceiveBlock({_0})")]
    QueueReceiveBlock(QueueReceiveBlockEvent),
    #[display(fmt = "QueueReceiveFromIsr({_0})")]
    QueueReceiveFromIsr(QueueReceiveFromIsrEvent),
    #[display(fmt = "QueuePeek({_0})")]
    QueuePeek(QueuePeekEvent),
    #[display(fmt = "QueuePeekBlock({_0})")]
    QueuePeekBlock(QueuePeekBlockEvent),
    #[display(fmt = "QueueSendFront({_0})")]
    QueueSendFront(QueueSendFrontEvent),
    #[display(fmt = "QueueSendFrontBlock({_0})")]
    QueueSendFrontBlock(QueueSendFrontBlockEvent),
    #[display(fmt = "QueueSendFrontFromIsr({_0})")]
    QueueSendFrontFromIsr(QueueSendFrontFromIsrEvent),

    #[display(fmt = "MutexGive({_0})")]
    MutexGive(MutexGiveEvent),
    #[display(fmt = "MutexGiveBlock({_0})")]
    MutexGiveBlock(MutexGiveBlockEvent),
    #[display(fmt = "MutexGiveRecursive({_0})")]
    MutexGiveRecursive(MutexGiveRecursiveEvent),
    #[display(fmt = "MutexTake({_0})")]
    MutexTake(MutexTakeEvent),
    #[display(fmt = "MutexTakeBlock({_0})")]
    MutexTakeBlock(MutexTakeBlockEvent),
    #[display(fmt = "MutexTakeRecursive({_0})")]
    MutexTakeRecursive(MutexTakeRecursiveEvent),
    #[display(fmt = "MutexTakeRecursiveBlock({_0})")]
    MutexTakeRecursiveBlock(MutexTakeRecursiveBlockEvent),

    #[display(fmt = "SemaphoreGive({_0})")]
    SemaphoreGive(SemaphoreGiveEvent),
    #[display(fmt = "SemaphoreGiveBlock({_0})")]
    SemaphoreGiveBlock(SemaphoreGiveBlockEvent),
    #[display(fmt = "SemaphoreGiveFromIsr({_0})")]
    SemaphoreGiveFromIsr(SemaphoreGiveFromIsrEvent),
    #[display(fmt = "SemaphoreTake({_0})")]
    SemaphoreTake(SemaphoreTakeEvent),
    #[display(fmt = "SemaphoreTakeBlock({_0})")]
    SemaphoreTakeBlock(SemaphoreTakeBlockEvent),
    #[display(fmt = "SemaphoreTakeFromIsr({_0})")]
    SemaphoreTakeFromIsr(SemaphoreTakeFromIsrEvent),
    #[display(fmt = "SemaphorePeek({_0})")]
    SemaphorePeek(SemaphorePeekEvent),
    #[display(fmt = "SemaphorePeekBlock({_0})")]
    SemaphorePeekBlock(SemaphorePeekBlockEvent),

    #[display(fmt = "User({_0})")]
    User(UserEvent),

    #[display(fmt = "UnusedStack({_0})")]
    UnusedStack(UnusedStackEvent),

    #[display(fmt = "BaseEvent({_0})")]
    Unknown(BaseEvent),
}

impl Event {
    /// Get the event count (sequence number).
    /// NOTE:
    /// * V10: TraceStart reports 1 (doesn't track the internal header/timestamp-info/etc)
    /// * V12: TraceStart reports 6 (does track the internal header/timestamp-info/etc)
    pub fn event_count(&self) -> EventCount {
        use Event::*;
        match self {
            TraceStart(e) => e.event_count,
            TsConfig(e) => e.event_count,
            ObjectName(e) => e.event_count,
            TaskPriority(e) => e.event_count,
            TaskPriorityInherit(e) => e.event_count,
            TaskPriorityDisinherit(e) => e.event_count,
            IsrDefine(e) => e.event_count,
            TaskCreate(e) => e.event_count,
            QueueCreate(e) => e.event_count,
            MutexCreate(e) => e.event_count,
            SemaphoreBinaryCreate(e) => e.event_count,
            SemaphoreCountingCreate(e) => e.event_count,
            TaskReady(e) => e.event_count,
            IsrBegin(e) => e.event_count,
            IsrResume(e) => e.event_count,
            TaskBegin(e) => e.event_count,
            TaskResume(e) => e.event_count,
            TaskActivate(e) => e.event_count,
            MemoryAlloc(e) => e.event_count,
            MemoryFree(e) => e.event_count,
            QueueSend(e) => e.event_count,
            QueueSendBlock(e) => e.event_count,
            QueueSendFromIsr(e) => e.event_count,
            QueueReceive(e) => e.event_count,
            QueueReceiveBlock(e) => e.event_count,
            QueueReceiveFromIsr(e) => e.event_count,
            QueuePeek(e) => e.event_count,
            QueuePeekBlock(e) => e.event_count,
            QueueSendFront(e) => e.event_count,
            QueueSendFrontBlock(e) => e.event_count,
            QueueSendFrontFromIsr(e) => e.event_count,
            MutexGive(e) => e.event_count,
            MutexGiveBlock(e) => e.event_count,
            MutexGiveRecursive(e) => e.event_count,
            MutexTake(e) => e.event_count,
            MutexTakeBlock(e) => e.event_count,
            MutexTakeRecursive(e) => e.event_count,
            MutexTakeRecursiveBlock(e) => e.event_count,
            SemaphoreGive(e) => e.event_count,
            SemaphoreGiveBlock(e) => e.event_count,
            SemaphoreGiveFromIsr(e) => e.event_count,
            SemaphoreTake(e) => e.event_count,
            SemaphoreTakeBlock(e) => e.event_count,
            SemaphoreTakeFromIsr(e) => e.event_count,
            SemaphorePeek(e) => e.event_count,
            SemaphorePeekBlock(e) => e.event_count,
            User(e) => e.event_count,
            UnusedStack(e) => e.event_count,
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
            TaskPriorityInherit(e) => e.timestamp,
            TaskPriorityDisinherit(e) => e.timestamp,
            IsrDefine(e) => e.timestamp,
            TaskCreate(e) => e.timestamp,
            QueueCreate(e) => e.timestamp,
            MutexCreate(e) => e.timestamp,
            SemaphoreBinaryCreate(e) => e.timestamp,
            SemaphoreCountingCreate(e) => e.timestamp,
            TaskReady(e) => e.timestamp,
            IsrBegin(e) => e.timestamp,
            IsrResume(e) => e.timestamp,
            TaskBegin(e) => e.timestamp,
            TaskResume(e) => e.timestamp,
            TaskActivate(e) => e.timestamp,
            MemoryAlloc(e) => e.timestamp,
            MemoryFree(e) => e.timestamp,
            QueueSend(e) => e.timestamp,
            QueueSendBlock(e) => e.timestamp,
            QueueSendFromIsr(e) => e.timestamp,
            QueueReceive(e) => e.timestamp,
            QueueReceiveBlock(e) => e.timestamp,
            QueueReceiveFromIsr(e) => e.timestamp,
            QueuePeek(e) => e.timestamp,
            QueuePeekBlock(e) => e.timestamp,
            QueueSendFront(e) => e.timestamp,
            QueueSendFrontBlock(e) => e.timestamp,
            QueueSendFrontFromIsr(e) => e.timestamp,
            MutexGive(e) => e.timestamp,
            MutexGiveBlock(e) => e.timestamp,
            MutexGiveRecursive(e) => e.timestamp,
            MutexTake(e) => e.timestamp,
            MutexTakeBlock(e) => e.timestamp,
            MutexTakeRecursive(e) => e.timestamp,
            MutexTakeRecursiveBlock(e) => e.timestamp,
            SemaphoreGive(e) => e.timestamp,
            SemaphoreGiveBlock(e) => e.timestamp,
            SemaphoreGiveFromIsr(e) => e.timestamp,
            SemaphoreTake(e) => e.timestamp,
            SemaphoreTakeBlock(e) => e.timestamp,
            SemaphoreTakeFromIsr(e) => e.timestamp,
            SemaphorePeek(e) => e.timestamp,
            SemaphorePeekBlock(e) => e.timestamp,
            User(e) => e.timestamp,
            UnusedStack(e) => e.timestamp,
            Unknown(e) => e.timestamp,
        }
    }
}

pub type DroppedEventCount = u64;

/// Event counter that tracks rollovers and discontinuities.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}", "self.count()")]
pub struct TrackingEventCounter {
    count: u16,
    rollovers: u32,
}

impl TrackingEventCounter {
    /// Creates a new counter with an initial value of zero
    pub const fn zero() -> Self {
        Self {
            count: 0,
            rollovers: 0,
        }
    }

    /// Sets the initial counter value and reset the rollover tracking.
    pub fn set_initial_count(&mut self, count: EventCount) {
        self.count = count.0;
        self.rollovers = 0;
    }

    /// Updates the event count handling rollovers.
    /// Returns the number of dropped events, if any.
    /// NOTE: must be called at least once per event count type (u16) rollover interval
    pub fn update(&mut self, event_count: EventCount) -> Option<DroppedEventCount> {
        let prev_count = self.count();

        // Handle rollover
        if event_count.0 <= self.count {
            self.rollovers += 1;
        }
        self.count = event_count.0;

        let diff = self.count() - prev_count;
        if diff != 1 {
            // SAFETY: diff will always be >=1 due to the rollover handling above
            Some(diff - 1)
        } else {
            None
        }
    }

    pub fn count(&self) -> u64 {
        u64::from(self.rollovers) << u16::BITS | u64::from(self.count)
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

    #[test]
    fn event_counter_tracking() {
        let mut ec = TrackingEventCounter::zero();
        assert_eq!(ec.count(), 0);

        // Reset initial count works
        ec.set_initial_count(EventCount(u16::MAX));
        assert_eq!(ec.count(), u16::MAX.into());

        // Non-rollover discontinuities
        ec.set_initial_count(EventCount(0));
        assert_eq!(ec.count(), 0);
        assert_eq!(ec.update(EventCount(10)), Some(9)); // Missed events 1..=9
        assert_eq!(ec.count(), 10);
        assert_eq!(ec.update(EventCount(12)), Some(1)); // Missed event 11
        assert_eq!(ec.count(), 12);
        assert_eq!(ec.update(EventCount(13)), None);
        assert_eq!(ec.count(), 13);

        // Rollover discontinuities
        ec.set_initial_count(EventCount(10));
        assert_eq!(ec.count(), 10);
        assert_eq!(
            ec.update(EventCount(10_u16.wrapping_add(u16::MAX))), // 9
            Some(u64::from(u16::MAX - 1)) // Missed events 11..<wrap-around>..=8
        );
        assert_eq!(ec.count(), u64::from(u16::MAX) + 10);
        assert_eq!(ec.update(EventCount(10)), None);
        assert_eq!(ec.count(), u64::from(u16::MAX) + 11);
        assert_eq!(ec.update(EventCount(12)), Some(1));
        assert_eq!(ec.count(), u64::from(u16::MAX) + 13);

        // Similar, but show that updating with same event count means a rollover
        ec.set_initial_count(EventCount(10));
        assert_eq!(ec.count(), 10);
        assert_eq!(
            ec.update(EventCount(10)),
            Some(u64::from(u16::MAX)) // Missed events 11..<wrap-around>..=9
        );
        assert_eq!(ec.count(), u64::from(u16::MAX) + 11);
    }
}
