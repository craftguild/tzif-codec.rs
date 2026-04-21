use crate::common::{AssertErr, AssertOk, TestResult};
use tzif_codec::{DataBlock, TzifError, TzifFile, Version};

use super::support::{ltt, v1_header};

#[test]
fn parser_rejects_truncated_counted_arrays_before_using_them() -> TestResult {
    let bytes = v1_header(0, 0, 0, 2, 1, 4);
    let err = TzifFile::parse(&bytes).assert_err()?;

    assert!(matches!(
        err,
        TzifError::UnexpectedEof {
            offset: 44,
            context: "data block"
        }
    ));

    Ok(())
}

#[test]
fn parser_rejects_invalid_header_magic_and_version() -> TestResult {
    let mut invalid_magic = v1_header(0, 0, 0, 0, 1, 4);
    invalid_magic[0] = b'X';
    let err = TzifFile::parse(&invalid_magic).assert_err()?;
    assert!(matches!(err, TzifError::InvalidMagic { offset: 0 }));

    let mut invalid_version = v1_header(0, 0, 0, 0, 1, 4);
    invalid_version[4] = b'5';
    let err = TzifFile::parse(&invalid_version).assert_err()?;
    assert!(matches!(err, TzifError::InvalidVersion(b'5')));

    Ok(())
}

#[test]
fn parser_rejects_version_two_plus_header_version_mismatch() -> TestResult {
    let mut bytes = TzifFile::v2(DataBlock::placeholder(), DataBlock::placeholder(), "")
        .to_bytes()
        .assert_ok()?;
    let second_header = bytes
        .windows(4)
        .enumerate()
        .filter_map(|(index, value)| (value == b"TZif").then_some(index))
        .nth(1)
        .assert_ok()?;
    bytes[second_header + 4] = b'3';

    let err = TzifFile::parse(&bytes).assert_err()?;
    assert!(matches!(
        err,
        TzifError::VersionMismatch {
            first: Version::V2,
            second: Version::V3
        }
    ));

    Ok(())
}

#[test]
fn parser_rejects_transition_type_indexes_outside_local_time_types() -> TestResult {
    let bytes = TzifFile::v1(DataBlock {
        transition_times: vec![0],
        transition_types: vec![0],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .to_bytes()
    .assert_ok()?;
    let transition_type_offset = 44 + 4;
    let mut invalid = bytes;
    invalid[transition_type_offset] = 1;

    let err = TzifFile::parse(&invalid).assert_err()?;
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
fn parser_rejects_designation_indexes_outside_designation_table() -> TestResult {
    let bytes = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .to_bytes()
    .assert_ok()?;
    let designation_index_offset = 44 + 4 + 1;
    let mut invalid = bytes;
    invalid[designation_index_offset] = 4;

    let err = TzifFile::parse(&invalid).assert_err()?;
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
fn parser_rejects_invalid_boolean_indicators() -> TestResult {
    let bytes = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![false],
        ut_local_indicators: vec![false],
    })
    .to_bytes()
    .assert_ok()?;
    let standard_wall_offset = 44 + 6 + 4;
    let mut invalid = bytes;
    invalid[standard_wall_offset] = 2;

    let err = TzifFile::parse(&invalid).assert_err()?;
    assert!(matches!(
        err,
        TzifError::InvalidBooleanIndicator {
            field: "standard_wall_indicators",
            index: 0,
            value: 2
        }
    ));

    Ok(())
}
