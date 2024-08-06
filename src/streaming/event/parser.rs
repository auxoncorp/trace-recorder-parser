use crate::streaming::event::*;
use crate::streaming::{EntryTable, Error, HeaderInfo};
use crate::time::{Frequency, Ticks};
use crate::types::{
    format_symbol_string, Endianness, FormatString, FormattedString, Heap, ObjectClass,
    ObjectHandle, ObjectName, Priority, Protocol, SymbolString, TimerCounter, TrimmedString,
    UserEventChannel,
};
use byteordered::ByteOrdered;
use std::io::{self, Read};
use tracing::error;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EventParser {
    /// Endianness of the data
    endianness: byteordered::Endianness,

    /// Initial heap from the entry table, maintained by the parser
    heap: Heap,

    /// Event ID for custom printf events, if enabled
    custom_printf_event_id: Option<EventId>,

    /// Local scratch buffer for reading strings
    buf: Vec<u8>,

    /// Local scratch buffer for reading argument data
    arg_buf: Vec<u8>,
}

impl EventParser {
    pub fn new(endianness: Endianness, heap: Heap) -> Self {
        Self {
            endianness: byteordered::Endianness::from(endianness),
            heap,
            custom_printf_event_id: None,
            buf: Vec::with_capacity(256),
            arg_buf: Vec::with_capacity(256),
        }
    }

    pub fn set_custom_printf_event_id(&mut self, custom_printf_event_id: EventId) {
        self.custom_printf_event_id = Some(custom_printf_event_id);
    }

    pub fn system_heap(&self) -> &Heap {
        &self.heap
    }

    pub fn next_event<R: Read>(
        &mut self,
        mut r: &mut R,
        entry_table: &mut EntryTable,
    ) -> Result<Option<(EventCode, Event)>, Error> {
        let first_word = {
            let mut r = ByteOrdered::le(&mut r);
            let word = match r.read_u32() {
                Ok(w) => w,
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
                Err(e) => return Err(e.into()),
            };
            match word {
                HeaderInfo::PSF_LITTLE_ENDIAN => {
                    return Err(Error::TraceRestarted(Endianness::Little))
                }
                HeaderInfo::PSF_BIG_ENDIAN => return Err(Error::TraceRestarted(Endianness::Big)),
                _ => word.to_le_bytes(),
            }
        };

        let mut first_word_reader = ByteOrdered::new(first_word.as_slice(), self.endianness);
        let mut r = ByteOrdered::new(r, self.endianness);

        let event_code = match first_word_reader.read_u16() {
            Ok(ec) => EventCode(ec),
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let event_type = event_code.event_type();
        let event_id = event_code.event_id();
        let event_count = EventCount(first_word_reader.read_u16()?);
        let timestamp = Timestamp(r.read_u32()?.into());
        let num_params = event_code.parameter_count();

        if let Some(expected_parameter_count) = event_type.expected_parameter_count() {
            if usize::from(num_params) != expected_parameter_count {
                return Err(Error::InvalidEventParameterCount(
                    event_code.event_id(),
                    expected_parameter_count,
                    num_params,
                ));
            }
        }

        Ok(match event_type {
            EventType::TraceStart => {
                let handle = object_handle(&mut r, event_id)?;
                let sym = entry_table
                    .symbol(handle)
                    .ok_or(Error::ObjectLookup(handle))?;
                let event = TraceStartEvent {
                    event_count,
                    timestamp,
                    current_task_handle: handle,
                    current_task: sym.clone().into(),
                };
                Some((event_code, Event::TraceStart(event)))
            }

            EventType::TsConfig => {
                let uses_custom_timer = match num_params.0 {
                    4 => false,
                    // TRC_HWTC_TYPE == TRC_CUSTOM_TIMER_INCR || TRC_HWTC_TYPE == TRC_CUSTOM_TIMER_DECR
                    5 => true,
                    _ => {
                        return Err(Error::InvalidEventParameterCount(
                            event_code.event_id(),
                            4, // base count
                            num_params,
                        ));
                    }
                };
                let frequency = Frequency(r.read_u32()?);
                let tick_rate_hz = r.read_u32()?;
                let hwtc_type = r.read_u32()?;
                let isr_chaining_threshold = r.read_u32()?;
                let htc_period = if uses_custom_timer {
                    r.read_u32()?.into()
                } else {
                    None
                };
                let event = TsConfigEvent {
                    event_count,
                    timestamp,
                    frequency,
                    tick_rate_hz,
                    hwtc_type: TimerCounter::from_hwtc_type(hwtc_type)
                        .ok_or(Error::InvalidTimerCounter(hwtc_type))?,
                    isr_chaining_threshold,
                    htc_period,
                };
                Some((event_code, Event::TsConfig(event)))
            }

            EventType::ObjectName => {
                // Always expect at least a handle
                if num_params.0 < 1 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        1,
                        num_params,
                    ));
                }
                let handle = object_handle(&mut r, event_id)?;
                let symbol: SymbolString = self
                    .read_string(&mut r, (usize::from(num_params) - 1) * 4)?
                    .into();
                entry_table.entry(handle).set_symbol(symbol.clone());
                let event = ObjectNameEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: symbol,
                };
                Some((event_code, Event::ObjectName(event)))
            }

            EventType::TaskPriority
            | EventType::TaskPriorityInherit
            | EventType::TaskPriorityDisinherit => {
                let handle = object_handle(&mut r, event_id)?;
                let priority = Priority(r.read_u32()?);
                let entry = entry_table.entry(handle);
                entry.states.set_priority(priority);
                let sym = entry.symbol.as_ref().ok_or(Error::ObjectLookup(handle))?;
                let event = TaskEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: sym.clone().into(),
                    priority,
                };
                Some((
                    event_code,
                    match event_type {
                    EventType::TaskPriority => Event::TaskPriority(event),
                    EventType::TaskPriorityInherit => Event::TaskPriorityInherit(event),
                    _ /*EventType::TaskPriorityDisinherit*/ => Event::TaskPriorityDisinherit(event),
                },
                ))
            }

            EventType::DefineIsr => {
                // Always expect at least a handle
                if num_params.0 < 1 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        1,
                        num_params,
                    ));
                }
                let handle = object_handle(&mut r, event_id)?;
                let priority = Priority(r.read_u32()?);
                let symbol: SymbolString = self
                    .read_string(&mut r, (usize::from(num_params) - 2) * 4)?
                    .into();
                let entry = entry_table.entry(handle);
                entry.states.set_priority(priority);
                entry.set_symbol(symbol.clone());
                entry.set_class(ObjectClass::Isr);
                let event = IsrEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: symbol.into(),
                    priority,
                };
                Some((event_code, Event::IsrDefine(event)))
            }

            EventType::TaskCreate => {
                let handle = object_handle(&mut r, event_id)?;
                let priority = Priority(r.read_u32()?);
                let entry = entry_table.entry(handle);
                entry.states.set_priority(priority);
                entry.set_class(ObjectClass::Task);
                let sym = entry.symbol.as_ref().ok_or(Error::ObjectLookup(handle))?;
                let event = TaskEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: sym.clone().into(),
                    priority,
                };
                Some((event_code, Event::TaskCreate(event)))
            }

            EventType::TaskReady => {
                let handle = object_handle(&mut r, event_id)?;
                let entry = entry_table.entry(handle);
                let sym = entry.symbol.as_ref().ok_or(Error::ObjectLookup(handle))?;
                let event = TaskEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: sym.clone().into(),
                    priority: entry.states.priority(),
                };
                Some((event_code, Event::TaskReady(event)))
            }

            EventType::TaskSwitchIsrBegin => {
                let handle = object_handle(&mut r, event_id)?;
                let entry = entry_table.entry(handle);
                let sym = entry.symbol.as_ref().ok_or(Error::ObjectLookup(handle))?;
                let event = IsrEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: sym.clone().into(),
                    priority: entry.states.priority(),
                };
                Some((event_code, Event::IsrBegin(event)))
            }

            EventType::TaskSwitchIsrResume => {
                let handle = object_handle(&mut r, event_id)?;
                let entry = entry_table.entry(handle);
                let sym = entry.symbol.as_ref().ok_or(Error::ObjectLookup(handle))?;
                let event = IsrEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: sym.clone().into(),
                    priority: entry.states.priority(),
                };
                Some((event_code, Event::IsrResume(event)))
            }

            EventType::TaskSwitchTaskResume => {
                let handle = object_handle(&mut r, event_id)?;
                let entry = entry_table.entry(handle);
                let sym = entry.symbol.as_ref().ok_or(Error::ObjectLookup(handle))?;
                let event = TaskEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: sym.clone().into(),
                    priority: entry.states.priority(),
                };
                Some((event_code, Event::TaskResume(event)))
            }

            EventType::TaskActivate => {
                if (num_params.0 != 1) && (num_params.0 != 2) {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        1,
                        num_params,
                    ));
                }
                let handle = object_handle(&mut r, event_id)?;
                let entry = entry_table.entry(handle);

                if num_params.0 == 2 {
                    let priority = Priority(r.read_u32()?);
                    entry.states.set_priority(priority);
                }

                let sym = entry.symbol.as_ref().ok_or(Error::ObjectLookup(handle))?;
                let event = TaskEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: sym.clone().into(),
                    priority: entry.states.priority(),
                };
                Some((event_code, Event::TaskActivate(event)))
            }

            EventType::TaskNotify | EventType::TaskNotifyFromIsr => {
                let handle: ObjectHandle = object_handle(&mut r, event_id)?;
                let entry = entry_table.entry(handle);
                let event = TaskNotifyEvent {
                    event_count,
                    timestamp,
                    handle,
                    task_name: entry.symbol.clone().map(ObjectName::from),
                    ticks_to_wait: None,
                };
                Some((
                    event_code,
                    match event_type {
                            EventType::TaskNotify => Event::TaskNotify(event),
                            _ /*EventType::TaskNotifyFromIsr*/ => Event::TaskNotifyFromIsr(event),
                        },
                ))
            }

            EventType::TaskNotifyWait | EventType::TaskNotifyWaitBlock => {
                let handle: ObjectHandle = object_handle(&mut r, event_id)?;
                let ticks_to_wait = Some(Ticks(r.read_u32()?));
                let entry = entry_table.entry(handle);
                let event = TaskNotifyEvent {
                    event_count,
                    timestamp,
                    handle,
                    task_name: entry.symbol.clone().map(ObjectName::from),
                    ticks_to_wait,
                };
                Some((
                    event_code,
                    match event_type {
                            EventType::TaskNotifyWait => Event::TaskNotifyWait(event),
                            _ /*EventType::TaskNotifyWaitBlock*/ => Event::TaskNotifyWaitBlock(event),
                        },
                ))
            }

            EventType::MemoryAlloc | EventType::MemoryFree => {
                let address = r.read_u32()?;
                let size = r.read_u32()?;
                if matches!(event_type, EventType::MemoryAlloc) {
                    self.heap.handle_alloc(size);
                } else {
                    self.heap.handle_free(size);
                }
                let event = MemoryEvent {
                    event_count,
                    timestamp,
                    address,
                    size,
                    heap: self.heap,
                };
                Some((
                    event_code,
                    if matches!(event_type, EventType::MemoryAlloc) {
                        Event::MemoryAlloc(event)
                    } else {
                        Event::MemoryFree(event)
                    },
                ))
            }

            EventType::QueueCreate => {
                let handle = object_handle(&mut r, event_id)?;
                let queue_length = r.read_u32()?;
                let entry = entry_table.entry(handle);
                entry.set_class(ObjectClass::Queue);
                let event = QueueCreateEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: entry.symbol.clone().map(ObjectName::from),
                    queue_length,
                };
                Some((event_code, Event::QueueCreate(event)))
            }

            EventType::QueueSend
            | EventType::QueueSendBlock
            | EventType::QueueSendFromIsr
            | EventType::QueueReceiveFromIsr
            | EventType::QueueSendFront
            | EventType::QueueSendFrontBlock
            | EventType::QueueSendFrontFromIsr => {
                let handle: ObjectHandle = object_handle(&mut r, event_id)?;
                let messages_waiting = r.read_u32()?;
                let event = QueueEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: entry_table.symbol(handle).cloned().map(ObjectName::from),
                    ticks_to_wait: None,
                    messages_waiting,
                };
                Some((
                    event_code,
                    match event_type {
                        EventType::QueueSend => Event::QueueSend(event),
                        EventType::QueueSendBlock => Event::QueueSendBlock(event),
                        EventType::QueueSendFromIsr => Event::QueueSendFromIsr(event),
                        EventType::QueueReceiveFromIsr => Event::QueueReceiveFromIsr(event),
                        EventType::QueueSendFront => Event::QueueSendFront(event),
                        EventType::QueueSendFrontBlock => Event::QueueSendFrontBlock(event),
                        _ /*EventType::QueueSendFrontFromIsr*/ => Event::QueueSendFrontFromIsr(event),
                    },
                ))
            }

            EventType::QueueReceive
            | EventType::QueueReceiveBlock
            | EventType::QueuePeek
            | EventType::QueuePeekBlock => {
                let handle: ObjectHandle = object_handle(&mut r, event_id)?;
                let ticks_to_wait = Some(Ticks(r.read_u32()?));
                let messages_waiting = r.read_u32()?;
                let event = QueueEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: entry_table.symbol(handle).cloned().map(ObjectName::from),
                    ticks_to_wait,
                    messages_waiting,
                };
                Some((
                    event_code,
                    match event_type {
                        EventType::QueueReceive => Event::QueueReceive(event),
                        EventType::QueueReceiveBlock => Event::QueueReceiveBlock(event),
                        EventType::QueuePeek => Event::QueuePeek(event),
                        _ /*EventType::QueuePeekBlock*/ => Event::QueuePeekBlock(event),
                    },
                ))
            }

            EventType::MutexCreate => {
                let handle: ObjectHandle = object_handle(&mut r, event_id)?;
                let _unused = r.read_u32()?;
                let entry = entry_table.entry(handle);
                entry.set_class(ObjectClass::Mutex);
                let event = MutexCreateEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: entry.symbol.clone().map(ObjectName::from),
                };
                Some((event_code, Event::MutexCreate(event)))
            }

            EventType::MutexGive | EventType::MutexGiveBlock | EventType::MutexGiveRecursive => {
                let handle: ObjectHandle = object_handle(&mut r, event_id)?;
                let entry = entry_table.entry(handle);
                entry.set_class(ObjectClass::Mutex);
                let event = MutexEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: entry.symbol.clone().map(ObjectName::from),
                    ticks_to_wait: None,
                };
                Some((
                    event_code,
                    match event_type {
                            EventType::MutexGive => Event::MutexGive(event),
                            EventType::MutexGiveBlock => Event::MutexGiveBlock(event),
                            _ /*EventType::MutexGiveRecursive*/ => Event::MutexGiveRecursive(event),
                        },
                ))
            }

            EventType::MutexTake
            | EventType::MutexTakeBlock
            | EventType::MutexTakeRecursive
            | EventType::MutexTakeRecursiveBlock => {
                let handle: ObjectHandle = object_handle(&mut r, event_id)?;
                let ticks_to_wait = Some(Ticks(r.read_u32()?));
                let entry = entry_table.entry(handle);
                entry.set_class(ObjectClass::Mutex);
                let event = MutexEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: entry.symbol.clone().map(ObjectName::from),
                    ticks_to_wait,
                };
                Some((
                    event_code,
                    match event_type {
                            EventType::MutexTake => Event::MutexTake(event),
                            EventType::MutexTakeBlock => Event::MutexTakeBlock(event),
                            EventType::MutexTakeRecursive => Event::MutexTakeRecursive(event),
                            _ /*EventType::MutexTakeRecursiveBlock*/ => Event::MutexTakeRecursiveBlock(event),
                        },
                ))
            }

            EventType::SemaphoreBinaryCreate => {
                let handle: ObjectHandle = object_handle(&mut r, event_id)?;
                let _unused = r.read_u32()?;
                let entry = entry_table.entry(handle);
                entry.set_class(ObjectClass::Semaphore);
                let event = SemaphoreCreateEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: entry.symbol.clone().map(ObjectName::from),
                    count: None,
                };
                Some((event_code, Event::SemaphoreBinaryCreate(event)))
            }

            EventType::SemaphoreCountingCreate => {
                let handle = object_handle(&mut r, event_id)?;
                let count = Some(r.read_u32()?);
                let entry = entry_table.entry(handle);
                entry.set_class(ObjectClass::Semaphore);
                let event = SemaphoreCreateEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: entry.symbol.clone().map(ObjectName::from),
                    count,
                };
                Some((event_code, Event::SemaphoreCountingCreate(event)))
            }

            EventType::SemaphoreGive
            | EventType::SemaphoreGiveBlock
            | EventType::SemaphoreGiveFromIsr
            | EventType::SemaphoreTakeFromIsr => {
                let handle: ObjectHandle = object_handle(&mut r, event_id)?;
                let count = r.read_u32()?;
                let event = SemaphoreEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: entry_table.symbol(handle).cloned().map(ObjectName::from),
                    ticks_to_wait: None,
                    count,
                };
                Some((
                    event_code,
                    match event_type {
                        EventType::SemaphoreGive => Event::SemaphoreGive(event),
                        EventType::SemaphoreGiveBlock => Event::SemaphoreGiveBlock(event),
                        EventType::SemaphoreGiveFromIsr => Event::SemaphoreGiveFromIsr(event),
                        _ /*EventType::SemaphoreTakeFromIsr*/ => Event::SemaphoreTakeFromIsr(event),
                    },
                ))
            }

            EventType::SemaphoreTake
            | EventType::SemaphoreTakeBlock
            | EventType::SemaphorePeek
            | EventType::SemaphorePeekBlock => {
                let handle: ObjectHandle = object_handle(&mut r, event_id)?;
                let ticks_to_wait = Some(Ticks(r.read_u32()?));
                let count = r.read_u32()?;
                let event = SemaphoreEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: entry_table.symbol(handle).cloned().map(ObjectName::from),
                    ticks_to_wait,
                    count,
                };
                Some((
                    event_code,
                    match event_type {
                        EventType::SemaphoreTake => Event::SemaphoreTake(event),
                        EventType::SemaphoreTakeBlock => Event::SemaphoreTakeBlock(event),
                        EventType::SemaphorePeek => Event::SemaphorePeek(event),
                        _ /*EventType::SemaphorePeekBlock*/ => Event::SemaphorePeekBlock(event),
                    },
                ))
            }

            EventType::EventGroupCreate => {
                let handle: ObjectHandle = object_handle(&mut r, event_id)?;
                let event_bits = r.read_u32()?;
                let entry = entry_table.entry(handle);
                entry.set_class(ObjectClass::EventGroup);
                let event = EventGroupCreateEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: entry.symbol.clone().map(ObjectName::from),
                    event_bits,
                };
                Some((event_code, Event::EventGroupCreate(event)))
            }

            EventType::EventGroupSync
            | EventType::EventGroupWaitBits
            | EventType::EventGroupClearBits
            | EventType::EventGroupClearBitsFromIsr
            | EventType::EventGroupSetBits
            | EventType::EventGroupSetBitsFromIsr
            | EventType::EventGroupSyncBlock
            | EventType::EventGroupWaitBitsBlock => {
                let handle = object_handle(&mut r, event_id)?;
                let bits = r.read_u32()?;
                let event = EventGroupEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: entry_table.symbol(handle).cloned().map(ObjectName::from),
                    bits,
                };
                Some((
                    event_code,
                    match event_type {
                        EventType::EventGroupSync => Event::EventGroupSync(event),
                        EventType::EventGroupWaitBits => Event::EventGroupWaitBits(event),
                        EventType::EventGroupClearBits => Event::EventGroupClearBits(event),
                        EventType::EventGroupClearBitsFromIsr => Event::EventGroupClearBitsFromIsr(event),
                        EventType::EventGroupSetBits => Event::EventGroupSetBits(event),
                        EventType::EventGroupSetBitsFromIsr => Event::EventGroupSetBitsFromIsr(event),
                        EventType::EventGroupSyncBlock => Event::EventGroupSyncBlock(event),
                        _ /*EventType::EventGroupWaitBitsBlock*/ => Event::EventGroupWaitBitsBlock(event),
                    },
                ))
            }

            EventType::MessageBufferCreate => {
                let handle: ObjectHandle = object_handle(&mut r, event_id)?;
                let buffer_size = r.read_u32()?;
                let entry = entry_table.entry(handle);
                entry.set_class(ObjectClass::MessageBuffer);
                let event = MessageBufferCreateEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: entry.symbol.clone().map(ObjectName::from),
                    buffer_size,
                };
                Some((event_code, Event::MessageBufferCreate(event)))
            }

            EventType::MessageBufferSend
            | EventType::MessageBufferReceive
            | EventType::MessageBufferSendFromIsr
            | EventType::MessageBufferReceiveFromIsr
            | EventType::MessageBufferReset => {
                let handle = object_handle(&mut r, event_id)?;
                let bytes_in_buffer = r.read_u32()?;
                let event = MessageBufferEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: entry_table.symbol(handle).cloned().map(ObjectName::from),
                    bytes_in_buffer,
                };
                Some((
                    event_code,
                    match event_type {
                        EventType::MessageBufferSend => Event::MessageBufferSend(event),
                        EventType::MessageBufferReceive => Event::MessageBufferReceive(event),
                        EventType::MessageBufferSendFromIsr => Event::MessageBufferSendFromIsr(event),
                        EventType::MessageBufferReceiveFromIsr => Event::MessageBufferReceiveFromIsr(event),
                        _ /*EventType::MessageBufferReset*/ => Event::MessageBufferReset(event),
                    },
                ))
            }

            EventType::MessageBufferSendBlock | EventType::MessageBufferReceiveBlock => {
                let handle = object_handle(&mut r, event_id)?;
                let event = MessageBufferBlockEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: entry_table.symbol(handle).cloned().map(ObjectName::from),
                };
                Some((
                    event_code,
                    match event_type {
                        EventType::MessageBufferSendBlock => Event::MessageBufferSendBlock(event),
                        _ /*EventType::MessageBufferReceiveBlock*/ => Event::MessageBufferReceiveBlock(event),
                    },
                ))
            }

            EventType::StateMachineCreate => {
                let handle = object_handle(&mut r, event_id)?;
                let _unused = r.read_u32()?;
                let entry = entry_table.entry(handle);
                entry.set_class(ObjectClass::StateMachine);
                let sym = entry.symbol.as_ref().ok_or(Error::ObjectLookup(handle))?;
                let event = StateMachineCreateEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: sym.clone().into(),
                };
                Some((event_code, Event::StateMachineCreate(event)))
            }

            EventType::StateMachineStateCreate => {
                let state_handle = object_handle(&mut r, event_id)?;
                let state_machine_handle = object_handle(&mut r, event_id)?;
                let entry = entry_table.entry(state_handle);
                entry.set_class(ObjectClass::StateMachine);
                let state_machine_sym = entry_table
                    .entry(state_machine_handle)
                    .symbol
                    .as_ref()
                    .map(|s| ObjectName::from(s.clone()))
                    .ok_or(Error::ObjectLookup(state_machine_handle))?;
                let state_sym = entry_table
                    .entry(state_handle)
                    .symbol
                    .as_ref()
                    .map(|s| ObjectName::from(s.clone()))
                    .ok_or(Error::ObjectLookup(state_handle))?;
                let event = StateMachineStateEvent {
                    event_count,
                    timestamp,
                    handle: state_machine_handle,
                    name: state_machine_sym,
                    state_handle,
                    state: state_sym,
                };
                Some((event_code, Event::StateMachineStateCreate(event)))
            }

            EventType::StateMachineStateChange => {
                let state_machine_handle = object_handle(&mut r, event_id)?;
                let state_handle = object_handle(&mut r, event_id)?;
                let state_machine_sym = entry_table
                    .entry(state_machine_handle)
                    .symbol
                    .as_ref()
                    .map(|s| ObjectName::from(s.clone()))
                    .ok_or(Error::ObjectLookup(state_machine_handle))?;
                let state_sym = entry_table
                    .entry(state_handle)
                    .symbol
                    .as_ref()
                    .map(|s| ObjectName::from(s.clone()))
                    .ok_or(Error::ObjectLookup(state_handle))?;
                let event = StateMachineStateChangeEvent {
                    event_count,
                    timestamp,
                    handle: state_machine_handle,
                    name: state_machine_sym,
                    state_handle,
                    state: state_sym,
                };
                Some((event_code, Event::StateMachineStateChange(event)))
            }

            EventType::UnusedStack => {
                let handle = object_handle(&mut r, event_id)?;
                let low_mark = r.read_u32()?;
                let sym = entry_table
                    .symbol(handle)
                    .ok_or(Error::ObjectLookup(handle))?;
                let event = UnusedStackEvent {
                    event_count,
                    timestamp,
                    handle,
                    task: sym.clone().into(),
                    low_mark,
                };
                Some((event_code, Event::UnusedStack(event)))
            }

            EventType::UserEvent(raw_arg_count) => {
                // Always expect at least a channel
                if num_params.0 < 1 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        1,
                        num_params,
                    ));
                }

                // Account for fixed user events when can occupy part of the user event ID space
                let (is_fixed, arg_count) = if event_id.0 >= FIXED_USER_EVENT_ID
                    && usize::from(raw_arg_count) >= usize::from(num_params)
                {
                    (
                        true,
                        UserEventArgRecordCount((event_id.0 - FIXED_USER_EVENT_ID) as u8),
                    )
                } else {
                    (false, raw_arg_count)
                };

                if usize::from(arg_count) >= usize::from(num_params) {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        usize::from(arg_count),
                        num_params,
                    ));
                }

                // Parse out <channel-handle> [args] <format-string>
                let channel_handle = object_handle(&mut r, event_id)?;
                let channel = entry_table
                    .symbol(channel_handle)
                    .map(|sym| UserEventChannel::Custom(sym.clone().into()))
                    .unwrap_or(UserEventChannel::Default);

                self.arg_buf.clear();

                let format_string = if is_fixed {
                    let fmt_string_handle = object_handle(&mut r, event_id)?;

                    let num_arg_bytes = usize::from(arg_count.0) * 4;
                    if num_arg_bytes != 0 {
                        self.arg_buf.resize(num_arg_bytes, 0);
                        r.read_exact(&mut self.arg_buf)?;
                    }

                    let res = entry_table
                        .symbol(fmt_string_handle)
                        .map(|s| TrimmedString::from_str(s))
                        .ok_or(Error::FixedUserEventFmtStringLookup(fmt_string_handle));
                    match res {
                        Ok(fmt_string) => fmt_string,
                        Err(e) => {
                            // Need to read out the rest of the arg data so the parser can skip over the
                            // invalid data
                            // +2 since we already read channel and fmt string words
                            let remaining_param_words =
                                num_params.0.saturating_sub(arg_count.0 + 2);
                            if remaining_param_words != 0 {
                                let mut parameters = [0; EventParameterCount::MAX];
                                r.read_u32_into(
                                    &mut parameters[..usize::from(remaining_param_words)],
                                )?;
                            }
                            return Err(e);
                        }
                    }
                } else {
                    // arg_count includes the format string, we want the args, if any
                    let not_fmt_str_arg_count = if arg_count.0 != 0 {
                        usize::from(arg_count) - 1
                    } else {
                        0
                    };
                    let num_arg_bytes = not_fmt_str_arg_count * 4;
                    if num_arg_bytes != 0 {
                        self.arg_buf.resize(num_arg_bytes, 0);
                        r.read_exact(&mut self.arg_buf)?;
                    }

                    let num_fmt_str_bytes =
                        (usize::from(num_params) - 1 - not_fmt_str_arg_count) * 4;
                    self.read_string(&mut r, num_fmt_str_bytes)?
                };

                let (formatted_string, args) = match format_symbol_string(
                    entry_table,
                    Protocol::Streaming,
                    self.endianness.into(),
                    &format_string,
                    &self.arg_buf,
                ) {
                    Ok((fs, args)) => (fs, args),
                    Err(e) => {
                        error!("Failed to parse user event format string arguments, using the raw symbol instead. {e}");
                        (
                            FormattedString(format_string.clone().into()),
                            Default::default(),
                        )
                    }
                };

                let event = UserEvent {
                    event_count,
                    timestamp,
                    channel,
                    format_string: FormatString(format_string.0),
                    formatted_string,
                    args,
                };
                Some((event_code, Event::User(event)))
            }

            EventType::Unknown(_)
                if self
                    .custom_printf_event_id
                    .map(|id| id == event_id)
                    .unwrap_or(false) =>
            {
                if num_params.0 != 0 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        0,
                        num_params,
                    ));
                }

                let channel_handle = object_handle(&mut r, event_id)?;
                let channel = entry_table
                    .symbol(channel_handle)
                    .map(|sym| UserEventChannel::Custom(sym.clone().into()))
                    .unwrap_or(UserEventChannel::Default);

                let args_len = r.read_u16()?;
                let fmt_len = r.read_u16()?;

                self.arg_buf.clear();
                let num_arg_bytes = usize::from(args_len) * 4;
                if num_arg_bytes != 0 {
                    self.arg_buf.resize(num_arg_bytes, 0);
                    r.read_exact(&mut self.arg_buf)?;
                }

                let format_string = self.read_string(&mut r, fmt_len.into())?;

                let (formatted_string, args) = match format_symbol_string(
                    entry_table,
                    Protocol::Streaming,
                    self.endianness.into(),
                    &format_string,
                    &self.arg_buf,
                ) {
                    Ok((fs, args)) => (fs, args),
                    Err(e) => {
                        error!("Failed to parse custom printf event format string arguments, using the raw symbol instead. {e}");
                        (
                            FormattedString(format_string.clone().into()),
                            Default::default(),
                        )
                    }
                };

                let event = UserEvent {
                    event_count,
                    timestamp,
                    channel,
                    format_string: FormatString(format_string.0),
                    formatted_string,
                    args,
                };
                Some((event_code, Event::User(event)))
            }

            // Return the base event type for everything else
            _ => {
                let mut parameters = [0; EventParameterCount::MAX];
                r.read_u32_into(&mut parameters[..usize::from(num_params)])?;
                let event = BaseEvent {
                    code: event_code,
                    event_count,
                    timestamp,
                    parameters,
                };
                Some((event_code, Event::Unknown(event)))
            }
        })
    }

    fn read_string<R: Read>(&mut self, r: &mut R, max_len: usize) -> Result<TrimmedString, Error> {
        self.buf.clear();
        self.buf.resize(max_len, 0);
        r.read_exact(&mut self.buf)?;
        Ok(TrimmedString::from_raw(&self.buf))
    }
}

fn object_handle<T: byteordered::byteorder::ReadBytesExt, E: byteordered::Endian>(
    r: &mut ByteOrdered<T, E>,
    event_id: EventId,
) -> Result<ObjectHandle, Error> {
    let oh = r.read_u32()?;
    ObjectHandle::new(oh).ok_or(Error::InvalidObjectHandle(event_id))
}
