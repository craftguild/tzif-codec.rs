use crate::common::{AssertErr, AssertOk, TestResult};
use tzif_codec::{DataBlock, LeapSecond, TzifError, TzifFile, Version};

use super::support::ltt;

#[test]
fn writer_rejects_invalid_leap_second_tables() -> TestResult {
    let cases = [
        (
            vec![LeapSecond {
                occurrence: -1,
                correction: 1,
            }],
            "negative-first",
            Version::V4,
        ),
        (
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
            "not-ascending",
            Version::V4,
        ),
        (
            vec![LeapSecond {
                occurrence: 94_694_401,
                correction: 2,
            }],
            "truncated-v3",
            Version::V3,
        ),
        (
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
            "expiration-v3",
            Version::V3,
        ),
        (
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
            "bad-delta",
            Version::V4,
        ),
    ];

    for (leap_seconds, expected, version) in cases {
        let block = DataBlock {
            transition_times: vec![],
            transition_types: vec![],
            local_time_types: vec![ltt(0, false, 0)],
            designations: b"UTC\0".to_vec(),
            leap_seconds,
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        };
        let file = match version {
            Version::V3 => TzifFile::v3(DataBlock::placeholder(), block, ""),
            Version::V4 => TzifFile::v4(DataBlock::placeholder(), block, ""),
            _ => unreachable!(),
        };
        let err = file.to_bytes().assert_err()?;
        match expected {
            "negative-first" => assert!(matches!(
                err,
                TzifError::FirstLeapSecondOccurrenceNegative { value: -1 }
            )),
            "not-ascending" => assert!(matches!(
                err,
                TzifError::LeapSecondOccurrencesNotAscending { index: 1 }
            )),
            "truncated-v3" => assert!(matches!(
                err,
                TzifError::LeapSecondTruncationRequiresVersion4 {
                    version: Version::V3
                }
            )),
            "expiration-v3" => assert!(matches!(
                err,
                TzifError::LeapSecondExpirationRequiresVersion4 {
                    version: Version::V3
                }
            )),
            "bad-delta" => assert!(matches!(
                err,
                TzifError::InvalidLeapSecondCorrection { index: 1 }
            )),
            _ => unreachable!(),
        }
    }

    Ok(())
}

#[test]
fn writer_rejects_leap_second_occurrences_outside_utc_month_end() -> TestResult {
    let err = TzifFile::v4(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![],
            transition_types: vec![],
            local_time_types: vec![ltt(0, false, 0)],
            designations: b"UTC\0".to_vec(),
            leap_seconds: vec![LeapSecond {
                occurrence: 10,
                correction: 1,
            }],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "",
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
    let block = DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"UTC\0".to_vec(),
        leap_seconds: vec![
            LeapSecond {
                occurrence: 126_230_402,
                correction: 3,
            },
            LeapSecond {
                occurrence: 1_719_532_827,
                correction: 3,
            },
        ],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    };

    TzifFile::v4(DataBlock::placeholder(), block, "")
        .to_bytes()
        .assert_ok()?;

    Ok(())
}
