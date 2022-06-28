use derive_more::{
    Add, AddAssign, Binary, Deref, Display, Into, LowerHex, MulAssign, Octal, Sum, UpperHex,
};
use std::ops;

/// Frequency of the clock/timer/counter used as time base
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    Display,
    Deref,
    Into,
    Binary,
    Octal,
    LowerHex,
    UpperHex,
    Add,
    Sum,
    AddAssign,
    MulAssign,
)]
#[display(fmt = "{_0}")]
pub struct Frequency(pub(crate) u32);

impl Frequency {
    pub fn is_unitless(&self) -> bool {
        self.0 == 0
    }
}

/// Timestamp (in ticks).
/// Stores accumulated differential timestamps.
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    Display,
    Binary,
    Octal,
    LowerHex,
    UpperHex,
    Add,
    Sum,
    AddAssign,
    MulAssign,
)]
#[display(fmt = "{_0}")]
pub struct Timestamp(pub(crate) u64);

impl Timestamp {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn ticks(&self) -> u64 {
        self.0
    }

    // TODO - add time base/Frequency conversions for units
}

impl ops::Add<DifferentialTimestamp> for Timestamp {
    type Output = Timestamp;

    fn add(self, dt: DifferentialTimestamp) -> Timestamp {
        Timestamp(
            self.0
                .checked_add(u64::from(dt.0))
                .expect("Overflow when adding differential time to timestamp"),
        )
    }
}

impl ops::AddAssign<DifferentialTimestamp> for Timestamp {
    fn add_assign(&mut self, dt: DifferentialTimestamp) {
        self.0 = self
            .0
            .checked_add(u64::from(dt.0))
            .expect("Overflow when adding differential time to timestamp")
    }
}

/// Time (in ticks) since the previous event in the recorder log.
/// Can be up to 4 bytes in size, depending on how many DTS bytes are
/// available in the event at hand and how much time has ellasped since
/// the previous event.
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Debug,
    Display,
    Binary,
    Octal,
    LowerHex,
    UpperHex,
    Add,
    Sum,
    AddAssign,
    MulAssign,
)]
#[display(fmt = "{_0}")]
pub struct DifferentialTimestamp(pub(crate) u32);

impl DifferentialTimestamp {
    pub fn ticks(&self) -> u32 {
        self.0
    }
}

impl DifferentialTimestamp {
    /// Construct a differential timestamp from the data of an XTS8 event.
    /// XTS8 events contain the upper 3 bytes, and the event following contains
    /// the lower byte.
    pub(crate) fn from_xts8(xts_8: u8, xts_16: u16) -> Self {
        DifferentialTimestamp(u32::from(xts_8) << 24 | (u32::from(xts_16) << 8))
    }

    /// Construct a differential timestamp from the data of an XTS16 event.
    /// XTS16 events contain the upper 2 bytes, and the event following contains
    /// the lower 2 bytes.
    pub(crate) fn from_xts16(xts_16: u16) -> Self {
        DifferentialTimestamp(u32::from(xts_16) << 16)
    }

    pub fn zero() -> Self {
        Self(0)
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }
}

impl ops::AddAssign<Dts8> for DifferentialTimestamp {
    fn add_assign(&mut self, dts: Dts8) {
        self.0 = self
            .0
            .checked_add(u32::from(dts.0))
            .expect("Overflow when adding DTS8 to differential time")
    }
}

impl ops::AddAssign<Dts16> for DifferentialTimestamp {
    fn add_assign(&mut self, dts: Dts16) {
        self.0 = self
            .0
            .checked_add(u32::from(dts.0))
            .expect("Overflow when adding DTS16 to differential time")
    }
}

/// The lower 8-bit portion of a differential timestamp recorded in an event
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
)]
#[display(fmt = "{_0}")]
pub struct Dts8(pub(crate) u8);

/// The lower 16-bit portion of a differential timestamp recorded in an event
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
)]
#[display(fmt = "{_0}")]
pub struct Dts16(pub(crate) u16);

// TODO - add more time tests, Frequency and time base conversions
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn differential_time_xts16() {
        let mut accumulated_time = Timestamp::zero();
        accumulated_time.0 += 0x0F;
        assert_eq!(accumulated_time.ticks(), 0x0F);

        let xts_16 = 0x00_03;
        let mut dts_for_next_event = DifferentialTimestamp::from_xts16(xts_16);
        assert_eq!(dts_for_next_event.ticks(), 0x00_03_00_00);

        let dts = Dts16(0x5F_D5);
        dts_for_next_event += dts;
        assert_eq!(dts_for_next_event.ticks(), 0x00_03_5F_D5);

        accumulated_time += dts_for_next_event;
        assert_eq!(accumulated_time.ticks(), 0x00_03_5F_D5 + 0x0F);
    }
}
