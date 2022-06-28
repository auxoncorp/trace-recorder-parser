use crate::snapshot::Error;
use derive_more::Display;
use std::io::{Read, Seek};

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum MarkerBytes {
    #[display(fmt = "StartMarker ({:X?})", "MarkerBytes::Start.as_bytes()")]
    Start,
    #[display(fmt = "EndMarker ({:X?})", "MarkerBytes::End.as_bytes()")]
    End,
}

impl MarkerBytes {
    pub(crate) const SIZE: usize = 12;

    pub(crate) const fn as_bytes(self) -> &'static [u8] {
        use MarkerBytes::*;
        match self {
            Start => &[
                0x01, 0x02, 0x03, 0x04, 0x71, 0x72, 0x73, 0x74, 0xF1, 0xF2, 0xF3, 0xF4,
            ],
            End => &[
                0x0A, 0x0B, 0x0C, 0x0D, 0x71, 0x72, 0x73, 0x74, 0xF1, 0xF2, 0xF3, 0xF4,
            ],
        }
    }

    pub(crate) fn read<R: Read + Seek>(self, r: &mut R) -> Result<(), Error> {
        let pos = r.stream_position()?;
        let mut bytes: [u8; MarkerBytes::SIZE] = [0; 12];
        r.read_exact(&mut bytes)?;
        if bytes.as_slice() != self.as_bytes() {
            Err(Error::MarkerBytes(pos, bytes, self))
        } else {
            Ok(())
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
pub enum DebugMarker {
    #[display(fmt = "DebugMarker0 (0x{:X})", "DebugMarker::Marker0.into_u32()")]
    Marker0,
    #[display(fmt = "DebugMarker1 (0x{:X})", "DebugMarker::Marker1.into_u32()")]
    Marker1,
    #[display(fmt = "DebugMarker2 (0x{:X})", "DebugMarker::Marker2.into_u32()")]
    Marker2,
    #[display(fmt = "DebugMarker3 (0x{:X})", "DebugMarker::Marker3.into_u32()")]
    Marker3,
}

impl DebugMarker {
    const fn into_u32(self) -> u32 {
        use DebugMarker::*;
        match self {
            Marker0 => 0xF0F0F0F0,
            Marker1 => 0xF1F1F1F1,
            Marker2 => 0xF2F2F2F2,
            Marker3 => 0xF3F3F3F3,
        }
    }

    pub(crate) fn read<R: Read + Seek>(self, r: &mut R) -> Result<(), Error> {
        let pos = r.stream_position()?;
        let mut bytes: [u8; 4] = [0; 4];
        r.read_exact(&mut bytes)?;
        let marker = u32::from_le_bytes(bytes);
        if marker != self.into_u32() {
            Err(Error::DebugMarker(pos, marker, self))
        } else {
            Ok(())
        }
    }
}
