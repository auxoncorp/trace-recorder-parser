use crate::streaming::event::*;
use crate::streaming::{EntryTable, Error};
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
            buf: Vec::with_capacity(256),
            arg_buf: Vec::with_capacity(256),
        }
    }

    pub fn system_heap(&self) -> &Heap {
        &self.heap
    }

    pub fn next_event<R: Read>(
        &mut self,
        r: &mut R,
        entry_table: &mut EntryTable,
    ) -> Result<Option<(EventCode, Event)>, Error> {
        let mut r = ByteOrdered::new(r, self.endianness);

        let event_code = match r.read_u16() {
            Ok(ec) => EventCode(ec),
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let event_type = event_code.event_type();
        let event_id = event_code.event_id();
        let event_count = EventCount(r.read_u16()?);
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

            EventType::UserEvent(arg_count) => {
                // Always expect at least a channel
                if num_params.0 < 1 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        1,
                        num_params,
                    ));
                }
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

                // arg_count includes the format string, we want the args, if any
                let not_fmt_str_arg_count = if arg_count.0 != 0 {
                    usize::from(arg_count) - 1
                } else {
                    0
                };
                let num_arg_bytes = not_fmt_str_arg_count * 4;
                self.arg_buf.clear();
                if num_arg_bytes != 0 {
                    self.arg_buf.resize(num_arg_bytes, 0);
                    r.read_exact(&mut self.arg_buf)?;
                }

                let num_fmt_str_bytes = (usize::from(num_params) - 1 - not_fmt_str_arg_count) * 4;
                let format_string = self.read_string(&mut r, num_fmt_str_bytes)?;

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
