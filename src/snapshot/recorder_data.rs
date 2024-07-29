use crate::snapshot::event::{Event, EventParser, EventRecord, EventType};
use crate::snapshot::markers::{DebugMarker, MarkerBytes};
use crate::snapshot::object_properties::{ObjectProperties, ObjectPropertyTable};
use crate::snapshot::symbol_table::{SymbolCrc6, SymbolTable};
use crate::snapshot::Error;
use crate::time::Frequency;
use crate::types::{
    Endianness, FloatEncoding, KernelPortIdentity, KernelVersion, ObjectClass, ObjectHandle,
    OffsetBytes, Protocol, TrimmedString,
};
use byteordered::ByteOrdered;
use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Seek, SeekFrom};
use tracing::{debug, error, warn};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct RecorderData {
    pub protocol: Protocol,
    pub kernel_version: KernelVersion,
    pub kernel_port: KernelPortIdentity,
    pub endianness: Endianness,
    pub minor_version: u8,
    pub irq_priority_order: u8,
    pub filesize: u32,
    // Note that num_events can be greater than max_events when the buffer is full/overwritten
    pub num_events: u32,
    pub max_events: u32,
    pub next_free_index: u32,
    pub buffer_is_full: bool,
    pub frequency: Frequency,
    pub abs_time_last_event: u32,
    pub abs_time_last_event_second: u32,
    pub recorder_active: bool,
    pub isr_tail_chaining_threshold: u32,
    pub heap_mem_usage: u32,
    pub heap_mem_max_usage: u32,
    pub is_using_16bit_handles: bool,
    pub object_property_table: ObjectPropertyTable,
    pub symbol_table: SymbolTable,
    pub float_encoding: FloatEncoding,
    pub internal_error_occured: bool,
    pub system_info: String,

    /// Offset of the recorder data start markers
    start_offset: OffsetBytes,
    /// Offset of the recorder data event data
    event_data_offset: OffsetBytes,
    // TODO - add user event buffer offset here when supported
}

impl RecorderData {
    pub fn locate_and_parse<R: Read + Seek>(r: &mut R) -> Result<Self, Error> {
        let mut tmp_buffer = VecDeque::with_capacity(1024);
        let mut r = ByteOrdered::native(r);

        // Locate the start marker bytes
        let mut offset = r.stream_position()?;
        tmp_buffer.clear();
        tmp_buffer.resize(MarkerBytes::SIZE, 0);
        r.read_exact(tmp_buffer.make_contiguous())?;
        let start_offset = loop {
            if tmp_buffer.make_contiguous() == MarkerBytes::Start.as_bytes() {
                break offset;
            }

            let _ = tmp_buffer.pop_front();
            tmp_buffer.push_back(r.read_u8()?);
            offset += 1;
        };

        debug!(start_offset = start_offset, "Found start markers");
        r.seek(SeekFrom::Start(start_offset))?;
        MarkerBytes::Start.read(&mut r)?;

        let kvi_pos = r.stream_position()?;
        let mut kernel_version_identity: [u8; 2] = [0; 2];
        r.read_exact(&mut kernel_version_identity)?;
        let kernel_version = KernelVersion(kernel_version_identity);
        let kernel_port = kernel_version
            .port_identity()
            .map_err(|e| Error::KernelVersion(kvi_pos, e.0))?;
        let endianness = kernel_version
            .endianness()
            .map_err(|e| Error::KernelVersion(kvi_pos, e.0))?;
        debug!(kernel_version = %kernel_version, kernel_port = %kernel_port, endianness = ?endianness, "Found kernel version");
        let minor_version = r.read_u8()?;
        debug!(minor_version = minor_version, "Found minor version");

        if kernel_port != KernelPortIdentity::FreeRtos {
            warn!("Kernel port {kernel_port} is not officially supported");
        }

        if minor_version != 7 {
            warn!("Version {minor_version} is not officially supported");
        }

        let irq_priority_order = r.read_u8()?;

        // The remaining fields are endian-aware
        let mut r = ByteOrdered::new(r.into_inner(), byteordered::Endianness::from(endianness));
        let filesize = r.read_u32()?;
        debug!(filesize = filesize, "Found recorder data region size");

        let num_events = r.read_u32()?;
        let max_events = r.read_u32()?;
        let next_free_index = r.read_u32()?;
        let buffer_is_full = r.read_u32()?;
        let frequency = Frequency(r.read_u32()?);
        let abs_time_last_event = r.read_u32()?;
        let abs_time_last_event_second = r.read_u32()?;
        let recorder_active = r.read_u32()?;
        let isr_tail_chaining_threshold = r.read_u32()?;
        let heap_mem_max_usage = r.read_u32()?;
        let heap_mem_usage = r.read_u32()?;
        DebugMarker::Marker0.read(&mut r)?;
        let is_using_16bit_handles = r.read_u32()? != 0;

        if is_using_16bit_handles {
            return Err(Error::Unsupported16bitHandles);
        }

        if frequency.is_unitless() {
            warn!("Time base frequency is zero, units will be in ticks only");
        }

        // Object property table starts here
        let object_property_table_offset = r.stream_position()?;
        let num_object_classes = r.read_u32()?;
        let object_property_table_size = r.read_u32()?;
        debug!(
            object_property_table_offset = object_property_table_offset,
            num_object_classes = num_object_classes,
            object_property_table_size = object_property_table_size,
            "Found object property table region"
        );

        let num_object_classes_u16_allocation_size_words =
            round_up_nearest_2(num_object_classes) as usize;
        let num_object_classes_u8_allocation_size_words =
            round_up_nearest_4(num_object_classes) as usize;

        // This is used to calculate the index in the dynamic object table
        // (handle - 1 - nofStaticObjects = index)
        let num_objects_per_class: Vec<u16> = if is_using_16bit_handles {
            let mut words = vec![0; num_object_classes_u16_allocation_size_words];
            r.read_u16_into(&mut words)?;
            words
        } else {
            let mut words = vec![0; num_object_classes_u8_allocation_size_words];
            r.read_exact(&mut words)?;
            words.into_iter().map(|w| w.into()).collect()
        };

        let mut name_len_per_class = vec![0; num_object_classes_u8_allocation_size_words];
        r.read_exact(&mut name_len_per_class)?;

        let mut total_bytes_per_class = vec![0; num_object_classes_u8_allocation_size_words];
        r.read_exact(&mut total_bytes_per_class)?;

        let mut start_index_of_class = vec![0; num_object_classes_u16_allocation_size_words];
        r.read_u16_into(&mut start_index_of_class)?;

        let pos_at_prop_table = r.stream_position()?;
        let mut queue_object_properties = BTreeMap::new();
        let mut semaphore_object_properties = BTreeMap::new();
        let mut mutex_object_properties = BTreeMap::new();
        let mut task_object_properties = BTreeMap::new();
        let mut isr_object_properties = BTreeMap::new();
        let mut timer_object_properties = BTreeMap::new();
        let mut event_group_object_properties = BTreeMap::new();
        let mut stream_buffer_object_properties = BTreeMap::new();
        let mut message_buffer_object_properties = BTreeMap::new();
        for obj_class in ObjectClass::enumerate().iter() {
            let obj_class_index = obj_class.into_usize();
            let num_objects = num_objects_per_class[obj_class_index];
            let name_len = name_len_per_class[obj_class_index];
            let total_bytes_per_obj = total_bytes_per_class[obj_class_index];
            let start_index = start_index_of_class[obj_class_index];

            if total_bytes_per_obj == 0 {
                error!("Skipping empty object class {obj_class} property table entry");
                // Keep on trying
                continue;
            }

            if obj_class_index as u32 >= num_object_classes {
                warn!("Skipping unsupported object class {obj_class} property table entry");
                r.seek(SeekFrom::Current(i64::from(
                    total_bytes_per_obj as u32 * num_objects as u32,
                )))?;
                continue;
            }

            let class_offset = r.stream_position()?;
            if (class_offset - pos_at_prop_table) != u64::from(start_index) {
                warn!("Offset of object class {obj_class} {class_offset}, relative to the property table {} doesn't match the reported start index {start_index}", class_offset - pos_at_prop_table);
            }
            let end_of_class =
                class_offset + u64::from(num_objects as u32 * total_bytes_per_obj as u32);

            // Object handles (traceHandle) == object index + 1
            let mut raw_obj_handle = 1;

            // Read each entry in the class
            while r.stream_position()? < end_of_class {
                let obj_start_pos = r.stream_position()?;

                // Zero length name is invalid (pretty sure), but try and tolerate it
                if name_len == 0 {
                    warn!("Skipping object class {obj_class} entry because name length is zero");
                    r.seek(SeekFrom::Current(i64::from(total_bytes_per_obj)))?;
                    continue;
                }

                // Read object name
                tmp_buffer.clear();
                tmp_buffer.resize(name_len as _, 0);
                r.read_exact(tmp_buffer.make_contiguous())?;

                if tmp_buffer[0] == 0 {
                    // Empty entry
                    r.seek(SeekFrom::Current(i64::from(total_bytes_per_obj - name_len)))?;
                    continue;
                }

                // First name byte can be 0x01 to indicate a used object that hasn't had a name set yet
                let name = if tmp_buffer[0] == 0x01 {
                    None
                } else {
                    Some(TrimmedString::from_raw(tmp_buffer.make_contiguous()).into())
                };

                // Read properties
                let mut properties = [0; 4];
                for p in properties.iter_mut().take(obj_class.properties_size()) {
                    *p = r.read_u8()?;
                }

                // SAFETY: we initialize the raw_obj_handle to 1 above and only ever
                // increment
                let obj_handle = ObjectHandle::new_unchecked(raw_obj_handle);
                raw_obj_handle = raw_obj_handle.saturating_add(1);

                match obj_class {
                    ObjectClass::Queue => {
                        let obj = ObjectProperties::new(name, properties);
                        debug!("Found object property {obj} at {obj_start_pos}");
                        queue_object_properties.insert(obj_handle, obj);
                    }
                    ObjectClass::Semaphore => {
                        let obj = ObjectProperties::new(name, properties);
                        debug!("Found object property {obj} at {obj_start_pos}");
                        semaphore_object_properties.insert(obj_handle, obj);
                    }
                    ObjectClass::Mutex => {
                        let obj = ObjectProperties::new(name, properties);
                        debug!("Found object property {obj} at {obj_start_pos}");
                        mutex_object_properties.insert(obj_handle, obj);
                    }
                    ObjectClass::Task => {
                        let obj = ObjectProperties::new(name, properties);
                        debug!("Found object property {obj} at {obj_start_pos}");
                        task_object_properties.insert(obj_handle, obj);
                    }
                    ObjectClass::Isr => {
                        let obj = ObjectProperties::new(name, properties);
                        debug!("Found object property {obj} at {obj_start_pos}");
                        isr_object_properties.insert(obj_handle, obj);
                    }
                    ObjectClass::Timer => {
                        let obj = ObjectProperties::new(name, properties);
                        debug!("Found object property {obj} at {obj_start_pos}");
                        timer_object_properties.insert(obj_handle, obj);
                    }
                    ObjectClass::EventGroup => {
                        let obj = ObjectProperties::new(name, properties);
                        debug!("Found object property {obj} at {obj_start_pos}");
                        event_group_object_properties.insert(obj_handle, obj);
                    }
                    ObjectClass::StreamBuffer => {
                        let obj = ObjectProperties::new(name, properties);
                        debug!("Found object property {obj} at {obj_start_pos}");
                        stream_buffer_object_properties.insert(obj_handle, obj);
                    }
                    ObjectClass::MessageBuffer => {
                        let obj = ObjectProperties::new(name, properties);
                        debug!("Found object property {obj} at {obj_start_pos}");
                        message_buffer_object_properties.insert(obj_handle, obj);
                    }
                    ObjectClass::StateMachine => {
                        // NOTE: unsupported
                    }
                }
            }
        }

        // Seek past any remaining unused bytes from aligned allocation
        let pos_after_prop_table = r.stream_position()?;
        let prop_table_bytes_read = (pos_after_prop_table - pos_at_prop_table) as i64;
        let prop_table_allocation_size = i64::from(round_up_nearest_4(object_property_table_size));
        if prop_table_bytes_read < prop_table_allocation_size {
            r.seek(SeekFrom::Current(
                prop_table_allocation_size - prop_table_bytes_read,
            ))?;
        }

        DebugMarker::Marker1.read(&mut r)?;

        // Symbol table starts here
        let symbol_table_offset = r.stream_position()?;
        let symbol_table_size = r.read_u32()?;
        debug!(
            symbol_table_offset = symbol_table_offset,
            symbol_table_size = symbol_table_size,
            "Found symbol table region"
        );

        // symbolTableType.nextFreeSymbolIndex is initialized to 1,
        // so the first 4 bytes are zero initialized.
        // Entry 0 is reserved. Any reference to entry 0 implies NULL
        let next_free_symbol_index = r.read_u32()?;
        if next_free_symbol_index > symbol_table_size {
            warn!("Next free symbol index {next_free_symbol_index} exceeds symbol table size {symbol_table_size}");
        }
        let end_of_symbol_table_region =
            r.stream_position()? + u64::from(round_up_nearest_4(symbol_table_size));
        let start_of_symbol_table_bytes = r.stream_position()?;
        let end_of_symbol_entries = start_of_symbol_table_bytes + u64::from(next_free_symbol_index);

        let unused_index_slot = r.read_u8()?;
        if unused_index_slot != 0 {
            warn!(
                "Reserved symbol table entry 0 contains an invalid value 0x{unused_index_slot:X}"
            );
        }

        // Read in the populated symbol table entries
        let mut symbol_table = SymbolTable::default();
        while r.stream_position()? < end_of_symbol_entries {
            let start_of_symbol_table_entry = r.stream_position()?;

            // 4-byte metadata
            let _next_entry_index = r.read_u16()?;
            let channel = r.read_u16()?;
            // Followed by (double) null-terminated symbol string
            tmp_buffer.clear();
            loop {
                let sym_byte = r.read_u8()?;
                if sym_byte == 0 {
                    // They double null-terminate for some reason, I think it's a bug and a waste :/
                    let extra_null = r.read_u8()?;
                    if extra_null != 0 {
                        warn!(
                            "Found non-zero NULL terminated symbol table entry at offeset {}",
                            r.stream_position()?
                        );
                    }
                    break;
                } else {
                    tmp_buffer.push_back(sym_byte);
                }
            }
            let crc = SymbolCrc6::new(tmp_buffer.make_contiguous());
            symbol_table.insert(
                ObjectHandle::new(
                    ((start_of_symbol_table_entry - start_of_symbol_table_bytes) & 0xFFFF) as u32,
                )
                .ok_or(Error::InvalidSymbolTableIndex(start_of_symbol_table_entry))?,
                ObjectHandle::new(channel.into()),
                crc,
                TrimmedString::from_raw(tmp_buffer.make_contiguous()).into(),
            );
        }

        // Seek past the unused symbol table entries
        r.seek(SeekFrom::Start(end_of_symbol_table_region))?;

        // Used for lookups - Up to 64 linked lists within the symbol table
        // connecting all entries with the same 6 bit checksum.
        // This field holds the current list heads.
        // (index == crc6 of symbol, data == symbol table index)
        // Only used for fast lookups on-device, so we skip over it.
        r.seek(SeekFrom::Current(
            (std::mem::size_of::<u16>() * SymbolTable::NUM_LATEST_ENTRY_OF_CHECKSUMS) as _,
        ))?;

        // When TRC_CFG_INCLUDE_FLOAT_SUPPORT == 1, the value should be (float) 1,
        // otherwise (u32) 0.
        // Also used for endian detection of floats
        let float_encoding = FloatEncoding::from_bits(r.read_u32()?);

        let internal_error_occured = r.read_u32()?;
        if internal_error_occured != 0 {
            warn!("The 'internal_error_occured' field is set to {internal_error_occured}");
        }

        DebugMarker::Marker2.read(&mut r)?;

        // Read systemInfo string
        tmp_buffer.clear();
        tmp_buffer.resize(NUM_SYSTEM_INFO_BYTES, 0);
        r.read_exact(tmp_buffer.make_contiguous())?;
        let system_info = TrimmedString::from_raw(tmp_buffer.make_contiguous()).0;
        if !system_info.is_empty() {
            debug!(system_info = %system_info, "Found system info");
        }

        DebugMarker::Marker3.read(&mut r)?;

        // Store the offset of the event data, 4-byte records, and skip over it
        let event_data_offset = r.stream_position()?;
        r.seek(SeekFrom::Current(4 * i64::from(max_events)))?;

        // If TRC_CFG_USE_SEPARATE_USER_EVENT_BUFFER == 1 then this will be the bufferID field
        // otherwise it's the first 16 bits of the endOfSecondaryBlocks field
        let maybe_user_event_buffer_id = r.read_u16()?;
        if maybe_user_event_buffer_id == 0 {
            // TRC_CFG_USE_SEPARATE_USER_EVENT_BUFFER == 0
            // Read the rest of endOfSecondaryBlocks (always zero)
            let end_of_secondary_blocks = r.read_u16()?;
            if end_of_secondary_blocks != 0 {
                warn!("End of secondary blocks field ({end_of_secondary_blocks}) should be zero");
            }
        } else {
            // TODO - add support for this and put info in the data
            return Err(Error::UnsupportedUserEventBuffer);
        }

        MarkerBytes::End.read(&mut r)?;

        Ok(RecorderData {
            protocol: Protocol::Snapshot,
            kernel_version,
            kernel_port,
            endianness,
            minor_version,
            irq_priority_order,
            filesize,
            num_events,
            max_events,
            next_free_index,
            buffer_is_full: buffer_is_full != 0,
            frequency,
            abs_time_last_event,
            abs_time_last_event_second,
            recorder_active: recorder_active != 0,
            isr_tail_chaining_threshold,
            heap_mem_usage,
            heap_mem_max_usage,
            is_using_16bit_handles,
            object_property_table: ObjectPropertyTable {
                queue_object_properties,
                semaphore_object_properties,
                mutex_object_properties,
                task_object_properties,
                isr_object_properties,
                timer_object_properties,
                event_group_object_properties,
                stream_buffer_object_properties,
                message_buffer_object_properties,
            },
            symbol_table,
            float_encoding,
            internal_error_occured: internal_error_occured != 0,
            system_info,

            // Internal stuff
            start_offset,
            event_data_offset,
        })
    }

    pub fn event_records<'r, R: Read + Seek + Send>(
        &'r self,
        r: &'r mut R,
    ) -> Result<Box<dyn Iterator<Item = Result<EventRecord, Error>> + Send + 'r>, Error> {
        if (self.num_events < self.max_events) || ((self.num_events % self.max_events) == 0) {
            // Buffer is still still contiguous, can iterate from start of memory
            let num_events_clamped = std::cmp::min(self.num_events, self.max_events);
            r.seek(SeekFrom::Start(self.event_data_offset))?;
            Ok(Box::new((0..num_events_clamped).map(|_| {
                let mut record = [0; EventRecord::SIZE];
                r.read_exact(&mut record)?;
                Ok(EventRecord::new(record))
            })))
        } else {
            // Buffer full and has wrapped, chain the two regions together
            // starting at the tail to end of the buffer region, then start
            // of the memory region to head

            // Seek to the tail
            let num_tail_region_events = self.max_events - self.next_free_index;
            let tail_offset = self.next_free_index * EventRecord::SIZE as u32;
            r.seek(SeekFrom::Start(
                self.event_data_offset + u64::from(tail_offset),
            ))?;

            let iter = (0..self.max_events).map(move |event_index| {
                let mut record = [0; EventRecord::SIZE];
                r.read_exact(&mut record)?;

                // Last tail record, seek to the start of the memory region for head region
                if event_index + 1 == num_tail_region_events {
                    r.seek(SeekFrom::Start(self.event_data_offset))?;
                }

                Ok(EventRecord::new(record))
            });

            Ok(Box::new(iter))
        }
    }

    pub fn events<'r, R: Read + Seek + Send>(
        &'r self,
        r: &'r mut R,
    ) -> Result<impl Iterator<Item = Result<(EventType, Event), Error>> + 'r, Error> {
        let mut parser = EventParser::new(self.endianness.into());
        let iter = self.event_records(r)?.filter_map(move |item| match item {
            Ok(er) => match parser
                .parse(&self.object_property_table, &self.symbol_table, er)
                .map_err(Error::from)
            {
                Ok(maybe_ev) => maybe_ev.map(Ok),
                Err(e) => Some(Err(e)),
            },
            Err(e) => Some(Err(e)),
        });
        Ok(iter)
    }
}

/// Max size of the system info string
const NUM_SYSTEM_INFO_BYTES: usize = 80;

// Rounded up to the closest multiple of 2
// Used in the data struct allocation to avoid alignment issues
fn round_up_nearest_2(n: u32) -> u32 {
    2 * ((n + 1) / 2)
}

// Rounded up to the closest multiple of 4
// Used in the data struct allocation to avoid alignment issues
fn round_up_nearest_4(n: u32) -> u32 {
    4 * ((n + 3) / 4)
}
