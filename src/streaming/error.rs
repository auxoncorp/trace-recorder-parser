use crate::streaming::event::{EventId, EventParameterCount, EventType};
use crate::streaming::{ObjectDataTableEntry, SymbolTableEntry};
use crate::types::{FormattedStringError, ObjectHandle};
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid kernel version {0:X?}")]
    KernelVersion([u8; 2]),

    #[error("Invalid PSF endianness identifier {0:X?}")]
    PSFEndiannessIdentifier(u32),

    #[error(
        "Symbol table slot size must be greater than {} (SYMBOL_TABLE_SLOT_SIZE)",
        SymbolTableEntry::MIN_SIZE
    )]
    InvalidSymbolTableSlotSize,

    #[error(
        "Object data table slot size must be greater than {} (OBJECT_DATA_SLOT_SIZE)",
        ObjectDataTableEntry::MIN_SIZE
    )]
    InvalidObjectDataTableSlotSize,

    #[error("Event ID {0} expects {1} parameters but reported having {2}")]
    InvalidEventParameterCount(EventId, usize, EventParameterCount),

    #[error("Missing {} event", EventType::TraceStart)]
    MissingStartEvent,

    #[error(
        "Invalid start event ID {0}, expected {}",
        EventId::from(EventType::TraceStart)
    )]
    InvalidStartEvent(EventId),

    #[error("Missing {} event", EventType::TsConfig)]
    MissingTsConfigEvent,

    #[error(
        "Invalid TS config event ID {0}, expected {}",
        EventId::from(EventType::TsConfig)
    )]
    InvalidTsConfigEvent(EventId),

    #[error("Found an event with object handle {0} that doesn't exist in the symbol table")]
    ObjectSymbolLookup(ObjectHandle),

    #[error("Found an event with object handle {0} that doesn't exist in the object data table")]
    ObjectDataLookup(ObjectHandle),

    #[error("Found an event ({0}) with an invalid zero value object handle")]
    InvalidObjectHandle(EventId),

    #[error(transparent)]
    FormattedString(#[from] FormattedStringError),

    #[error(
        "Encountered and IO error while reading the input stream ({})",
        .0.kind()
    )]
    Io(#[from] io::Error),
}
