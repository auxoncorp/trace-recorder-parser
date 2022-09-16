use crate::streaming::event::*;
use crate::streaming::{Error, ObjectDataTable, SymbolTable};
use crate::time::{Frequency, Timestamp};
use crate::types::{
    format_symbol_string, Endianness, FormatString, FormattedString, IsrName, ObjectClass,
    ObjectHandle, Priority, Protocol, SymbolString, TaskName, TimerCounter, TrimmedString,
    UserEventChannel,
};
use byteordered::ByteOrdered;
use std::io::{self, Read};
use tracing::error;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EventParser {
    /// Endianness of the data
    endianness: byteordered::Endianness,

    /// Initial heap counter from the header struct
    heap_counter: u32,

    /// Local scratch buffer for reading strings
    buf: Vec<u8>,

    /// Local scratch buffer for reading argument data
    arg_buf: Vec<u8>,
}

impl EventParser {
    pub fn new(endianness: Endianness, heap_counter: u32) -> Self {
        Self {
            endianness: byteordered::Endianness::from(endianness),
            heap_counter,
            buf: Vec::with_capacity(256),
            arg_buf: Vec::with_capacity(256),
        }
    }

    pub fn next_event<R: Read>(
        &mut self,
        r: &mut R,
        symbol_table: &mut SymbolTable,
        object_data_table: &mut ObjectDataTable,
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

        Ok(match event_type {
            EventType::TraceStart => {
                // TODO - add a method on EventType for param_count
                // enum with int and variable variants
                // or just a min_param_count
                if num_params.0 != 3 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        3,
                        num_params,
                    ));
                }
                let os_ticks = r.read_u32()?;
                let handle = object_handle(&mut r, event_id)?;
                let session_counter = r.read_u32()?;
                let sym = symbol_table
                    .get(handle)
                    .ok_or(Error::ObjectSymbolLookup(handle))?;
                let event = TraceStartEvent {
                    event_count,
                    timestamp,
                    os_ticks,
                    current_task: TaskName(sym.symbol.0.clone()),
                    session_counter,
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
                            3, // base count
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
                symbol_table.insert(handle, symbol.clone());
                let event = ObjectNameEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: symbol,
                };
                Some((event_code, Event::ObjectName(event)))
            }

            EventType::TaskPriority => {
                if num_params.0 != 2 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        2,
                        num_params,
                    ));
                }
                let handle = object_handle(&mut r, event_id)?;
                let priority = Priority(r.read_u32()?);
                object_data_table.insert(handle, priority);
                let sym = symbol_table
                    .get(handle)
                    .ok_or(Error::ObjectSymbolLookup(handle))?;
                let event = TaskEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: TaskName(sym.symbol.0.clone()),
                    priority,
                };
                Some((event_code, Event::TaskPriority(event)))
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
                object_data_table.insert(handle, priority);
                object_data_table.update_class(handle, ObjectClass::Isr);
                let symbol: SymbolString = self
                    .read_string(&mut r, (usize::from(num_params) - 2) * 4)?
                    .into();
                symbol_table.insert(handle, symbol.clone());
                let event = IsrEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: IsrName(symbol.0),
                    priority,
                };
                Some((event_code, Event::IsrDefine(event)))
            }

            EventType::TaskCreate => {
                if num_params.0 != 2 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        2,
                        num_params,
                    ));
                }
                let handle = object_handle(&mut r, event_id)?;
                let priority = Priority(r.read_u32()?);
                object_data_table.insert(handle, priority);
                object_data_table.update_class(handle, ObjectClass::Task);
                let sym = symbol_table
                    .get(handle)
                    .ok_or(Error::ObjectSymbolLookup(handle))?;
                let event = TaskEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: TaskName(sym.symbol.0.clone()),
                    priority,
                };
                Some((event_code, Event::TaskCreate(event)))
            }

            EventType::TaskReady => {
                if num_params.0 != 1 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        1,
                        num_params,
                    ));
                }
                let handle = object_handle(&mut r, event_id)?;
                let obj = object_data_table
                    .get(handle)
                    .ok_or(Error::ObjectDataLookup(handle))?;
                let sym = symbol_table
                    .get(handle)
                    .ok_or(Error::ObjectSymbolLookup(handle))?;
                let event = TaskEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: TaskName(sym.symbol.0.clone()),
                    priority: obj.priority,
                };
                Some((event_code, Event::TaskReady(event)))
            }

            EventType::TaskSwitchIsrBegin => {
                if num_params.0 != 1 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        1,
                        num_params,
                    ));
                }
                let handle = object_handle(&mut r, event_id)?;
                let obj = object_data_table
                    .get(handle)
                    .ok_or(Error::ObjectDataLookup(handle))?;
                let sym = symbol_table
                    .get(handle)
                    .ok_or(Error::ObjectSymbolLookup(handle))?;
                let event = IsrEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: IsrName(sym.symbol.0.clone()),
                    priority: obj.priority,
                };
                Some((event_code, Event::IsrBegin(event)))
            }

            EventType::TaskSwitchIsrResume => {
                if num_params.0 != 1 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        1,
                        num_params,
                    ));
                }
                let handle = object_handle(&mut r, event_id)?;
                let obj = object_data_table
                    .get(handle)
                    .ok_or(Error::ObjectDataLookup(handle))?;
                let sym = symbol_table
                    .get(handle)
                    .ok_or(Error::ObjectSymbolLookup(handle))?;
                let event = IsrEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: IsrName(sym.symbol.0.clone()),
                    priority: obj.priority,
                };
                Some((event_code, Event::IsrResume(event)))
            }

            EventType::TaskSwitchTaskResume => {
                if num_params.0 != 1 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        1,
                        num_params,
                    ));
                }
                let handle = object_handle(&mut r, event_id)?;
                let obj = object_data_table
                    .get(handle)
                    .ok_or(Error::ObjectDataLookup(handle))?;
                let sym = symbol_table
                    .get(handle)
                    .ok_or(Error::ObjectSymbolLookup(handle))?;
                let event = TaskEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: TaskName(sym.symbol.0.clone()),
                    priority: obj.priority,
                };
                Some((event_code, Event::TaskResume(event)))
            }

            EventType::TaskActivate => {
                if num_params.0 != 2 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        2,
                        num_params,
                    ));
                }
                let handle = object_handle(&mut r, event_id)?;
                let priority = Priority(r.read_u32()?);
                object_data_table.insert(handle, priority);
                let sym = symbol_table
                    .get(handle)
                    .ok_or(Error::ObjectSymbolLookup(handle))?;
                let event = TaskEvent {
                    event_count,
                    timestamp,
                    handle,
                    name: TaskName(sym.symbol.0.clone()),
                    priority,
                };
                Some((event_code, Event::TaskActivate(event)))
            }

            EventType::MemoryAlloc | EventType::MemoryFree => {
                if num_params.0 != 2 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        2,
                        num_params,
                    ));
                }
                let address = r.read_u32()?;
                let size = r.read_u32()?;
                if matches!(event_type, EventType::MemoryAlloc) {
                    self.heap_counter = self.heap_counter.saturating_add(size);
                } else {
                    self.heap_counter = self.heap_counter.saturating_sub(size);
                }
                let event = MemoryEvent {
                    event_count,
                    timestamp,
                    address,
                    size,
                    heap_counter: self.heap_counter,
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
                if num_params.0 != 2 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        2,
                        num_params,
                    ));
                }
                let handle = object_handle(&mut r, event_id)?;
                let queue_length = r.read_u32()?;
                object_data_table.update_class(handle, ObjectClass::Queue);
                let event = QueueCreateEvent {
                    event_count,
                    timestamp,
                    handle,
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
                if num_params.0 != 2 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        2,
                        num_params,
                    ));
                }
                let handle: ObjectHandle = object_handle(&mut r, event_id)?;
                let messages_waiting = r.read_u32()?;
                let event = QueueEvent {
                    event_count,
                    timestamp,
                    handle,
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
                if num_params.0 != 3 {
                    return Err(Error::InvalidEventParameterCount(
                        event_code.event_id(),
                        3,
                        num_params,
                    ));
                }
                let handle: ObjectHandle = object_handle(&mut r, event_id)?;
                let ticks_to_wait = Some(r.read_u32()?);
                let messages_waiting = r.read_u32()?;
                let event = QueueEvent {
                    event_count,
                    timestamp,
                    handle,
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
                let channel = symbol_table
                    .get(channel_handle)
                    .map(|se| UserEventChannel::Custom(se.symbol.clone().into()))
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
                    symbol_table,
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
