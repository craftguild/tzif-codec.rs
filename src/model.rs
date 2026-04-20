use crate::{validate::validate_file, TzifError};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Version {
    V1,
    V2,
    V3,
    V4,
}

impl Version {
    pub(crate) const fn from_byte(byte: u8) -> Result<Self, TzifError> {
        match byte {
            0 => Ok(Self::V1),
            b'2' => Ok(Self::V2),
            b'3' => Ok(Self::V3),
            b'4' => Ok(Self::V4),
            _ => Err(TzifError::InvalidVersion(byte)),
        }
    }

    pub(crate) const fn byte(self) -> u8 {
        match self {
            Self::V1 => 0,
            Self::V2 => b'2',
            Self::V3 => b'3',
            Self::V4 => b'4',
        }
    }

    pub(crate) const fn is_v2_plus(self) -> bool {
        !matches!(self, Self::V1)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TzifFile {
    pub version: Version,
    pub v1: DataBlock,
    pub v2_plus: Option<DataBlock>,
    pub footer: Option<String>,
}

impl TzifFile {
    #[must_use]
    pub const fn v1(block: DataBlock) -> Self {
        Self {
            version: Version::V1,
            v1: block,
            v2_plus: None,
            footer: None,
        }
    }

    pub fn v2(v1: DataBlock, v2: DataBlock, footer: impl Into<String>) -> Self {
        Self::v2_plus(Version::V2, v1, v2, footer)
    }

    pub fn v3(v1: DataBlock, v3: DataBlock, footer: impl Into<String>) -> Self {
        Self::v2_plus(Version::V3, v1, v3, footer)
    }

    pub fn v4(v1: DataBlock, v4: DataBlock, footer: impl Into<String>) -> Self {
        Self::v2_plus(Version::V4, v1, v4, footer)
    }

    /// Validates this file against the structural `TZif` rules implemented by this crate.
    ///
    /// # Errors
    ///
    /// Returns an error when the file contains invalid counts, indexes, version-specific
    /// data, footer content, or leap-second records.
    pub fn validate(&self) -> Result<(), TzifError> {
        validate_file(self)
    }

    fn v2_plus(
        version: Version,
        v1: DataBlock,
        block: DataBlock,
        footer: impl Into<String>,
    ) -> Self {
        Self {
            version,
            v1,
            v2_plus: Some(block),
            footer: Some(footer.into()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DataBlock {
    pub transition_times: Vec<i64>,
    pub transition_types: Vec<u8>,
    pub local_time_types: Vec<LocalTimeType>,
    pub designations: Vec<u8>,
    pub leap_seconds: Vec<LeapSecond>,
    pub standard_wall_indicators: Vec<bool>,
    pub ut_local_indicators: Vec<bool>,
}

impl DataBlock {
    pub fn new(local_time_types: Vec<LocalTimeType>, designations: impl Into<Vec<u8>>) -> Self {
        Self {
            local_time_types,
            designations: designations.into(),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn placeholder() -> Self {
        Self::new(
            vec![LocalTimeType {
                utc_offset: 0,
                is_dst: false,
                designation_index: 0,
            }],
            vec![0],
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalTimeType {
    pub utc_offset: i32,
    pub is_dst: bool,
    pub designation_index: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeapSecond {
    pub occurrence: i64,
    pub correction: i32,
}
