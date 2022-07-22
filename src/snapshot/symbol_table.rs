use crate::types::{ObjectHandle, SymbolString, SymbolTableExt};
use derive_more::{Binary, Display, Into, LowerHex, Octal, UpperHex};
use std::collections::BTreeMap;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct SymbolTable {
    /// The key is the byte offset of this entry within the originating table in memory,
    /// referenced by user event payloads
    pub symbols: BTreeMap<ObjectHandle, SymbolTableEntry>,
}

impl SymbolTable {
    /// Up to 64 linked lists within the symbol table
    /// connecting all entries with the same 6 bit checksum
    pub(crate) const NUM_LATEST_ENTRY_OF_CHECKSUMS: usize = 64;

    pub fn insert(
        &mut self,
        handle: ObjectHandle,
        channel_index: Option<ObjectHandle>,
        crc: SymbolCrc6,
        symbol: SymbolString,
    ) {
        self.symbols.insert(
            handle,
            SymbolTableEntry {
                channel_index,
                crc,
                symbol,
            },
        );
    }

    pub fn get(&self, handle: ObjectHandle) -> Option<&SymbolTableEntry> {
        self.symbols.get(&handle)
    }
}

impl SymbolTableExt for SymbolTable {
    fn symbol(&self, handle: ObjectHandle) -> Option<&SymbolString> {
        self.get(handle).map(|ste| &ste.symbol)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}", symbol)]
pub struct SymbolTableEntry {
    /// Reference to a symbol table entry, a label for vTracePrintF
    /// format strings only (the handle of the destination channel)
    pub channel_index: Option<ObjectHandle>,
    /// 6-bit CRC of the binary symbol (before lossy UTF8 string conversion)
    pub crc: SymbolCrc6,
    /// The symbol (lossy converted to UTF8)
    pub symbol: SymbolString,
}

#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    Into,
    Display,
    Binary,
    Octal,
    LowerHex,
    UpperHex,
)]
#[display(fmt = "{_0:X}")]
pub struct SymbolCrc6(u8);

impl SymbolCrc6 {
    pub(crate) fn new(s: &[u8]) -> Self {
        let mut crc: u32 = 0;
        for b in s.iter() {
            crc += *b as u32;
        }
        Self((crc & 0x3F) as u8)
    }
}
