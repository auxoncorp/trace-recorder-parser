use crate::snapshot::event::*;
use crate::snapshot::object_properties::ObjectPropertyTable;
use crate::snapshot::symbol_table::SymbolTable;
use crate::time::{DifferentialTimestamp, Dts16, Dts8, Timestamp};
use crate::types::{
    format_symbol_string, FormatString, FormattedString, FormattedStringError, IsrName,
    ObjectClass, ObjectHandle, Protocol, UserEventChannel,
};
use byteordered::{ByteOrdered, Endianness};
use derive_more::From;
use std::io;
use thiserror::Error;
use tracing::error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Found an invalid zero value symbol table index")]
    InvalidSymbolTableIndex,

    #[error(
        "Found a user event with format string index {0} that doesn't exist in the symbol table"
    )]
    FormatSymbolLookup(ObjectHandle),

    #[error(
        "Found a user event with channel string index {0} that doesn't exist in the symbol table"
    )]
    ChannelSymbolLookup(ObjectHandle),

    #[error(transparent)]
    FormattedString(#[from] FormattedStringError),

    #[error("Found an invalid zero value object property table handle")]
    InvalidObjectHandle,

    #[error(
        "Found an event with object handle {0} that doesn't exist in the object properties table"
    )]
    ObjectLookup(ObjectHandle),

    #[error(
          "Encountered and IO error while parsing the event stream ({})",
          .0.kind()
      )]
    Io(#[from] io::Error),
}

#[derive(Debug)]
pub struct EventParser {
    /// Endianness of the data
    endianness: Endianness,

    /// Timestamp accumulated from differential timestamps
    accumulated_time: Timestamp,

    /// Differential timestamp upper bytes from an XTS8 or XTS16 event that precedes
    /// an event to form a complete differential timestamp for the event
    dts_for_next_event: DifferentialTimestamp,

    /// Number of user event argument records that follow the base user event record
    user_arg_record_count: usize,

    /// User event record buffer, all other events are single records
    user_event_records: Vec<EventRecord>,
}

impl EventParser {
    pub fn new(endianness: Endianness) -> Self {
        Self {
            endianness,
            accumulated_time: Timestamp::zero(),
            dts_for_next_event: DifferentialTimestamp::zero(),
            user_arg_record_count: 0,
            user_event_records: Vec::with_capacity(UserEventArgRecordCount::MAX),
        }
    }

    pub fn parse(
        &mut self,
        obj_props: &ObjectPropertyTable,
        symbol_table: &SymbolTable,
        record: EventRecord,
    ) -> Result<Option<(EventType, Event)>, Error> {
        let event_code = record.event_code();
        let event_type = EventType::from(event_code);

        // User events are special; they can span multiple records
        if self.is_capturing_user_event_records() {
            self.capture_user_event_record(record);
            return Ok(self
                .parse_user_event(symbol_table)?
                .map(|(et, ue)| (et, Event::User(ue))));
        }

        // Everything else have a u8 event code prefix in the record
        Ok(match event_type {
            EventType::TaskSwitchIsrBegin | EventType::TaskSwitchIsrResume => {
                let mut r = ByteOrdered::runtime(record.as_slice(), self.endianness);
                let _event_code = r.read_u8()?;
                let handle =
                    ObjectHandle::new(r.read_u8()?.into()).ok_or(Error::InvalidObjectHandle)?;
                let dts = Dts16(r.read_u16()?);
                let obj = obj_props
                    .isr_object_properties
                    .get(&handle)
                    .ok_or(Error::ObjectLookup(handle))?;
                let event = IsrEvent {
                    handle,
                    name: IsrName(obj.display_name().to_string()),
                    priority: obj.priority(),
                    timestamp: self.get_timestamp(dts.into()),
                };
                Some((
                    event_type,
                    if event_type == EventType::TaskSwitchIsrBegin {
                        Event::IsrBegin(event)
                    } else {
                        Event::IsrResume(event)
                    },
                ))
            }

            EventType::TaskReady
            | EventType::TaskSwitchTaskBegin
            | EventType::TaskSwitchTaskResume => {
                let mut r = ByteOrdered::runtime(record.as_slice(), self.endianness);
                let _event_code = r.read_u8()?;
                let handle =
                    ObjectHandle::new(r.read_u8()?.into()).ok_or(Error::InvalidObjectHandle)?;
                let dts = Dts16(r.read_u16()?);
                let obj = obj_props
                    .task_object_properties
                    .get(&handle)
                    .ok_or(Error::ObjectLookup(handle))?;
                let event = TaskEvent {
                    handle,
                    name: TaskName(obj.display_name().to_string()),
                    state: obj.state(),
                    priority: obj.current_priority(),
                    timestamp: self.get_timestamp(dts.into()),
                };
                Some((
                    event_type,
                    match event_type {
                        EventType::TaskReady => Event::TaskReady(event),
                        EventType::TaskSwitchTaskBegin => Event::TaskBegin(event),
                        _ /*EventType::TaskSwitchTaskResume*/ => Event::TaskResume(event),
                    },
                ))
            }

            EventType::CreateObject(occ) => {
                let handle = self.parse_generic_kernel_call(&record)?;
                match occ.into_class() {
                    ObjectClass::Task => {
                        let obj = obj_props
                            .task_object_properties
                            .get(&handle)
                            .ok_or(Error::ObjectLookup(handle))?;
                        Some((
                            event_type,
                            Event::TaskCreate(TaskEvent {
                                handle,
                                name: TaskName(obj.display_name().to_string()),
                                state: obj.state(),
                                priority: obj.current_priority(),
                                timestamp: self.accumulated_time,
                            }),
                        ))
                    }
                    // Other object classes not handled currently
                    _ => Some((event_type, Event::Unknown(self.accumulated_time, record))),
                }
            }

            EventType::UserEvent(arg_record_cnt) => {
                self.begin_user_event(arg_record_cnt, record);
                self.parse_user_event(symbol_table)?
                    .map(|(et, ue)| (et, Event::User(ue)))
            }

            // NOTE XTS events aren't surfaced to the user, since they're just added to
            // fulfill the differential timestamps of actual events
            EventType::Xts8 => {
                let mut r = ByteOrdered::runtime(record.as_slice(), self.endianness);
                let _event_code = r.read_u8()?;
                let xts_8 = r.read_u8()?;
                let xts_16 = r.read_u16()?;
                self.dts_for_next_event = DifferentialTimestamp::from_xts8(xts_8, xts_16);
                None
            }
            EventType::Xts16 => {
                let mut r = ByteOrdered::runtime(record.as_slice(), self.endianness);
                let _event_code = r.read_u8()?;
                let _unused = r.read_u8()?;
                let xts_16 = r.read_u16()?;
                self.dts_for_next_event = DifferentialTimestamp::from_xts16(xts_16);
                None
            }

            EventType::LowPowerBegin | EventType::LowPowerEnd => {
                let mut r = ByteOrdered::runtime(record.as_slice(), self.endianness);
                let _event_code = r.read_u8()?;
                let _unused = r.read_u8()?;
                let dts = Dts16(r.read_u16()?);
                let event = LowPowerEvent {
                    timestamp: self.get_timestamp(dts.into()),
                };
                Some((
                    event_type,
                    if event_type == EventType::LowPowerBegin {
                        Event::LowPowerBegin(event)
                    } else {
                        Event::LowPowerEnd(event)
                    },
                ))
            }

            // If the record was being written at the time of reading, skip it
            EventType::EventBeingWritten => None,

            // The rest of the match arms are only to handle the various DTS-carrying
            // event records and return Event::Unknown
            EventType::NewTime => {
                self.parse_generic_kernel_call_with_numeric_param(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::Send(_)
            | EventType::Receive(_)
            | EventType::SendFromIsr(_)
            | EventType::ReceiveFromIsr(_) => {
                self.parse_generic_kernel_call(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::CreateObjectFailed(_) => {
                self.parse_generic_kernel_call_with_numeric_param(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::SendFailed(_)
            | EventType::ReceiveFailed(_)
            | EventType::SendFromIsrFailed(_)
            | EventType::ReceiveFromIsrFailed(_)
            | EventType::ReceiveBlock(_)
            | EventType::SendBlock(_)
            | EventType::Peek(_)
            | EventType::DeleteObject(_) => {
                self.parse_generic_kernel_call(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::TaskDelayUntil | EventType::TaskDelay => {
                self.parse_generic_kernel_call_with_numeric_param(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::TaskSuspend | EventType::TaskResume | EventType::TaskResumeFromIsr => {
                self.parse_generic_kernel_call(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::TaskPrioritySet
            | EventType::TaskPriorityInherit
            | EventType::TaskPriorityDisinherit => {
                self.parse_generic_kernel_call_with_param(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::PendFuncCall
            | EventType::PendFuncCallFromIsr
            | EventType::PendFuncCallFailed
            | EventType::PendFuncCallFromIsrFailed => {
                self.parse_generic_kernel_call(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::MemoryMallocSize | EventType::MemoryFreeSize => {
                self.parse_generic_mem_size(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::TimerCreate | EventType::TimerDeleteObject => {
                self.parse_generic_kernel_call(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::TimerStart
            | EventType::TimerReset
            | EventType::TimerStop
            | EventType::TimerChangePeriod
            | EventType::TimerStartFromIsr
            | EventType::TimerResetFromIsr
            | EventType::TimerStopFromIsr => {
                self.parse_generic_kernel_call_with_param(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::TimerCreateFailed => {
                self.parse_generic_kernel_call_with_numeric_param(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::TimerStartFailed
            | EventType::TimerResetFailed
            | EventType::TimerStopFailed
            | EventType::TimerChangePeriodFailed
            | EventType::TimerDeleteFailed
            | EventType::TimerStartFromIsrFailed
            | EventType::TimerResetFromIsrFailed
            | EventType::TimerStopFromIsrFailed => {
                self.parse_generic_kernel_call_with_param(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::EventGroupCreate | EventType::EventGroupDeleteObject => {
                self.parse_generic_kernel_call(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::EventGroupCreateFailed => {
                self.parse_generic_kernel_call_with_numeric_param(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::EventGroupSyncBlock
            | EventType::EventGroupSyncEnd
            | EventType::EventGroupWaitBitsBlock
            | EventType::EventGroupWaitBitsEnd
            | EventType::EventGroupClearBits
            | EventType::EventGroupClearBitsFromIsr
            | EventType::EventGroupSetBits
            | EventType::EventGroupSyncEndFailed
            | EventType::EventGroupWaitBitsEndFailed
            | EventType::EventGroupSetBitsFromIsr
            | EventType::EventGroupSetBitsFromIsrFailed => {
                self.parse_generic_kernel_call_with_param(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::TaskInstanceFinishedNextKse | EventType::TaskInstanceFinishedDirect => {
                self.parse_generic_task_instance_status(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::TaskNotify
            | EventType::TaskNotifyFromIsr
            | EventType::TaskNotifyGiveFromIsr => {
                self.parse_generic_kernel_call(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::TaskNotifyTake
            | EventType::TaskNotifyTakeBlock
            | EventType::TaskNotifyTakeFailed
            | EventType::TaskNotifyWait
            | EventType::TaskNotifyWaitBlock
            | EventType::TaskNotifyWaitFailed => {
                self.parse_generic_kernel_call_with_param(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::TimerExpired => {
                self.parse_generic_kernel_call(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::QueuePeekBlock
            | EventType::SemaphortPeekBlock
            | EventType::MutexPeekBlock
            | EventType::QueuePeekFailed
            | EventType::SemaphortPeekFailed
            | EventType::MutexPeekFailed => {
                self.parse_generic_kernel_call(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::StreambufferReset | EventType::MessagebufferReset => {
                self.parse_generic_kernel_call(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::MemoryMallocSizeFailed => {
                self.parse_generic_mem_size(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            EventType::UnusedStack => {
                self.parse_generic_kernel_call_with_param(&record)?;
                Some((event_type, Event::Unknown(self.accumulated_time, record)))
            }

            // Default case, pass back a raw event record
            //
            // NOTE take extra care to handle DTS-carrying events
            // elsewhere to maintain the accumulated time.
            //
            // This *must* only be for records that don't have DTS fields.
            _ => Some((event_type, Event::Unknown(self.accumulated_time, record))),
        })
    }

    /// Combines an events DTS (lower 8 or 16 bits) to the possibly
    /// existing XTS DTS to form a complete DTS.
    /// Then adds that to the timestamp accumulator for an absolute event timestamp.
    fn get_timestamp(&mut self, dts: Dts) -> Timestamp {
        // Form a complete DTS
        match dts {
            Dts::Dts8(dts) => {
                self.dts_for_next_event += dts;
            }
            Dts::Dts16(dts) => {
                self.dts_for_next_event += dts;
            }
        }

        // Add it to the accumulated time
        self.accumulated_time += self.dts_for_next_event;

        // Done with the DTS
        self.dts_for_next_event.clear();

        self.accumulated_time
    }

    /// Process the DTS portion of a record containing a `struct KernelCall`
    fn parse_generic_kernel_call(&mut self, record: &EventRecord) -> Result<ObjectHandle, Error> {
        let mut r = ByteOrdered::runtime(record.as_slice(), self.endianness);
        let _event_code = r.read_u8()?;
        let obj_handle =
            ObjectHandle::new(r.read_u8()?.into()).ok_or(Error::InvalidObjectHandle)?;
        let dts = Dts8(r.read_u8()?);
        let _timestamp = self.get_timestamp(dts.into());
        Ok(obj_handle)
    }

    /// Process the DTS portion of a record containing a `struct KernelCallWithParamAndHandle`
    fn parse_generic_kernel_call_with_param(&mut self, record: &EventRecord) -> Result<(), Error> {
        let mut r = ByteOrdered::runtime(record.as_slice(), self.endianness);
        let _event_code = r.read_u8()?;
        let _obj_handle = r.read_u8()?;
        let _param = r.read_u8()?;
        let dts = Dts8(r.read_u8()?);
        let _timestamp = self.get_timestamp(dts.into());
        Ok(())
    }

    /// Process the DTS portion of a record containing a `struct KernelCallWithParam16`
    fn parse_generic_kernel_call_with_numeric_param(
        &mut self,
        record: &EventRecord,
    ) -> Result<(), Error> {
        let mut r = ByteOrdered::runtime(record.as_slice(), self.endianness);
        let _event_code = r.read_u8()?;
        let dts = Dts8(r.read_u8()?);
        let _timestamp = self.get_timestamp(dts.into());
        Ok(())
    }

    /// Process the DTS portion of a record containing a `struct MemEventSize`
    fn parse_generic_mem_size(&mut self, record: &EventRecord) -> Result<(), Error> {
        let mut r = ByteOrdered::runtime(record.as_slice(), self.endianness);
        let _event_code = r.read_u8()?;
        let dts = Dts8(r.read_u8()?);
        let _size = r.read_u16()?;
        let _timestamp = self.get_timestamp(dts.into());
        Ok(())
    }

    /// Process the DTS portion of a record containing a `struct TaskInstanceStatusEvent`
    fn parse_generic_task_instance_status(&mut self, record: &EventRecord) -> Result<(), Error> {
        let mut r = ByteOrdered::runtime(record.as_slice(), self.endianness);
        let _event_code = r.read_u8()?;
        let _unused1 = r.read_u8()?;
        let _unused2 = r.read_u8()?;
        let dts = Dts8(r.read_u8()?);
        let _timestamp = self.get_timestamp(dts.into());
        Ok(())
    }

    fn begin_user_event(
        &mut self,
        user_arg_record_count: UserEventArgRecordCount,
        record: EventRecord,
    ) {
        self.user_arg_record_count = usize::from(user_arg_record_count.0);
        self.user_event_records.push(record);
    }

    fn end_user_event(&mut self) {
        self.user_event_records.clear();
        self.user_arg_record_count = 0;
    }

    fn capture_user_event_record(&mut self, record: EventRecord) {
        self.user_event_records.push(record);
    }

    fn is_capturing_user_event_records(&self) -> bool {
        !self.user_event_records.is_empty()
    }

    fn parse_user_event(
        &mut self,
        symbol_table: &SymbolTable,
    ) -> Result<Option<(EventType, UserEvent)>, Error> {
        if self.user_event_records.len() == (self.user_arg_record_count + 1) {
            // SAFETY: we just ensured we have at least the base record
            let base_record = self.user_event_records[0].as_slice();
            let mut r = ByteOrdered::runtime(base_record, self.endianness);
            let event_code = EventCode(r.read_u8()?);
            let event_type = EventType::from(event_code);
            let dts = Dts8(r.read_u8()?);
            let format_string_index =
                ObjectHandle::new(r.read_u16()?.into()).ok_or(Error::InvalidSymbolTableIndex)?;

            let sym_entry = symbol_table
                .get(format_string_index)
                .ok_or(Error::FormatSymbolLookup(format_string_index))?;

            let channel = sym_entry
                .channel_index
                .and_then(|ci| {
                    symbol_table
                        .get(ci)
                        .map(|se| UserEventChannel::Custom(se.symbol.clone().into()))
                })
                .unwrap_or(UserEventChannel::Default);

            let arg_bytes: Vec<u8> = self
                .user_event_records
                .iter()
                .skip(1)
                .flat_map(|r| r.as_slice().iter())
                .cloned()
                .collect();
            let (formatted_string, args) = match format_symbol_string(
                symbol_table,
                Protocol::Snapshot,
                self.endianness.into(),
                &sym_entry.symbol,
                &arg_bytes,
            ) {
                Ok((fs, args)) => (fs, args),
                Err(e) => {
                    error!("Failed to parse user event format string arguments, using the raw symbol instead. {e}");
                    (
                        FormattedString(sym_entry.symbol.to_string()),
                        Default::default(),
                    )
                }
            };
            let event = UserEvent {
                timestamp: self.get_timestamp(dts.into()),
                channel,
                format_string: FormatString(sym_entry.symbol.0.clone()),
                formatted_string,
                args,
            };
            self.end_user_event();
            Ok(Some((event_type, event)))
        } else {
            // Waiting for more arg records
            Ok(None)
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, From)]
enum Dts {
    Dts8(Dts8),
    Dts16(Dts16),
}
