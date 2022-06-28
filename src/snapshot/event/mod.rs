use crate::snapshot::object_properties::ObjectClass;
use derive_more::{Binary, Deref, Display, Into, LowerHex, Octal, UpperHex};

pub use isr::{IsrBeginEvent, IsrEvent, IsrName, IsrResumeEvent};
pub use low_power::{LowPowerBeginEvent, LowPowerEndEvent, LowPowerEvent};
pub use parser::EventParser;
pub use task::{TaskBeginEvent, TaskEvent, TaskName, TaskReadyEvent, TaskResumeEvent};
pub use user::{FormattedString, UserEvent, UserEventArgRecordCount, UserEventChannel};

pub mod isr;
pub mod low_power;
pub mod parser;
pub mod task;
pub mod user;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{_0:X?}")]
pub struct EventRecord([u8; EventRecord::SIZE]);

impl EventRecord {
    pub(crate) const SIZE: usize = 4;

    pub(crate) fn new(record: [u8; EventRecord::SIZE]) -> Self {
        Self(record)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn event_code(&self) -> EventCode {
        EventCode(self.0[0])
    }
}

/// Event codes for snapshot mode
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
pub struct EventCode(u8);

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
#[display(fmt = "{}", "self.into_class()")]
pub struct ObjectClassCode(pub(crate) u8);

impl ObjectClassCode {
    /// Extract the object class code from an event code (lower 3 bits)
    pub(crate) fn from_raw(ec: u8) -> Self {
        Self(ec & 0x07)
    }

    pub(crate) fn into_raw(self) -> u8 {
        self.0
    }

    pub fn into_class(self) -> ObjectClass {
        use ObjectClass::*;
        match self.0 {
            0 => Queue,
            1 => Semaphore,
            2 => Mutex,
            3 => Task,
            4 => Isr,
            5 => Timer,
            6 => EventGroup,
            // Class codes are only 3 bits, they don't represent Streambuffer/Messagebuffer
            _ => StreamBuffer,
        }
    }
}

/// Event types for snapshot mode
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum EventType {
    #[display(fmt = "NULL")]
    Null,

    // EVENTGROUP_DIV
    // Miscellaneous events
    #[display(fmt = "XPS")]
    Xps,
    #[display(fmt = "TASK_READY")]
    TaskReady,
    #[display(fmt = "NEW_TIME")]
    NewTime,

    // EVENTGROUP_TS
    // Events for storing task-switches and interrupts.
    // The RESUME events are generated if the task/interrupt is already marked active.
    #[display(fmt = "TS_ISR_BEGIN")]
    TaskSwitchIsrBegin,
    #[display(fmt = "TS_ISR_RESUME")]
    TaskSwitchIsrResume,
    #[display(fmt = "TS_TASK_BEGIN")]
    TaskSwitchTaskBegin,
    #[display(fmt = "TS_TASK_RESUME")]
    TaskSwitchTaskResume,

    // EVENTGROUP_OBJCLOSE_NAME
    #[display(fmt = "OBJCLOSE_NAME({_0})")]
    ObjectCloseName(ObjectClassCode),

    // EVENTGROUP_OBJCLOSE_PROP
    #[display(fmt = "OBJCLOSE_PROPERTY({_0})")]
    ObjectCloseProperty(ObjectClassCode),

    // EVENTGROUP_CREATE
    #[display(fmt = "CREATE_OBJECT({_0})")]
    CreateObject(ObjectClassCode),

    // EVENTGROUP_SEND
    #[display(fmt = "SEND({_0})")]
    Send(ObjectClassCode),

    // EVENTGROUP_RECEIVE
    #[display(fmt = "RECEIVE({_0})")]
    Receive(ObjectClassCode),

    // Send/Give operations, from ISR
    #[display(fmt = "SEND_FROM_ISR({_0})")]
    SendFromIsr(ObjectClassCode),

    // Receive/Take operations, from ISR
    #[display(fmt = "RECEIVE_FROM_ISR({_0})")]
    ReceiveFromIsr(ObjectClassCode),

    // Failed create calls - memory allocation failed
    #[display(fmt = "CREATE_OBJECT_FAILED({_0})")]
    CreateObjectFailed(ObjectClassCode),

    // Failed send/give - timeout
    #[display(fmt = "SEND_FAILED({_0})")]
    SendFailed(ObjectClassCode),

    // Failed receive/take - timeout
    #[display(fmt = "RECEIVE_FAILED({_0})")]
    ReceiveFailed(ObjectClassCode),

    // Failed non-blocking send/give - queue full
    #[display(fmt = "SEND_FROM_ISR_FAILED({_0})")]
    SendFromIsrFailed(ObjectClassCode),

    // Failed non-blocking receive/take - queue empty
    #[display(fmt = "RECEIVE_FROM_ISR_FAILED({_0})")]
    ReceiveFromIsrFailed(ObjectClassCode),

    // Events when blocking on receive/take
    #[display(fmt = "RECEIVE_BLOCK({_0})")]
    ReceiveBlock(ObjectClassCode),

    // Events when blocking on send/give
    #[display(fmt = "SEND_BLOCK({_0})")]
    SendBlock(ObjectClassCode),

    // Events on queue peek (receive)
    #[display(fmt = "PEEK({_0})")]
    Peek(ObjectClassCode),

    // Events on object delete (vTaskDelete or vQueueDelete)
    #[display(fmt = "DELETE_OBJECT({_0})")]
    DeleteObject(ObjectClassCode),

    // Other events - object class is implied: TASK
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
    #[display(fmt = "TASK_PRIORITY_SET")]
    TaskPrioritySet,
    #[display(fmt = "TASK_PRIORITY_INHERIT")]
    TaskPriorityInherit,
    #[display(fmt = "TASK_PRIORITY_DISINHERIT")]
    TaskPriorityDisinherit,

    #[display(fmt = "PEND_FUNC_CALL")]
    PendFuncCall,
    #[display(fmt = "PEND_FUNC_CALL_FROM_ISR")]
    PendFuncCallFromIsr,
    #[display(fmt = "PEND_FUNC_CALL_FAILED")]
    PendFuncCallFailed,
    #[display(fmt = "PEND_FUNC_CALL_FROM_ISR_FAILED")]
    PendFuncCallFromIsrFailed,
    #[display(fmt = "MEM_MALLOC_SIZE")]
    MemoryMallocSize,
    #[display(fmt = "MEM_MALLOC_ADDRESS")]
    MemoryMallocAddress,
    #[display(fmt = "MEM_FREE_SIZE")]
    MemoryFreeSize,
    #[display(fmt = "MEM_FREE_ADDRESS")]
    MemoryFreeAddress,

    // EVENTGROUP_USEREVENT
    // User events
    // Note that user event code range is 0x98..=0xA7
    // Allow for 0-15 arguments (the number of arg *records* (not arg count) is added to event code)
    // num_arg_records = EventCode - 0x98
    #[display(fmt = "USER_EVENT({_0})")]
    UserEvent(UserEventArgRecordCount),

    // EVENTGROUP_SYS
    #[display(fmt = "XTS8")]
    Xts8,
    #[display(fmt = "XTS16")]
    Xts16,
    #[display(fmt = "EVENT_BEING_WRITTEN")]
    EventBeingWritten,
    #[display(fmt = "RESERVED_DUMMY_CODE")]
    ReservedDummyCode,
    #[display(fmt = "LOW_POWER_BEGIN")]
    LowPowerBegin,
    #[display(fmt = "LOW_POWER_END")]
    LowPowerEnd,
    #[display(fmt = "XID")]
    Xid,
    #[display(fmt = "XTS16L")]
    Xts16l,

    // EVENTGROUP_TIMER
    #[display(fmt = "TIMER_CREATE")]
    TimerCreate,
    #[display(fmt = "TIMER_START")]
    TimerStart,
    #[display(fmt = "TIMER_RESET")]
    TimerReset,
    #[display(fmt = "TIMER_STOP")]
    TimerStop,
    #[display(fmt = "TIMER_CHANGE_PERIOD")]
    TimerChangePeriod,
    #[display(fmt = "TIMER_DELETE_OBJECT")]
    TimerDeleteObject,
    #[display(fmt = "TIMER_START_FROM_ISR")]
    TimerStartFromIsr,
    #[display(fmt = "TIMER_RESET_FROM_ISR")]
    TimerResetFromIsr,
    #[display(fmt = "TIMER_STOP_FROM_ISR")]
    TimerStopFromIsr,
    #[display(fmt = "TIMER_CREATE_FAILED")]
    TimerCreateFailed,
    #[display(fmt = "TIMER_START_FAILED")]
    TimerStartFailed,
    #[display(fmt = "TIMER_RESET_FAILED")]
    TimerResetFailed,
    #[display(fmt = "TIMER_STOP_FAILED")]
    TimerStopFailed,
    #[display(fmt = "TIMER_CHANGE_PERIOD_FAILED")]
    TimerChangePeriodFailed,
    #[display(fmt = "TIMER_DELETE_FAILED")]
    TimerDeleteFailed,
    #[display(fmt = "TIMER_START_FROM_ISR_FAILED")]
    TimerStartFromIsrFailed,
    #[display(fmt = "TIMER_RESET_FROM_ISR_FAILED")]
    TimerResetFromIsrFailed,
    #[display(fmt = "TIMER_STOP_FROM_ISR_FAILED")]
    TimerStopFromIsrFailed,

    // EVENTGROUP_EG
    #[display(fmt = "EVENT_GROUP_CREATE")]
    EventGroupCreate,
    #[display(fmt = "EVENT_GROUP_CREATE_FAILED")]
    EventGroupCreateFailed,
    #[display(fmt = "EVENT_GROUP_SYNC_BLOCK")]
    EventGroupSyncBlock,
    #[display(fmt = "EVENT_GROUP_SYNC_END")]
    EventGroupSyncEnd,
    #[display(fmt = "EVENT_GROUP_WAIT_BITS_BLOCK")]
    EventGroupWaitBitsBlock,
    #[display(fmt = "EVENT_GROUP_WAIT_BITS_END")]
    EventGroupWaitBitsEnd,
    #[display(fmt = "EVENT_GROUP_CLEAR_BITS")]
    EventGroupClearBits,
    #[display(fmt = "EVENT_GROUP_CLEAR_BITS_FROM_ISR")]
    EventGroupClearBitsFromIsr,
    #[display(fmt = "EVENT_GROUP_SET_BITS")]
    EventGroupSetBits,
    #[display(fmt = "EVENT_GROUP_DELETE_OBJECT")]
    EventGroupDeleteObject,
    #[display(fmt = "EVENT_GROUP_SYNC_END_FAILED")]
    EventGroupSyncEndFailed,
    #[display(fmt = "EVENT_GROUP_WAIT_BITS_END_FAILED")]
    EventGroupWaitBitsEndFailed,
    #[display(fmt = "EVENT_GROUP_SET_BITS_FROM_ISR")]
    EventGroupSetBitsFromIsr,
    #[display(fmt = "EVENT_GROUP_SET_BITS_FROM_ISR_FAILED")]
    EventGroupSetBitsFromIsrFailed,

    #[display(fmt = "TASK_INSTANCE_FINISHED_NEXT_KSE")]
    TaskInstanceFinishedNextKse,
    #[display(fmt = "TASK_INSTANCE_FINISHED_DIRECT")]
    TaskInstanceFinishedDirect,

    // TRACE_TASK_NOTIFY_GROUP
    #[display(fmt = "TASK_NOTIFY")]
    TaskNotify,
    #[display(fmt = "TASK_NOTIFY_TAKE")]
    TaskNotifyTake,
    #[display(fmt = "TASK_NOTIFY_TAKE_BLOCK")]
    TaskNotifyTakeBlock,
    #[display(fmt = "TASK_NOTIFY_FAILED")]
    TaskNotifyTakeFailed,
    #[display(fmt = "TASK_NOTIFY_WAIT")]
    TaskNotifyWait,
    #[display(fmt = "TASK_NOTIFY_WAIT_BLOCK")]
    TaskNotifyWaitBlock,
    #[display(fmt = "TASK_NOTIFY_WAIT_FAILED")]
    TaskNotifyWaitFailed,
    #[display(fmt = "TASK_NOTIFY_FROM_ISR")]
    TaskNotifyFromIsr,
    #[display(fmt = "TASK_NOTIFY_GIVE_FROM_ISR")]
    TaskNotifyGiveFromIsr,

    #[display(fmt = "TIMER_EXPIRED")]
    TimerExpired,

    // Events on queue peek (receive)
    #[display(fmt = "QUEUE_PEEK_BLOCK")]
    QueuePeekBlock,
    #[display(fmt = "SEMAPHORE_PEEK_BLOCK")]
    SemaphortPeekBlock,
    #[display(fmt = "MUTEX_PEEK_BLOCK")]
    MutexPeekBlock,

    // Events on queue peek (receive)
    #[display(fmt = "QUEUE_PEEK_FAILED")]
    QueuePeekFailed,
    #[display(fmt = "SEMAPHORE_PEEK_FAILED")]
    SemaphortPeekFailed,
    #[display(fmt = "MUTEX_PEEK_FAILED")]
    MutexPeekFailed,

    // EVENTGROUP_STREAMBUFFER_DIV
    #[display(fmt = "STREAMBUFFER_RESET")]
    StreambufferReset,
    #[display(fmt = "MESSAGEBUFFER_")]
    MessagebufferReset,
    #[display(fmt = "STREAMBUFFER_OBJCLOSE_NAME")]
    StreambufferObjectCloseName,
    #[display(fmt = "MESSAGEBUFFER_OBJCLOSE_NAME")]
    MessagebufferObjectCloseName,
    #[display(fmt = "STREAMBUFFER_OBJCLOSE_PROPERTY")]
    StreambufferObjectCloseProperty,
    #[display(fmt = "MESSAGEBUFFER_OBJCLOSE_PROPERTY")]
    MessagebufferObjectCloseProperty,

    // EVENTGROUP_MALLOC_FAILED
    #[display(fmt = "MEM_MALLOC_SIZE_FAILED")]
    MemoryMallocSizeFailed,
    #[display(fmt = "MEM_MALLOC_ADDRESS_FAILED")]
    MemoryFreeAddressFailed,

    #[display(fmt = "UNUSED_STACK")]
    UnusedStack,

    // Variant to handle unknown/unsupported event code
    #[display(fmt = "Unknown(0x{_0:02X})")]
    Unknown(EventCode),
}

impl From<EventCode> for EventType {
    fn from(ec: EventCode) -> Self {
        use EventType::*;
        match u8::from(ec) {
            0x00 => Null,

            0x01 => Xps,
            0x02 => TaskReady,
            0x03 => NewTime,

            0x04 => TaskSwitchIsrBegin,
            0x05 => TaskSwitchIsrResume,
            0x06 => TaskSwitchTaskBegin,
            0x07 => TaskSwitchTaskResume,

            raw @ 0x08..=0x0F => ObjectCloseName(ObjectClassCode::from_raw(raw)),

            raw @ 0x10..=0x17 => ObjectCloseProperty(ObjectClassCode::from_raw(raw)),

            raw @ 0x18..=0x1F => CreateObject(ObjectClassCode::from_raw(raw)),

            raw @ 0x20..=0x27 => Send(ObjectClassCode::from_raw(raw)),

            raw @ 0x28..=0x2F => Receive(ObjectClassCode::from_raw(raw)),

            raw @ 0x30..=0x37 => SendFromIsr(ObjectClassCode::from_raw(raw)),

            raw @ 0x38..=0x3F => ReceiveFromIsr(ObjectClassCode::from_raw(raw)),

            raw @ 0x40..=0x47 => CreateObjectFailed(ObjectClassCode::from_raw(raw)),

            raw @ 0x48..=0x4F => SendFailed(ObjectClassCode::from_raw(raw)),

            raw @ 0x50..=0x57 => ReceiveFailed(ObjectClassCode::from_raw(raw)),

            raw @ 0x58..=0x5F => SendFromIsrFailed(ObjectClassCode::from_raw(raw)),

            raw @ 0x60..=0x67 => ReceiveFromIsrFailed(ObjectClassCode::from_raw(raw)),

            raw @ 0x68..=0x6F => ReceiveBlock(ObjectClassCode::from_raw(raw)),

            raw @ 0x70..=0x77 => SendBlock(ObjectClassCode::from_raw(raw)),

            raw @ 0x78..=0x7F => Peek(ObjectClassCode::from_raw(raw)),

            raw @ 0x80..=0x87 => DeleteObject(ObjectClassCode::from_raw(raw)),

            0x88 => TaskDelayUntil,
            0x89 => TaskDelay,
            0x8A => TaskSuspend,
            0x8B => TaskResume,
            0x8C => TaskResumeFromIsr,
            0x8D => TaskPrioritySet,
            0x8E => TaskPriorityInherit,
            0x8F => TaskPriorityDisinherit,

            0x90 => PendFuncCall,
            0x91 => PendFuncCallFromIsr,
            0x92 => PendFuncCallFailed,
            0x93 => PendFuncCallFromIsrFailed,
            0x94 => MemoryMallocSize,
            0x95 => MemoryMallocAddress,
            0x96 => MemoryFreeSize,
            0x97 => MemoryFreeAddress,

            raw @ 0x98..=0xA7 => UserEvent(UserEventArgRecordCount(raw - 0x98)),

            0xA8 => Xts8,
            0xA9 => Xts16,
            0xAA => EventBeingWritten,
            0xAB => ReservedDummyCode,
            0xAC => LowPowerBegin,
            0xAD => LowPowerEnd,
            0xAE => Xid,
            0xAF => Xts16l,

            0xB0 => TimerCreate,
            0xB1 => TimerStart,
            0xB2 => TimerReset,
            0xB3 => TimerStop,
            0xB4 => TimerChangePeriod,
            0xB5 => TimerDeleteObject,
            0xB6 => TimerStartFromIsr,
            0xB7 => TimerResetFromIsr,
            0xB8 => TimerStopFromIsr,
            0xB9 => TimerCreateFailed,
            0xBA => TimerStartFailed,
            0xBB => TimerResetFailed,
            0xBC => TimerStopFailed,
            0xBD => TimerChangePeriodFailed,
            0xBE => TimerDeleteFailed,
            0xBF => TimerStartFromIsrFailed,
            0xC0 => TimerResetFromIsrFailed,
            0xC1 => TimerStopFromIsrFailed,

            0xC2 => EventGroupCreate,
            0xC3 => EventGroupCreateFailed,
            0xC4 => EventGroupSyncBlock,
            0xC5 => EventGroupSyncEnd,
            0xC6 => EventGroupWaitBitsBlock,
            0xC7 => EventGroupWaitBitsEnd,
            0xC8 => EventGroupClearBits,
            0xC9 => EventGroupClearBitsFromIsr,
            0xCA => EventGroupSetBits,
            0xCB => EventGroupDeleteObject,
            0xCC => EventGroupSyncEndFailed,
            0xCD => EventGroupWaitBitsEndFailed,
            0xCE => EventGroupSetBitsFromIsr,
            0xCF => EventGroupSetBitsFromIsrFailed,

            0xD0 => TaskInstanceFinishedNextKse,
            0xD1 => TaskInstanceFinishedDirect,

            0xD2 => TaskNotify,
            0xD3 => TaskNotifyTake,
            0xD4 => TaskNotifyTakeBlock,
            0xD5 => TaskNotifyTakeFailed,
            0xD6 => TaskNotifyWait,
            0xD7 => TaskNotifyWaitBlock,
            0xD8 => TaskNotifyWaitFailed,
            0xD9 => TaskNotifyFromIsr,
            0xDA => TaskNotifyGiveFromIsr,

            0xDB => TimerExpired,

            0xDC => QueuePeekBlock,
            0xDD => SemaphortPeekBlock,
            0xDE => MutexPeekBlock,

            0xDF => QueuePeekFailed,
            0xE0 => SemaphortPeekFailed,
            0xE1 => MutexPeekFailed,

            0xE2 => StreambufferReset,
            0xE3 => MessagebufferReset,
            0xE4 => StreambufferObjectCloseName,
            0xE5 => MessagebufferObjectCloseName,
            0xE6 => StreambufferObjectCloseProperty,
            0xE7 => MessagebufferObjectCloseProperty,

            0xE8 => MemoryMallocSizeFailed,
            0xE9 => MemoryFreeAddressFailed,

            0xEA => UnusedStack,

            _ => Unknown(ec),
        }
    }
}

impl From<EventType> for EventCode {
    fn from(et: EventType) -> Self {
        use EventType::*;
        let ec = match et {
            Null => 0x00,

            Xps => 0x01,
            TaskReady => 0x02,
            NewTime => 0x03,

            TaskSwitchIsrBegin => 0x04,
            TaskSwitchIsrResume => 0x05,
            TaskSwitchTaskBegin => 0x06,
            TaskSwitchTaskResume => 0x07,

            ObjectCloseName(occ) => 0x08 + occ.into_raw(),

            ObjectCloseProperty(occ) => 0x10 + occ.into_raw(),

            CreateObject(occ) => 0x18 + occ.into_raw(),

            Send(occ) => 0x20 + occ.into_raw(),

            Receive(occ) => 0x28 + occ.into_raw(),

            SendFromIsr(occ) => 0x30 + occ.into_raw(),

            ReceiveFromIsr(occ) => 0x38 + occ.into_raw(),

            CreateObjectFailed(occ) => 0x40 + occ.into_raw(),

            SendFailed(occ) => 0x48 + occ.into_raw(),

            ReceiveFailed(occ) => 0x50 + occ.into_raw(),

            SendFromIsrFailed(occ) => 0x58 + occ.into_raw(),

            ReceiveFromIsrFailed(occ) => 0x60 + occ.into_raw(),

            ReceiveBlock(occ) => 0x68 + occ.into_raw(),

            SendBlock(occ) => 0x70 + occ.into_raw(),

            Peek(occ) => 0x78 + occ.into_raw(),

            DeleteObject(occ) => 0x80 + occ.into_raw(),

            TaskDelayUntil => 0x88,
            TaskDelay => 0x89,
            TaskSuspend => 0x8A,
            TaskResume => 0x8B,
            TaskResumeFromIsr => 0x8C,
            TaskPrioritySet => 0x8D,
            TaskPriorityInherit => 0x8E,
            TaskPriorityDisinherit => 0x8F,

            PendFuncCall => 0x90,
            PendFuncCallFromIsr => 0x91,
            PendFuncCallFailed => 0x92,
            PendFuncCallFromIsrFailed => 0x93,
            MemoryMallocSize => 0x94,
            MemoryMallocAddress => 0x95,
            MemoryFreeSize => 0x96,
            MemoryFreeAddress => 0x97,

            UserEvent(arc) => 0x98 + arc.0,

            Xts8 => 0xA8,
            Xts16 => 0xA9,
            EventBeingWritten => 0xAA,
            ReservedDummyCode => 0xAB,
            LowPowerBegin => 0xAC,
            LowPowerEnd => 0xAD,
            Xid => 0xAE,
            Xts16l => 0xAF,

            TimerCreate => 0xB0,
            TimerStart => 0xB1,
            TimerReset => 0xB2,
            TimerStop => 0xB3,
            TimerChangePeriod => 0xB4,
            TimerDeleteObject => 0xB5,
            TimerStartFromIsr => 0xB6,
            TimerResetFromIsr => 0xB7,
            TimerStopFromIsr => 0xB8,
            TimerCreateFailed => 0xB9,
            TimerStartFailed => 0xBA,
            TimerResetFailed => 0xBB,
            TimerStopFailed => 0xBC,
            TimerChangePeriodFailed => 0xBD,
            TimerDeleteFailed => 0xBE,
            TimerStartFromIsrFailed => 0xBF,
            TimerResetFromIsrFailed => 0xC0,
            TimerStopFromIsrFailed => 0xC1,

            EventGroupCreate => 0xC2,
            EventGroupCreateFailed => 0xC3,
            EventGroupSyncBlock => 0xC4,
            EventGroupSyncEnd => 0xC5,
            EventGroupWaitBitsBlock => 0xC6,
            EventGroupWaitBitsEnd => 0xC7,
            EventGroupClearBits => 0xC8,
            EventGroupClearBitsFromIsr => 0xC9,
            EventGroupSetBits => 0xCA,
            EventGroupDeleteObject => 0xCB,
            EventGroupSyncEndFailed => 0xCC,
            EventGroupWaitBitsEndFailed => 0xCD,
            EventGroupSetBitsFromIsr => 0xCE,
            EventGroupSetBitsFromIsrFailed => 0xCF,

            TaskInstanceFinishedNextKse => 0xD0,
            TaskInstanceFinishedDirect => 0xD1,

            TaskNotify => 0xD2,
            TaskNotifyTake => 0xD3,
            TaskNotifyTakeBlock => 0xD4,
            TaskNotifyTakeFailed => 0xD5,
            TaskNotifyWait => 0xD6,
            TaskNotifyWaitBlock => 0xD7,
            TaskNotifyWaitFailed => 0xD8,
            TaskNotifyFromIsr => 0xD9,
            TaskNotifyGiveFromIsr => 0xDA,

            TimerExpired => 0xDB,

            QueuePeekBlock => 0xDC,
            SemaphortPeekBlock => 0xDD,
            MutexPeekBlock => 0xDE,

            QueuePeekFailed => 0xDF,
            SemaphortPeekFailed => 0xE0,
            MutexPeekFailed => 0xE1,

            StreambufferReset => 0xE2,
            MessagebufferReset => 0xE3,
            StreambufferObjectCloseName => 0xE4,
            MessagebufferObjectCloseName => 0xE5,
            StreambufferObjectCloseProperty => 0xE6,
            MessagebufferObjectCloseProperty => 0xE7,

            MemoryMallocSizeFailed => 0xE8,
            MemoryFreeAddressFailed => 0xE9,

            UnusedStack => 0xEA,

            Unknown(raw) => raw.0,
        };
        EventCode(ec)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum Event {
    #[display(fmt = "TaskBegin({_0})")]
    IsrBegin(IsrBeginEvent),
    #[display(fmt = "IsrResume({_0})")]
    IsrResume(IsrResumeEvent),
    #[display(fmt = "TaskBegin({_0})")]
    TaskBegin(TaskBeginEvent),
    #[display(fmt = "TaskReady({_0})")]
    TaskReady(TaskReadyEvent),
    #[display(fmt = "TaskResume({_0})")]
    TaskResume(TaskResumeEvent),
    #[display(fmt = "LowPowerBegin({_0})")]
    LowPowerBegin(LowPowerBeginEvent),
    #[display(fmt = "LowPowerEnd({_0})")]
    LowPowerEnd(LowPowerEndEvent),
    #[display(fmt = "User({_0})")]
    User(UserEvent),
    #[display(fmt = "EventRecord({_0})")]
    Unknown(EventRecord),
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn event_type_roundtrip() {
        for raw in 0..=0xFF {
            let ec = EventCode(raw);
            let et = EventType::from(ec);
            assert_eq!(ec, EventCode::from(et));
        }
    }

    #[test]
    fn obj_class_code_roundtrip() {
        for raw in 0..=0x07 {
            let occ = ObjectClassCode(raw);
            let oc = occ.into_class();
            assert_eq!(raw as usize, oc.into_usize());
        }
    }
}
