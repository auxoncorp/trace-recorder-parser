use crate::streaming::Error;
use crate::types::{
    Endianness, KernelPortIdentity, KernelVersion, PlatformCfgVersion, TrimmedString,
};
use byteordered::ByteOrdered;
use std::io::Read;
use tracing::{debug, warn};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct HeaderInfo {
    pub endianness: Endianness,
    pub format_version: u16,
    pub kernel_version: KernelVersion,
    pub kernel_port: KernelPortIdentity,
    pub options: u32,
    pub irq_priority_order: u32,
    pub num_cores: u32,
    pub isr_tail_chaining_threshold: u32,
    pub platform_cfg: String,
    pub platform_cfg_version: PlatformCfgVersion,
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
        if format_version != 10 && !(12..=14).contains(&format_version) {
            warn!("Version {format_version} is not officially supported");
        }

        // Everything after platform is version specific
        let options = r.read_u32()?;
        let irq_priority_order = options & 0x01;
        // v14+ puts TRC_STREAM_PORT_MULTISTREAM_SUPPORT in bits 8:9
        let num_cores = r.read_u32()? & 0xFF;
        let isr_tail_chaining_threshold = r.read_u32()?;

        let platform_cfg_version_patch;
        let platform_cfg_version_minor;
        let platform_cfg_version_major;
        let mut platform_cfg_bytes: [u8; 8] = [0; 8];

        if format_version == 10 || format_version == 12 {
            r.read_exact(&mut platform_cfg_bytes)?;

            platform_cfg_version_patch = r.read_u16()?;
            platform_cfg_version_minor = r.read_u8()?;
            platform_cfg_version_major = r.read_u8()?;
        } else {
            // v13+
            platform_cfg_version_patch = r.read_u16()?;
            platform_cfg_version_minor = r.read_u8()?;
            platform_cfg_version_major = r.read_u8()?;

            r.read_exact(&mut platform_cfg_bytes)?;
        }

        let platform_cfg_version = PlatformCfgVersion {
            major: platform_cfg_version_major,
            minor: platform_cfg_version_minor,
            patch: platform_cfg_version_patch,
        };
        let platform_cfg = TrimmedString::from_raw(&platform_cfg_bytes).into();

        Ok(Self {
            endianness,
            format_version,
            kernel_version,
            kernel_port,
            options,
            irq_priority_order,
            num_cores,
            isr_tail_chaining_threshold,
            platform_cfg,
            platform_cfg_version,
        })
    }
}
