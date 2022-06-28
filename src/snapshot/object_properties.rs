use derive_more::{Binary, Deref, Display, Into, LowerHex, Octal, UpperHex};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::num::NonZeroU16;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ObjectPropertyTable {
    pub queue_object_properties: BTreeMap<ObjectHandle, ObjectProperties<QueueObjectClass>>,
    pub semaphore_object_properties: BTreeMap<ObjectHandle, ObjectProperties<SemaphoreObjectClass>>,
    pub mutex_object_properties: BTreeMap<ObjectHandle, ObjectProperties<MutexObjectClass>>,
    pub task_object_properties: BTreeMap<ObjectHandle, ObjectProperties<TaskObjectClass>>,
    pub isr_object_properties: BTreeMap<ObjectHandle, ObjectProperties<IsrObjectClass>>,
    pub timer_object_properties: BTreeMap<ObjectHandle, ObjectProperties<TimerObjectClass>>,
    pub event_group_object_properties:
        BTreeMap<ObjectHandle, ObjectProperties<EventGroupObjectClass>>,
    pub stream_buffer_object_properties:
        BTreeMap<ObjectHandle, ObjectProperties<StreamBufferObjectClass>>,
    pub message_buffer_object_properties:
        BTreeMap<ObjectHandle, ObjectProperties<MessageBufferObjectClass>>,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum ObjectClass {
    #[display(fmt = "Queue")]
    Queue = 0,
    #[display(fmt = "Semaphore")]
    Semaphore = 1,
    #[display(fmt = "Mutex")]
    Mutex = 2,
    #[display(fmt = "Task")]
    Task = 3,
    #[display(fmt = "ISR")]
    Isr = 4,
    #[display(fmt = "Timer")]
    Timer = 5,
    #[display(fmt = "EventGroup")]
    EventGroup = 6,
    #[display(fmt = "StreamBuffer")]
    StreamBuffer = 7,
    #[display(fmt = "MessageBuffer")]
    MessageBuffer = 8,
}

impl ObjectClass {
    pub(crate) fn into_usize(self) -> usize {
        self as _
    }

    pub(crate) fn enumerate() -> &'static [Self] {
        use ObjectClass::*;
        &[
            Queue,
            Semaphore,
            Mutex,
            Task,
            Isr,
            Timer,
            EventGroup,
            StreamBuffer,
            MessageBuffer,
        ]
    }

    pub(crate) fn properties_size(self) -> usize {
        use ObjectClass::*;
        match self {
            Queue => 1,
            Semaphore => 1,
            Mutex => 1,
            Task => 4,
            Isr => 2,
            Timer => 1,
            EventGroup => 4,
            StreamBuffer => 4,
            MessageBuffer => 4,
        }
    }
}

pub trait ObjectClassExt {
    fn class() -> ObjectClass;
}

#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    Deref,
    Into,
    Display,
    Binary,
    Octal,
    LowerHex,
    UpperHex,
)]
#[display(fmt = "{_0}")]
pub struct ObjectHandle(pub(crate) NonZeroU16);

impl ObjectHandle {
    pub(crate) fn new(handle: u16) -> Option<Self> {
        Some(ObjectHandle(NonZeroU16::new(handle)?))
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}('{}')", "self.class()", "self.display_name()")]
pub struct ObjectProperties<C: ObjectClassExt> {
    name: Option<String>,
    properties: [u8; 4],
    _class: PhantomData<C>,
}

impl<C: ObjectClassExt> ObjectProperties<C> {
    const UNNAMED_OBJECT: &'static str = "<unnamed>";

    pub(crate) fn new(name: Option<String>, properties: [u8; 4]) -> Self {
        ObjectProperties {
            name,
            properties,
            _class: PhantomData,
        }
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn display_name(&self) -> &str {
        self.name().unwrap_or(Self::UNNAMED_OBJECT)
    }

    pub fn class(&self) -> ObjectClass {
        C::class()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct QueueObjectClass;
impl ObjectClassExt for QueueObjectClass {
    fn class() -> ObjectClass {
        ObjectClass::Queue
    }
}

impl ObjectProperties<QueueObjectClass> {
    /// Current number of message in queue
    pub fn queue_length(&self) -> u8 {
        self.properties[0]
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct SemaphoreObjectClass;
impl ObjectClassExt for SemaphoreObjectClass {
    fn class() -> ObjectClass {
        ObjectClass::Semaphore
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum SemaphoreState {
    #[display(fmt = "Cleared")]
    Cleared,
    #[display(fmt = "Signaled")]
    Signaled,
}

impl ObjectProperties<SemaphoreObjectClass> {
    pub fn state(&self) -> SemaphoreState {
        if self.properties[0] == 0 {
            SemaphoreState::Cleared
        } else {
            SemaphoreState::Signaled
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct MutexObjectClass;
impl ObjectClassExt for MutexObjectClass {
    fn class() -> ObjectClass {
        ObjectClass::Mutex
    }
}
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Into, Display)]
#[display(fmt = "{_0}")]
pub struct TaskHandle(u8);

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum MutexOwner {
    #[display(fmt = "TaskHandle({_0})")]
    TaskHandle(TaskHandle),
    #[display(fmt = "Free")]
    Free,
}

impl ObjectProperties<MutexObjectClass> {
    pub fn owner(&self) -> MutexOwner {
        let owner = self.properties[0];
        if owner == 0 {
            MutexOwner::Free
        } else {
            MutexOwner::TaskHandle(TaskHandle(owner))
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct TaskObjectClass;
impl ObjectClassExt for TaskObjectClass {
    fn class() -> ObjectClass {
        ObjectClass::Task
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Into, Display)]
#[display(fmt = "{_0}")]
pub struct TaskPriority(u8);

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum TaskState {
    #[display(fmt = "Inactive")]
    Inactive,
    #[display(fmt = "Active")]
    Active,
}

impl ObjectProperties<TaskObjectClass> {
    pub fn current_priority(&self) -> TaskPriority {
        TaskPriority(self.properties[0])
    }

    pub fn state(&self) -> TaskState {
        if self.properties[1] == 0 {
            TaskState::Inactive
        } else {
            TaskState::Active
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct IsrObjectClass;
impl ObjectClassExt for IsrObjectClass {
    fn class() -> ObjectClass {
        ObjectClass::Isr
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Into, Display)]
#[display(fmt = "{_0}")]
pub struct IsrPriority(u8);

impl ObjectProperties<IsrObjectClass> {
    pub fn priority(&self) -> IsrPriority {
        IsrPriority(self.properties[0])
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct TimerObjectClass;
impl ObjectClassExt for TimerObjectClass {
    fn class() -> ObjectClass {
        ObjectClass::Timer
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EventGroupObjectClass;
impl ObjectClassExt for EventGroupObjectClass {
    fn class() -> ObjectClass {
        ObjectClass::EventGroup
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct StreamBufferObjectClass;
impl ObjectClassExt for StreamBufferObjectClass {
    fn class() -> ObjectClass {
        ObjectClass::StreamBuffer
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct MessageBufferObjectClass;
impl ObjectClassExt for MessageBufferObjectClass {
    fn class() -> ObjectClass {
        ObjectClass::MessageBuffer
    }
}
