use crate::streaming::Error;
use crate::types::{
    Endianness, ObjectHandle, SymbolString, SymbolTableExt, TrimmedString, STARTUP_TASK_NAME,
};
use byteordered::ByteOrdered;
use derive_more::Display;
use std::collections::BTreeMap;
use std::io::Read;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct SymbolTable {
    /// The address portion of a symbol table entry is the key.
    /// This is either an object address (task, queue, etc) or the address of the
    /// symbol table "slot" in memory (self-referential, i.e. user event strings).
    pub symbols: BTreeMap<ObjectHandle, SymbolTableEntry>,
}

impl Default for SymbolTable {
    fn default() -> Self {
        let mut symbols = BTreeMap::new();
        symbols.insert(
            ObjectHandle::NO_TASK,
            SymbolTableEntry {
                symbol: SymbolString(STARTUP_TASK_NAME.to_owned()),
            },
        );
        Self { symbols }
    }
}

impl SymbolTable {
    pub fn insert(&mut self, handle: ObjectHandle, symbol: SymbolString) {
        self.symbols.insert(handle, SymbolTableEntry { symbol });
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
    /// The symbol (lossy converted to UTF8)
    pub symbol: SymbolString,
}

impl SymbolTableEntry {
    /// Symbol entries consist of a 4-byte address and at least a NULL symbol byte
    pub(crate) const MIN_SIZE: usize = 4 + 1;
}

impl SymbolTable {
    pub(crate) fn read<R: Read>(
        r: &mut R,
        endianness: Endianness,
        symbol_size: usize,
        symbol_count: usize,
    ) -> Result<Self, Error> {
        if symbol_count == 0 {
            // Empty symbol table
            return Ok(Default::default());
        } else if symbol_size < SymbolTableEntry::MIN_SIZE {
            // 4-bytes for address plus at least a NULL
            return Err(Error::InvalidSymbolTableSlotSize);
        }

        let mut r = ByteOrdered::new(r, byteordered::Endianness::from(endianness));
        let mut symbol_table = SymbolTable::default();
        let mut buf = Vec::with_capacity(symbol_size);

        for _idx in 0..symbol_count {
            let address = r.read_u32()?;
            buf.clear();
            buf.resize(symbol_size - 4, 0);
            r.read_exact(&mut buf)?;
            if let Some(oh) = ObjectHandle::new(address) {
                let symbol = TrimmedString::from_raw(&buf).into();
                symbol_table.insert(oh, symbol);
            }
        }

        Ok(symbol_table)
    }
}
