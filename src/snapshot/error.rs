use crate::snapshot::event::parser;
use crate::snapshot::markers::{DebugMarker, MarkerBytes};
use crate::snapshot::OffsetBytes;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid marker bytes {1:X?} at offset {0}. Expected {2}")]
    MarkerBytes(OffsetBytes, [u8; 12], MarkerBytes),

    #[error("Invalid debug marker 0x{1:X} at offset {0}. Expected {2}")]
    DebugMarker(OffsetBytes, u32, DebugMarker),

    #[error("Invalid kernel version {1:X?} at offset {0}")]
    KernelVersion(OffsetBytes, [u8; 2]),

    #[error("Found an invalid zero value symbol table index at offset {0}")]
    InvalidSymbolTableIndex(OffsetBytes),

    #[error("User event buffers are not supported (TRC_CFG_USE_SEPARATE_USER_EVENT_BUFFER == 1)")]
    UnsupportedUserEventBuffer,

    #[error(transparent)]
    Parser(#[from] parser::Error),

    #[error(
        "Encountered and IO error while reading the input stream ({})",
        .0.kind()
    )]
    Io(#[from] io::Error),
}
