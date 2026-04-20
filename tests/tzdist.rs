mod common;

use common::{AssertErr, AssertOk, TestResult};
use tzif_codec::{
    validate_tzdist_capability_formats, DataBlock, LeapSecond, LocalTimeType, TzdistError,
    TzdistTruncation, TzifError, TzifFile, TzifMediaType,
};

#[test]
fn validates_tzdist_capability_formats() -> TestResult {
    validate_tzdist_capability_formats([TzifMediaType::APPLICATION_TZIF]).assert_ok()?;
    validate_tzdist_capability_formats([
        TzifMediaType::APPLICATION_TZIF,
        TzifMediaType::APPLICATION_TZIF_LEAP,
    ])
    .assert_ok()?;
    validate_tzdist_capability_formats([
        TzifMediaType::APPLICATION_TZIF,
        TzifMediaType::APPLICATION_TZIF_LEAP,
    ])
    .assert_ok()?;

    let err =
        validate_tzdist_capability_formats([TzifMediaType::APPLICATION_TZIF_LEAP]).assert_err()?;
    assert_eq!(err, TzdistError::TzifLeapCapabilityRequiresTzif);

    Ok(())
}

#[test]
fn parses_known_tzif_media_types() -> TestResult {
    assert_eq!(
        TzifMediaType::try_from(TzifMediaType::APPLICATION_TZIF).assert_ok()?,
        TzifMediaType::Tzif
    );
    assert_eq!(
        TzifMediaType::try_from(TzifMediaType::APPLICATION_TZIF_LEAP).assert_ok()?,
        TzifMediaType::TzifLeap
    );
    assert_eq!(
        TzifMediaType::Tzif.as_str(),
        TzifMediaType::APPLICATION_TZIF
    );
    assert_eq!(
        TzifMediaType::TzifLeap.as_str(),
        TzifMediaType::APPLICATION_TZIF_LEAP
    );

    let err = TzifMediaType::try_from("text/calendar").assert_err()?;
    assert_eq!(
        err,
        TzdistError::UnsupportedMediaType("text/calendar".to_string())
    );

    Ok(())
}

#[test]
fn validates_media_type_against_leap_second_records() -> TestResult {
    let without_leaps = TzifFile::v1(DataBlock::new(vec![ltt(0, false, 0)], b"UTC\0".to_vec()));
    without_leaps
        .validate_for_media_type(TzifMediaType::Tzif)
        .assert_ok()?;
    assert_eq!(
        without_leaps.suggested_media_type(),
        TzifMediaType::APPLICATION_TZIF
    );

    let with_leaps = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![LeapSecond {
            occurrence: 78_796_800,
            correction: 1,
        }],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    });
    assert_eq!(
        with_leaps.suggested_media_type(),
        TzifMediaType::APPLICATION_TZIF_LEAP
    );
    assert_eq!(
        with_leaps
            .validate_for_media_type(TzifMediaType::Tzif)
            .assert_err()?,
        TzdistError::LeapSecondsNotAllowedForApplicationTzif
    );
    with_leaps
        .validate_for_media_type(TzifMediaType::TzifLeap)
        .assert_ok()?;

    Ok(())
}

#[test]
fn validates_start_truncated_tzif_shape() -> TestResult {
    let file = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![100],
            transition_types: vec![1],
            local_time_types: vec![ltt(0, false, 0), ltt(3_600, false, 4)],
            designations: b"-00\0STD\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "",
    );

    file.validate_tzdist_truncation(TzdistTruncation::start(100))
        .assert_ok()?;
    assert_eq!(
        file.validate_tzdist_truncation(TzdistTruncation::start(99))
            .assert_err()?,
        TzdistError::StartTruncationTransitionMismatch {
            expected: 99,
            actual: 100,
        }
    );

    Ok(())
}

#[test]
fn start_truncation_requires_type_zero_placeholder() -> TestResult {
    let file = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![100],
            transition_types: vec![0],
            local_time_types: vec![ltt(0, false, 0)],
            designations: b"UTC\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "",
    );

    assert_eq!(
        file.validate_tzdist_truncation(TzdistTruncation::start(100))
            .assert_err()?,
        TzdistError::StartTruncationTypeZeroMustBePlaceholder
    );

    Ok(())
}

#[test]
fn validates_end_truncated_tzif_shape() -> TestResult {
    let file = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![100, 200],
            transition_types: vec![1, 0],
            local_time_types: vec![ltt(0, false, 0), ltt(3_600, false, 4)],
            designations: b"-00\0STD\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "",
    );

    file.validate_tzdist_truncation(TzdistTruncation::end(200))
        .assert_ok()?;
    assert_eq!(
        file.validate_tzdist_truncation(TzdistTruncation::end(201))
            .assert_err()?,
        TzdistError::EndTruncationTransitionMismatch {
            expected: 201,
            actual: 200,
        }
    );

    Ok(())
}

#[test]
fn end_truncation_requires_empty_footer_and_last_placeholder() -> TestResult {
    let non_empty_footer = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![200],
            transition_types: vec![0],
            local_time_types: vec![ltt(0, false, 0)],
            designations: b"-00\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "UTC0",
    );
    assert_eq!(
        non_empty_footer
            .validate_tzdist_truncation(TzdistTruncation::end(200))
            .assert_err()?,
        TzdistError::EndTruncationRequiresEmptyFooter
    );

    let non_placeholder_last = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![200],
            transition_types: vec![0],
            local_time_types: vec![ltt(0, false, 0)],
            designations: b"UTC\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "",
    );
    assert_eq!(
        non_placeholder_last
            .validate_tzdist_truncation(TzdistTruncation::end(200))
            .assert_err()?,
        TzdistError::EndTruncationLastTypeMustBePlaceholder
    );

    Ok(())
}

#[test]
fn truncation_requires_v2_plus_transitions_and_valid_range() -> TestResult {
    let v1 = TzifFile::v1(DataBlock::placeholder());
    assert_eq!(
        v1.validate_tzdist_truncation(TzdistTruncation::start(0))
            .assert_err()?,
        TzdistError::TruncationRequiresVersion2Plus
    );

    let without_transitions = TzifFile::v2(DataBlock::placeholder(), DataBlock::placeholder(), "");
    assert_eq!(
        without_transitions
            .validate_tzdist_truncation(TzdistTruncation::start(0))
            .assert_err()?,
        TzdistError::TruncationRequiresVersion2PlusTransitions
    );

    let invalid_range = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![100],
            transition_types: vec![0],
            local_time_types: vec![ltt(0, false, 0)],
            designations: b"-00\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "",
    );
    assert_eq!(
        invalid_range
            .validate_tzdist_truncation(TzdistTruncation::range(100, 100))
            .assert_err()?,
        TzdistError::InvalidTruncationRange {
            start: 100,
            end: 100,
        }
    );

    Ok(())
}

#[test]
fn tzdist_validation_wraps_invalid_tzif_errors() -> TestResult {
    let invalid = TzifFile::v1(DataBlock {
        transition_times: vec![0],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    });

    let err = invalid
        .validate_for_media_type(TzifMediaType::Tzif)
        .assert_err()?;
    assert!(matches!(
        err,
        TzdistError::InvalidTzif(TzifError::CountMismatch {
            field: "transition_types",
            expected: 1,
            actual: 0,
        })
    ));

    Ok(())
}

const fn ltt(utc_offset: i32, is_dst: bool, designation_index: u8) -> LocalTimeType {
    LocalTimeType {
        utc_offset,
        is_dst,
        designation_index,
    }
}
