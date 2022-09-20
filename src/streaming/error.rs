use crate::streaming::entry_table::{Entry, EntryStates};
use crate::streaming::event::{EventId, EventParameterCount};
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
        "Entry table symbol size must be greater than {} (TRC_ENTRY_TABLE_SLOT_SYMBOL_SIZE)",
        Entry::MIN_SYMBOL_SIZE
    )]
    InvalidEntryTableSymbolSize,

    #[error(
        "Entry table state count must be greater than or equal to {} (TRC_ENTRY_TABLE_STATE_COUNT)",
        EntryStates::NUM_STATES
    )]
    InvalidEntryTableStateCount,

    #[error("Event ID {0} expects {1} parameters but reported having {2}")]
    InvalidEventParameterCount(EventId, usize, EventParameterCount),

    #[error("TsConfig event contains an invalid timer counter type {0}")]
    InvalidTimerCounter(u32),

    #[error("Found an event with object handle {0} that doesn't exist in the entry table")]
    ObjectLookup(ObjectHandle),

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
