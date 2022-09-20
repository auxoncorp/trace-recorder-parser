use crate::streaming::Error;
use crate::time::{Frequency, Timestamp};
use crate::types::{Endianness, TimerCounter};
use byteordered::ByteOrdered;
use std::io::Read;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct TimestampInfo {
    pub timer_type: TimerCounter,
    pub timer_frequency: Frequency,
    pub timer_period: u32,
    pub timer_wraparounds: u32,
    pub os_tick_rate_hz: Frequency,
    pub latest_timestamp: Timestamp,
    pub os_tick_count: u32,
}

impl TimestampInfo {
    pub(crate) fn read<R: Read>(r: &mut R, endianness: Endianness) -> Result<Self, Error> {
        let mut r = ByteOrdered::new(r, byteordered::Endianness::from(endianness));

        let hwtc_type = r.read_u32()?;
        let timer_type =
            TimerCounter::from_hwtc_type(hwtc_type).ok_or(Error::InvalidTimerCounter(hwtc_type))?;
        // NOTE: we assume TRC_BASE_TYPE and TRC_UNSIGNED_BASE_TYPE are 32-bit
        let timer_frequency = Frequency(r.read_u32()?);
        let timer_period = r.read_u32()?;
        let timer_wraparounds = r.read_u32()?;
        let os_tick_rate_hz = Frequency(r.read_u32()?);
        let latest_timestamp = Timestamp(r.read_u32()?.into());
        let os_tick_count = r.read_u32()?;

        Ok(TimestampInfo {
            timer_type,
            timer_frequency,
            timer_period,
            timer_wraparounds,
            os_tick_rate_hz,
            latest_timestamp,
            os_tick_count,
        })
    }
}
