use crate::streaming::Error;
use crate::types::{Endianness, ObjectClass, ObjectHandle, Priority};
use byteordered::ByteOrdered;
use derive_more::Display;
use std::collections::BTreeMap;
use std::io::Read;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ObjectDataTable {
    pub objects: BTreeMap<ObjectHandle, ObjectDataTableEntry>,

    /// Map of object handles to classes, managed by the parser not part of the protocol
    pub classes: BTreeMap<ObjectHandle, ObjectClass>,
}

impl ObjectDataTable {
    pub fn insert(&mut self, handle: ObjectHandle, priority: Priority) {
        self.objects
            .insert(handle, ObjectDataTableEntry { priority });
    }

    pub fn get(&self, handle: ObjectHandle) -> Option<&ObjectDataTableEntry> {
        self.objects.get(&handle)
    }

    pub fn update_class(&mut self, handle: ObjectHandle, class: ObjectClass) {
        self.classes.insert(handle, class);
    }

    pub fn class(&self, handle: ObjectHandle) -> Option<ObjectClass> {
        self.classes.get(&handle).copied()
    }
}

impl Default for ObjectDataTable {
    fn default() -> Self {
        let mut objects = BTreeMap::new();
        objects.insert(
            ObjectHandle::NO_TASK,
            ObjectDataTableEntry {
                priority: Priority(1),
            },
        );
        let mut classes = BTreeMap::new();
        classes.insert(ObjectHandle::NO_TASK, ObjectClass::Task);
        Self { objects, classes }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "priority:{priority}")]
pub struct ObjectDataTableEntry {
    /// Object priority
    pub priority: Priority,
}

impl ObjectDataTableEntry {
    /// Object data entries consist of a 4-byte address and a 4-byte priority
    pub(crate) const MIN_SIZE: usize = 4 + 4;
}

impl ObjectDataTable {
    pub(crate) fn read<R: Read>(
        r: &mut R,
        endianness: Endianness,
        object_data_size: usize,
        object_data_count: usize,
    ) -> Result<Self, Error> {
        if object_data_count == 0 {
            // Empty table
            return Ok(Default::default());
        } else if object_data_size < ObjectDataTableEntry::MIN_SIZE {
            return Err(Error::InvalidObjectDataTableSlotSize);
        }

        let mut r = ByteOrdered::new(r, byteordered::Endianness::from(endianness));
        let mut object_table = ObjectDataTable::default();

        for _idx in 0..object_data_count {
            let address = r.read_u32()?;
            let priority = Priority(r.read_u32()?);
            if let Some(oh) = ObjectHandle::new(address) {
                object_table.insert(oh, priority);
            }
        }

        Ok(object_table)
    }
}
