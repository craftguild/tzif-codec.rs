use crate::common::{AssertErr, AssertOk, TestResult};
use tzif_codec::{DataBlock, TzifError, TzifFile};

use super::support::ltt;

#[test]
fn public_validate_checks_tzif_without_encoding() -> TestResult {
    let valid = TzifFile::v1(DataBlock::placeholder());
    assert!(valid.validate().is_ok());

    let invalid = TzifFile::v1(DataBlock {
        transition_times: vec![0],
        transition_types: vec![1],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    });
    let err = invalid.validate().assert_err()?;

    assert!(matches!(
        err,
        TzifError::InvalidTransitionType {
            index: 0,
            transition_type: 1
        }
    ));

    Ok(())
}

#[test]
fn writer_rejects_zero_typecnt_and_charcnt() -> TestResult {
    let missing_type = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .to_bytes()
    .assert_err()?;
    assert!(matches!(missing_type, TzifError::EmptyLocalTimeTypes));

    let missing_designation = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: vec![],
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .to_bytes()
    .assert_err()?;
    assert!(matches!(missing_designation, TzifError::EmptyDesignations));

    Ok(())
}

#[test]
fn writer_rejects_indicator_counts_that_are_neither_zero_nor_typecnt() -> TestResult {
    let standard_wall_mismatch = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0), ltt(3600, false, 4)],
        designations: b"UTC\0ONE\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![true],
        ut_local_indicators: vec![],
    })
    .to_bytes()
    .assert_err()?;
    assert!(matches!(
        standard_wall_mismatch,
        TzifError::CountMismatch {
            field: "standard_wall_indicators",
            expected: 2,
            actual: 1
        }
    ));

    let ut_local_mismatch = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0), ltt(3600, false, 4)],
        designations: b"UTC\0ONE\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![true],
    })
    .to_bytes()
    .assert_err()?;
    assert!(matches!(
        ut_local_mismatch,
        TzifError::CountMismatch {
            field: "ut_local_indicators",
            expected: 2,
            actual: 1
        }
    ));

    Ok(())
}

#[test]
fn writer_rejects_transition_type_indexes_outside_local_time_types() -> TestResult {
    let err = TzifFile::v1(DataBlock {
        transition_times: vec![0],
        transition_types: vec![1],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .to_bytes()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::InvalidTransitionType {
            index: 0,
            transition_type: 1
        }
    ));

    Ok(())
}

#[test]
fn writer_rejects_designation_indexes_outside_designation_table() -> TestResult {
    let err = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 4)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .to_bytes()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::InvalidDesignationIndex {
            index: 0,
            designation_index: 4
        }
    ));

    Ok(())
}

#[test]
fn writer_rejects_transition_times_that_are_not_strictly_ascending() -> TestResult {
    let err = TzifFile::v1(DataBlock {
        transition_times: vec![0, 0],
        transition_types: vec![0, 0],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .to_bytes()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::TransitionTimesNotAscending { index: 1 }
    ));

    Ok(())
}

#[test]
fn writer_rejects_unterminated_designation_indexes() -> TestResult {
    let err = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .to_bytes()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::UnterminatedDesignation {
            index: 0,
            designation_index: 0
        }
    ));

    Ok(())
}

#[test]
fn writer_rejects_invalid_utc_offsets_and_designations() -> TestResult {
    let invalid_offset = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(i32::MIN, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .to_bytes()
    .assert_err()?;
    assert!(matches!(
        invalid_offset,
        TzifError::InvalidUtcOffset { index: 0 }
    ));

    for designations in [
        b"AB\0".to_vec(),
        b"TOO-LONG\0".to_vec(),
        b"JST!\0".to_vec(),
        vec![0xe6, 0x97, 0xa5, 0],
    ] {
        let err = TzifFile::v1(DataBlock {
            transition_times: vec![],
            transition_types: vec![],
            local_time_types: vec![ltt(0, false, 0)],
            designations,
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        })
        .to_bytes()
        .assert_err()?;
        assert!(matches!(
            err,
            TzifError::InvalidDesignation { index: 0, .. }
        ));
    }

    Ok(())
}

#[test]
fn writer_allows_placeholder_empty_designation_and_unspecified_minus_zero() -> TestResult {
    TzifFile::v1(DataBlock::placeholder())
        .to_bytes()
        .assert_ok()?;

    TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"-00\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .to_bytes()
    .assert_ok()?;

    Ok(())
}

#[test]
fn writer_rejects_ut_local_without_standard_wall() -> TestResult {
    for standard_wall_indicators in [vec![false], vec![]] {
        let err = TzifFile::v1(DataBlock {
            transition_times: vec![],
            transition_types: vec![],
            local_time_types: vec![ltt(0, false, 0)],
            designations: b"UTC\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators,
            ut_local_indicators: vec![true],
        })
        .to_bytes()
        .assert_err()?;

        assert!(matches!(
            err,
            TzifError::InvalidUtLocalIndicatorCombination { index: 0 }
        ));
    }

    Ok(())
}
