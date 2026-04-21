use crate::common::{AssertErr, AssertOk, TestResult};
use tzif_codec::{DataBlock, LeapSecond, TzifError, TzifFile, Version};

use super::support::ltt;

fn leap_second_block(leap_seconds: Vec<LeapSecond>) -> DataBlock {
    DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds,
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    }
}

fn leap_second_file(version: Version, leap_seconds: Vec<LeapSecond>) -> TzifFile {
    let block = leap_second_block(leap_seconds);
    match version {
        Version::V3 => TzifFile::v3(DataBlock::placeholder(), block, ""),
        Version::V4 => TzifFile::v4(DataBlock::placeholder(), block, ""),
        _ => unreachable!(),
    }
}

fn assert_invalid_leap_second_table(
    version: Version,
    leap_seconds: Vec<LeapSecond>,
    expected: &TzifError,
) -> TestResult {
    let err = leap_second_file(version, leap_seconds)
        .to_bytes()
        .assert_err()?;
    assert_eq!(&err, expected);
    Ok(())
}

#[test]
fn writer_rejects_invalid_leap_second_tables() -> TestResult {
    assert_invalid_leap_second_table(
        Version::V4,
        vec![LeapSecond {
            occurrence: -1,
            correction: 1,
        }],
        &TzifError::FirstLeapSecondOccurrenceNegative { value: -1 },
    )?;
    assert_invalid_leap_second_table(
        Version::V4,
        vec![
            LeapSecond {
                occurrence: 78_796_800,
                correction: 1,
            },
            LeapSecond {
                occurrence: 78_796_800,
                correction: 2,
            },
        ],
        &TzifError::LeapSecondOccurrencesNotAscending { index: 1 },
    )?;
    assert_invalid_leap_second_table(
        Version::V3,
        vec![LeapSecond {
            occurrence: 94_694_401,
            correction: 2,
        }],
        &TzifError::LeapSecondTruncationRequiresVersion4 {
            version: Version::V3,
        },
    )?;
    assert_invalid_leap_second_table(
        Version::V3,
        vec![
            LeapSecond {
                occurrence: 78_796_800,
                correction: 1,
            },
            LeapSecond {
                occurrence: 94_694_401,
                correction: 1,
            },
        ],
        &TzifError::LeapSecondExpirationRequiresVersion4 {
            version: Version::V3,
        },
    )?;
    assert_invalid_leap_second_table(
        Version::V4,
        vec![
            LeapSecond {
                occurrence: 78_796_800,
                correction: 1,
            },
            LeapSecond {
                occurrence: 94_694_401,
                correction: 3,
            },
        ],
        &TzifError::InvalidLeapSecondCorrection { index: 1 },
    )?;
    Ok(())
}

#[test]
fn writer_rejects_leap_second_occurrences_outside_utc_month_end() -> TestResult {
    let err = leap_second_file(
        Version::V4,
        vec![LeapSecond {
            occurrence: 10,
            correction: 1,
        }],
    )
    .to_bytes()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::LeapSecondOccurrenceNotAtMonthEnd { index: 0 }
    ));

    Ok(())
}

#[test]
fn writer_accepts_negative_leap_second_at_utc_month_end() -> TestResult {
    TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![LeapSecond {
            occurrence: 78_796_800,
            correction: -1,
        }],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    })
    .to_bytes()
    .assert_ok()?;

    Ok(())
}

#[test]
fn writer_allows_version_four_truncated_and_expiring_leap_second_tables() -> TestResult {
    leap_second_file(
        Version::V4,
        vec![
            LeapSecond {
                occurrence: 126_230_402,
                correction: 3,
            },
            LeapSecond {
                occurrence: 1_719_532_827,
                correction: 3,
            },
        ],
    )
    .to_bytes()
    .assert_ok()?;

    Ok(())
}
