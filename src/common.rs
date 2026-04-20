use crate::{TzifError, Version};

pub const TZIF_MAGIC: &[u8; 4] = b"TZif";
pub const HEADER_RESERVED_LEN: usize = 15;

#[derive(Clone, Copy)]
pub enum TimeSize {
    ThirtyTwo,
    SixtyFour,
}

impl TimeSize {
    pub const fn byte_len(self) -> usize {
        match self {
            Self::ThirtyTwo => 4,
            Self::SixtyFour => 8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub version: Version,
    pub isutcnt: usize,
    pub isstdcnt: usize,
    pub leapcnt: usize,
    pub timecnt: usize,
    pub typecnt: usize,
    pub charcnt: usize,
}

impl Header {
    pub fn data_block_len(self, time_size: TimeSize) -> Result<usize, TzifError> {
        let transition_times = checked_mul("transition_times", self.timecnt, time_size.byte_len())?;
        let transition_types = self.timecnt;
        let local_time_types = checked_mul("local_time_types", self.typecnt, 6)?;
        let designations = self.charcnt;
        let leap_seconds = checked_mul("leap_seconds", self.leapcnt, time_size.byte_len() + 4)?;
        checked_sum(
            "data_block",
            [
                transition_times,
                transition_types,
                local_time_types,
                designations,
                leap_seconds,
                self.isstdcnt,
                self.isutcnt,
            ],
        )
    }
}

fn checked_mul(field: &'static str, lhs: usize, rhs: usize) -> Result<usize, TzifError> {
    lhs.checked_mul(rhs)
        .ok_or(TzifError::DataBlockLengthOverflow { field })
}

fn checked_sum<const N: usize>(
    field: &'static str,
    values: [usize; N],
) -> Result<usize, TzifError> {
    values.into_iter().try_fold(0_usize, |sum, value| {
        sum.checked_add(value)
            .ok_or(TzifError::DataBlockLengthOverflow { field })
    })
}
