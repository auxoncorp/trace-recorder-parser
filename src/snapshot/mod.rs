use derive_more::{Display, Into};
use tracing::warn;

pub use error::Error;
pub use recorder_data::RecorderData;
pub use time::{DifferentialTimestamp, Dts16, Dts8, Frequency, Timestamp};

pub mod error;
pub mod event;
pub mod markers;
pub mod object_properties;
pub mod recorder_data;
pub mod symbol_table;
pub mod time;

pub type OffsetBytes = u64;

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

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Into, Display)]
#[display(fmt = "{_0:X?}")]
pub struct KernelVersion([u8; 2]);

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
    fn from_bits(bits: u32) -> Self {
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
}
