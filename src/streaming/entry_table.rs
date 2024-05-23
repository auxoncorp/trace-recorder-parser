use crate::streaming::Error;
use crate::types::{
    Endianness, Heap, ObjectClass, ObjectHandle, Priority, SymbolString, SymbolTableExt,
    TrimmedString, STARTUP_TASK_NAME, TZ_CTRL_TASK_NAME,
};
use byteordered::ByteOrdered;
use std::collections::BTreeMap;
use std::io::Read;
use tracing::debug;

/// The address field of an entry is the key.
/// This is either an object address (task, queue, etc) or the address of the
/// entry "slot" in memory (self-referential, i.e. user event strings).
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct EntryTable(BTreeMap<ObjectHandle, Entry>);

impl Default for EntryTable {
    fn default() -> Self {
        let mut entries = BTreeMap::new();
        let mut states = EntryStates::default();
        states.set_priority(Priority(1));
        entries.insert(
            ObjectHandle::NO_TASK,
            Entry {
                symbol: SymbolString(STARTUP_TASK_NAME.to_owned()).into(),
                options: 0,
                states,
                class: ObjectClass::Task.into(),
            },
        );
        Self(entries)
    }
}

impl EntryTable {
    pub fn entries(&self) -> &BTreeMap<ObjectHandle, Entry> {
        &self.0
    }

    pub fn symbol(&self, handle: ObjectHandle) -> Option<&SymbolString> {
        self.0.get(&handle).and_then(|e| e.symbol.as_ref())
    }

    pub fn class(&self, handle: ObjectHandle) -> Option<ObjectClass> {
        self.0.get(&handle).and_then(|e| e.class)
    }

    pub fn symbol_handle<S: AsRef<str>>(
        &self,
        symbol: S,
        class: Option<ObjectClass>,
    ) -> Option<ObjectHandle> {
        self.0.iter().find_map(|(handle, entry)| {
            let sym_match = entry.symbol.as_deref() == Some(symbol.as_ref());
            let class_match = match class {
                None => true,
                Some(c) => entry.class == Some(c),
            };
            if sym_match && class_match {
                Some(*handle)
            } else {
                None
            }
        })
    }

    pub(crate) fn system_heap(&self) -> Option<Heap> {
        self.0
            .values()
            .find_map(|entry| {
                if entry.symbol.as_deref() == Some(Entry::SYSTEM_HEAP_SYMBOL) {
                    Some(&entry.states)
                } else {
                    None
                }
            })
            .map(|states| Heap {
                current: states.heap_current(),
                high_water_mark: states.heap_high_water_mark(),
                max: states.heap_max(),
            })
    }

    pub(crate) fn entry(&mut self, handle: ObjectHandle) -> &mut Entry {
        self.0.entry(handle).or_default()
    }
}

impl SymbolTableExt for EntryTable {
    fn symbol(&self, handle: ObjectHandle) -> Option<&SymbolString> {
        EntryTable::symbol(self, handle)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Entry {
    /// The symbol (lossy converted to UTF8)                                                                           
    pub symbol: Option<SymbolString>,
    pub options: u32,
    pub states: EntryStates,
    pub class: Option<ObjectClass>,
}

impl Entry {
    /// See `TRC_ENTRY_TABLE_SLOT_SYMBOL_SIZE`
    pub const MIN_SYMBOL_SIZE: usize = 1;

    pub(crate) const SYSTEM_HEAP_SYMBOL: &'static str = "System Heap";

    pub(crate) fn set_symbol(&mut self, symbol: SymbolString) {
        self.symbol = symbol.into()
    }

    pub(crate) fn set_class(&mut self, class: ObjectClass) {
        self.class = class.into()
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct EntryStates([u32; EntryStates::NUM_STATES]);

impl EntryStates {
    /// See `TRC_ENTRY_TABLE_STATE_COUNT`
    pub const NUM_STATES: usize = 3;

    pub(crate) fn new_unchecked(states: &[u32]) -> Self {
        Self([states[0], states[1], states[2]])
    }

    pub(crate) const fn priority(&self) -> Priority {
        Priority(self.0[0])
    }

    pub(crate) fn set_priority(&mut self, priority: Priority) {
        self.0[0] = priority.0;
    }

    pub(crate) const fn heap_current(&self) -> u32 {
        self.0[0]
    }

    pub(crate) const fn heap_high_water_mark(&self) -> u32 {
        self.0[1]
    }

    pub(crate) const fn heap_max(&self) -> u32 {
        self.0[2]
    }
}

impl EntryTable {
    pub(crate) fn read<R: Read>(r: &mut R, endianness: Endianness) -> Result<Self, Error> {
        let mut r = ByteOrdered::new(r, byteordered::Endianness::from(endianness));
        let num_entries = r.read_u32()?;
        let symbol_size = r.read_u32()? as usize;
        let state_count = r.read_u32()? as usize;
        debug!(num_entries, symbol_size, state_count);

        if symbol_size < Entry::MIN_SYMBOL_SIZE {
            return Err(Error::InvalidEntryTableSymbolSize);
        } else if state_count < EntryStates::NUM_STATES {
            return Err(Error::InvalidEntryTableStateCount);
        }

        let mut table = EntryTable::default();
        if num_entries == 0 {
            Ok(table)
        } else {
            let mut buf = vec![0; symbol_size];
            let mut states_buf = vec![0_u32; state_count];
            for _ in 0..num_entries {
                let address = r.read_u32()?;
                r.read_u32_into(&mut states_buf)?;
                let states = EntryStates::new_unchecked(&states_buf);
                let options = r.read_u32()?;
                r.read_exact(&mut buf)?;
                if let Some(oh) = ObjectHandle::new(address) {
                    let symbol: SymbolString = TrimmedString::from_raw(&buf).into();

                    let class = if symbol.0 == TZ_CTRL_TASK_NAME {
                        Some(ObjectClass::Task)
                    } else {
                        None
                    };

                    table.0.insert(
                        oh,
                        Entry {
                            symbol: if !symbol.0.is_empty() {
                                Some(symbol)
                            } else {
                                None
                            },
                            options,
                            states,
                            class,
                        },
                    );
                }
            }
            Ok(table)
        }
    }
}
