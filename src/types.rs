//! Types common to both the snapshot and streaming protocol

use byteordered::ByteOrdered;
use derive_more::{Binary, Deref, Display, From, Into, LowerHex, Octal, UpperHex};
use ordered_float::OrderedFloat;
use std::fmt::Write as _;
use std::io;
use std::num::NonZeroU32;
use std::str::FromStr;
use thiserror::Error;
use tracing::warn;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum Protocol {
    #[display(fmt = "snapshot")]
    Snapshot,
    #[display(fmt = "streaming")]
    Streaming,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum Endianness {
    #[display(fmt = "little-endian")]
    Little,
    #[display(fmt = "big-endian")]
    Big,
}

impl From<byteordered::Endianness> for Endianness {
    fn from(e: byteordered::Endianness) -> Self {
        match e {
            byteordered::Endianness::Little => Endianness::Little,
            byteordered::Endianness::Big => Endianness::Big,
        }
    }
}

impl From<Endianness> for byteordered::Endianness {
    fn from(e: Endianness) -> byteordered::Endianness {
        match e {
            Endianness::Little => byteordered::Endianness::Little,
            Endianness::Big => byteordered::Endianness::Big,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum KernelPortIdentity {
    #[display(fmt = "FreeRTOS")]
    FreeRtos,
    #[display(fmt = "Zephyr")]
    Zephyr,
    #[display(fmt = "ThreadX")]
    ThreadX,
    #[display(fmt = "Unknown")]
    Unknown,
}

pub type OffsetBytes = u64;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Into, Display)]
#[display(fmt = "{_0:X?}")]
pub struct KernelVersion(pub(crate) [u8; 2]);

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Into, thiserror::Error)]
#[error("Invalid kernel version {0:?}")]
pub struct InvalidKernelVersion(pub [u8; 2]);

impl KernelVersion {
    const VERSION_CONSTANT: u8 = 0xAA;

    pub fn port_identity(&self) -> Result<KernelPortIdentity, InvalidKernelVersion> {
        let inner = self.join_inner_nibbles();
        let outer = self.join_outer_nibbles();
        match (inner, outer) {
            (Self::VERSION_CONSTANT, identity) | (identity, Self::VERSION_CONSTANT) => {
                match identity {
                    // TRACE_KERNEL_VERSION 0x1AA1
                    0x11 => Ok(KernelPortIdentity::FreeRtos),
                    // TRACE_KERNEL_VERSION 0x9AA9
                    0x99 => Ok(KernelPortIdentity::Zephyr),
                    // TRACE_KERNEL_VERSION 0xEAAE
                    0xEE => Ok(KernelPortIdentity::ThreadX),
                    _ => Err(InvalidKernelVersion(self.0)),
                }
            }
            _ => Err(InvalidKernelVersion(self.0)),
        }
    }

    pub fn endianness(&self) -> Result<Endianness, InvalidKernelVersion> {
        let inner = self.join_inner_nibbles();
        let outer = self.join_outer_nibbles();
        match (inner, outer) {
            (Self::VERSION_CONSTANT, _) => Ok(Endianness::Little),
            (_, Self::VERSION_CONSTANT) => Ok(Endianness::Big),
            _ => Err(InvalidKernelVersion(self.0)),
        }
    }

    /// Extract the lower nibble of the first byte and upper nibble
    /// of the second byte to form a single byte
    /// 0xAB_BA ([0xBA, 0xAB]) -> 0xAA
    fn join_outer_nibbles(&self) -> u8 {
        (self.0[0] & 0x0F) | (self.0[1] & 0xF0)
    }

    /// Extract the upper nibble of the first byte and lower nibble
    /// of the second byte to form a single byte
    /// 0xAB_BA ([0xBA, 0xAB]) -> 0xBB
    fn join_inner_nibbles(&self) -> u8 {
        (self.0[0] >> 4) | ((self.0[1] & 0x0F) << 4)
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Into, Display)]
#[display(fmt = "{major}.{minor}.{patch}")]
pub struct PlatformCfgVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u16,
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum FloatEncoding {
    // TRC_CFG_INCLUDE_FLOAT_SUPPORT == 0
    #[display(fmt = "Unsupported")]
    Unsupported,
    #[display(fmt = "little-endian")]
    LittleEndian,
    #[display(fmt = "big-endian")]
    BigEndian,
}

impl FloatEncoding {
    pub(crate) fn from_bits(bits: u32) -> Self {
        if bits == 0 {
            FloatEncoding::Unsupported
        } else if f32::from_bits(bits.to_le()) == 1.0 {
            FloatEncoding::LittleEndian
        } else if f32::from_bits(bits.to_be()) == 1.0 {
            FloatEncoding::BigEndian
        } else {
            warn!("Could not determine float encoding for 0x{bits:X}");
            FloatEncoding::Unsupported
        }
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
    Deref,
    Into,
    Display,
    Binary,
    Octal,
    LowerHex,
    UpperHex,
)]
#[display(fmt = "{_0}")]
pub struct ObjectHandle(pub(crate) NonZeroU32);

impl ObjectHandle {
    /// used for "task address" when no task has started, to indicate "(startup)" in streaming
    /// protocol
    pub const NO_TASK: Self = ObjectHandle::new_unchecked(2);

    pub(crate) const fn new(handle: u32) -> Option<Self> {
        if let Some(oh) = NonZeroU32::new(handle) {
            Some(Self(oh))
        } else {
            None
        }
    }

    pub(crate) const fn new_unchecked(handle: u32) -> Self {
        unsafe { ObjectHandle(NonZeroU32::new_unchecked(handle)) }
    }
}

impl From<ObjectHandle> for u32 {
    fn from(h: ObjectHandle) -> u32 {
        h.0.get()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum ObjectClass {
    #[display(fmt = "Queue")]
    Queue = 0,
    #[display(fmt = "Semaphore")]
    Semaphore = 1,
    #[display(fmt = "Mutex")]
    Mutex = 2,
    #[display(fmt = "Task")]
    Task = 3,
    #[display(fmt = "ISR")]
    Isr = 4,
    #[display(fmt = "Timer")]
    Timer = 5,
    #[display(fmt = "EventGroup")]
    EventGroup = 6,
    #[display(fmt = "StreamBuffer")]
    StreamBuffer = 7,
    #[display(fmt = "MessageBuffer")]
    MessageBuffer = 8,
}

impl ObjectClass {
    pub(crate) fn into_usize(self) -> usize {
        self as _
    }

    pub(crate) fn enumerate() -> &'static [Self] {
        use ObjectClass::*;
        &[
            Queue,
            Semaphore,
            Mutex,
            Task,
            Isr,
            Timer,
            EventGroup,
            StreamBuffer,
            MessageBuffer,
        ]
    }

    pub(crate) fn properties_size(self) -> usize {
        use ObjectClass::*;
        match self {
            Queue => 1,
            Semaphore => 1,
            Mutex => 1,
            Task => 4,
            Isr => 2,
            Timer => 1,
            EventGroup => 4,
            StreamBuffer => 4,
            MessageBuffer => 4,
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, thiserror::Error)]
#[error("Invalid object class")]
pub struct ParseObjectClassError;

impl FromStr for ObjectClass {
    type Err = ParseObjectClassError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ObjectClass::*;
        Ok(match s.to_lowercase().trim() {
            "queue" => Queue,
            "semaphore" => Semaphore,
            "mutex" => Mutex,
            "task" => Task,
            "isr" => Isr,
            "timer" => Timer,
            "eventgroup" => EventGroup,
            "streambuffer" => StreamBuffer,
            "messagebuffer" => MessageBuffer,
            _ => return Err(ParseObjectClassError),
        })
    }
}

pub const UNNAMED_OBJECT: &str = "<unnamed>";

pub(crate) trait SymbolTableExt {
    fn symbol(&self, handle: ObjectHandle) -> Option<&SymbolString>;
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Into, Display)]
#[display(fmt = "{_0}")]
pub struct SymbolString(pub(crate) String);

impl From<TrimmedString> for SymbolString {
    fn from(s: TrimmedString) -> Self {
        Self(s.0)
    }
}

impl AsRef<str> for SymbolString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for SymbolString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Into, Display)]
#[display(fmt = "{_0}")]
pub(crate) struct TrimmedString(pub(crate) String);

impl TrimmedString {
    pub(crate) fn from_raw(s: &[u8]) -> Self {
        let s = String::from_utf8_lossy(s);
        let substr = if let Some(idx) = s.find(char::from(0)) {
            &s[..idx]
        } else {
            &s
        };
        Self(substr.trim_end_matches(char::from(0)).to_string())
    }
}

impl std::ops::Deref for TrimmedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub const STARTUP_TASK_NAME: &str = "(startup)";
pub const TZ_CTRL_TASK_NAME: &str = "TzCtrl";

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Into, Display)]
#[display(fmt = "{_0}")]
pub struct ObjectName(pub(crate) String);

pub type TaskName = ObjectName;
pub type IsrName = ObjectName;
pub type QueueName = ObjectName;
pub type SemaphoreName = ObjectName;
pub type MutexName = ObjectName;
pub type EventGroupName = ObjectName;
pub type MessageBufferName = ObjectName;

impl From<SymbolString> for ObjectName {
    fn from(s: SymbolString) -> Self {
        Self(s.0)
    }
}

impl AsRef<str> for ObjectName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for ObjectName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Into, Display)]
#[display(fmt = "{_0}")]
pub struct Priority(pub(crate) u32);

pub type TaskPriority = Priority;
pub type IsrPriority = Priority;

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

#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    From,
    Into,
    Display,
    Binary,
    Octal,
    LowerHex,
    UpperHex,
    Deref,
)]
#[display(fmt = "{_0}")]
pub struct UserEventArgRecordCount(pub u8);

impl UserEventArgRecordCount {
    pub const MAX: usize = 15;
}

impl From<UserEventArgRecordCount> for usize {
    fn from(c: UserEventArgRecordCount) -> Self {
        c.0.into()
    }
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
pub struct FormatString(pub(crate) String);

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
    SymbolLookup(ObjectHandle),

    #[error(
            "Encountered and IO error while parsing user event format arguments ({})",
            .0.kind()
        )]
    Io(#[from] io::Error),
}

// TODO - float & float endianness support, warn if not supported and found
// TODO - tests for all this, like '%%' == "%"
// NOTE Assumes UTF8
pub(crate) fn format_symbol_string<S: SymbolTableExt>(
    symbol_table: &S,
    protocol: Protocol,
    endianness: Endianness,
    format_string: &str,
    arg_data: &[u8],
) -> Result<(FormattedString, Vec<Argument>), FormattedStringError> {
    let mut r = ByteOrdered::runtime(arg_data, byteordered::Endianness::from(endianness));
    let mut formatted_string = String::new();
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
                    let arg_index = ObjectHandle::new(match protocol {
                        Protocol::Snapshot => r.read_u16()?.into(),
                        Protocol::Streaming => r.read_u32()?,
                    })
                    .ok_or(FormattedStringError::InvalidSymbolTableIndex)?;
                    let symbol = symbol_table
                        .symbol(arg_index)
                        .ok_or(FormattedStringError::SymbolLookup(arg_index))?;
                    Argument::String(symbol.to_string())
                }
                'f' if !matches!(found_subspec, SubSpecifier::Long) => {
                    Argument::F32(r.read_f32()?.into())
                }
                'f' if matches!(found_subspec, SubSpecifier::Long) => {
                    Argument::F64(r.read_f64()?.into())
                }
                'd' if matches!(found_subspec, SubSpecifier::Short) => {
                    Argument::I16(match protocol {
                        Protocol::Snapshot => r.read_i16()?,
                        Protocol::Streaming => r.read_i32()? as i16,
                    })
                }
                'u' if matches!(found_subspec, SubSpecifier::Short) => {
                    Argument::U16(match protocol {
                        Protocol::Snapshot => r.read_u16()?,
                        Protocol::Streaming => r.read_u32()? as u16,
                    })
                }
                'd' if matches!(found_subspec, SubSpecifier::Octet) => {
                    Argument::I8(match protocol {
                        Protocol::Snapshot => r.read_i8()?,
                        Protocol::Streaming => r.read_i32()? as i8,
                    })
                }
                'u' if matches!(found_subspec, SubSpecifier::Octet) => {
                    Argument::U8(match protocol {
                        Protocol::Snapshot => r.read_u8()?,
                        Protocol::Streaming => r.read_u32()? as u8,
                    })
                }
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

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum TimerCounter {
    FreeRunning32Incr,
    FreeRunning32Decr,
    OsIncr,
    OsDecr,
    CustomIncr,
    CustomDecr,
}

impl TimerCounter {
    pub fn is_increment(&self) -> bool {
        use TimerCounter::*;
        matches!(self, FreeRunning32Incr | OsIncr | CustomIncr)
    }

    pub(crate) fn from_hwtc_type(tc: u32) -> Option<Self> {
        use TimerCounter::*;
        Some(match tc {
            1 => FreeRunning32Incr,
            2 => FreeRunning32Decr,
            3 => OsIncr,
            4 => OsDecr,
            5 => CustomIncr,
            6 => CustomDecr,
            _ => return None,
        })
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
pub struct Heap {
    pub current: u32,
    pub high_water_mark: u32,
    pub max: u32,
}

impl Heap {
    pub(crate) fn handle_alloc(&mut self, size: u32) {
        self.current = self.current.saturating_add(size);
        if self.current > self.high_water_mark {
            self.high_water_mark = self.current;
        }
    }

    pub(crate) fn handle_free(&mut self, size: u32) {
        self.current = self.current.saturating_sub(size);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn kernel_version_endianess_identity() {
        let kv = KernelVersion([0xA1, 0x1A]);
        assert_eq!(kv.port_identity(), Ok(KernelPortIdentity::FreeRtos));
        assert_eq!(kv.endianness(), Ok(Endianness::Little));
        let kv = KernelVersion([0x1A, 0xA1]);
        assert_eq!(kv.port_identity(), Ok(KernelPortIdentity::FreeRtos));
        assert_eq!(kv.endianness(), Ok(Endianness::Big));
        let kv = KernelVersion([0xAE, 0xEA]);
        assert_eq!(kv.port_identity(), Ok(KernelPortIdentity::ThreadX));
        assert_eq!(kv.endianness(), Ok(Endianness::Little));
        let kv = KernelVersion([0xEA, 0xAE]);
        assert_eq!(kv.port_identity(), Ok(KernelPortIdentity::ThreadX));
        assert_eq!(kv.endianness(), Ok(Endianness::Big));
        let kv = KernelVersion([0xA9, 0x9A]);
        assert_eq!(kv.port_identity(), Ok(KernelPortIdentity::Zephyr));
        assert_eq!(kv.endianness(), Ok(Endianness::Little));
        let kv = KernelVersion([0x9A, 0xA9]);
        assert_eq!(kv.port_identity(), Ok(KernelPortIdentity::Zephyr));
        assert_eq!(kv.endianness(), Ok(Endianness::Big));

        let kv = KernelVersion([0x9B, 0xB9]);
        assert_eq!(kv.port_identity(), Err(InvalidKernelVersion([0x9B, 0xB9])));
        assert_eq!(kv.endianness(), Err(InvalidKernelVersion([0x9B, 0xB9])));
    }

    #[test]
    fn float_encoding() {
        assert_eq!(FloatEncoding::from_bits(0), FloatEncoding::Unsupported);
        assert_eq!(
            FloatEncoding::from_bits(1.0_f32.to_bits().to_le()),
            FloatEncoding::LittleEndian
        );
        assert_eq!(
            FloatEncoding::from_bits(1.0_f32.to_bits().to_be()),
            FloatEncoding::BigEndian
        );
    }

    #[test]
    fn trimmed_string() {
        assert_eq!(TrimmedString::from_raw(b"foo bar").0.as_str(), "foo bar");
        assert_eq!(
            TrimmedString::from_raw(b"foo bar\0\0\0").0.as_str(),
            "foo bar"
        );
        assert_eq!(TrimmedString::from_raw(b"foo\0\0\0bar").0.as_str(), "foo");
        assert_eq!(TrimmedString::from_raw(b"\0foo\0\0\0bar").0.as_str(), "");
        assert_eq!(TrimmedString::from_raw(b"").0.as_str(), "");
    }

    #[test]
    fn string_formatting() {
        let mut sn_st = crate::snapshot::SymbolTable::default();
        let mut sr_st = crate::streaming::EntryTable::default();

        let fmt = "literal";
        assert_eq!(
            format_symbol_string(&sn_st, Protocol::Snapshot, Endianness::Little, fmt, &[]).unwrap(),
            (FormattedString(fmt.to_string()), vec![])
        );
        assert_eq!(
            format_symbol_string(&sr_st, Protocol::Streaming, Endianness::Little, fmt, &[])
                .unwrap(),
            (FormattedString(fmt.to_string()), vec![])
        );

        let fmt = "foo bar biz %%";
        let out = "foo bar biz %";
        assert_eq!(
            format_symbol_string(&sn_st, Protocol::Snapshot, Endianness::Little, fmt, &[]).unwrap(),
            (FormattedString(out.to_string()), vec![])
        );
        assert_eq!(
            format_symbol_string(&sr_st, Protocol::Streaming, Endianness::Little, fmt, &[])
                .unwrap(),
            (FormattedString(out.to_string()), vec![])
        );

        let fmt = "my int %d = %02u";
        let out = "my int -1 = 23";
        let arg_bytes: Vec<u8> = i32::to_le_bytes(-1)
            .into_iter()
            .chain(u32::to_le_bytes(23))
            .collect();
        assert_eq!(
            format_symbol_string(
                &sn_st,
                Protocol::Snapshot,
                Endianness::Little,
                fmt,
                &arg_bytes
            )
            .unwrap(),
            (
                FormattedString(out.to_string()),
                vec![Argument::I32(-1), Argument::U32(23)]
            )
        );
        assert_eq!(
            format_symbol_string(
                &sr_st,
                Protocol::Streaming,
                Endianness::Little,
                fmt,
                &arg_bytes
            )
            .unwrap(),
            (
                FormattedString(out.to_string()),
                vec![Argument::I32(-1), Argument::U32(23)]
            )
        );

        let fmt = "my float %f";
        let out = "my float -1.1";
        let arg_bytes: Vec<u8> = f32::to_le_bytes(-1.1).into_iter().collect();
        assert_eq!(
            format_symbol_string(
                &sn_st,
                Protocol::Snapshot,
                Endianness::Little,
                fmt,
                &arg_bytes
            )
            .unwrap(),
            (
                FormattedString(out.to_string()),
                vec![Argument::F32(OrderedFloat::from(-1.1_f32))]
            )
        );
        assert_eq!(
            format_symbol_string(
                &sr_st,
                Protocol::Streaming,
                Endianness::Little,
                fmt,
                &arg_bytes
            )
            .unwrap(),
            (
                FormattedString(out.to_string()),
                vec![Argument::F32(OrderedFloat::from(-1.1_f32))]
            )
        );

        let fmt = "small int %bd = medium int %hd";
        let out = "small int -4 = medium int -25";
        let arg_bytes: Vec<u8> = i8::to_le_bytes(-4)
            .into_iter()
            .chain(i16::to_le_bytes(-25))
            .collect();
        assert_eq!(
            format_symbol_string(
                &sn_st,
                Protocol::Snapshot,
                Endianness::Little,
                fmt,
                &arg_bytes
            )
            .unwrap(),
            (
                FormattedString(out.to_string()),
                vec![Argument::I8(-4), Argument::I16(-25)]
            )
        );
        let arg_bytes: Vec<u8> = i32::to_le_bytes(-4_i8 as i32)
            .into_iter()
            .chain(i32::to_le_bytes(-25_i16 as i32))
            .collect();
        assert_eq!(
            format_symbol_string(
                &sr_st,
                Protocol::Streaming,
                Endianness::Little,
                fmt,
                &arg_bytes
            )
            .unwrap(),
            (
                FormattedString(out.to_string()),
                vec![Argument::I8(-4), Argument::I16(-25)]
            )
        );

        let fmt = "my string = '%s'";
        let out = "my string = 'foo'";
        let str_arg = b"foo\0";
        let handle = ObjectHandle::new(1).unwrap();
        let symbol: SymbolString = TrimmedString::from_raw(str_arg).into();
        sn_st.insert(
            handle,
            None,
            crate::snapshot::symbol_table::SymbolCrc6::new(str_arg),
            symbol.clone(),
        );
        sr_st.entry(handle).set_symbol(symbol.clone());
        let arg_bytes = u32::to_le_bytes(handle.0.get());
        assert_eq!(
            format_symbol_string(
                &sn_st,
                Protocol::Snapshot,
                Endianness::Little,
                fmt,
                &arg_bytes
            )
            .unwrap(),
            (
                FormattedString(out.to_string()),
                vec![Argument::String(symbol.0.clone())]
            )
        );
        assert_eq!(
            format_symbol_string(
                &sr_st,
                Protocol::Streaming,
                Endianness::Little,
                fmt,
                &arg_bytes
            )
            .unwrap(),
            (
                FormattedString(out.to_string()),
                vec![Argument::String(symbol.0)]
            )
        );
    }
}
