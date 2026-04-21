use crate::common::{AssertErr, AssertOk, TestResult};
use tzif_codec::{DataBlock, TzifError, TzifFile, Version};

use super::support::ltt;

#[test]
fn writer_rejects_invalid_raw_footers() -> TestResult {
    for (footer, expected) in [
        ("UTC\0", "control"),
        ("UTC\n0", "control"),
        ("日本0", "ascii"),
        ("EST5EDT,M3.2.0/-1,M11.1.0", "extension"),
        ("not a tz", "syntax"),
    ] {
        let err = TzifFile::v2(DataBlock::placeholder(), DataBlock::placeholder(), footer)
            .to_bytes()
            .assert_err()?;
        match expected {
            "control" => assert!(matches!(err, TzifError::InvalidFooterControlByte)),
            "ascii" => assert!(matches!(err, TzifError::InvalidFooterAscii)),
            "extension" => assert!(matches!(
                err,
                TzifError::FooterExtensionRequiresVersion3 {
                    version: Version::V2
                }
            )),
            "syntax" => assert!(matches!(err, TzifError::InvalidFooterSyntax)),
            _ => unreachable!(),
        }
    }

    Ok(())
}

#[test]
fn writer_accepts_raw_footer_time_boundaries() -> TestResult {
    TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock::placeholder(),
        "STD0DST,M3.2.0/24:59:59,M11.1.0/0",
    )
    .to_bytes()
    .assert_ok()?;

    TzifFile::v3(
        DataBlock::placeholder(),
        DataBlock::placeholder(),
        "STD0DST,M3.2.0/-167,M11.1.0/167",
    )
    .to_bytes()
    .assert_ok()?;

    let err = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock::placeholder(),
        "STD0DST,M3.2.0/-167,M11.1.0/167",
    )
    .to_bytes()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::FooterExtensionRequiresVersion3 {
            version: Version::V2
        }
    ));

    Ok(())
}

#[test]
fn writer_accepts_posix_footer_with_dst_rules_omitted() -> TestResult {
    TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![1_710_054_000],
            transition_types: vec![1],
            local_time_types: vec![ltt(-5 * 3600, false, 0), ltt(-4 * 3600, true, 4)],
            designations: b"EST\0EDT\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "EST5EDT",
    )
    .to_bytes()
    .assert_ok()?;

    Ok(())
}

#[test]
fn writer_accepts_rfc_footer_all_year_daylight_saving_example() -> TestResult {
    TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![1_719_792_000],
            transition_types: vec![1],
            local_time_types: vec![ltt(-3 * 3600, false, 0), ltt(-4 * 3600, true, 4)],
            designations: b"XXX\0EDT\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "XXX3EDT4,0/0,J365/23",
    )
    .to_bytes()
    .assert_ok()?;

    Ok(())
}

#[test]
fn writer_accepts_rfc_footer_version_three_extension_example() -> TestResult {
    TzifFile::v3(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![1_717_200_000],
            transition_types: vec![1],
            local_time_types: vec![ltt(-3 * 3600, false, 0), ltt(-2 * 3600, true, 4)],
            designations: b"-03\0-02\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "<-03>3<-02>,M3.5.0/-2,M10.5.0/-1",
    )
    .to_bytes()
    .assert_ok()?;

    let err = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![1_717_200_000],
            transition_types: vec![1],
            local_time_types: vec![ltt(-3 * 3600, false, 0), ltt(-2 * 3600, true, 4)],
            designations: b"-03\0-02\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "<-03>3<-02>,M3.5.0/-2,M10.5.0/-1",
    )
    .to_bytes()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::FooterExtensionRequiresVersion3 {
            version: Version::V2
        }
    ));

    Ok(())
}

#[test]
fn writer_rejects_footer_inconsistent_with_last_transition() -> TestResult {
    let err = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![1_710_054_000],
            transition_types: vec![1],
            local_time_types: vec![ltt(-5 * 3600, false, 0), ltt(-4 * 3600, true, 4)],
            designations: b"EST\0EDT\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "EST5",
    )
    .to_bytes()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::FooterInconsistentWithLastTransition
    ));

    Ok(())
}

#[test]
fn writer_rejects_footer_rule_that_does_not_match_last_transition_time() -> TestResult {
    let err = TzifFile::v2(
        DataBlock::placeholder(),
        DataBlock {
            transition_times: vec![1_704_067_200],
            transition_types: vec![1],
            local_time_types: vec![ltt(-5 * 3600, false, 0), ltt(-4 * 3600, true, 4)],
            designations: b"EST\0EDT\0".to_vec(),
            leap_seconds: vec![],
            standard_wall_indicators: vec![],
            ut_local_indicators: vec![],
        },
        "EST5EDT,M3.2.0,M11.1.0",
    )
    .to_bytes()
    .assert_err()?;

    assert!(matches!(
        err,
        TzifError::FooterInconsistentWithLastTransition
    ));

    Ok(())
}
