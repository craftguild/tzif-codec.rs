use crate::Version;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TzifBuildError {
    #[error("time zone designation must not be empty")]
    EmptyDesignation,
    #[error("time zone designation {designation:?} must be ASCII")]
    NonAsciiDesignation { designation: String },
    #[error("time zone designation {designation:?} contains unsupported character {character:?}")]
    UnsupportedDesignationCharacter {
        designation: String,
        character: char,
    },
    #[error("time zone designation {designation:?} must be at least 3 characters")]
    DesignationTooShort { designation: String },
    #[error("time zone designation {designation:?} must be at most 6 characters")]
    DesignationTooLong { designation: String },
    #[error("duplicate time zone designation {0:?}")]
    DuplicateDesignation(String),
    #[error("unknown time zone designation {0:?}")]
    UnknownDesignation(String),
    #[error("explicit transition times must be sorted in ascending order")]
    UnsortedTransitions,
    #[error("UTC offset -2^31 is not valid in TZif local time types")]
    InvalidUtcOffset,
    #[error("UTC offset {seconds} cannot be represented in a POSIX TZ footer")]
    PosixOffsetOutOfRange { seconds: i32 },
    #[error("TZif version {version:?} cannot represent transition timestamp {timestamp}")]
    TransitionOutOfRangeForVersion { version: Version, timestamp: i64 },
    #[error("TZif version {version:?} cannot include a footer")]
    VersionCannotIncludeFooter { version: Version },
    #[error("TZif version {version:?} cannot represent the POSIX TZ string extension")]
    VersionCannotRepresentFooterExtension { version: Version },
    #[error("POSIX TZ month {month} is outside the range 1..=12")]
    InvalidPosixMonth { month: u8 },
    #[error("POSIX TZ week {week} is outside the range 1..=5")]
    InvalidPosixWeek { week: u8 },
    #[error("POSIX TZ weekday {weekday} is outside the range 0..=6")]
    InvalidPosixWeekday { weekday: u8 },
    #[error("POSIX TZ Julian day {day} is outside the range 1..=365")]
    InvalidPosixJulianDay { day: u16 },
    #[error("POSIX TZ zero-based day {day} is outside the range 0..=365")]
    InvalidPosixZeroBasedDay { day: u16 },
    #[error("POSIX TZ transition time {seconds} seconds is outside the supported range")]
    InvalidPosixTransitionTime { seconds: i32 },
    #[error(transparent)]
    InvalidTzif(#[from] TzifError),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TzdistError {
    #[error(transparent)]
    InvalidTzif(#[from] TzifError),
    #[error("unsupported TZif media type {0}")]
    UnsupportedMediaType(String),
    #[error("TZDIST capabilities must advertise application/tzif when advertising application/tzif-leap")]
    TzifLeapCapabilityRequiresTzif,
    #[error("application/tzif MUST NOT contain leap-second records; use application/tzif-leap")]
    LeapSecondsNotAllowedForApplicationTzif,
    #[error("TZDIST truncation requires a version 2 or later TZif file")]
    TruncationRequiresVersion2Plus,
    #[error("TZDIST truncation requires at least one version 2+ transition")]
    TruncationRequiresVersion2PlusTransitions,
    #[error("start truncation transition mismatch: expected {expected}, got {actual}")]
    StartTruncationTransitionMismatch { expected: i64, actual: i64 },
    #[error("start truncation requires time type 0 to be a -00 placeholder")]
    StartTruncationTypeZeroMustBePlaceholder,
    #[error("end truncation transition mismatch: expected {expected}, got {actual}")]
    EndTruncationTransitionMismatch { expected: i64, actual: i64 },
    #[error("end truncation requires an empty TZ string footer")]
    EndTruncationRequiresEmptyFooter,
    #[error("end truncation requires the last transition type to be a -00 placeholder")]
    EndTruncationLastTypeMustBePlaceholder,
    #[error("invalid TZDIST truncation range: start {start} must be before end {end}")]
    InvalidTruncationRange { start: i64, end: i64 },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum TzifError {
    #[error("input ended unexpectedly at byte {offset} while reading {context}")]
    UnexpectedEof {
        offset: usize,
        context: &'static str,
    },
    #[error("expected TZif magic at byte {offset}")]
    InvalidMagic { offset: usize },
    #[error("invalid TZif version byte 0x{0:02x}")]
    InvalidVersion(u8),
    #[error("version mismatch between first header {first:?} and second header {second:?}")]
    VersionMismatch { first: Version, second: Version },
    #[error("expected newline at byte {offset} before TZif footer")]
    MissingFooterStart { offset: usize },
    #[error("missing newline terminator for TZif footer starting at byte {offset}")]
    MissingFooterEnd { offset: usize },
    #[error("TZif footer is not valid UTF-8")]
    InvalidFooterUtf8,
    #[error("trailing data starts at byte {offset}")]
    TrailingData { offset: usize },
    #[error("{field} count {count} cannot fit in memory on this platform")]
    CountTooLarge { field: &'static str, count: u32 },
    #[error("data block byte length overflow while calculating {field}")]
    DataBlockLengthOverflow { field: &'static str },
    #[error("local time type {index} has invalid isdst value {value}")]
    InvalidDstIndicator { index: usize, value: u8 },
    #[error("{field} indicator {index} has invalid value {value}")]
    InvalidBooleanIndicator {
        field: &'static str,
        index: usize,
        value: u8,
    },
    #[error("version 1 files must not include a v2+ data block or footer")]
    UnexpectedV2PlusData,
    #[error("version {0:?} files must include a v2+ data block and footer")]
    MissingV2PlusData(Version),
    #[error("{field} has {actual} entries, but expected {expected}")]
    CountMismatch {
        field: &'static str,
        expected: usize,
        actual: usize,
    },
    #[error("{field} count {count} exceeds TZif u32 count range")]
    CountOverflow { field: &'static str, count: usize },
    #[error("local time type count must not be zero")]
    EmptyLocalTimeTypes,
    #[error("designation table must not be empty")]
    EmptyDesignations,
    #[error("local time type count {0} exceeds 256")]
    TooManyLocalTimeTypes(usize),
    #[error(
        "transition type {transition_type} at index {index} does not reference a local time type"
    )]
    InvalidTransitionType { index: usize, transition_type: u8 },
    #[error("designation index {designation_index} at local time type {index} is out of range")]
    InvalidDesignationIndex { index: usize, designation_index: u8 },
    #[error("transition time {value} at index {index} is outside the version 1 i32 range")]
    Version1TransitionOutOfRange { index: usize, value: i64 },
    #[error("leap-second occurrence {value} at index {index} is outside the version 1 i32 range")]
    Version1LeapSecondOutOfRange { index: usize, value: i64 },
    #[error("transition time at index {index} is not strictly ascending")]
    TransitionTimesNotAscending { index: usize },
    #[error(
        "designation index {designation_index} at local time type {index} has no NUL terminator"
    )]
    UnterminatedDesignation { index: usize, designation_index: u8 },
    #[error("local time type {index} has invalid UTC offset -2^31")]
    InvalidUtcOffset { index: usize },
    #[error("time zone designation at local time type {index} violates RFC 9636 designation requirements")]
    InvalidDesignation { index: usize, designation: Vec<u8> },
    #[error("UT/local indicator {index} is set without the corresponding standard/wall indicator")]
    InvalidUtLocalIndicatorCombination { index: usize },
    #[error("first leap-second occurrence {value} at index 0 must be non-negative")]
    FirstLeapSecondOccurrenceNegative { value: i64 },
    #[error("leap-second occurrence at index {index} is not strictly ascending")]
    LeapSecondOccurrencesNotAscending { index: usize },
    #[error("first leap-second correction {correction} must be +1 or -1 unless using version 4 truncation")]
    InvalidFirstLeapSecondCorrection { correction: i32 },
    #[error("leap-second correction at index {index} must differ from the previous correction by +1 or -1")]
    InvalidLeapSecondCorrection { index: usize },
    #[error("version {version:?} cannot contain a leap-second table truncated at the start")]
    LeapSecondTruncationRequiresVersion4 { version: Version },
    #[error("version {version:?} cannot contain a leap-second table expiration time")]
    LeapSecondExpirationRequiresVersion4 { version: Version },
    #[error("leap-second occurrence at index {index} does not occur at a UTC month boundary")]
    LeapSecondOccurrenceNotAtMonthEnd { index: usize },
    #[error("TZif footer must be ASCII")]
    InvalidFooterAscii,
    #[error("TZif footer must not contain NUL or newline bytes")]
    InvalidFooterControlByte,
    #[error("TZif version {version:?} cannot use the POSIX TZ string extension")]
    FooterExtensionRequiresVersion3 { version: Version },
    #[error("TZif footer is not a valid POSIX TZ string")]
    InvalidFooterSyntax,
    #[error("TZif footer is inconsistent with the last transition type")]
    FooterInconsistentWithLastTransition,
}
