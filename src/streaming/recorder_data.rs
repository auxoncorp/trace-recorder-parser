use crate::streaming::event::{Event, EventCode, EventParser};
use crate::streaming::{EntryTable, Error, HeaderInfo, TimestampInfo};
use crate::types::{Heap, Protocol};
use std::io::Read;
use tracing::debug;

/// Encapsulates all of the startup data needed to materialize the events
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct RecorderData {
    pub protocol: Protocol,
    pub header: HeaderInfo,
    pub timestamp_info: TimestampInfo,
    pub entry_table: EntryTable,
    parser: EventParser,
}

impl RecorderData {
    pub fn find<R: Read>(r: &mut R) -> Result<Self, Error> {
        debug!("Finding header info");
        let header = HeaderInfo::find(r)?;

        Self::read_common(header, r)
    }

    pub fn read<R: Read>(r: &mut R) -> Result<Self, Error> {
        debug!("Reading header info");
        let header = HeaderInfo::read(r)?;

        Self::read_common(header, r)
    }

    fn read_common<R: Read>(header: HeaderInfo, r: &mut R) -> Result<Self, Error> {
        debug!("Reading timestamp info");
        let timestamp_info = TimestampInfo::read(r, header.endianness, header.format_version)?;

        debug!("Reading entry table");
        let entry_table = EntryTable::read(r, header.endianness)?;

        let parser = EventParser::new(
            header.endianness,
            entry_table.system_heap().unwrap_or_default(),
        );

        Ok(Self {
            protocol: Protocol::Streaming,
            header,
            timestamp_info,
            entry_table,
            parser,
        })
    }

    pub fn system_heap(&self) -> &Heap {
        self.parser.system_heap()
    }

    pub fn read_event<R: Read>(&mut self, r: &mut R) -> Result<Option<(EventCode, Event)>, Error> {
        self.parser.next_event(r, &mut self.entry_table)
    }
}
