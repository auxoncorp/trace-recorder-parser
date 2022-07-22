use crate::streaming::Error;
use crate::types::{Endianness, KernelPortIdentity, KernelVersion};
use byteordered::ByteOrdered;
use std::io::Read;
use tracing::{debug, warn};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct HeaderInfo {
    pub endianness: Endianness,
    pub format_version: u16,
    pub kernel_version: KernelVersion,
    pub kernel_port: KernelPortIdentity,
    pub irq_priority_order: u32,
    pub heap_counter: u32,
    /// `SYMBOL_TABLE_SLOT_SIZE`, size in bytes of each symbol table entry
    pub symbol_size: usize,
    /// `TRC_CFG_SYMBOL_TABLE_SLOTS`, number of symbol table entries
    pub symbol_count: usize,
    /// `OBJECT_DATA_SLOT_SIZE`, size in bytes of each object
    pub object_data_size: usize,
    /// `TRC_CFG_OBJECT_DATA_SLOTS`, number of object data entries
    pub object_data_count: usize,
}

impl HeaderInfo {
    pub const WIRE_SIZE: usize = 24;
    pub const PSF_LITTLE_ENDIAN: u32 = 0x50_53_46_00;
    pub const PSF_BIG_ENDIAN: u32 = 0x00_46_53_50;

    pub fn read_psf_word<R: Read>(r: &mut R) -> Result<Endianness, Error> {
        let mut r = ByteOrdered::native(r);
        let mut psf = [0; 4];
        r.read_exact(&mut psf)?;
        let endianness = match u32::from_le_bytes(psf) {
            Self::PSF_LITTLE_ENDIAN => Endianness::Little,
            Self::PSF_BIG_ENDIAN => Endianness::Big,
            bad_psf => return Err(Error::PSFEndiannessIdentifier(bad_psf)),
        };
        Ok(endianness)
    }

    pub fn read<R: Read>(r: &mut R) -> Result<Self, Error> {
        let endianness = Self::read_psf_word(r)?;

        // The remaining fields are endian-aware
        let mut r = ByteOrdered::new(r, byteordered::Endianness::from(endianness));

        let format_version = r.read_u16()?;
        debug!(format_version = format_version, "Found format version");
        let platform = r.read_u16()?;
        let kernel_version = KernelVersion(platform.to_le_bytes());
        let kernel_port = kernel_version
            .port_identity()
            .map_err(|e| Error::KernelVersion(e.0))?;
        debug!(kernel_version = %kernel_version, kernel_port = %kernel_port, endianness = ?endianness, "Found kernel version");

        if kernel_port != KernelPortIdentity::FreeRtos {
            warn!("Kernel port {kernel_port} is not officially supported");
        }
        if format_version != 6 {
            warn!("Version {format_version} is not officially supported");
        }

        // Everything after this is version specific

        let irq_priority_order = r.read_u32()? & 0x01;
        let heap_counter = r.read_u32()?;
        let symbol_size = r.read_u16()?.into();
        let symbol_count = r.read_u16()?.into();
        let object_data_size = r.read_u16()?.into();
        let object_data_count = r.read_u16()?.into();

        Ok(Self {
            endianness,
            format_version,
            kernel_version,
            kernel_port,
            irq_priority_order,
            heap_counter,
            symbol_size,
            symbol_count,
            object_data_size,
            object_data_count,
        })
    }
}
