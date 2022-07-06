use crate::snapshot::symbol_table::{SymbolTable, SymbolTableEntryIndex};
use crate::snapshot::Timestamp;
use byteordered::{ByteOrdered, Endianness};
use derive_more::{Binary, Deref, Display, Into, LowerHex, Octal, UpperHex};
use ordered_float::OrderedFloat;
use std::fmt::Write as _;
use std::io;
use thiserror::Error;
use tracing::warn;

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
    Deref,
)]
#[display(fmt = "{_0}")]
pub struct UserEventArgRecordCount(pub(crate) u8);

impl UserEventArgRecordCount {
    pub const MAX: usize = 15;
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum UserEventChannel {
    #[display(fmt = "{}", UserEventChannel::DEFAULT)]
    Default,
    #[display(fmt = "{_0}")]
    Custom(String),
}

impl UserEventChannel {
    pub const DEFAULT: &'static str = "default";

    pub fn as_str(&self) -> &str {
        match self {
            UserEventChannel::Default => Self::DEFAULT,
            UserEventChannel::Custom(s) => s.as_str(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "[{timestamp}]:[{channel}]='{formatted_string}'")]
pub struct UserEvent {
    pub timestamp: Timestamp,
    pub channel: UserEventChannel,
    pub formatted_string: FormattedString,
    pub args: Vec<Argument>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}")]
pub enum Argument {
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    F32(OrderedFloat<f32>),
    F64(OrderedFloat<f64>),
    String(String),
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Into, Deref, Display)]
#[display(fmt = "{_0}")]
pub struct FormattedString(pub(crate) String);

#[derive(Debug, Error)]
pub enum FormattedStringError {
    #[error(
        "Found a user event string format argument with an invalid zero value symbol table index"
    )]
    InvalidSymbolTableIndex,

    #[error(
        "Found a user event string format argument with index {0} that doesn't exist in the symbol table"
    )]
    SymbolLookup(SymbolTableEntryIndex),

    #[error(
            "Encountered and IO error while parsing user event format arguments ({})",
            .0.kind()
        )]
    Io(#[from] io::Error),
}

// TODO - float & float endianness support, warn if not supported and found
// TODO - tests for all this, like '%%' == "%"
// N.B. Assumes UTF8
pub(crate) fn format_symbol_string(
    symbol_table: &SymbolTable,
    endianness: Endianness,
    format_string: &str,
    arg_data: &[u8],
) -> Result<(FormattedString, Vec<Argument>), FormattedStringError> {
    let mut r = ByteOrdered::runtime(arg_data, endianness);
    let mut formatted_string = String::new();
    //let mut formatted_string = format_string.to_string();
    let mut args = Vec::new();
    let mut found_format_specifier = false;
    let mut found_subspec = SubSpecifier::None;

    for in_c in format_string.chars() {
        let is_width_or_padding = in_c.is_numeric() || in_c == '#' || in_c == '.';
        if in_c == '%' {
            if found_format_specifier {
                found_format_specifier = false;
                formatted_string.push(in_c);
            } else {
                found_format_specifier = true;
                found_subspec = SubSpecifier::None;
            }
        } else if found_format_specifier && is_width_or_padding {
            // TODO - support width and padding, skip it for now
        } else if found_format_specifier && !is_width_or_padding && in_c == 'l' {
            found_subspec = SubSpecifier::Long;
        } else if found_format_specifier && !is_width_or_padding && in_c == 'h' {
            found_subspec = SubSpecifier::Short;
        } else if found_format_specifier && !is_width_or_padding && in_c == 'b' {
            found_subspec = SubSpecifier::Octet;
        } else if found_format_specifier && !is_width_or_padding {
            // TODO - add formatting support (x|X hexidecimal, etc)
            let arg = match in_c {
                'd' if matches!(found_subspec, SubSpecifier::None) => Argument::I32(r.read_i32()?),
                'u' if matches!(found_subspec, SubSpecifier::None) => Argument::U32(r.read_u32()?),
                'x' | 'X' => Argument::U32(r.read_u32()?),
                's' => {
                    let arg_index = SymbolTableEntryIndex::new(r.read_u16()?)
                        .ok_or(FormattedStringError::InvalidSymbolTableIndex)?;
                    let sym_entry = symbol_table
                        .entry(arg_index)
                        .ok_or(FormattedStringError::SymbolLookup(arg_index))?;
                    Argument::String(sym_entry.symbol.to_string())
                }
                'f' if !matches!(found_subspec, SubSpecifier::Long) => {
                    Argument::F32(r.read_f32()?.into())
                }
                'f' if matches!(found_subspec, SubSpecifier::Long) => {
                    Argument::F64(r.read_f64()?.into())
                }
                'd' if matches!(found_subspec, SubSpecifier::Short) => Argument::I16(r.read_i16()?),
                'u' if matches!(found_subspec, SubSpecifier::Short) => Argument::U16(r.read_u16()?),
                'd' if matches!(found_subspec, SubSpecifier::Octet) => Argument::I8(r.read_i8()?),
                'u' if matches!(found_subspec, SubSpecifier::Octet) => Argument::U8(r.read_u8()?),
                _ => {
                    warn!("Found unsupported format specifier '{in_c}' in user event format string '{format_string}'");
                    return Ok((
                        FormattedString(format_string.to_string()),
                        Default::default(),
                    ));
                }
            };

            let _ = write!(formatted_string, "{arg}");
            args.push(arg);

            found_format_specifier = false;
            found_subspec = SubSpecifier::None;
        } else {
            formatted_string.push(in_c);
        }
    }

    Ok((FormattedString(formatted_string), args))
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum SubSpecifier {
    None,
    Long,
    Short,
    Octet,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn string_formatting() {
        let st = SymbolTable {
            symbols: Default::default(),
        };

        assert_eq!(
            format_symbol_string(&st, Endianness::Little, "foo bar biz %%", &[]).unwrap(),
            (FormattedString("foo bar biz %".to_string()), vec![])
        );

        let arg_bytes: Vec<u8> = i32::to_le_bytes(-1)
            .into_iter()
            .chain(u32::to_le_bytes(23).into_iter())
            .collect();
        assert_eq!(
            format_symbol_string(&st, Endianness::Little, "my int %d = %02u", &arg_bytes).unwrap(),
            (
                FormattedString("my int -1 = 23".to_string()),
                vec![Argument::I32(-1), Argument::U32(23)]
            )
        );
    }
}
