use crate::streaming::Error;
use crate::types::Endianness;
use byteordered::ByteOrdered;
use std::io::Read;
use tracing::warn;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct ExtensionInfo {
    pub entry_count: usize,
    pub base_event_code: u16,
}

impl ExtensionInfo {
    pub(crate) const EVENTCODE_BASE: u16 = 256;

    pub(crate) fn read<R: Read>(r: &mut R, endianness: Endianness) -> Result<Self, Error> {
        let mut r = ByteOrdered::new(r, byteordered::Endianness::from(endianness));
        let entry_count = r.read_u16()?.into();
        let base_event_code = r.read_u16()?;
        if base_event_code < Self::EVENTCODE_BASE {
            warn!(
                "TRC_EXTENSION_EVENTCODE_BASE ({base_event_code}) should be greater than {}",
                Self::EVENTCODE_BASE,
            );
        }
        if entry_count != 0 {
            warn!("Skipping over unsupported extension info");
            let _entry_max_name_len = r.read_u8()?;
            let entry_size = r.read_u8()?;
            let mut buf = Vec::with_capacity(entry_size.into());
            for _ in 0..entry_count {
                buf.clear();
                buf.resize(entry_count, 0);
                r.read_exact(&mut buf)?;
            }
        }
        Ok(Self {
            entry_count,
            base_event_code,
        })
    }
}
