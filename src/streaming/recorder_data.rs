use crate::streaming::event::{Event, EventCode, EventParser, TraceStartEvent, TsConfigEvent};
use crate::streaming::{Error, ExtensionInfo, HeaderInfo, ObjectDataTable, SymbolTable};
use crate::types::{ObjectClass, Protocol, TZ_CTRL_TASK_NAME};
use std::io::Read;
use tracing::debug;

/// Encapsulates all of the startup data needed to materialize the events
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct RecorderData {
    pub protocol: Protocol,
    pub header: HeaderInfo,
    pub symbol_table: SymbolTable,
    pub object_data_table: ObjectDataTable,
    pub extension_info: ExtensionInfo,
    pub start_event: TraceStartEvent,
    pub ts_config_event: TsConfigEvent,

    parser: EventParser,
}

impl RecorderData {
    pub fn read<R: Read>(r: &mut R) -> Result<Self, Error> {
        let header = HeaderInfo::read(r)?;

        debug!("Reading symbol table");
        let mut symbol_table = SymbolTable::read(
            r,
            header.endianness,
            header.symbol_size,
            header.symbol_count,
        )?;

        debug!("Reading object data table");
        let mut object_data_table = ObjectDataTable::read(
            r,
            header.endianness,
            header.object_data_size,
            header.object_data_count,
        )?;

        // Seed the object data class for the TzCtrl task setup before
        // recording/streaming starts
        if let Some(tz_ctrl_handle) = symbol_table
            .symbols
            .iter()
            .find(|(_oh, ste)| ste.symbol.0 == TZ_CTRL_TASK_NAME)
            .map(|(oh, _ste)| oh)
        {
            object_data_table.update_class(*tz_ctrl_handle, ObjectClass::Task);
        }

        debug!("Reading extension info");
        let extension_info = ExtensionInfo::read(r, header.endianness)?;

        let mut parser = EventParser::new(header.endianness);

        debug!("Reading start event");
        let (event_code, event) = parser
            .next_event(r, &mut symbol_table, &mut object_data_table)?
            .ok_or(Error::MissingStartEvent)?;
        let start_event = if let Event::TraceStart(ev) = event {
            ev
        } else {
            return Err(Error::InvalidStartEvent(event_code.event_type().into()));
        };

        debug!("Reading TS config event");
        let (event_code, event) = parser
            .next_event(r, &mut symbol_table, &mut object_data_table)?
            .ok_or(Error::MissingTsConfigEvent)?;
        let ts_config_event = if let Event::TsConfig(ev) = event {
            ev
        } else {
            return Err(Error::InvalidTsConfigEvent(event_code.event_type().into()));
        };

        Ok(Self {
            protocol: Protocol::Streaming,
            header,
            symbol_table,
            object_data_table,
            extension_info,
            start_event,
            ts_config_event,
            parser,
        })
    }

    pub fn read_event<R: Read>(&mut self, r: &mut R) -> Result<Option<(EventCode, Event)>, Error> {
        self.parser
            .next_event(r, &mut self.symbol_table, &mut self.object_data_table)
    }
}
