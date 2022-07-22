pub use error::Error;
pub use object_properties::ObjectPropertyTable;
pub use recorder_data::RecorderData;
pub use symbol_table::{SymbolTable, SymbolTableEntry};

pub mod error;
pub mod event;
pub mod markers;
pub mod object_properties;
pub mod recorder_data;
pub mod symbol_table;
