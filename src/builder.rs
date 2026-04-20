use std::collections::{BTreeMap, BTreeSet};

use crate::{
    footer::footer_uses_tz_string_extension, DataBlock, LocalTimeType, TzifBuildError, TzifFile,
    Version,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VersionPolicy {
    #[default]
    Auto,
    Exact(Version),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PosixFooter {
    standard_designation: String,
    standard_offset_seconds: i32,
    daylight: Option<PosixDaylight>,
}

impl PosixFooter {
    #[must_use]
    pub fn fixed(designation: impl Into<String>, offset_seconds: i32) -> Self {
        Self {
            standard_designation: designation.into(),
            standard_offset_seconds: offset_seconds,
            daylight: None,
        }
    }

    #[must_use]
    #[expect(
        clippy::too_many_arguments,
        reason = "a POSIX daylight-saving footer is defined by these six fields"
    )]
    pub fn daylight_saving(
        standard_designation: impl Into<String>,
        standard_offset_seconds: i32,
        daylight_designation: impl Into<String>,
        daylight_offset_seconds: i32,
        start: PosixTransitionRule,
        end: PosixTransitionRule,
    ) -> Self {
        Self {
            standard_designation: standard_designation.into(),
            standard_offset_seconds,
            daylight: Some(PosixDaylight {
                designation: daylight_designation.into(),
                offset_seconds: daylight_offset_seconds,
                start,
                end,
                start_time: PosixTransitionTime::DEFAULT,
                end_time: PosixTransitionTime::DEFAULT,
            }),
        }
    }

    #[must_use]
    pub const fn start_time(mut self, time: PosixTransitionTime) -> Self {
        if let Some(daylight) = &mut self.daylight {
            daylight.start_time = time;
        }
        self
    }

    #[must_use]
    pub const fn end_time(mut self, time: PosixTransitionTime) -> Self {
        if let Some(daylight) = &mut self.daylight {
            daylight.end_time = time;
        }
        self
    }

    fn to_tz_string(&self, strict_designation: bool) -> Result<String, TzifBuildError> {
        validate_designation(&self.standard_designation, strict_designation)?;
        validate_posix_offset(self.standard_offset_seconds)?;
        let mut value = format!(
            "{}{}",
            posix_designation(&self.standard_designation),
            posix_offset(self.standard_offset_seconds)?
        );
        let Some(daylight) = &self.daylight else {
            return Ok(value);
        };
        validate_designation(&daylight.designation, strict_designation)?;
        validate_utc_offset(daylight.offset_seconds)?;
        daylight.start.validate()?;
        daylight.end.validate()?;
        daylight.start_time.validate()?;
        daylight.end_time.validate()?;

        value.push_str(&posix_designation(&daylight.designation));
        if Some(daylight.offset_seconds) != self.standard_offset_seconds.checked_add(3600) {
            validate_posix_offset(daylight.offset_seconds)?;
            value.push_str(&posix_offset(daylight.offset_seconds)?);
        }
        value.push(',');
        value.push_str(&daylight.start.to_tz_string());
        value.push_str(&daylight.start_time.to_tz_suffix());
        value.push(',');
        value.push_str(&daylight.end.to_tz_string());
        value.push_str(&daylight.end_time.to_tz_suffix());
        Ok(value)
    }

    fn uses_tz_string_extension(&self) -> bool {
        self.daylight.as_ref().is_some_and(|daylight| {
            daylight.start_time.uses_tz_string_extension()
                || daylight.end_time.uses_tz_string_extension()
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PosixDaylight {
    designation: String,
    offset_seconds: i32,
    start: PosixTransitionRule,
    end: PosixTransitionRule,
    start_time: PosixTransitionTime,
    end_time: PosixTransitionTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PosixTransitionRule {
    JulianWithoutLeapDay { day: u16 },
    ZeroBasedDay { day: u16 },
    MonthWeekday { month: u8, week: u8, weekday: u8 },
}

impl PosixTransitionRule {
    #[must_use]
    pub const fn julian_without_leap_day(day: u16) -> Self {
        Self::JulianWithoutLeapDay { day }
    }

    #[must_use]
    pub const fn zero_based_day(day: u16) -> Self {
        Self::ZeroBasedDay { day }
    }

    #[must_use]
    pub const fn month_weekday(month: u8, week: u8, weekday: u8) -> Self {
        Self::MonthWeekday {
            month,
            week,
            weekday,
        }
    }

    fn validate(self) -> Result<(), TzifBuildError> {
        match self {
            Self::JulianWithoutLeapDay { day } if !(1..=365).contains(&day) => {
                Err(TzifBuildError::InvalidPosixJulianDay { day })
            }
            Self::ZeroBasedDay { day } if day > 365 => {
                Err(TzifBuildError::InvalidPosixZeroBasedDay { day })
            }
            Self::MonthWeekday { month, .. } if !(1..=12).contains(&month) => {
                Err(TzifBuildError::InvalidPosixMonth { month })
            }
            Self::MonthWeekday { week, .. } if !(1..=5).contains(&week) => {
                Err(TzifBuildError::InvalidPosixWeek { week })
            }
            Self::MonthWeekday { weekday, .. } if weekday > 6 => {
                Err(TzifBuildError::InvalidPosixWeekday { weekday })
            }
            _ => Ok(()),
        }
    }

    fn to_tz_string(self) -> String {
        match self {
            Self::JulianWithoutLeapDay { day } => format!("J{day}"),
            Self::ZeroBasedDay { day } => day.to_string(),
            Self::MonthWeekday {
                month,
                week,
                weekday,
            } => format!("M{month}.{week}.{weekday}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PosixTransitionTime {
    seconds: i32,
}

impl PosixTransitionTime {
    const DEFAULT: Self = Self { seconds: 2 * 3600 };
    const MIN_SECONDS: i32 = -167 * 3600;
    const MAX_SECONDS: i32 = 167 * 3600;

    #[must_use]
    pub const fn seconds(seconds: i32) -> Self {
        Self { seconds }
    }

    #[must_use]
    pub fn hms(hours: i32, minutes: u8, seconds: u8) -> Self {
        if minutes > 59 || seconds > 59 {
            return Self { seconds: i32::MAX };
        }
        let sign = if hours < 0 { -1 } else { 1 };
        let seconds = hours
            .checked_mul(3600)
            .and_then(|value| value.checked_add(sign * i32::from(minutes) * 60))
            .and_then(|value| value.checked_add(sign * i32::from(seconds)))
            .unwrap_or(i32::MAX);
        Self { seconds }
    }

    fn validate(self) -> Result<(), TzifBuildError> {
        if !(Self::MIN_SECONDS..=Self::MAX_SECONDS).contains(&self.seconds) {
            return Err(TzifBuildError::InvalidPosixTransitionTime {
                seconds: self.seconds,
            });
        }
        Ok(())
    }

    fn to_tz_suffix(self) -> String {
        if self == Self::DEFAULT {
            String::new()
        } else {
            format!("/{}", posix_time(self.seconds))
        }
    }

    const fn uses_tz_string_extension(self) -> bool {
        self.seconds < 0 || self.seconds / 3600 > 24
    }
}

pub struct TzifBuilder;

impl TzifBuilder {
    #[must_use]
    pub fn fixed_offset(designation: impl Into<String>, offset_seconds: i32) -> FixedOffsetBuilder {
        FixedOffsetBuilder {
            designation: designation.into(),
            offset_seconds,
            version_policy: VersionPolicy::Auto,
        }
    }

    #[must_use]
    pub const fn transitions() -> ExplicitTransitionsBuilder {
        ExplicitTransitionsBuilder::new()
    }
}

#[derive(Clone, Debug)]
pub struct FixedOffsetBuilder {
    designation: String,
    offset_seconds: i32,
    version_policy: VersionPolicy,
}

impl FixedOffsetBuilder {
    #[must_use]
    pub const fn version_policy(mut self, version_policy: VersionPolicy) -> Self {
        self.version_policy = version_policy;
        self
    }

    #[must_use]
    pub const fn version(mut self, version: Version) -> Self {
        self.version_policy = VersionPolicy::Exact(version);
        self
    }

    /// Builds a `TZif` file for a fixed-offset zone.
    ///
    /// # Errors
    ///
    /// Returns an error if the designation, UTC offset, requested version, or generated
    /// POSIX footer cannot be represented as valid `TZif`.
    pub fn build(self) -> Result<TzifFile, TzifBuildError> {
        let has_footer = !matches!(self.version_policy, VersionPolicy::Exact(Version::V1));
        validate_designation(&self.designation, true)?;
        validate_utc_offset(self.offset_seconds)?;
        let block = DataBlock::new(
            vec![LocalTimeType {
                utc_offset: self.offset_seconds,
                is_dst: false,
                designation_index: 0,
            }],
            designation_table([self.designation.as_str()]),
        );
        let version = resolve_version(self.version_policy, &[], has_footer, false)?;
        Ok(match version {
            Version::V1 => TzifFile::v1(block),
            Version::V2 => TzifFile::v2(
                block.clone(),
                block,
                fixed_offset_footer(&self.designation, self.offset_seconds)?,
            ),
            Version::V3 => TzifFile::v3(
                block.clone(),
                block,
                fixed_offset_footer(&self.designation, self.offset_seconds)?,
            ),
            Version::V4 => TzifFile::v4(
                block.clone(),
                block,
                fixed_offset_footer(&self.designation, self.offset_seconds)?,
            ),
        })
    }
}

#[derive(Clone, Debug)]
pub struct ExplicitTransitionsBuilder {
    designations: Vec<String>,
    local_time_types: Vec<PendingLocalTimeType>,
    transitions: Vec<PendingTransition>,
    footer: Option<PendingFooter>,
    version_policy: VersionPolicy,
}

impl ExplicitTransitionsBuilder {
    const fn new() -> Self {
        Self {
            designations: Vec::new(),
            local_time_types: Vec::new(),
            transitions: Vec::new(),
            footer: None,
            version_policy: VersionPolicy::Auto,
        }
    }

    #[must_use]
    pub fn designation(mut self, designation: impl Into<String>) -> Self {
        self.designations.push(designation.into());
        self
    }

    #[must_use]
    pub fn local_time_type(
        mut self,
        designation: impl Into<String>,
        offset_seconds: i32,
        is_dst: bool,
    ) -> Self {
        self.local_time_types.push(PendingLocalTimeType {
            designation: designation.into(),
            offset_seconds,
            is_dst,
        });
        self
    }

    #[must_use]
    pub fn transition(mut self, timestamp: i64, designation: impl Into<String>) -> Self {
        self.transitions.push(PendingTransition {
            timestamp,
            designation: designation.into(),
        });
        self
    }

    #[must_use]
    pub fn footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(PendingFooter::Raw(footer.into()));
        self
    }

    #[must_use]
    pub fn posix_footer(mut self, footer: PosixFooter) -> Self {
        self.footer = Some(PendingFooter::Posix(footer));
        self
    }

    #[must_use]
    pub const fn version_policy(mut self, version_policy: VersionPolicy) -> Self {
        self.version_policy = version_policy;
        self
    }

    #[must_use]
    pub const fn version(mut self, version: Version) -> Self {
        self.version_policy = VersionPolicy::Exact(version);
        self
    }

    /// Builds a `TZif` file from explicitly supplied transitions and local time types.
    ///
    /// # Errors
    ///
    /// Returns an error if designations are invalid or duplicated, transitions are not
    /// strictly ascending, referenced local time types are missing, or the requested
    /// version cannot represent the supplied data.
    pub fn build(self) -> Result<TzifFile, TzifBuildError> {
        let designations = self.normalized_designations()?;
        let designation_indexes = designation_indexes(&designations)?;
        let local_time_types = self.build_local_time_types(&designation_indexes)?;
        let type_indexes = local_time_type_indexes(&self.local_time_types)?;
        let transitions = self.build_transitions(&type_indexes)?;
        let transition_times: Vec<i64> = transitions
            .iter()
            .map(|transition| transition.timestamp)
            .collect();
        let transition_types: Vec<u8> = transitions
            .iter()
            .map(|transition| transition.local_time_type_index)
            .collect();
        let block = DataBlock {
            transition_times,
            transition_types,
            local_time_types,
            designations: designation_table(designations.iter().map(String::as_str)),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        };
        let footer = self
            .footer
            .map(|footer| {
                Ok::<_, TzifBuildError>(BuiltFooter {
                    value: footer.to_tz_string(true)?,
                    uses_tz_string_extension: footer.uses_tz_string_extension(),
                })
            })
            .transpose()?;
        let version = resolve_version(
            self.version_policy,
            &block.transition_times,
            footer.is_some(),
            footer
                .as_ref()
                .is_some_and(|footer| footer.uses_tz_string_extension),
        )?;
        Ok(match version {
            Version::V1 => {
                if footer.is_some() {
                    return Err(TzifBuildError::VersionCannotIncludeFooter { version });
                }
                TzifFile::v1(block)
            }
            Version::V2 => TzifFile::v2(
                version_one_compatible_block(&block),
                block,
                footer.map(|footer| footer.value).unwrap_or_default(),
            ),
            Version::V3 => TzifFile::v3(
                version_one_compatible_block(&block),
                block,
                footer.map(|footer| footer.value).unwrap_or_default(),
            ),
            Version::V4 => TzifFile::v4(
                version_one_compatible_block(&block),
                block,
                footer.map(|footer| footer.value).unwrap_or_default(),
            ),
        })
    }

    fn normalized_designations(&self) -> Result<Vec<String>, TzifBuildError> {
        let mut values = self.designations.clone();
        for local_time_type in &self.local_time_types {
            if !values.contains(&local_time_type.designation) {
                values.push(local_time_type.designation.clone());
            }
        }
        for designation in &values {
            validate_designation(designation, true)?;
        }
        let mut seen = BTreeSet::new();
        for designation in &values {
            if !seen.insert(designation.clone()) {
                return Err(TzifBuildError::DuplicateDesignation(designation.clone()));
            }
        }
        Ok(values)
    }

    fn build_local_time_types(
        &self,
        designation_indexes: &BTreeMap<String, u8>,
    ) -> Result<Vec<LocalTimeType>, TzifBuildError> {
        if self.local_time_types.is_empty() {
            return Err(TzifBuildError::UnknownDesignation(
                "local time type".to_string(),
            ));
        }
        self.local_time_types
            .iter()
            .map(|local_time_type| {
                let designation_index = *designation_indexes
                    .get(&local_time_type.designation)
                    .ok_or_else(|| {
                        TzifBuildError::UnknownDesignation(local_time_type.designation.clone())
                    })?;
                validate_utc_offset(local_time_type.offset_seconds)?;
                Ok(LocalTimeType {
                    utc_offset: local_time_type.offset_seconds,
                    is_dst: local_time_type.is_dst,
                    designation_index,
                })
            })
            .collect()
    }

    fn build_transitions(
        &self,
        type_indexes: &BTreeMap<String, u8>,
    ) -> Result<Vec<BuiltTransition>, TzifBuildError> {
        let mut previous = None;
        let mut values = Vec::with_capacity(self.transitions.len());
        for transition in &self.transitions {
            if previous.is_some_and(|previous| transition.timestamp <= previous) {
                return Err(TzifBuildError::UnsortedTransitions);
            }
            previous = Some(transition.timestamp);
            let local_time_type_index =
                *type_indexes.get(&transition.designation).ok_or_else(|| {
                    TzifBuildError::UnknownDesignation(transition.designation.clone())
                })?;
            values.push(BuiltTransition {
                timestamp: transition.timestamp,
                local_time_type_index,
            });
        }
        Ok(values)
    }
}

#[derive(Clone, Debug)]
struct PendingLocalTimeType {
    designation: String,
    offset_seconds: i32,
    is_dst: bool,
}

#[derive(Clone, Debug)]
struct PendingTransition {
    timestamp: i64,
    designation: String,
}

#[derive(Clone, Copy, Debug)]
struct BuiltTransition {
    timestamp: i64,
    local_time_type_index: u8,
}

#[derive(Clone, Debug)]
enum PendingFooter {
    Raw(String),
    Posix(PosixFooter),
}

#[derive(Clone, Debug)]
struct BuiltFooter {
    value: String,
    uses_tz_string_extension: bool,
}

impl PendingFooter {
    fn to_tz_string(&self, strict_designation: bool) -> Result<String, TzifBuildError> {
        match self {
            Self::Raw(value) => Ok(value.clone()),
            Self::Posix(footer) => footer.to_tz_string(strict_designation),
        }
    }

    fn uses_tz_string_extension(&self) -> bool {
        match self {
            Self::Raw(value) => footer_uses_tz_string_extension(value),
            Self::Posix(footer) => footer.uses_tz_string_extension(),
        }
    }
}

fn validate_designation(designation: &str, strict: bool) -> Result<(), TzifBuildError> {
    if designation.is_empty() {
        return Err(TzifBuildError::EmptyDesignation);
    }
    if !designation.is_ascii() {
        return Err(TzifBuildError::NonAsciiDesignation {
            designation: designation.to_string(),
        });
    }
    for character in designation.chars() {
        if !(character.is_ascii_alphanumeric() || character == '+' || character == '-') {
            return Err(TzifBuildError::UnsupportedDesignationCharacter {
                designation: designation.to_string(),
                character,
            });
        }
    }
    if strict && designation.len() < 3 {
        return Err(TzifBuildError::DesignationTooShort {
            designation: designation.to_string(),
        });
    }
    if strict && designation.len() > 6 {
        return Err(TzifBuildError::DesignationTooLong {
            designation: designation.to_string(),
        });
    }
    Ok(())
}

const fn validate_utc_offset(offset_seconds: i32) -> Result<(), TzifBuildError> {
    if offset_seconds == i32::MIN {
        return Err(TzifBuildError::InvalidUtcOffset);
    }
    Ok(())
}

fn validate_posix_offset(offset_seconds: i32) -> Result<(), TzifBuildError> {
    validate_utc_offset(offset_seconds)?;
    let seconds = offset_seconds
        .checked_neg()
        .ok_or(TzifBuildError::InvalidUtcOffset)?;
    if seconds.abs() > 24 * 3600 + 59 * 60 + 59 {
        return Err(TzifBuildError::PosixOffsetOutOfRange {
            seconds: offset_seconds,
        });
    }
    Ok(())
}

fn designation_table<'a>(designations: impl IntoIterator<Item = &'a str>) -> Vec<u8> {
    let mut bytes = Vec::new();
    for designation in designations {
        bytes.extend_from_slice(designation.as_bytes());
        bytes.push(0);
    }
    bytes
}

fn designation_indexes(designations: &[String]) -> Result<BTreeMap<String, u8>, TzifBuildError> {
    let mut indexes = BTreeMap::new();
    let mut next = 0usize;
    for designation in designations {
        let index = u8::try_from(next).map_err(|_| {
            TzifBuildError::InvalidTzif(crate::TzifError::CountOverflow {
                field: "charcnt",
                count: next,
            })
        })?;
        indexes.insert(designation.clone(), index);
        next += designation.len() + 1;
    }
    Ok(indexes)
}

fn local_time_type_indexes(
    local_time_types: &[PendingLocalTimeType],
) -> Result<BTreeMap<String, u8>, TzifBuildError> {
    let mut indexes = BTreeMap::new();
    for (index, local_time_type) in local_time_types.iter().enumerate() {
        if indexes.contains_key(&local_time_type.designation) {
            return Err(TzifBuildError::DuplicateDesignation(
                local_time_type.designation.clone(),
            ));
        }
        let index = u8::try_from(index).map_err(|_| {
            TzifBuildError::InvalidTzif(crate::TzifError::TooManyLocalTimeTypes(index))
        })?;
        indexes.insert(local_time_type.designation.clone(), index);
    }
    Ok(indexes)
}

fn resolve_version(
    policy: VersionPolicy,
    transition_times: &[i64],
    has_footer: bool,
    footer_uses_tz_string_extension: bool,
) -> Result<Version, TzifBuildError> {
    let auto = if footer_uses_tz_string_extension {
        Version::V3
    } else {
        Version::V2
    };
    let version = match policy {
        VersionPolicy::Auto => auto,
        VersionPolicy::Exact(version) => version,
    };
    if version == Version::V1 && has_footer {
        return Err(TzifBuildError::VersionCannotIncludeFooter { version });
    }
    if version < Version::V3 && footer_uses_tz_string_extension {
        return Err(TzifBuildError::VersionCannotRepresentFooterExtension { version });
    }
    if version == Version::V1 {
        for &timestamp in transition_times {
            if i32::try_from(timestamp).is_err() {
                return Err(TzifBuildError::TransitionOutOfRangeForVersion { version, timestamp });
            }
        }
    }
    Ok(version)
}

fn version_one_compatible_block(block: &DataBlock) -> DataBlock {
    let mut v1 = DataBlock {
        transition_times: Vec::new(),
        transition_types: Vec::new(),
        local_time_types: block.local_time_types.clone(),
        designations: block.designations.clone(),
        leap_seconds: block
            .leap_seconds
            .iter()
            .copied()
            .filter(|leap_second| i32::try_from(leap_second.occurrence).is_ok())
            .collect(),
        standard_wall_indicators: block.standard_wall_indicators.clone(),
        ut_local_indicators: block.ut_local_indicators.clone(),
    };

    for (&transition_time, &transition_type) in block
        .transition_times
        .iter()
        .zip(block.transition_types.iter())
    {
        if i32::try_from(transition_time).is_ok() {
            v1.transition_times.push(transition_time);
            v1.transition_types.push(transition_type);
        }
    }

    v1
}

fn fixed_offset_footer(designation: &str, offset_seconds: i32) -> Result<String, TzifBuildError> {
    validate_posix_offset(offset_seconds)?;
    Ok(format!(
        "{}{}",
        posix_designation(designation),
        posix_offset(offset_seconds)?
    ))
}

fn posix_designation(designation: &str) -> String {
    if designation.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        designation.to_string()
    } else {
        format!("<{designation}>")
    }
}

fn posix_offset(offset_seconds: i32) -> Result<String, TzifBuildError> {
    validate_posix_offset(offset_seconds)?;
    let seconds = offset_seconds
        .checked_neg()
        .ok_or(TzifBuildError::InvalidUtcOffset)?;
    Ok(posix_duration(seconds))
}

fn posix_time(seconds: i32) -> String {
    posix_duration(seconds)
}

fn posix_duration(seconds: i32) -> String {
    let sign = if seconds < 0 { "-" } else { "" };
    let seconds = seconds.abs();
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if seconds != 0 {
        format!("{sign}{hours}:{minutes:02}:{seconds:02}")
    } else if minutes != 0 {
        format!("{sign}{hours}:{minutes:02}")
    } else {
        format!("{sign}{hours}")
    }
}
