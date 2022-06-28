use derive_more::{Binary, Deref, Display, Into, LowerHex, Octal, UpperHex};
use std::collections::BTreeSet;
use std::num::NonZeroU16;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct SymbolTable {
    pub symbols: BTreeSet<SymbolTableEntry>,
}

impl SymbolTable {
    /// Up to 64 linked lists within the symbol table
    /// connecting all entries with the same 6 bit checksum
    pub(crate) const NUM_LATEST_ENTRY_OF_CHECKSUMS: usize = 64;

    pub fn entry(&self, index: SymbolTableEntryIndex) -> Option<&SymbolTableEntry> {
        self.symbols.iter().find(|s| s.index == index)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}", symbol)]
pub struct SymbolTableEntry {
    /// The byte offset of this entry within the originating table in memory,
    /// referenced by user event payloads
    pub index: SymbolTableEntryIndex,
    /// Reference to a symbol table entry, a label for vTracePrintF
    /// format strings only (the handle of the destination channel)
    pub channel_index: Option<SymbolTableEntryIndex>,
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
#[display(fmt = "{_0}")]
pub struct SymbolTableEntryIndex(pub(crate) NonZeroU16);

impl SymbolTableEntryIndex {
    pub(crate) fn new(index: u16) -> Option<Self> {
        Some(SymbolTableEntryIndex(NonZeroU16::new(index)?))
    }
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

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Into, Deref, Display)]
#[display(fmt = "{_0}")]
pub struct SymbolString(pub(crate) String);

impl SymbolString {
    pub(crate) fn from_raw(s: &[u8]) -> Self {
        Self(String::from_utf8_lossy(s).to_string())
    }
}
