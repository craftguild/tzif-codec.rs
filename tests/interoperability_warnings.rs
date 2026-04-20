#![allow(
    clippy::needless_pass_by_value,
    reason = "integration tests pass owned warning fixtures"
)]

mod common;

use common::{AssertOk, TestResult};
use tzif_codec::{DataBlock, InteroperabilityWarning, LeapSecond, LocalTimeType, TzifFile};

#[test]
fn warns_when_version_one_data_omits_representable_v2_transitions() -> TestResult {
    let file = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![0],
            transition_types: vec![0],
            local_time_types: vec![ltt(0, false, 0)],
            designations: b"UTC\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "",
    );

    assert_contains(
        &file,
        InteroperabilityWarning::VersionOneDataMayBeIncomplete,
    )?;

    Ok(())
}

#[test]
fn warns_about_footer_dependent_files() -> TestResult {
    let file = TzifFile::v3(DataBlock::placeholder(), DataBlock::placeholder(), "<UTC>0");
    let warnings = file.interoperability_warnings().assert_ok()?;

    assert!(warnings
        .contains(&InteroperabilityWarning::VersionThreeOrLaterFooterMayConfuseVersionTwoReaders));
    assert!(warnings.contains(&InteroperabilityWarning::FooterMayBeIgnoredByReaders));
    assert!(warnings.contains(&InteroperabilityWarning::FooterContainsAngleBracket));

    Ok(())
}

#[test]
fn warns_about_version_four_leap_second_tables() -> TestResult {
    let mut block = DataBlock::placeholder();
    block.leap_seconds.push(LeapSecond {
        occurrence: 1_483_228_826,
        correction: 27,
    });
    let file = TzifFile::v4(DataBlock::placeholder(), block, "");

    assert_contains(
        &file,
        InteroperabilityWarning::VersionFourLeapSecondTableMayConfuseStrictRfc8536Readers,
    )?;

    Ok(())
}

#[test]
fn warns_when_first_transition_is_not_an_early_noop() -> TestResult {
    let file = TzifFile::v1(DataBlock {
        transition_times: vec![0],
        transition_types: vec![1],
        local_time_types: vec![ltt(0, false, 0), ltt(3_600, false, 4)],
        designations: b"UTC\0ONE\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    });
    let warnings = file.interoperability_warnings().assert_ok()?;

    assert!(warnings.contains(&InteroperabilityWarning::MissingEarlyNoOpTransition { block: "v1" }));
    assert!(warnings.contains(
        &InteroperabilityWarning::FirstTransitionAfterRecommendedCompatibilityPoint {
            block: "v1",
            timestamp: 0,
        }
    ));

    Ok(())
}

#[test]
fn warns_about_extreme_and_negative_transitions() -> TestResult {
    let file = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![i64::MIN, -1],
            transition_types: vec![0, 0],
            local_time_types: vec![ltt(0, false, 0)],
            designations: b"UTC\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "",
    );
    let warnings = file.interoperability_warnings().assert_ok()?;

    assert!(
        warnings.contains(&InteroperabilityWarning::MinimumI64Transition {
            block: "v2_plus",
            index: 0,
        })
    );
    assert!(warnings.contains(
        &InteroperabilityWarning::TransitionBeforeRecommendedLowerBound {
            block: "v2_plus",
            index: 0,
            timestamp: i64::MIN,
        }
    ));
    assert!(
        warnings.contains(&InteroperabilityWarning::NegativeTransition {
            block: "v2_plus",
            index: 1,
            timestamp: -1,
        })
    );

    Ok(())
}

#[test]
fn warns_about_non_portable_designations() -> TestResult {
    let file = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![ltt(0, false, 0)],
        designations: b"-00\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    });
    let warnings = file.interoperability_warnings().assert_ok()?;

    assert!(
        warnings.contains(&InteroperabilityWarning::UnspecifiedLocalTimeDesignation {
            block: "v1",
            index: 0,
        })
    );

    Ok(())
}

#[test]
fn warns_about_offsets_known_to_break_old_readers() -> TestResult {
    let file = TzifFile::v1(DataBlock {
        transition_times: vec![],
        transition_types: vec![],
        local_time_types: vec![
            ltt(14 * 3_600, false, 0),
            ltt(-30, false, 5),
            ltt(5 * 3_600 + 10 * 60, false, 9),
            ltt(3_600, false, 13),
            ltt(0, true, 17),
            ltt(94_000, false, 21),
        ],
        designations: b"LINT\0NEG\0ODD\0IST\0GMT\0BIG\0".to_vec(),
        leap_seconds: vec![LeapSecond {
            occurrence: 78_796_800,
            correction: 1,
        }],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    });
    let warnings = file.interoperability_warnings().assert_ok()?;

    assert!(
        warnings.contains(&InteroperabilityWarning::OffsetOutsideConventionalRange {
            block: "v1",
            index: 0,
            offset: 14 * 3_600,
        })
    );
    assert!(
        warnings.contains(&InteroperabilityWarning::OffsetOutsideRecommendedRange {
            block: "v1",
            index: 5,
            offset: 94_000,
        })
    );
    assert!(
        warnings.contains(&InteroperabilityWarning::NegativeSubHourOffset {
            block: "v1",
            index: 1,
            offset: -30,
        })
    );
    assert!(
        warnings.contains(&InteroperabilityWarning::OffsetNotMultipleOfMinute {
            block: "v1",
            index: 1,
            offset: -30,
        })
    );
    assert!(
        warnings.contains(&InteroperabilityWarning::OffsetNotMultipleOfQuarterHour {
            block: "v1",
            index: 2,
            offset: 5 * 3_600 + 10 * 60,
        })
    );
    assert!(
        warnings.contains(&InteroperabilityWarning::LeapSecondWithSubMinuteOffset {
            block: "v1",
            offset: -30,
        })
    );
    assert!(warnings.contains(
        &InteroperabilityWarning::DaylightOffsetLessThanStandardOffset {
            block: "v1",
            daylight_offset: 0,
            standard_offset: 94_000,
        }
    ));

    Ok(())
}

#[test]
fn warns_about_unused_local_time_types_and_designation_octets() -> TestResult {
    let file = TzifFile::v1(DataBlock {
        transition_times: vec![0],
        transition_types: vec![1],
        local_time_types: vec![ltt(0, false, 0), ltt(3_600, false, 4), ltt(7_200, false, 8)],
        designations: b"UTC\0ONE\0TWO\0XXX\0".to_vec(),
        leap_seconds: vec![],
        standard_wall_indicators: vec![],
        ut_local_indicators: vec![],
    });
    let warnings = file.interoperability_warnings().assert_ok()?;

    assert!(
        warnings.contains(&InteroperabilityWarning::UnusedLocalTimeType {
            block: "v1",
            index: 2,
        })
    );
    for index in 12..=15 {
        assert!(
            warnings
                .contains(&InteroperabilityWarning::UnusedDesignationOctet { block: "v1", index }),
            "missing unused designation octet warning for index {index}: {warnings:?}"
        );
    }

    Ok(())
}

const fn ltt(utc_offset: i32, is_dst: bool, designation_index: u8) -> LocalTimeType {
    LocalTimeType {
        utc_offset,
        is_dst,
        designation_index,
    }
}

fn assert_contains(file: &TzifFile, warning: InteroperabilityWarning) -> TestResult {
    let warnings = file.interoperability_warnings().assert_ok()?;
    assert!(
        warnings.contains(&warning),
        "missing warning {warning:?}; got {warnings:?}"
    );
    Ok(())
}
